use burn::backend::{Autodiff, NdArray, ndarray::NdArrayDevice};
use burn::config::Config;
use burn::optim::{GradientsParams, Optimizer};
use burn::prelude::ElementConversion;
use burn::tensor::{Int, Tensor, TensorData, activation};

use crate::actor_critic::{Actor, Critic};
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
    #[config(default = 0.01)]
    pub entropy_coef: f32,
    /// Adam learning rate.
    #[config(default = 3e-4)]
    pub learning_rate: f64,
    /// Discount factor.
    #[config(default = 0.99)]
    pub gamma: f32,
    /// GAE lambda for advantage estimation.
    #[config(default = 0.95)]
    pub gae_lambda: f32,
    /// Number of PPO epochs per rollout batch.
    #[config(default = 2)]
    pub num_epochs: usize,
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
/// Actor and critic are trained with separate backward passes.
///
/// For the actor, each episode is re-rolled as a full sequence through the GRU
/// (starting from hidden=None, matching collection), so log_probs_new are exact.
///
/// For the critic (MLP), all steps are processed in one flat batch — no hidden
/// state, no episode splitting needed.
pub fn ppo_update<OA, OC>(
    mut actor: Actor<B>,
    mut critic: Critic<B>,
    actor_optim: &mut OA,
    critic_optim: &mut OC,
    rollout: &Rollout,
    advantages: &[f32],
    returns: &[f32],
    config: &PpoConfig,
    device: &NdArrayDevice,
) -> (Actor<B>, Critic<B>, PpoStats)
where
    OA: Optimizer<Actor<B>, B>,
    OC: Optimizer<Critic<B>, B>,
{
    let n = rollout.len();
    let feat_dim = rollout.states[0].len();
    let episodes = episode_ranges(&rollout.dones);
    let mut stats = PpoStats::default();

    for epoch in 0..config.num_epochs {
        // ── Actor: single batched GRU forward over all episodes ───────────────
        //
        // Pad all episodes to max_T and run one GRU call instead of n_ep separate
        // calls. Padding positions are zero-filled and masked out after the forward.
        // Real steps have identical hidden states to the sequential version —
        // same inputs, same GRU from hidden=None, so gradients are exact.
        //
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

        // [n_ep, max_T, num_actions] → flatten → select real rows → [n, num_actions]
        let logits_3d = actor.forward_batch(features_pad, prev_pad);
        let n_act = logits_3d.shape().dims[2];
        let logits_flat = logits_3d.reshape([n_ep * max_t, n_act]);

        let idx = Tensor::<B, 1, Int>::from_data(TensorData::new(real_idx, [n]), device);
        let logits = logits_flat.gather(0, idx.unsqueeze_dim::<2>(1).expand([n, n_act]));

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

        // Normalize advantages
        let adv = Tensor::<B, 1>::from_data(
            TensorData::new(advantages.to_vec(), [n]),
            device,
        );
        let adv_mean = adv.clone().mean();
        let adv_std  = (adv.clone() - adv_mean.clone())
            .powf_scalar(2.0f32)
            .mean()
            .sqrt();
        let adv = (adv - adv_mean) / (adv_std + 1e-8);

        // Probability ratio and clipped surrogate objective
        let log_ratio = log_probs_new - log_probs_old;
        let ratio     = log_ratio.clone().exp();

        // Extract ratio values for clip_fraction before ratio is consumed.
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

        // ── Critic: flat batch, no episode splitting ───────────────────────────
        let flat_states: Vec<f32> = rollout.states.iter().flatten().cloned().collect();
        let features = Tensor::<B, 2>::from_data(
            TensorData::new(flat_states, [n, feat_dim]),
            device,
        );
        let values = critic.forward(features).reshape([n]); // [n, 1] → [n]
        let ret = Tensor::<B, 1>::from_data(
            TensorData::new(returns.to_vec(), [n]),
            device,
        );
        let value_loss = (values - ret).powf_scalar(2.0f32).mean();

        // Save stats from final epoch
        if epoch + 1 == config.num_epochs {
            stats.policy_loss = policy_loss.clone().into_scalar().elem();
            stats.value_loss  = value_loss.clone().into_scalar().elem();
            stats.entropy     = entropy.clone().into_scalar().elem();

            // approx KL ≈ mean(-log_ratio) = mean(log_old - log_new)
            let log_ratio_vec: Vec<f32> =
                log_ratio.into_data().to_vec().unwrap_or_default();
            stats.approx_kl = log_ratio_vec.iter().map(|&x| -x).sum::<f32>()
                / n as f32;

            stats.clip_fraction = ratio_vec
                .iter()
                .filter(|&&r| (r - 1.0).abs() > config.clip_epsilon)
                .count() as f32
                / ratio_vec.len().max(1) as f32;
        }

        // ── Actor backward ────────────────────────────────────────────────────
        let actor_loss = policy_loss - entropy.mul_scalar(config.entropy_coef);
        let actor_grads = actor_loss.backward();
        let actor_grads = GradientsParams::from_grads(actor_grads, &actor);
        actor = actor_optim.step(config.learning_rate, actor, actor_grads);

        // ── Critic backward ───────────────────────────────────────────────────
        let critic_grads = value_loss.backward();
        let critic_grads = GradientsParams::from_grads(critic_grads, &critic);
        critic = critic_optim.step(config.learning_rate, critic, critic_grads);
    }

    (actor, critic, stats)
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
