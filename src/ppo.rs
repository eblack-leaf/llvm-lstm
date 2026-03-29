use burn::backend::{Autodiff, NdArray, ndarray::NdArrayDevice};
use burn::config::Config;
use burn::optim::{GradientsParams, Optimizer};
use burn::prelude::ElementConversion;
use burn::tensor::{Int, Tensor, TensorData, activation};
use burn::tensor::backend::AutodiffBackend;

use crate::actor_critic::ActorCritic;
use crate::actor_critic_tfx::TransformerActorCritic;
use crate::rollout::Rollout;

type B = Autodiff<NdArray>;

#[derive(Config, Debug)]
pub struct PpoConfig {
    /// Clipped surrogate objective epsilon.
    #[config(default = 0.2)]
    pub clip_epsilon: f32,
    /// Weight of the value function loss term.
    #[config(default = 0.5)]
    pub value_loss_coef: f32,
    /// Entropy bonus coefficient — encourages exploration.
    #[config(default = 0.03)]
    pub entropy_coef: f32,
    /// Adam learning rate.
    #[config(default = 1e-4)]
    pub learning_rate: f64,
    /// Discount factor.
    #[config(default = 0.99)]
    pub gamma: f32,
    /// GAE lambda for advantage estimation.
    /// Low lambda = TD-like (low variance, higher bias) — better when terminal
    /// reward dominates and the value function has a long horizon to predict.
    /// High lambda = MC-like (high variance, lower bias).
    #[config(default = 0.8)]
    pub gae_lambda: f32,
    /// Number of PPO epochs per rollout batch.
    #[config(default = 3)]
    pub num_epochs: usize,
    /// Stop updates when approx KL exceeds this threshold.
    #[config(default = 0.15)]
    pub target_kl: f32,
}

#[derive(Debug, Clone, Default)]
pub struct PpoStats {
    pub policy_loss: f32,
    pub value_loss: f32,
    pub entropy: f32,
    pub approx_kl: f32,
    pub clip_fraction: f32,
}

