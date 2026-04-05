use crate::config::{BurnAutoDiff, BurnDevice, Cfg};
use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;
use crate::ppo::metrics::PpoLosses;
use crate::ppo::model::{ACTIONS, Actor, Input};
use indicatif::ProgressBar;
use burn::module::AutodiffModule;
use burn::optim::{GradientsParams, Optimizer};
use burn::prelude::{Bool, Int};
use burn::tensor::activation::log_softmax;
use burn::tensor::{Tensor, TensorData};

pub(crate) mod advantages;
pub(crate) mod checkpoint;
pub(crate) mod episode;
pub(crate) mod logging;
pub(crate) mod metrics;
pub(crate) mod model;
pub(crate) mod returns;
pub(crate) mod step;
pub(crate) mod tokens;

/// Per-step data for the PPO update: the exact inputs seen during rollout, the action
/// taken, and the return/advantage computed afterwards.
pub(crate) struct BatchStep {
    /// 68-dim input: [base_features (34) | delta_features (34)] at state s_t.
    pub(crate) features: Vec<f32>,
    /// Action sequence fed to the model at step t: [Start, a_0, ..., a_{t-1}].
    pub(crate) action_seq: Vec<i64>,
    /// Index of the taken action in ACTIONS (used to slice log-probs).
    pub(crate) taken_action_idx: usize,
    pub(crate) old_log_prob: f32,
    pub(crate) ret: f32,
    /// One-step lookahead speedups for all ACTIONS from the pre-action IR state.
    /// Available when cfg.lookahead_benchmark is enabled; None otherwise.
    pub(crate) lookahead: Option<[f32; 29]>,
}

/// Flat collection of all steps across all episodes in one epoch.
pub(crate) struct Batch {
    pub(crate) steps: Vec<BatchStep>,
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

    /// Flatten all episode results + pre-computed returns/advantages into a Batch.
    /// For step t, the model input is actions[0..=t] (the growing prefix) and
    /// features = [base | delta_at_t]. No padding — each step keeps its exact sequence.
    pub(crate) fn batch(results: &[Results], returns: &[Vec<f32>]) -> Batch {
        let mut steps = Vec::new();
        for (ep, ep_rets) in results.iter().zip(returns) {
            for t in 0..ep.log_probs.len() {
                // features = base || delta at state s_t
                let features: Vec<f32> = ep
                    .base_features
                    .iter()
                    .chain(&ep.steps[t].delta_features)
                    .copied()
                    .collect();
                // actions[0] = Start prefix; actions[0..=t] is the sequence seen at step t
                let action_seq: Vec<i64> = ep.actions[0..=t].iter().map(|p| *p as i64).collect();
                // actions[t+1] is the action taken at step t
                let taken = ep.actions[t + 1];
                let taken_action_idx = ACTIONS
                    .iter()
                    .position(|&p| p == taken)
                    .expect("taken action not in ACTIONS");
                let lookahead = ep.steps[t].lookahead.as_ref().map(|la| **la);
                steps.push(BatchStep {
                    features,
                    action_seq,
                    taken_action_idx,
                    old_log_prob: ep.log_probs[t],
                    ret: ep_rets[t],
                    lookahead,
                });
            }
        }
        Batch { steps }
    }

