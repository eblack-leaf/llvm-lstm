use crate::config::{BurnAutoDiff, BurnDevice, Cfg};
use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;
use crate::ppo::metrics::PpoLosses;
use crate::ppo::model::{ACTIONS, AutoActor};
use burn::module::AutodiffModule;
use burn::optim::{GradientsParams, Optimizer};
use burn::tensor::activation::log_softmax;
use burn::tensor::{Tensor, TensorData};
use indicatif::ProgressBar;

pub(crate) mod advantages;
pub(crate) mod checkpoint;
pub(crate) mod episode;
pub(crate) mod logging;
pub(crate) mod metrics;
pub(crate) mod model;
pub(crate) mod noop;
pub(crate) mod returns;

pub(crate) struct Ppo {
    clip_epsilon: f32,
    value_coef: f32,
    entropy_coef: f32,
    ppo_epochs: usize,
    mini_batch_size: usize,
    /// Stop remaining inner epochs if per-minibatch KL exceeds this (0 = disabled).
    kl_target: f32,
}

impl Ppo {
    pub(crate) fn new(cfg: &Cfg) -> Self {
        Self {
            clip_epsilon: cfg.clip_epsilon,
            value_coef: cfg.value_coef,
            entropy_coef: cfg.entropy_coef,
            ppo_epochs: cfg.ppo_epochs,
            mini_batch_size: cfg.mini_batch_size,
            kl_target: cfg.kl_target,
        }
    }

