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

    // Previous logged stats for computing deltas.
    let mut prev_ev:  Option<f32> = None;
    let mut prev_ent: Option<f32> = None;
    let mut prev_kl:  Option<f32> = None;

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
        let base_work_dir  = &worker_work_dir;
        // Clear the IR cache each iteration — stochastic exploration means cross-
        // iteration cache hits are near-zero after the first few steps, and the
        // cache grows unboundedly otherwise.  Within-iteration parallel workers
        // still benefit from sharing early-step IR.
        let ir_cache_dir = worker_work_dir.join("ir_cache");
        if ir_cache_dir.exists() {
            std::fs::remove_dir_all(&ir_cache_dir).ok();
        }
        std::fs::create_dir_all(&ir_cache_dir).ok();

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

                let mut worker_env = LlvmEnv::new_with_baselines(worker_config, baselines.clone())?
                    .with_ir_cache(ir_cache_dir.clone());
                let mut state      = worker_env.reset_to(func_index)?;
                let func_name      = worker_env.current_function_name().unwrap_or_else(|| "?".into());

                let device  = NdArrayDevice::default();
                let mut rng = rand::thread_rng();

                let mut rollout        = Rollout::new();
                let mut hidden: Option<Tensor<NdArray, 2>>        = None;
                let mut critic_hidden: Option<Tensor<NdArray, 2>> = None;
                let mut prev_action: i64 = 0;
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
                        actor_s.forward(features.clone(), prev_act.clone(), hidden);
                    let (value_tensor, new_critic_hidden) =
                        critic_s.forward(features, prev_act, critic_hidden);
                    let value_scalar: f32 = value_tensor
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
                        ep_pb.inc(1);
                        break;
                    }

                    hidden        = Some(new_hidden);
                    critic_hidden = Some(new_critic_hidden);
                    prev_action   = action as i64;
                    state         = step.state;
                }

                Ok((rollout, func_name, episode_reward))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        // Collect rollouts and update per-function EMA (sequential — no races).
        let mut rollouts: Vec<Rollout> = Vec::with_capacity(config.episodes_per_iteration);
        let mut rollout_funcs: Vec<String> = Vec::with_capacity(config.episodes_per_iteration);
        for (rollout, func_name, episode_reward) in episode_results {
            let ema = fn_ema.entry(func_name.clone()).or_insert(episode_reward);
            *ema = 0.9 * *ema + 0.1 * episode_reward;
            rollout_funcs.push(func_name);
            rollouts.push(rollout);
        }

        // ── Compute advantages ────────────────────────────────────────────────
        step_pb.set_message("computing advantages");
        let mut raw_advantages: Vec<Vec<f32>> = Vec::new();
        let mut all_returns: Vec<f32> = Vec::new();

        for rollout in &rollouts {
            let (adv, ret) =
                rollout.compute_advantages(config.ppo.gamma, config.ppo.gae_lambda, 0.0);
            raw_advantages.push(adv);
            all_returns.extend(ret);
        }

        let mut fn_adv_sum: HashMap<String, f32> = HashMap::new();
        let mut fn_adv_cnt: HashMap<String, usize> = HashMap::new();
        for (adv, func) in raw_advantages.iter().zip(rollout_funcs.iter()) {
            *fn_adv_sum.entry(func.clone()).or_insert(0.0) += adv.iter().sum::<f32>();
            *fn_adv_cnt.entry(func.clone()).or_insert(0)   += adv.len();
        }

        let mut all_advantages: Vec<f32> = Vec::new();
        for (adv, func) in raw_advantages.iter().zip(rollout_funcs.iter()) {
            let mean = fn_adv_sum[func] / fn_adv_cnt[func] as f32;
            all_advantages.extend(adv.iter().map(|&a| a - mean));
        }

        let combined = Rollout::merge(&rollouts);

        // ── PPO update ────────────────────────────────────────────────────────
        step_pb.set_message("ppo update");
        // Compute explained variance before the update (old values vs. raw returns).
        let ev = explained_variance(&all_returns, &combined.values);

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

        let iter_secs = iter_start.elapsed().as_secs_f32();
        train_pb.inc(1);
        train_pb.set_message(format!("last {iter_secs:.0}s"));
        step_pb.set_message("—");

        // ── Logging ───────────────────────────────────────────────────────────
        if iteration % config.log_interval == 0 {
            let ent_frac = stats.entropy / max_entropy;
            let kl       = stats.approx_kl;
            let clip     = stats.clip_fraction;

            // Delta indicators vs previous log point (↑↓ with sign).
            let delta = |cur: f32, prev: Option<f32>| -> String {
                match prev {
                    None => "     ".to_string(),
                    Some(p) => {
                        let d = cur - p;
                        if d.abs() < 0.005 { "     ".to_string() }
                        else if d > 0.0    { format!(" \x1b[32m↑{:.2}\x1b[0m", d.abs()) }
                        else               { format!(" \x1b[31m↓{:.2}\x1b[0m", d.abs()) }
                    }
                }
            };

            // Color a value based on healthy/warning/bad thresholds.
            // good_range: (lo, hi) where the value is "healthy".
            let color = |s: String, good: bool, warn: bool| -> String {
                if good      { format!("\x1b[32m{s}\x1b[0m") }   // green
                else if warn { format!("\x1b[33m{s}\x1b[0m") }   // yellow
                else         { format!("\x1b[31m{s}\x1b[0m") }   // red
            };

            let ev_s = color(
                format!("{ev:+.2}"),
                ev > 0.3,
                ev > 0.0,
            );
            let ent_s = color(
                format!("{ent_frac:.2}"),
                ent_frac > 0.4,
                ent_frac > 0.2,
            );
            let kl_s = color(
                format!("{kl:.4}"),
                kl < 0.05,
                kl < 0.1,
            );
            let clip_s = color(
                format!("{clip:.2}"),
                clip < 0.3,
                clip < 0.5,
            );

            let dev  = delta(ev,       prev_ev);
            let dent = delta(ent_frac, prev_ent);
            let dkl  = delta(kl,       prev_kl);

            // Per-function EMA summary: color positive green, negative red.
            let mut fn_summary: Vec<(&str, f32)> = fn_ema
                .iter()
                .map(|(k, &v)| (k.as_str(), v))
                .collect();
            fn_summary.sort_by_key(|(k, _)| *k);
            train_pb.println(format!(
                "  [{iteration:>4}] steps={:>4}  ev={ev_s}{dev}  ent={ent_s}{dent}  kl={kl_s}{dkl}  clip={clip_s}",
                combined.len(),
            ));
            for (k, v) in &fn_summary {
                let s = format!("         {k:>24} = {v:+.3}");
                let colored = if *v > 0.05       { format!("\x1b[32m{s}\x1b[0m") }
                              else if *v > -0.05 { format!("\x1b[33m{s}\x1b[0m") }
                              else               { format!("\x1b[31m{s}\x1b[0m") };
                train_pb.println(colored);
            }

            prev_ev  = Some(ev);
            prev_ent = Some(ent_frac);
            prev_kl  = Some(kl);
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

/// Explained variance: 1 - Var(returns - values) / Var(returns).
/// Returns 0 when Var(returns) is near zero (constant target).
fn explained_variance(returns: &[f32], values: &[f32]) -> f32 {
    let n = returns.len() as f32;
    let ret_mean = returns.iter().sum::<f32>() / n;
    let ret_var  = returns.iter().map(|r| (r - ret_mean).powi(2)).sum::<f32>() / n;
    if ret_var < 1e-8 {
        return 0.0;
    }
    let residuals: Vec<f32> = returns.iter().zip(values.iter()).map(|(r, v)| r - v).collect();
    let res_mean = residuals.iter().sum::<f32>() / n;
    let res_var  = residuals.iter().map(|r| (r - res_mean).powi(2)).sum::<f32>() / n;
    1.0 - res_var / ret_var
}