    /// Run `ppo_epochs` gradient steps over the batch.
    /// Each epoch shuffles steps into mini-batches of `mini_batch_size`. Each mini-batch is a
    /// single padded forward pass; progress ticks once per mini-batch so the bar stays live.
    /// Returns the updated model, optimizer, and average losses across all ppo_epochs.
    pub(crate) fn update<A, O>(
        &self,
        mut model: A,
        mut optimizer: O,
        batch: &Batch,
        lr: f64,
        cfg: &Cfg,
        device: &BurnDevice,
        ppo_bar: &ProgressBar,
        advantages_impl: &dyn Advantages,
    ) -> (A, O, PpoLosses)
    where
        A: Actor<BurnAutoDiff> + AutodiffModule<BurnAutoDiff>,
        O: Optimizer<A, BurnAutoDiff>,
    {
        if batch.steps.is_empty() {
            return (model, optimizer, PpoLosses { policy_loss: 0.0, value_loss: 0.0, entropy: 0.0, kl_div: 0.0 });
        }

        let n_features = batch.steps[0].features.len();
        let num_steps = batch.steps.len();
        let num_chunks = num_steps.div_ceil(self.mini_batch_size);

        let mut sum_policy = 0.0_f32;
        let mut sum_value = 0.0_f32;
        let mut sum_entropy = 0.0_f32;
        let mut sum_kl = 0.0_f32;
        let mut update_count = 0usize;

        for ep in 0..self.ppo_epochs {
            for (chunk_idx, chunk) in batch.steps.chunks(self.mini_batch_size).enumerate() {
                let n = chunk.len();
                let max_seq = chunk.iter().map(|s| s.action_seq.len()).max().unwrap_or(1);

                // Features [n, n_features]
                let features_data: Vec<f32> = chunk.iter()
                    .flat_map(|s| s.features.iter().copied())
                    .collect();

                // Padded action sequences [n, max_seq] and mask [n, max_seq+1]
                let mut actions_data: Vec<i64> = Vec::with_capacity(n * max_seq);
                let mut mask_data: Vec<i64> = Vec::with_capacity(n * (max_seq + 1));
                let mut action_lens: Vec<usize> = Vec::with_capacity(n);
                for step in chunk {
                    let sl = step.action_seq.len();
                    action_lens.push(sl);
                    actions_data.extend(step.action_seq.iter().copied());
                    for _ in sl..max_seq { actions_data.push(0); }
                    mask_data.push(0i64); // IR token: always real
                    for _ in 0..sl { mask_data.push(0i64); }
                    for _ in sl..max_seq { mask_data.push(1i64); }
                }

                let features = Tensor::<BurnAutoDiff, 2>::from_data(
                    TensorData::new(features_data, [n, n_features]),
                    device,
                );
                let actions = Tensor::<BurnAutoDiff, 2, Int>::from_data(
                    TensorData::new(actions_data, [n, max_seq]),
                    device,
                );
                let mask_pad: Tensor<BurnAutoDiff, 2, Bool> =
                    Tensor::<BurnAutoDiff, 2, Int>::from_data(
                        TensorData::new(mask_data, [n, max_seq + 1]),
                        device,
                    )
                    .equal_elem(1i64);

                let input = Input {
                    features,
                    actions,
                    mask_pad: Some(mask_pad),
                    action_lens: Some(action_lens),
                };
                let output = model.forward(cfg, input);

                // policy: [n, 1, num_actions] → [n, num_actions]
                let logits = output.policy.flatten::<2>(0, 1);
                // value: [n, 1] → [n]
                let pred_v = output.value.flatten::<1>(0, 1);

                // Per-step log-prob of the taken action.
                let log_probs_all = log_softmax(logits.clone(), 1); // [n, num_actions]
                let taken_idx_data: Vec<i64> = chunk.iter()
                    .map(|s| s.taken_action_idx as i64)
                    .collect();
                let taken_idx = Tensor::<BurnAutoDiff, 2, Int>::from_data(
                    TensorData::new(taken_idx_data, [n, 1]),
                    device,
                );
                let new_log_probs = log_probs_all.gather(1, taken_idx).flatten::<1>(0, 1); // [n]

                let old_lp_data: Vec<f32> = chunk.iter().map(|s| s.old_log_prob).collect();
                let ret_data: Vec<f32> = chunk.iter().map(|s| s.ret).collect();

                let old_log_probs = Tensor::<BurnAutoDiff, 1>::from_data(
                    TensorData::new(old_lp_data, [n]),
                    device,
                );
                let targets = Tensor::<BurnAutoDiff, 1>::from_data(
                    TensorData::new(ret_data, [n]),
                    device,
                );

                // Advantages recomputed from current V each PPO epoch via the
                // implementor — baseline stays fresh as V improves.
                let pred_v_f32: Vec<f32> = pred_v.clone().detach().into_data().to_vec::<f32>().unwrap();
                let adv_data = advantages_impl.compute_live(chunk, &pred_v_f32);
                let advantages = Tensor::<BurnAutoDiff, 1>::from_data(
                    TensorData::new(adv_data, [n]),
                    device,
                );

                // Approx KL: mean(old_lp − new_lp). Detached — diagnostic only.
                let approx_kl = (old_log_probs.clone() - new_log_probs.clone())
                    .mean()
                    .into_scalar();

                // Policy loss [n]: clipped surrogate (negated, to minimise).
                let ratio = (new_log_probs - old_log_probs).exp();
                let a = ratio.clone() * advantages.clone();
                let b = ratio.clamp(1.0 - self.clip_epsilon, 1.0 + self.clip_epsilon) * advantages;
                let diff = (a.clone() - b.clone()).abs();
                let policy_loss = -((a + b - diff) / 2.0); // [n]

                // Value loss [n]: MSE.
                let d = pred_v - targets;
                let value_loss = d.clone() * d; // [n]

                // Entropy [n]: H(π) = -Σ p log p, summed over action dim.
                let log_p = log_softmax(logits, 1);
                let p = log_p.clone().exp();
                let entropy = -(p * log_p).sum_dim(1).flatten::<1>(0, 1); // [n]

                // Extract scalars before backward — weighted by chunk size so the
                // tail chunk (which may be smaller than mini_batch_size) doesn't
                // count the same as a full chunk in the per-epoch averages.
                let p_mean = policy_loss.clone().mean().into_scalar();
                let v_mean = (value_loss.clone() * self.value_coef).mean().into_scalar();
                let e_mean = entropy.clone().mean().into_scalar();
                sum_policy += p_mean * n as f32;
                sum_value += v_mean * n as f32;
                sum_entropy += e_mean * n as f32;
                sum_kl += approx_kl * n as f32;
                update_count += n;

                let total = (policy_loss + value_loss * self.value_coef
                    - entropy * self.entropy_coef)
                    .mean();
                let grads = total.backward();
                let grads = GradientsParams::from_grads(grads, &model);
                model = optimizer.step(lr, model, grads);

                ppo_bar.set_message(format!(
                    "ep {}/{} mb {}/{} loss={:.4}",
                    ep + 1,
                    self.ppo_epochs,
                    chunk_idx + 1,
                    num_chunks,
                    p_mean + v_mean - e_mean * self.entropy_coef,
                ));
                ppo_bar.inc(1);
            }
        }

        let n = update_count.max(1) as f32;
        let losses = PpoLosses {
            policy_loss: sum_policy / n,
            value_loss: sum_value / n,
            entropy: sum_entropy / n,
            kl_div: sum_kl / n,
        };
        (model, optimizer, losses)
    }
}