    /// PPO update for autoregressive models (Auto-TFX and Auto-GRU).
    ///
    /// Uses `AutoActor::replay_batch` to process the whole mini-batch in one call —
    /// one batched GRU forward (GRU) or K batched transformer calls (TFX).
    pub(crate) fn update_auto<A, O>(
        &self,
        mut model: A,
        mut optimizer: O,
        results: &[Results],
        returns: &[Vec<f32>],
        advantages: &[Vec<f32>],
        lr: f64,
        device: &BurnDevice,
        ppo_bar: &ProgressBar,
    ) -> (A, O, PpoLosses)
    where
        A: AutoActor<BurnAutoDiff> + AutodiffModule<BurnAutoDiff>,
        O: Optimizer<A, BurnAutoDiff>,
    {
        if results.is_empty() {
            return (model, optimizer, PpoLosses::zero());
        }

        let num_episodes = results.len();
        let num_chunks = num_episodes.div_ceil(self.mini_batch_size);

        let mut sum_policy = 0.0_f32;
        let mut sum_value = 0.0_f32;
        let mut sum_entropy = 0.0_f32;
        let mut sum_kl = 0.0_f32;
        let mut sum_clip_frac = 0.0_f32;
        let mut total_chunks = 0usize;

        'outer: for ppo_ep in 0..self.ppo_epochs {
            for chunk_idx in 0..num_chunks {
                let start = chunk_idx * self.mini_batch_size;
                let end = (start + self.mini_batch_size).min(num_episodes);
                let chunk_size = end - start;

                // ── Build flat CPU-side data for the chunk ────────────────────
                let mut old_lp_data: Vec<f32> = Vec::new();
                let mut taken_data: Vec<i64> = Vec::new();
                let mut adv_data: Vec<f32> = Vec::new();
                let mut target_data: Vec<f32> = Vec::new();
                // Per-step weights: 1 / (chunk_size * ep_len), averaging first within
                // episodes then across them.
                let mut weight_data: Vec<f32> = Vec::new();
                let mut action_idx_vecs: Vec<Vec<usize>> = Vec::with_capacity(chunk_size);

                for ep_idx in start..end {
                    let r = &results[ep_idx];
                    let ep_rets = &returns[ep_idx];
                    let ep_advs = &advantages[ep_idx];
                    let ep_len = r.ep_len;
                    let w = 1.0 / (chunk_size as f32 * ep_len as f32);

                    let indices: Vec<usize> = r
                        .actions
                        .iter()
                        .map(|&p| ACTIONS.iter().position(|&a| a == p).expect("in ACTIONS"))
                        .collect();

                    for t in 0..ep_len {
                        old_lp_data.push(r.log_probs[t]);
                        taken_data.push(indices[t] as i64);
                        adv_data.push(ep_advs[t]);
                        target_data.push(ep_rets[t]);
                        weight_data.push(w);
                    }
                    action_idx_vecs.push(indices);
                }

                let total_steps = old_lp_data.len();

                // ── One batched replay call ───────────────────────────────────
                let batch_ir: Vec<&[Vec<f32>]> = (start..end)
                    .map(|i| results[i].ir_features_per_step.as_slice())
                    .collect();
                let batch_acts: Vec<&[usize]> =
                    action_idx_vecs.iter().map(|v| v.as_slice()).collect();

                let (logits_flat, values_flat) = model.replay_batch(&batch_ir, &batch_acts, device);

                // ── Build tensors ─────────────────────────────────────────────
                let old_lp = Tensor::<BurnAutoDiff, 1>::from_data(
                    TensorData::new(old_lp_data, [total_steps]),
                    device,
                );
                let taken_idx = Tensor::<BurnAutoDiff, 1, burn::prelude::Int>::from_data(
                    TensorData::new(taken_data, [total_steps]),
                    device,
                );
                let adv = Tensor::<BurnAutoDiff, 1>::from_data(
                    TensorData::new(adv_data, [total_steps]),
                    device,
                );
                let targets = Tensor::<BurnAutoDiff, 1>::from_data(
                    TensorData::new(target_data, [total_steps]),
                    device,
                );
                let step_weights = Tensor::<BurnAutoDiff, 1>::from_data(
                    TensorData::new(weight_data, [total_steps]),
                    device,
                );

                // ── PPO clipped surrogate loss ────────────────────────────────
                let log_probs_all = log_softmax(logits_flat, 1);
                let new_log_probs = log_probs_all
                    .clone()
                    .gather(1, taken_idx.reshape([total_steps, 1]))
                    .squeeze::<1>();

                let ratio = (new_log_probs.clone() - old_lp.clone()).exp();
                let clipped = ratio
                    .clone()
                    .clamp(1.0 - self.clip_epsilon, 1.0 + self.clip_epsilon);
                let surr1 = ratio.clone() * adv.clone();
                let surr2 = clipped * adv;
                let min_surr = (surr1.clone() + surr2.clone() - (surr1 - surr2).abs()) / 2.0;

                let lo = 1.0_f32 - self.clip_epsilon;
                let hi = 1.0_f32 + self.clip_epsilon;
                let chunk_clip_frac = ratio
                    .detach()
                    .into_data()
                    .to_vec::<f32>()
                    .map(|v| {
                        v.iter().filter(|&&r| r < lo || r > hi).count() as f32
                            / v.len().max(1) as f32
                    })
                    .unwrap_or(0.0);

                let diff = values_flat - targets;
                let entropy_per_step = -(log_probs_all.clone().exp() * log_probs_all)
                    .sum_dim(1)
                    .squeeze::<1>();

                let policy_loss = -(min_surr * step_weights.clone()).sum();
                let value_loss = (diff.clone() * diff * step_weights.clone()).sum();
                let entropy = (entropy_per_step * step_weights).sum();

                let total_loss = policy_loss.clone() + value_loss.clone() * self.value_coef
                    - entropy.clone() * self.entropy_coef;

                // ── Metrics ───────────────────────────────────────────────────
                let p_m = policy_loss.clone().into_scalar();
                let v_m = value_loss.into_scalar() * self.value_coef;
                let e_m = entropy.into_scalar();
                let kl_m = (old_lp - new_log_probs.detach()).mean().into_scalar();

                // ── Backward + step ───────────────────────────────────────────
                let grads = total_loss.backward();
                let grads = GradientsParams::from_grads(grads, &model);
                model = optimizer.step(lr, model, grads);

                sum_policy += p_m;
                sum_value += v_m;
                sum_entropy += e_m;
                sum_kl += kl_m;
                sum_clip_frac += chunk_clip_frac;
                total_chunks += 1;

                if self.kl_target > 0.0 && kl_m > self.kl_target && ppo_ep > 0 {
                    ppo_bar
                        .set_message(format!("KL {kl_m:.3} > {:.3} — early stop", self.kl_target));
                    ppo_bar.finish_and_clear();
                    break 'outer;
                }

                ppo_bar.set_message(format!(
                    "ep {}/{} mb {}/{} loss={:.4}",
                    ppo_ep + 1,
                    self.ppo_epochs,
                    chunk_idx + 1,
                    num_chunks,
                    p_m + v_m - e_m * self.entropy_coef,
                ));
                ppo_bar.inc(1);
            }
        }

        let n = total_chunks.max(1) as f32;
        (
            model,
            optimizer,
            PpoLosses {
                policy_loss: sum_policy / n,
                value_loss: sum_value / n,
                entropy: sum_entropy / n,
                kl_div: sum_kl / n,
                clip_frac: sum_clip_frac / n,
            },
        )
    }
}