/// PPO update over one full rollout batch.
///
/// Actor and critic share a GRU trunk; a single forward pass produces both
/// logits and values, and a single combined backward pass updates all weights.
/// The combined loss is `policy_loss - entropy_coef*entropy + value_loss_coef*value_loss`.
/// Updates are gated by `target_kl`: if the approximate KL divergence from the
/// old policy exceeds the threshold, the epoch is skipped entirely.
pub fn ppo_update<O>(
    mut model: ActorCritic<B>,
    optim: &mut O,
    rollout: &Rollout,
    advantages: &[f32],
    returns: &[f32],
    config: &PpoConfig,
    device: &NdArrayDevice,
) -> (ActorCritic<B>, PpoStats)
where
    O: Optimizer<ActorCritic<B>, B>,
{
    let n = rollout.len();
    let feat_dim = rollout.states[0].len();
    let episodes = episode_ranges(&rollout.dones);
    let mut stats = PpoStats::default();

    for epoch in 0..config.num_epochs {
        // ── Build padded episode buffers for batched GRU forward ──────────────
        let n_ep  = episodes.len();
        let max_t = episodes.iter().map(|r| r.len()).max().unwrap_or(1);

        let mut feat_buf = vec![0.0f32; n_ep * max_t * feat_dim];
        let mut prev_buf = vec![0i64;   n_ep * max_t];
        // For each rollout step, its row in the flat [n_ep * max_t] layout.
        let mut real_idx: Vec<i64> = Vec::with_capacity(n);

        for (ei, range) in episodes.iter().enumerate() {
            for (t, state) in rollout.states[range.clone()].iter().enumerate() {
                let fi = (ei * max_t + t) * feat_dim;
                feat_buf[fi..fi + feat_dim].copy_from_slice(state);
                real_idx.push((ei * max_t + t) as i64);
            }
            // prev_actions[ep][t] = action[t-1], already 0 at t=0 from vec init
            for (t, &a) in rollout.actions[range.clone()].iter().enumerate() {
                if t + 1 < range.len() {
                    prev_buf[ei * max_t + t + 1] = a as i64;
                }
            }
        }

        let features_pad = Tensor::<B, 3>::from_data(
            TensorData::new(feat_buf, [n_ep, max_t, feat_dim]),
            device,
        );
        let prev_pad = Tensor::<B, 2, Int>::from_data(
            TensorData::new(prev_buf, [n_ep, max_t]),
            device,
        );

        // ── Single forward pass — shared trunk returns logits + values ─────────
        let (logits_3d, values_3d) = model.forward_batch(features_pad, prev_pad);

        let n_act       = logits_3d.shape().dims[2];
        let logits_flat = logits_3d.reshape([n_ep * max_t, n_act]);
        let values_flat = values_3d.reshape([n_ep * max_t, 1]);

        let idx = Tensor::<B, 1, Int>::from_data(TensorData::new(real_idx, [n]), device);
        let logits = logits_flat.gather(0, idx.clone().unsqueeze_dim::<2>(1).expand([n, n_act]));
        let values = values_flat
            .gather(0, idx.unsqueeze_dim::<2>(1).expand([n, 1]))
            .reshape([n]);

        let log_probs_all = activation::log_softmax(logits.clone(), 1);
        let probs_all     = activation::softmax(logits, 1);

        // Gather log_prob at the taken action for each step
        let action_idx = Tensor::<B, 2, Int>::from_data(
            TensorData::new(
                rollout.actions.iter().map(|&a| a as i64).collect::<Vec<_>>(),
                [n, 1],
            ),
            device,
        );
        let log_probs_new = log_probs_all.clone().gather(1, action_idx).reshape([n]);

        // Entropy: H = -Σ_a p(a)*log_p(a), mean over batch
        let entropy = -(probs_all * log_probs_all).sum_dim(1).reshape([n]).mean();

        // Old log probs stored during collection
        let log_probs_old = Tensor::<B, 1>::from_data(
            TensorData::new(rollout.log_probs.clone(), [n]),
            device,
        );

        // Scale advantages — broadcast per-episode (G0 − EMA) values from training.rs.
        // Divide by RMS to standardize gradient scale across batches.
        let adv = Tensor::<B, 1>::from_data(
            TensorData::new(advantages.to_vec(), [n]),
            device,
        );
        let adv_std = adv.clone().powf_scalar(2.0f32).mean().sqrt();
        let adv = adv / (adv_std + 1e-8);

        // Probability ratio and clipped surrogate objective
        let log_ratio = log_probs_new - log_probs_old;
        let ratio     = log_ratio.clone().exp();

        // Extract ratio values for clip_fraction on the final epoch.
        let ratio_vec: Vec<f32> = if epoch + 1 == config.num_epochs {
            ratio.clone().into_data().to_vec().unwrap_or_default()
        } else {
            Vec::new()
        };

        let clipped = ratio.clone().clamp(
            1.0 - config.clip_epsilon,
            1.0 + config.clip_epsilon,
        );
        // min(ratio*A, clipped*A) via (a + b - |a - b|) / 2
        let obj1 = ratio   * adv.clone();
        let obj2 = clipped * adv;
        let policy_loss =
            -((obj1.clone() + obj2.clone() - (obj1 - obj2).abs()) / 2.0).mean();

        let ret = Tensor::<B, 1>::from_data(
            TensorData::new(returns.to_vec(), [n]),
            device,
        );
        let value_loss = (values - ret).powf_scalar(2.0f32).mean();

        // Compute KL for early stopping and (on final epoch) stats
        let log_ratio_vec: Vec<f32> = log_ratio.into_data().to_vec().unwrap_or_default();
        let approx_kl_now: f32 = log_ratio_vec.iter().map(|&x| -x).sum::<f32>() / n as f32;

        // Save stats from final epoch
        if epoch + 1 == config.num_epochs {
            stats.policy_loss = policy_loss.clone().into_scalar().elem();
            stats.value_loss  = value_loss.clone().into_scalar().elem();
            stats.entropy     = entropy.clone().into_scalar().elem();
            stats.approx_kl   = approx_kl_now;

            stats.clip_fraction = ratio_vec
                .iter()
                .filter(|&&r| (r - 1.0).abs() > config.clip_epsilon)
                .count() as f32
                / ratio_vec.len().max(1) as f32;
        }

        if approx_kl_now.abs() <= config.target_kl {
            // Combined loss: policy gradient + entropy bonus + value function
            let total_loss = policy_loss
                - entropy.mul_scalar(config.entropy_coef)
                + value_loss.mul_scalar(config.value_loss_coef);
            let grads = total_loss.backward();
            let grads = GradientsParams::from_grads(grads, &model);
            model = optim.step(config.learning_rate, model, grads);
        }
    }

    (model, stats)
}

