use crate::config::{BurnAutoDiff, BurnDevice, Cfg};
use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;
use crate::ppo::metrics::PpoLosses;
use crate::ppo::model::{ACTIONS, Actor, Input};
use indicatif::ProgressBar;
use burn::module::AutodiffModule;
use burn::optim::{GradientsParams, Optimizer};
use burn::prelude::Int;
use burn::tensor::activation::log_softmax;
use burn::tensor::{Tensor, TensorData};
use burn::backend::Autodiff;

pub(crate) mod advantages;
pub(crate) mod checkpoint;
pub(crate) mod episode;
pub(crate) mod logging;
pub(crate) mod metrics;
pub(crate) mod model;
pub(crate) mod returns;
pub(crate) mod step;
pub(crate) mod tokens;

/// Per-slot training data for one episode.
pub(crate) struct BatchStep {
    pub(crate) taken_action_idx: usize,
    pub(crate) old_log_prob: f32,
    pub(crate) ret: f32,
    pub(crate) advantage: f32,
}

/// All slots from one episode, grouped for the transformer forward pass.
pub(crate) struct BatchEpisode {
    pub(crate) ir_features: Vec<f32>,
    pub(crate) steps: Vec<BatchStep>,
    pub(crate) episode_return: f32,
}

/// Collection of episodes for one PPO update.
pub(crate) struct Batch {
    pub(crate) episodes: Vec<BatchEpisode>,
}

pub(crate) struct Ppo {
    clip_epsilon: f32,
    value_coef: f32,
    entropy_coef: f32,
    ppo_epochs: usize,
    mini_batch_size: usize,
}

impl Ppo {
    pub(crate) fn new(cfg: &Cfg) -> Self {
        Self {
            clip_epsilon: cfg.clip_epsilon,
            value_coef: cfg.value_coef,
            entropy_coef: cfg.entropy_coef,
            ppo_epochs: cfg.ppo_epochs,
            mini_batch_size: cfg.mini_batch_size,
        }
    }

    /// Build per-episode batch from episode results, pre-computed returns, and pre-computed advantages.
    pub(crate) fn batch(results: &[Results], returns: &[Vec<f32>], advantages: &[Vec<f32>]) -> Batch {
        let episodes = results.iter().zip(returns).zip(advantages).map(|((ep, ep_rets), ep_advs)| {
            let episode_return = ep_rets.first().copied().unwrap_or(0.0);
            let steps = ep.actions.iter().enumerate().map(|(t, &action)| {
                let taken_action_idx = ACTIONS
                    .iter()
                    .position(|&p| p == action)
                    .expect("action not in ACTIONS");
                BatchStep {
                    taken_action_idx,
                    old_log_prob: ep.log_probs[t],
                    ret: ep_rets[t],
                    advantage: ep_advs[t],
                }
            }).collect();
            BatchEpisode {
                ir_features: ep.ir_features.clone(),
                steps,
                episode_return,
            }
        }).collect();
        Batch { episodes }
    }

