use burn::config::Config;
use burn::optim::{GradientsParams, Optimizer};
use burn::prelude::ElementConversion;
use burn::tensor::{Int, Tensor, TensorData, activation};
use burn::tensor::backend::AutodiffBackend;

use crate::actor_critic_tfx::TransformerActorCritic;
use crate::rollout::Rollout;

#[derive(Config, Debug)]
pub struct PpoConfig {
    /// Clipped surrogate objective epsilon.
    #[config(default = 0.2)]
    pub clip_epsilon: f32,
    /// Entropy bonus coefficient — encourages exploration.
    #[config(default = 0.005)]
    pub entropy_coef: f32,
    /// Adam learning rate.
    #[config(default = 1e-4)]
    pub learning_rate: f64,
    /// Discount factor.
    #[config(default = 0.99)]
    pub gamma: f32,
    /// GAE lambda for advantage estimation.
    /// Must stay high (≥0.95) when the terminal benchmark reward dominates g0:
    /// with T=40 steps the terminal reward weight at step 0 is (γλ)^39, so
    /// lambda=0.8 → 0.06% weight (signal gone), lambda=0.97 → 21% weight.
    #[config(default = 0.97)]
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

/// PPO update for the Transformer actor (pure policy, no value head).
///
/// Generic over backend so it works with both NdArray (CPU) and Wgpu (GPU).
/// Always uses `forward_batch` with base-IR features — per-step IR mode removed.
pub fn ppo_update_tfx<Bx, O>(
    mut model: TransformerActorCritic<Bx>,
    optim: &mut O,
    rollout: &Rollout,
    advantages: &[f32],
    config: &PpoConfig,
    device: &Bx::Device,
) -> (TransformerActorCritic<Bx>, PpoStats)
where
    Bx: AutodiffBackend,
    O: Optimizer<TransformerActorCritic<Bx>, Bx>,
{
    let n        = rollout.len();
    let feat_dim = rollout.states[0].len();
    let episodes = episode_ranges(&rollout.dones);
    let mut stats = PpoStats::default();

    for epoch in 0..config.num_epochs {
        let n_ep  = episodes.len();
        let max_t = episodes.iter().map(|r| r.len()).max().unwrap_or(1);

        let mut prev_buf = vec![0i64; n_ep * max_t];
        let mut real_idx: Vec<i64> = Vec::with_capacity(n);

        for (ei, range) in episodes.iter().enumerate() {
            for t in 0..range.len() {
                real_idx.push((ei * max_t + t) as i64);
            }
            for (t, &a) in rollout.actions[range.clone()].iter().enumerate() {
                if t + 1 < range.len() {
                    prev_buf[ei * max_t + t + 1] = a as i64;
                }
            }
        }

        let prev_pad = Tensor::<Bx, 2, Int>::from_data(
            TensorData::new(prev_buf, [n_ep, max_t]),
            device,
        );

        // Base features: one vector per episode from the episode's first state.
        // Works for both "base" (34-d) and "base+current" (68-d) ir_modes.
        let mut base_feat_buf = vec![0.0f32; n_ep * feat_dim];
        for (ei, range) in episodes.iter().enumerate() {
            base_feat_buf[ei * feat_dim..(ei + 1) * feat_dim]
                .copy_from_slice(&rollout.states[range.start]);
        }
        let base_features = Tensor::<Bx, 2>::from_data(
            TensorData::new(base_feat_buf, [n_ep, feat_dim]),
            device,
        );

        let logits_3d   = model.forward_batch(base_features, prev_pad);
        let n_act       = logits_3d.shape().dims[2];
        let logits_flat = logits_3d.reshape([n_ep * max_t, n_act]);

        let idx    = Tensor::<Bx, 1, Int>::from_data(TensorData::new(real_idx, [n]), device);
        let logits = logits_flat.gather(0, idx.unsqueeze_dim::<2>(1).expand([n, n_act]));

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
        let entropy       = -(probs_all * log_probs_all).sum_dim(1).reshape([n]).mean();

        let log_probs_old = Tensor::<Bx, 1>::from_data(
            TensorData::new(rollout.log_probs.clone(), [n]),
            device,
        );

        // Advantages are pre-normalised by build_advantages; re-normalise here
        // to handle any residual mean introduced by the weighting pass.
        let adv      = Tensor::<Bx, 1>::from_data(TensorData::new(advantages.to_vec(), [n]), device);
        let adv_mean = adv.clone().mean();
        let adv_std  = (adv.clone() - adv_mean.clone()).powf_scalar(2.0f32).mean().sqrt();
        let adv      = (adv - adv_mean) / (adv_std + 1e-8);

        let log_ratio = log_probs_new - log_probs_old;
        let ratio     = log_ratio.clone().exp();

        let ratio_vec: Vec<f32> = if epoch + 1 == config.num_epochs {
            ratio.clone().into_data().to_vec().unwrap_or_default()
        } else {
            Vec::new()
        };

        let clipped = ratio.clone().clamp(1.0 - config.clip_epsilon, 1.0 + config.clip_epsilon);
        let obj1 = ratio   * adv.clone();
        let obj2 = clipped * adv;
        let policy_loss = -((obj1.clone() + obj2.clone() - (obj1 - obj2).abs()) / 2.0).mean();

        let log_ratio_vec: Vec<f32> = log_ratio.into_data().to_vec().unwrap_or_default();
        let approx_kl_now: f32 = log_ratio_vec.iter().map(|&x| -x).sum::<f32>() / n as f32;

        if epoch + 1 == config.num_epochs {
            stats.policy_loss   = policy_loss.clone().into_scalar().elem();
            stats.value_loss    = 0.0;
            stats.entropy       = entropy.clone().into_scalar().elem();
            stats.approx_kl     = approx_kl_now;
            stats.clip_fraction = ratio_vec
                .iter()
                .filter(|&&r| (r - 1.0).abs() > config.clip_epsilon)
                .count() as f32
                / ratio_vec.len().max(1) as f32;
        }

        if approx_kl_now.abs() <= config.target_kl {
            let total_loss = policy_loss - entropy.mul_scalar(config.entropy_coef);
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