/// PPO update for the Transformer actor-critic — identical algorithm, typed for
/// `TransformerActorCritic`. Generic over the backend so it works with both the
/// NdArray CPU backend and the Wgpu GPU backend.
pub fn ppo_update_tfx<Bx, O>(
    mut model: TransformerActorCritic<Bx>,
    optim: &mut O,
    rollout: &Rollout,
    advantages: &[f32],
    returns: &[f32],
    config: &PpoConfig,
    device: &Bx::Device,
) -> (TransformerActorCritic<Bx>, PpoStats)
where
    Bx: AutodiffBackend,
    O: Optimizer<TransformerActorCritic<Bx>, Bx>,
{
    let n = rollout.len();
    let feat_dim = rollout.states[0].len();
    let episodes = episode_ranges(&rollout.dones);
    let mut stats = PpoStats::default();

    for epoch in 0..config.num_epochs {
        let n_ep  = episodes.len();
        let max_t = episodes.iter().map(|r| r.len()).max().unwrap_or(1);

        let mut feat_buf = vec![0.0f32; n_ep * max_t * feat_dim];
        let mut prev_buf = vec![0i64;   n_ep * max_t];
        let mut real_idx: Vec<i64> = Vec::with_capacity(n);

        for (ei, range) in episodes.iter().enumerate() {
            for (t, state) in rollout.states[range.clone()].iter().enumerate() {
                let fi = (ei * max_t + t) * feat_dim;
                feat_buf[fi..fi + feat_dim].copy_from_slice(state);
                real_idx.push((ei * max_t + t) as i64);
            }
            for (t, &a) in rollout.actions[range.clone()].iter().enumerate() {
                if t + 1 < range.len() {
                    prev_buf[ei * max_t + t + 1] = a as i64;
                }
            }
        }

        let features_pad = Tensor::<Bx, 3>::from_data(
            TensorData::new(feat_buf, [n_ep, max_t, feat_dim]),
            device,
        );
        let prev_pad = Tensor::<Bx, 2, Int>::from_data(
            TensorData::new(prev_buf, [n_ep, max_t]),
            device,
        );

        let (logits_3d, values_3d) = model.forward_batch(features_pad, prev_pad);

        let n_act       = logits_3d.shape().dims[2];
        let logits_flat = logits_3d.reshape([n_ep * max_t, n_act]);
        let values_flat = values_3d.reshape([n_ep * max_t, 1]);

        let idx = Tensor::<Bx, 1, Int>::from_data(TensorData::new(real_idx, [n]), device);
        let logits = logits_flat.gather(0, idx.clone().unsqueeze_dim::<2>(1).expand([n, n_act]));
        let values = values_flat
            .gather(0, idx.unsqueeze_dim::<2>(1).expand([n, 1]))
            .reshape([n]);

        let log_probs_all = activation::log_softmax(logits.clone(), 1);
        let probs_all     = activation::softmax(logits, 1);

        let action_idx = Tensor::<Bx, 2, Int>::from_data(
            TensorData::new(
                rollout.actions.iter().map(|&a| a as i64).collect::<Vec<_>>(),
                [n, 1],
            ),
            device,
        );
        let log_probs_new = log_probs_all.clone().gather(1, action_idx).reshape([n]);

        let entropy = -(probs_all * log_probs_all).sum_dim(1).reshape([n]).mean();

        let log_probs_old = Tensor::<Bx, 1>::from_data(
            TensorData::new(rollout.log_probs.clone(), [n]),
            device,
        );

        let adv = Tensor::<Bx, 1>::from_data(
            TensorData::new(advantages.to_vec(), [n]),
            device,
        );
        let adv_std = adv.clone().powf_scalar(2.0f32).mean().sqrt();
        let adv = adv / (adv_std + 1e-8);

        let log_ratio = log_probs_new - log_probs_old;
        let ratio     = log_ratio.clone().exp();

        let ratio_vec: Vec<f32> = if epoch + 1 == config.num_epochs {
            ratio.clone().into_data().to_vec().unwrap_or_default()
        } else {
            Vec::new()
        };

        let clipped = ratio.clone().clamp(
            1.0 - config.clip_epsilon,
            1.0 + config.clip_epsilon,
        );
        let obj1 = ratio   * adv.clone();
        let obj2 = clipped * adv;
        let policy_loss =
            -((obj1.clone() + obj2.clone() - (obj1 - obj2).abs()) / 2.0).mean();

        let ret = Tensor::<Bx, 1>::from_data(
            TensorData::new(returns.to_vec(), [n]),
            device,
        );
        let value_loss = (values - ret).powf_scalar(2.0f32).mean();

        let log_ratio_vec: Vec<f32> = log_ratio.into_data().to_vec().unwrap_or_default();
        let approx_kl_now: f32 = log_ratio_vec.iter().map(|&x| -x).sum::<f32>() / n as f32;

        if epoch + 1 == config.num_epochs {
            stats.policy_loss = policy_loss.clone().into_scalar().elem();
            stats.value_loss  = value_loss.clone().into_scalar().elem();
            stats.entropy     = entropy.clone().into_scalar().elem();
            stats.approx_kl   = approx_kl_now;
            stats.clip_fraction = ratio_vec
                .iter()
                .filter(|&&r| (r - 1.0).abs() > config.clip_epsilon)
                .count() as f32
                / ratio_vec.len().max(1) as f32;
        }

        if approx_kl_now.abs() <= config.target_kl {
            let total_loss = policy_loss
                - entropy.mul_scalar(config.entropy_coef)
                + value_loss.mul_scalar(config.value_loss_coef);
            let grads = total_loss.backward();
            let grads = GradientsParams::from_grads(grads, &model);
            model = optim.step(config.learning_rate, model, grads);
        }
    }

    (model, stats)
}

/// Returns index ranges for each episode in the flat rollout buffer.
/// Episodes end at steps where `done == true`.
fn episode_ranges(dones: &[bool]) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for (i, &done) in dones.iter().enumerate() {
        if done {
            ranges.push(start..i + 1);
            start = i + 1;
        }
    }
    // Partial episode at the end (shouldn't occur when collection always runs to done)
    if start < dones.len() {
        ranges.push(start..dones.len());
    }
    ranges
}
