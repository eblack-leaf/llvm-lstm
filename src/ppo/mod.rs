use crate::config::{BurnAutoDiff, BurnDevice, Cfg};
use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;
use crate::ppo::flat_batch::FlatBatch;
use crate::ppo::metrics::PpoLosses;
use crate::ppo::model::{ACTIONS, Actor, Input};
use burn::backend::Autodiff;
use burn::module::AutodiffModule;
use burn::optim::{GradientsParams, Optimizer};
use burn::prelude::{Backend, Int};
use burn::tensor::activation::log_softmax;
use burn::tensor::{Tensor, TensorData};
use indicatif::ProgressBar;

pub(crate) mod advantages;
pub(crate) mod checkpoint;
pub(crate) mod episode;
mod flat_batch;
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
    pub(crate) fn batch(
        results: &[Results],
        returns: &[Vec<f32>],
        advantages: &[Vec<f32>],
    ) -> Batch {
        let episodes = results
            .iter()
            .zip(returns)
            .zip(advantages)
            .map(|((ep, ep_rets), ep_advs)| {
                let episode_return = ep_rets.first().copied().unwrap_or(0.0);
                let steps = ep
                    .actions
                    .iter()
                    .enumerate()
                    .map(|(t, &action)| {
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
                    })
                    .collect();
                BatchEpisode {
                    ir_features: ep.ir_features.clone(),
                    steps,
                    episode_return,
                }
            })
            .collect();
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
            return (model, optimizer, PpoLosses::zero());
        }

        // Build flat batch once
        let flat_batch = FlatBatch::from_episodes(&batch.episodes, device);
        let max_k = flat_batch.ir_features.dims()[1];
        let n_features = flat_batch.ir_features.dims()[2];

        // Local helper closures for gathering steps
        let gather_3d = |x: &Tensor<BurnAutoDiff, 3>,
                         gather: &Tensor<BurnAutoDiff, 2, Int>|
         -> Tensor<BurnAutoDiff, 2> {
            let flat = x.clone().flatten::<2>(0, 1);
            let total = gather.dims()[0];
            let idx = gather
                .clone()
                .slice([0..total, 0..1])
                .mul_scalar(max_k as i64)
                .add(gather.clone().slice([0..total, 1..2]))
                .reshape([total]);
            flat.select(0, idx)
        };
        let gather_2d = |x: &Tensor<BurnAutoDiff, 2>,
                         gather: &Tensor<BurnAutoDiff, 2, Int>|
         -> Tensor<BurnAutoDiff, 1> {
            let flat = x.clone().flatten::<1>(0, 1);
            let total = gather.dims()[0];
            let idx = gather
                .clone()
                .slice([0..total, 0..1])
                .mul_scalar(max_k as i64)
                .add(gather.clone().slice([0..total, 1..2]))
                .reshape([total]);
            flat.select(0, idx)
        };

        // Precompute episode boundaries (global step indices)
        let episode_boundaries: Vec<(usize, usize)> = {
            let mut b = Vec::with_capacity(batch.episodes.len());
            let mut cur = 0;
            for ep in &batch.episodes {
                let start = cur;
                let end = cur + ep.steps.len();
                b.push((start, end));
                cur = end;
            }
            b
        };

        let num_episodes = batch.episodes.len();
        let num_chunks = num_episodes.div_ceil(self.mini_batch_size);

        let mut sum_policy = 0.0_f32;
        let mut sum_value = 0.0_f32;
        let mut sum_entropy = 0.0_f32;
        let mut sum_kl = 0.0_f32;
        let mut total_steps_processed = 0usize;

        for ppo_ep in 0..self.ppo_epochs {
            for chunk_idx in 0..num_chunks {
                let start_ep = chunk_idx * self.mini_batch_size;
                let end_ep = (start_ep + self.mini_batch_size).min(num_episodes);
                let step_start = episode_boundaries[start_ep].0;
                let step_end = episode_boundaries[end_ep - 1].1;
                let chunk_num_steps = step_end - step_start;

                // Slice IR features
                let chunk_ir = flat_batch.ir_features.clone().slice([
                    start_ep..end_ep,
                    0..max_k,
                    0..n_features,
                ]);

                // Slice flat tensors
                let chunk_gather = flat_batch
                    .gather_indices
                    .clone()
                    .slice([step_start..step_end, 0..2]);
                let chunk_taken = flat_batch.taken_idx.clone().slice([step_start..step_end]);
                let chunk_old_lp = flat_batch
                    .old_log_probs
                    .clone()
                    .slice([step_start..step_end]);
                let chunk_adv = flat_batch.advantages.clone().slice([step_start..step_end]);
                let chunk_targets = flat_batch.targets.clone().slice([step_start..step_end]);

                // Forward pass
                let output = model.forward(
                    cfg,
                    Input {
                        ir_features: chunk_ir,
                    },
                );
                let policy_logits = output.policy.squeeze::<3>();
                let values = output.value.squeeze::<2>();

                // Gather valid steps
                let step_logits = gather_3d(&policy_logits, &chunk_gather);
                let step_values = gather_2d(&values, &chunk_gather);

                // Compute log probs
                let log_probs_all = log_softmax(step_logits, 1);
                let new_log_probs = log_probs_all
                    .clone()
                    .gather(1, chunk_taken.clone().reshape([chunk_taken.dims()[0], 1]))
                    .squeeze::<1>();

                // PPO clipped policy loss components
                let ratio = (new_log_probs.clone() - chunk_old_lp.clone()).exp();
                let clipped_ratio = ratio
                    .clone()
                    .clamp(1.0 - self.clip_epsilon, 1.0 + self.clip_epsilon);
                let surr1 = ratio * chunk_adv.clone();
                let surr2 = clipped_ratio * chunk_adv;
                let min_surr = (surr1.clone() + surr2.clone() - (surr1 - surr2).abs()) / 2.0;

                // Value loss component
                let diff = step_values - chunk_targets;

                // Entropy component
                let p = log_probs_all.clone().exp();
                let neg_entropy_per_step = (p * log_probs_all).sum_dim(1).squeeze::<1>(); // [total_steps]

                // ---------- Episode‑wise weighting ----------
                // Build weight tensor: for each step, weight = 1 / (num_episodes * K_ep)
                let mut step_weights = vec![0.0_f32; chunk_num_steps];
                for (ep_idx, &(g_start, g_end)) in episode_boundaries.iter().enumerate() {
                    if g_start >= step_end || g_end <= step_start {
                        continue;
                    }
                    let local_start = g_start.saturating_sub(step_start);
                    let local_end = g_end.saturating_sub(step_start);
                    let ep_len = g_end - g_start;
                    let w = 1.0 / (num_episodes as f32 * ep_len as f32);
                    for t in local_start..local_end {
                        step_weights[t] = w;
                    }
                }
                let step_weights_tensor = Tensor::<BurnAutoDiff, 1>::from_data(
                    TensorData::new(step_weights, [chunk_num_steps]),
                    device,
                );

                // Weighted losses (sum, because weights already include 1/num_episodes and 1/ep_len)
                let policy_loss = -(min_surr * step_weights_tensor.clone()).sum();
                let value_loss = ((diff.clone() * diff) * step_weights_tensor.clone()).sum();
                let entropy = (-neg_entropy_per_step * step_weights_tensor).sum();

                let total_loss = policy_loss.clone() + value_loss.clone() * self.value_coef
                    - entropy.clone() * self.entropy_coef;

                // Backward and optimizer step
                let grads = total_loss.backward();
                let grads = GradientsParams::from_grads(grads, &model);
                model = optimizer.step(lr, model, grads);

                // Metrics (use the same weighted values, detached)
                let steps_f = chunk_num_steps as f32;
                let policy_metric = policy_loss.clone().into_scalar();
                let value_metric = value_loss.clone().into_scalar();
                let entropy_metric = entropy.clone().into_scalar();
                let kl_metric = (chunk_old_lp - new_log_probs.detach()).mean().into_scalar();

                sum_policy += policy_metric * steps_f;
                sum_value += value_metric * self.value_coef * steps_f;
                sum_entropy += entropy_metric * steps_f;
                sum_kl += kl_metric * steps_f;
                total_steps_processed += chunk_num_steps;

                ppo_bar.set_message(format!(
                    "ep {}/{} mb {}/{} loss={:.4}",
                    ppo_ep + 1,
                    self.ppo_epochs,
                    chunk_idx + 1,
                    num_chunks,
                    policy_metric + self.value_coef * value_metric
                        - self.entropy_coef * entropy_metric,
                ));
                ppo_bar.inc(1);
            }
        }

        let n = (total_steps_processed.max(1)) as f32;
        let losses = PpoLosses {
            policy_loss: sum_policy / n,
            value_loss: sum_value / n,
            entropy: sum_entropy / n,
            kl_div: sum_kl / n,
        };
        (model, optimizer, losses)
    }
}