    /// PPO update. Mini-batches are groups of episodes. For each episode, one
    /// transformer forward is run over all K slots simultaneously (inter-slot
    /// attention preserved). Losses are accumulated across the mini-batch before
    /// a single backward + optimizer step.
    pub(crate) fn update<A, O>(
        &self,
        mut model: A,
        mut optimizer: O,
        batch: &Batch,
        lr: f64,
        cfg: &Cfg,
        device: &BurnDevice,
        ppo_bar: &ProgressBar,
    ) -> (A, O, PpoLosses)
    where
        A: Actor<BurnAutoDiff> + AutodiffModule<BurnAutoDiff>,
        O: Optimizer<A, BurnAutoDiff>,
    {
        if batch.episodes.is_empty() {
            return (model, optimizer, PpoLosses { policy_loss: 0.0, value_loss: 0.0, entropy: 0.0, kl_div: 0.0 });
        }

        let n_features = batch.episodes[0].ir_features.len();
        let num_episodes = batch.episodes.len();
        let num_chunks = num_episodes.div_ceil(self.mini_batch_size);

        let mut sum_policy  = 0.0_f32;
        let mut sum_value   = 0.0_f32;
        let mut sum_entropy = 0.0_f32;
        let mut sum_kl      = 0.0_f32;
        let mut update_count = 0usize;

        for ppo_ep in 0..self.ppo_epochs {
            for (chunk_idx, chunk) in batch.episodes.chunks(self.mini_batch_size).enumerate() {
                let total_steps: usize = chunk.iter().map(|e| e.steps.len()).sum();
                let n_episodes = chunk.len();

                // One batched forward for the whole chunk.
                // Pad all episodes to max_k slots; causal prefix-independence ensures
                // outputs at positions 0..ep_len are unaffected by padding positions.
                let max_k = chunk.iter().map(|e| e.steps.len()).max().unwrap_or(1);

                let feat_data: Vec<f32> = chunk.iter()
                    .flat_map(|ep| ep.ir_features.iter().copied().cycle().take(max_k * n_features))
                    .collect();
                let ir_features = Tensor::<BurnAutoDiff, 3>::from_data(
                    TensorData::new(feat_data, [n_episodes, max_k, n_features]),
                    device,
                );
                let output = model.forward(cfg, Input { ir_features });
                // output.policy: [N, max_k, 1, num_actions]
                // output.value:  [N, max_k, 1]

                let num_actions = ACTIONS.len();

                // Extract forward values as f32 once per chunk for metric computation.
                // Metrics never need autodiff — keeping them on f32 avoids N_eps × 4
                // unnecessary autodiff tensor clones per chunk.
                let policy_f32: Vec<f32> = output.policy.clone()
                    .into_data().convert::<f32>().to_vec().unwrap(); // [N * max_k * 1 * num_actions]
                let value_f32: Vec<f32> = output.value.clone()
                    .into_data().convert::<f32>().to_vec().unwrap();  // [N * max_k * 1]

                let mut total_loss: Option<Tensor<BurnAutoDiff, 1>> = None;

                for (ep_idx, episode) in chunk.iter().enumerate() {
                    let k = episode.steps.len();

                    // Extract this episode's outputs for slots 0..k.
                    // policy: [1, k, 1, num_actions] → [k, num_actions]
                    let logits = output.policy.clone()
                        .narrow(0, ep_idx, 1).narrow(1, 0, k)
                        .flatten::<2>(0, 2);                              // [k, num_actions]
                    // value: [1, k, 1] → [k] per-slot estimates
                    let pred_v = output.value.clone()
                        .narrow(0, ep_idx, 1).narrow(1, 0, k)
                        .flatten::<1>(0, 2);                              // [k]

                    let log_probs_all = log_softmax(logits.clone(), 1);   // [k, num_actions]

                    let taken_idx_data: Vec<i64> = episode.steps.iter()
                        .map(|s| s.taken_action_idx as i64).collect();
                    let taken_idx = Tensor::<BurnAutoDiff, 2, Int>::from_data(
                        TensorData::new(taken_idx_data.clone(), [k, 1]),
                        device,
                    );
                    let new_log_probs = log_probs_all.gather(1, taken_idx).flatten::<1>(0, 1); // [k]

                    let old_lp_data: Vec<f32> = episode.steps.iter()
                        .map(|s| s.old_log_prob).collect();
                    let old_log_probs = Tensor::<BurnAutoDiff, 1>::from_data(
                        TensorData::new(old_lp_data.clone(), [k]),
                        device,
                    );

                    // Pre-computed advantages from the rollout (implementor decides normalisation).
                    let adv_data: Vec<f32> = episode.steps.iter()
                        .map(|s| s.advantage).collect();
                    let advantages = Tensor::<BurnAutoDiff, 1>::from_data(
                        TensorData::new(adv_data.clone(), [k]),
                        device,
                    );

                    // PPO clipped policy loss
                    let ratio = (new_log_probs - old_log_probs).exp();
                    let a = ratio.clone() * advantages.clone();
                    let b = ratio.clamp(1.0 - self.clip_epsilon, 1.0 + self.clip_epsilon) * advantages;
                    let diff = (a.clone() - b.clone()).abs();
                    let policy_loss = -((a + b - diff) / 2.0).mean();    // [1]

                    // Value loss
                    let targets = Tensor::<BurnAutoDiff, 1>::from_data(
                        TensorData::new(vec![episode.episode_return; k], [k]),
                        device,
                    );
                    let d = pred_v - targets;
                    let value_loss = (d.clone() * d).mean();              // [1]

                    // Entropy
                    let log_p = log_softmax(logits, 1);
                    let p = log_p.clone().exp();
                    let entropy = -(p * log_p).sum_dim(1).flatten::<1>(0, 1).mean(); // [1]

                    // --- Metrics: pure f32, no autodiff nodes ---
                    let k_f = k as f32;
                    let ep_base = ep_idx * max_k;

                    // log-softmax over actions in f32, pick the taken action per slot
                    let new_lp_f32: Vec<f32> = (0..k).map(|t| {
                        let off = (ep_base + t) * num_actions;
                        let sl  = &policy_f32[off..off + num_actions];
                        let mx  = sl.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                        let lse = sl.iter().map(|&x| (x - mx).exp()).sum::<f32>().ln() + mx;
                        sl[taken_idx_data[t] as usize] - lse
                    }).collect();

                    let approx_kl: f32 = old_lp_data.iter().zip(&new_lp_f32)
                        .map(|(o, n)| o - n).sum::<f32>() / k_f;

                    let policy_s: f32 = new_lp_f32.iter().zip(&old_lp_data).zip(&adv_data)
                        .map(|((n, o), a)| {
                            let r = (n - o).exp();
                            let rc = r.clamp(1.0 - self.clip_epsilon, 1.0 + self.clip_epsilon);
                            -(r * a).min(rc * a)
                        }).sum::<f32>() / k_f;

                    let target = episode.episode_return;
                    let value_s: f32 = (0..k).map(|t| {
                        let v = value_f32[ep_base + t];
                        (v - target).powi(2)
                    }).sum::<f32>() / k_f;

                    let entropy_s: f32 = (0..k).map(|t| {
                        let off = (ep_base + t) * num_actions;
                        let sl  = &policy_f32[off..off + num_actions];
                        let mx  = sl.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                        let lse = sl.iter().map(|&x| (x - mx).exp()).sum::<f32>().ln() + mx;
                        -sl.iter().map(|&x| { let lp = x - lse; lp.exp() * lp }).sum::<f32>()
                    }).sum::<f32>() / k_f;

                    sum_policy  += policy_s * k_f;
                    sum_value   += value_s * self.value_coef * k_f;
                    sum_entropy += entropy_s * k_f;
                    sum_kl      += approx_kl * k_f;
                    update_count += k;

                    let ep_loss = policy_loss
                        + value_loss * self.value_coef
                        - entropy * self.entropy_coef;

                    total_loss = Some(match total_loss {
                        None       => ep_loss,
                        Some(prev) => prev + ep_loss,
                    });
                }

                if let Some(loss) = total_loss {
                    let grads = (loss / n_episodes as f32).backward();
                    let grads = GradientsParams::from_grads(grads, &model);
                    model = optimizer.step(lr, model, grads);
                }

                let avg_p = sum_policy / update_count.max(1) as f32;
                let avg_v = sum_value  / update_count.max(1) as f32;
                let avg_e = sum_entropy / update_count.max(1) as f32;
                ppo_bar.set_message(format!(
                    "ep {}/{} mb {}/{} loss={:.4}",
                    ppo_ep + 1, self.ppo_epochs,
                    chunk_idx + 1, num_chunks,
                    avg_p + avg_v - avg_e * self.entropy_coef,
                ));
                ppo_bar.inc(1);
            }
        }

        let n = update_count.max(1) as f32;
        let losses = PpoLosses {
            policy_loss: sum_policy  / n,
            value_loss:  sum_value   / n,
            entropy:     sum_entropy / n,
            kl_div:      sum_kl      / n,
        };
        (model, optimizer, losses)
    }
}
