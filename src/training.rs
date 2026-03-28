use std::collections::HashMap;
use std::time::{Duration, Instant};

use rayon::prelude::*;

use anyhow::Result;
use burn::backend::{Autodiff, NdArray, ndarray::NdArrayDevice};
use burn::config::Config;
use burn::module::AutodiffModule;
use burn::optim::AdamConfig;
use burn::prelude::ElementConversion;
use burn::tensor::{Int, Tensor, TensorData};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::Rng;

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

    // Save fields needed by parallel workers — config.env is moved into LlvmEnv::new below.
    let worker_functions_dir  = config.env.functions_dir.clone();
    let worker_work_dir       = config.env.work_dir.clone();
    let worker_reward_mode    = config.env.reward_mode.clone();
    let worker_max_seq_length = config.env.max_seq_length;
    let worker_benchmark_runs = config.env.benchmark_runs;

    let mut env = LlvmEnv::new(config.env)?;
    env.compute_baselines()?;

    let mut actor        = ActorConfig::new().init::<B>(&device);
    let mut critic       = CriticConfig::new().init::<B>(&device);
    let mut actor_optim  = AdamConfig::new().init::<B, Actor<B>>();
    let mut critic_optim = AdamConfig::new().init::<B, Critic<B>>();

    // ── Progress bars ─────────────────────────────────────────────────────────
    let multi = MultiProgress::new();

    let train_pb = multi.add(ProgressBar::new(config.total_iterations as u64));
    train_pb.set_style(
        ProgressStyle::default_bar()
            .template("  training  {bar:30.green}  {pos:>4}/{len}  {elapsed}  {msg}")
            .unwrap(),
    );

    let ep_pb = multi.add(ProgressBar::new(config.episodes_per_iteration as u64));
    ep_pb.set_style(
        ProgressStyle::default_bar()
            .template("  episode   {bar:20.cyan}   {pos}/{len}  {msg}")
            .unwrap(),
    );

    let step_pb = multi.add(ProgressBar::new_spinner());
    step_pb.set_style(
        ProgressStyle::default_spinner()
            .template("  {spinner:.yellow}  {msg}")
            .unwrap(),
    );
    step_pb.enable_steady_tick(Duration::from_millis(120));

    // Exponential moving average of episode reward per function (α=0.1).
    let mut fn_ema: HashMap<String, f32> = HashMap::new();

    // Maximum entropy for the action space — used to normalise entropy to [0,1].
    let max_entropy = (ActorConfig::new().num_actions as f32).ln();

    // Pre-computed baselines shared (read-only) across parallel workers.
    let baselines = env.baselines().clone();
    let n_funcs   = env.num_functions();

    // ── Training loop ─────────────────────────────────────────────────────────
    for iteration in 0..config.total_iterations {
        let iter_start = Instant::now();
        ep_pb.set_position(0);

        // ── Collect episodes in parallel ──────────────────────────────────────
        // Each episode gets its own LlvmEnv with a unique work dir so that
        // simultaneous compilations of the same function don't clobber each other.
        let actor_inf  = actor.valid();
        let critic_inf = critic.valid();
        let base_work_dir = &worker_work_dir;

        // Actor<NdArray> contains OnceCell which is !Sync, so par_iter().map() won't
        // compile (closure needs Sync). map_with() side-steps this: the models live
        // in per-thread state (T: Send+Clone only, not Sync).
        let episode_results: Vec<(Rollout, String, f32)> = (0..config.episodes_per_iteration)
            .into_par_iter()
            .map_with(
                (actor_inf, critic_inf),
                |(actor_s, critic_s), ep| -> anyhow::Result<(Rollout, String, f32)> {
                let func_index = (iteration * config.episodes_per_iteration + ep) % n_funcs;

                // Unique scratch directory — avoids file races between parallel episodes.
                let worker_dir = base_work_dir.join(format!("worker-{ep}"));
                let worker_config = EnvConfig::new(
                    worker_functions_dir.clone(),
                    worker_dir,
                    worker_reward_mode.clone(),
                )
                .with_max_seq_length(worker_max_seq_length)
                .with_benchmark_runs(worker_benchmark_runs);

                let mut worker_env = LlvmEnv::new_with_baselines(worker_config, baselines.clone())?;
                let mut state      = worker_env.reset_to(func_index)?;
                let func_name      = worker_env.current_function_name().unwrap_or_else(|| "?".into());

                let device  = NdArrayDevice::default();
                let mut rng = rand::thread_rng();

                let mut rollout       = Rollout::new();
                let mut hidden: Option<Tensor<NdArray, 2>> = None;
                let mut prev_action: i64 = 0;
                let mut step_idx: usize  = 0;
                let mut episode_reward   = 0.0f32;

                loop {
                    let features = Tensor::<NdArray, 2>::from_data(
                        TensorData::new(state.features.clone(), [1, state.features.len()]),
                        &device,
                    );
                    let prev_act = Tensor::<NdArray, 1, Int>::from_data(
                        TensorData::new(vec![prev_action], [1]),
                        &device,
                    );

                    let (logits, new_hidden) =
                        actor_s.forward(features.clone(), prev_act, hidden);
                    let value_scalar: f32 = critic_s
                        .forward(features)
                        .reshape([1])
                        .into_scalar()
                        .elem();

                    let logits_vec: Vec<f32> = logits.into_data().to_vec()?;
                    let action   = sample_categorical(&logits_vec, &mut rng);
                    let log_prob = log_softmax_at(&logits_vec, action);

                    let step = worker_env.step(action)?;
                    episode_reward += step.reward;

                    rollout.push(
                        state.features.clone(),
                        action,
                        log_prob,
                        step.reward,
                        value_scalar,
                        step.done,
                    );

                    if step.done {
                        train_pb.println(format!(
                            "    [{func_name}]  steps={step_idx}  reward={episode_reward:+.4}",
                        ));
                        ep_pb.inc(1);
                        break;
                    }

                    hidden = Some(new_hidden);
                    prev_action = action as i64;
                    step_idx += 1;
                    state = step.state;
                }

                Ok((rollout, func_name, episode_reward))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        // Collect rollouts and update per-function EMA (sequential — no races).
        let mut rollouts: Vec<Rollout> = Vec::with_capacity(config.episodes_per_iteration);
        for (rollout, func_name, episode_reward) in episode_results {
            let ema = fn_ema.entry(func_name).or_insert(episode_reward);
            *ema = 0.9 * *ema + 0.1 * episode_reward;
            rollouts.push(rollout);
        }

        // ── Compute advantages ────────────────────────────────────────────────
        step_pb.set_message("computing advantages");
        let mut all_advantages: Vec<f32> = Vec::new();
        let mut all_returns: Vec<f32> = Vec::new();

        for rollout in &rollouts {
            let (adv, ret) =
                rollout.compute_advantages(config.ppo.gamma, config.ppo.gae_lambda, 0.0);
            all_advantages.extend(adv);
            all_returns.extend(ret);
        }

        // Normalize returns so the critic learns from a stable target distribution.
        // Without this the raw PerStep returns vary wildly in scale and the critic
        // diverges early (value loss explodes).
        let ret_mean = all_returns.iter().sum::<f32>() / all_returns.len() as f32;
        let ret_std  = (all_returns.iter()
            .map(|r| (r - ret_mean).powi(2))
            .sum::<f32>()
            / all_returns.len() as f32)
            .sqrt();
        let norm_returns: Vec<f32> = all_returns.iter()
            .map(|r| (r - ret_mean) / (ret_std + 1e-8))
            .collect();

        let combined = Rollout::merge(&rollouts);

        // ── PPO update ────────────────────────────────────────────────────────
        step_pb.set_message("ppo update");
        let stats;
        (actor, critic, stats) = ppo_update(
            actor,
            critic,
            &mut actor_optim,
            &mut critic_optim,
            &combined,
            &all_advantages,
            &norm_returns,
            &config.ppo,
            &device,
        );

        let iter_secs = iter_start.elapsed().as_secs_f32();
        train_pb.inc(1);
        train_pb.set_message(format!("last {iter_secs:.0}s"));
        step_pb.set_message("—");

        // ── Logging ───────────────────────────────────────────────────────────
        if iteration % config.log_interval == 0 {
            let ev = 1.0 - stats.value_loss;   // explained variance: 0=no skill, 1=perfect
            let ent_frac = stats.entropy / max_entropy; // 1=uniform, 0=deterministic

            // Per-function EMA summary, sorted by name
            let mut fn_summary: Vec<(&str, f32)> = fn_ema
                .iter()
                .map(|(k, &v)| (k.as_str(), v))
                .collect();
            fn_summary.sort_by_key(|(k, _)| *k);
            let fn_str: String = fn_summary
                .iter()
                .map(|(k, v)| format!("{k}={v:+.2}"))
                .collect::<Vec<_>>()
                .join("  ");

            train_pb.println(format!(
                "  [{iteration:>4}] steps={:>3}  ev={ev:+.2}  ent={ent_frac:.2}  kl={:.4}  clip={:.2}",
                combined.len(),
                stats.approx_kl,
                stats.clip_fraction,
            ));
            if !fn_str.is_empty() {
                train_pb.println(format!("         {fn_str}"));
            }
        }

        // ── Checkpoint ────────────────────────────────────────────────────────
        if iteration % config.eval_interval == 0 && iteration > 0 {
            // TODO: actor.save_file / critic.save_file
        }
    }

    train_pb.finish_with_message("complete");
    ep_pb.finish_and_clear();
    step_pb.finish_and_clear();
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
    logits.len() - 1
}

/// Log-probability of `action` under the softmax distribution defined by `logits`.
fn log_softmax_at(logits: &[f32], action: usize) -> f32 {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let log_sum_exp = logits.iter().map(|x| (x - max).exp()).sum::<f32>().ln() + max;
    logits[action] - log_sum_exp
}