/// Gather steps from a padded 3D tensor [n_episodes, max_k, features] → [total_steps, features]
fn gather_steps_3d<B: Backend>(
    x: &Tensor<B, 3>,
    gather_indices: &Tensor<B, 2, Int>,
    max_k: usize,
) -> Tensor<B, 2> {
    let flat = x.clone().flatten::<2>(0, 1); // [n_episodes * max_k, features]
    let total_steps = gather_indices.dims()[0];
    let flat_indices = gather_indices
        .clone()
        .slice([0..total_steps, 0..1])
        .mul_scalar(max_k as i64)
        .add(gather_indices.clone().slice([0..total_steps, 1..2]))
        .reshape([total_steps]);
    flat.select(0, flat_indices) // [total_steps, features]
}

/// Gather steps from a padded 2D tensor [n_episodes, max_k] → [total_steps]
fn gather_steps_2d<B: Backend>(
    x: &Tensor<B, 2>,
    gather_indices: &Tensor<B, 2, Int>,
    max_k: usize,
) -> Tensor<B, 1> {
    let flat = x.clone().flatten::<1>(0, 1); // [n_episodes * max_k]
    let total_steps = gather_indices.dims()[0];
    let flat_indices = gather_indices
        .clone()
        .slice([0..total_steps, 0..1])
        .mul_scalar(max_k as i64)
        .add(gather_indices.clone().slice([0..total_steps, 1..2]))
        .reshape([total_steps]);
    flat.select(0, flat_indices) // [total_steps]
}
