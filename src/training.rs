use anyhow::Result;
use burn::backend::{Autodiff, NdArray, ndarray::NdArrayDevice};
use burn::config::Config;
use burn::module::AutodiffModule;
use burn::optim::AdamConfig;
use burn::prelude::ElementConversion;
use burn::tensor::{Int, Tensor, TensorData};
use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::actor_critic::{Actor, ActorConfig, Critic, CriticConfig};
use crate::env::{EnvConfig, LlvmEnv};
use crate::ppo::{PpoConfig, ppo_update};
use crate::rollout::Rollout;

/// The autodiff-enabled backend used for training.
type B = Autodiff<NdArray>;

#[derive(Config, Debug)]
pub struct TrainConfig {
    /// Environment settings (benchmark paths, episode length, reward mode).
    pub env: EnvConfig,
    /// PPO hyperparameters.
    #[config(default = "PpoConfig::new()")]
    pub ppo: PpoConfig,
    /// Total number of rollout-collect + PPO-update iterations.
    #[config(default = 1000)]
    pub total_iterations: usize,
    /// Number of complete episodes to collect per iteration (one per env cycle).
    /// Each episode runs until done, so advantages are always clean.
    #[config(default = 6)]
    pub episodes_per_iteration: usize,
    /// Run full evaluation every N iterations.
    #[config(default = 50)]
    pub eval_interval: usize,
    /// Directory to write model checkpoints.
    pub checkpoint_dir: String,
    /// Print training stats every N iterations.
    #[config(default = 10)]
    pub log_interval: usize,
}

pub fn train(config: TrainConfig) -> Result<()> {
    let device = NdArrayDevice::default();
    let mut rng = StdRng::from_entropy();

    // Single env — cycles through all benchmark functions round-robin.
    // Baselines are computed once up front (slow, but only done once).
    let mut env = LlvmEnv::new(config.env)?;
    eprintln!("Computing baselines for all benchmark functions...");
    env.compute_baselines()?;
    eprintln!("Baselines ready. Starting training.");

    let mut actor  = ActorConfig::new().init::<B>(&device);
    let mut critic = CriticConfig::new().init::<B>(&device);
    let mut actor_optim  = AdamConfig::new().init::<B, Actor<B>>();
    let mut critic_optim = AdamConfig::new().init::<B, Critic<B>>();

    for iteration in 0..config.total_iterations {
        // ── Collect episodes ──────────────────────────────────────────────────
        //
        // Each episode runs to completion (done=true) in its own Rollout.
        // .valid() gives the NdArray (non-autodiff) model — no graph built.
        //
        let mut rollouts: Vec<Rollout> = Vec::new();

        for _ in 0..config.episodes_per_iteration {
            let actor_inf  = actor.valid();
            let critic_inf = critic.valid();
            let mut rollout = Rollout::new();
            let mut state = env.reset()?;
            let mut hidden: Option<Tensor<NdArray, 2>> = None;
            let mut prev_action: i64 = 0;

            loop {
                let features = Tensor::<NdArray, 2>::from_data(
                    TensorData::new(state.features.clone(), [1, state.features.len()]),
                    &device,
                );
                let prev_act = Tensor::<NdArray, 1, Int>::from_data(
                    TensorData::new(vec![prev_action], [1]),
                    &device,
                );

                let (logits, new_hidden) = actor_inf.forward(features.clone(), prev_act, hidden);
                let value_scalar: f32 = critic_inf
                    .forward(features)
                    .into_scalar()
                    .elem();

                let logits_vec: Vec<f32> = logits.into_data().to_vec()?;
                let action   = sample_categorical(&logits_vec, &mut rng);
                let log_prob = log_softmax_at(&logits_vec, action);

                let step = env.step(action)?;

                rollout.push(
                    state.features.clone(),
                    action,
                    log_prob,
                    step.reward,
                    value_scalar,
                    step.done,
                );

                if step.done {
                    break;
                }

                hidden = Some(new_hidden);
                prev_action = action as i64;
                state = step.state;
            }

            rollouts.push(rollout);
        }

        // ── Compute advantages per episode, then flatten ──────────────────────
        //
        // last_value=0.0 is always correct here because every episode ends with
        // done=true — there is no future reward to bootstrap from.
        //
        let mut all_advantages: Vec<f32> = Vec::new();
        let mut all_returns: Vec<f32> = Vec::new();

        for rollout in &rollouts {
            let (adv, ret) =
                rollout.compute_advantages(config.ppo.gamma, config.ppo.gae_lambda, 0.0);
            all_advantages.extend(adv);
            all_returns.extend(ret);
        }

        let combined = Rollout::merge(&rollouts);

        // ── PPO update ────────────────────────────────────────────────────────
        let stats;
        (actor, critic, stats) = ppo_update(
            actor,
            critic,
            &mut actor_optim,
            &mut critic_optim,
            &combined,
            &all_advantages,
            &all_returns,
            &config.ppo,
            &device,
        );

        // ── Logging ───────────────────────────────────────────────────────────
        if iteration % config.log_interval == 0 {
            eprintln!(
                "[{iteration:>4}] steps={:>4}  policy={:.4}  value={:.4}  entropy={:.4}  kl={:.4}",
                combined.len(),
                stats.policy_loss,
                stats.value_loss,
                stats.entropy,
                stats.approx_kl,
            );
        }

        // ── Checkpoint ────────────────────────────────────────────────────────
        if iteration % config.eval_interval == 0 && iteration > 0 {
            // TODO: actor.save_file / critic.save_file
        }
    }

    Ok(())
}

/// Sample an action index from a categorical distribution defined by `logits`.
fn sample_categorical(logits: &[f32], rng: &mut impl Rng) -> usize {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp: Vec<f32> = logits.iter().map(|x| (x - max).exp()).collect();
    let sum: f32 = exp.iter().sum();

    let u: f32 = rng.r#gen();
    let mut cumsum = 0.0f32;
    for (i, e) in exp.iter().enumerate() {
        cumsum += e / sum;
        if u <= cumsum {
            return i;
        }
    }
    logits.len() - 1 // fallback for floating-point edge cases
}

/// Log-probability of `action` under the softmax distribution defined by `logits`.
///
/// Uses the log-sum-exp trick for numerical stability.
fn log_softmax_at(logits: &[f32], action: usize) -> f32 {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let log_sum_exp = logits.iter().map(|x| (x - max).exp()).sum::<f32>().ln() + max;
    logits[action] - log_sum_exp
}
