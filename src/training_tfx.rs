use std::collections::HashMap;
use std::time::{Duration, Instant};

use rayon::prelude::*;

use anyhow::Result;
use burn::backend::Autodiff;

// ── Backend selection ─────────────────────────────────────────────────────────
// Default: NdArray (CPU).  GPU: add `wgpu` to burn features in Cargo.toml and
// recompile.  No code changes needed — the type aliases below wire it through.
#[cfg(not(feature = "wgpu"))]
use burn::backend::{NdArray, ndarray::NdArrayDevice};
#[cfg(feature = "wgpu")]
use burn::backend::wgpu::{Wgpu, WgpuDevice};

#[cfg(not(feature = "wgpu"))]
type Inner = NdArray;
#[cfg(feature = "wgpu")]
type Inner = Wgpu;

#[cfg(not(feature = "wgpu"))]
type Dev = NdArrayDevice;
#[cfg(feature = "wgpu")]
type Dev = WgpuDevice;
use burn::grad_clipping::GradientClippingConfig;
use burn::module::AutodiffModule;
use burn::optim::AdamConfig;
use burn::prelude::ElementConversion;
use burn::prelude::Module as _;
use burn::record::CompactRecorder;
use burn::tensor::{Int, Tensor, TensorData};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::Rng;

use crate::actor_critic_tfx::{TransformerActorCritic, TransformerActorCriticConfig};
use crate::env::{EnvConfig, LlvmEnv, RewardBreakdown};
use crate::ppo::ppo_update_tfx;
use crate::rollout::Rollout;
use crate::training::TrainConfig;

type B = Autodiff<Inner>;

pub fn train(config: TrainConfig) -> Result<()> {
    let device = Dev::default();

    let worker_functions_dir  = config.env.functions_dir.clone();
    let worker_work_dir       = config.env.work_dir.clone();
    let worker_reward_mode    = config.env.reward_mode.clone();
    let worker_max_seq_length = config.env.max_seq_length;
    let worker_benchmark_runs = config.env.benchmark_runs;

    let mut env = LlvmEnv::new(config.env)?;
    env.compute_baselines()?;

    let mut model = TransformerActorCriticConfig::new().init::<B>(&device);
    let grad_clip = Some(GradientClippingConfig::Norm(0.5));
    let mut optim = AdamConfig::new()
        .with_grad_clipping(grad_clip)
        .init::<B, TransformerActorCritic<B>>();

    let multi   = MultiProgress::new();
    let train_pb = multi.add(ProgressBar::new(config.total_iterations as u64));
    train_pb.set_style(
        ProgressStyle::default_bar()
            .template("  training  {bar:30.green}  {pos:>4}/{len}  {elapsed}  {msg}")
            .unwrap(),
    );
    let ep_pb = multi.add(ProgressBar::new(0));
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

    let mut fn_ema: HashMap<String, f32> = HashMap::new();
    let mut best_mean_ema: f32 = 0.0;
    let mut prev_ev:  Option<f32> = None;
    let mut prev_ent: Option<f32> = None;
    let mut prev_kl:  Option<f32> = None;

    let max_entropy = (TransformerActorCriticConfig::new().num_actions as f32).ln();

    let baselines = env.baselines().clone();
    let n_funcs   = env.num_functions();

    let total_episodes = n_funcs * config.episodes_per_function;
    ep_pb.set_length(total_episodes as u64);

    for iteration in 0..config.total_iterations {
        let iter_start = Instant::now();
        ep_pb.set_position(0);

        let model_inf = model.valid();
        let base_work_dir = &worker_work_dir;

        // ── Collect episodes in parallel ──────────────────────────────────────
        // Transformer rollout: no hidden state threading. Instead each worker
        // accumulates the full (features, prev_action) sequence for the episode
        // and re-runs the Transformer over the growing sequence at each step.
        // Compute cost is O(t²) per step vs O(t) for GRU, but entirely negligible
        // compared to compilation + benchmarking time.
        let episode_results: Vec<(Rollout, String, f32, Option<RewardBreakdown>)> = (0..total_episodes)
            .into_par_iter()
            .map_with(
                model_inf,
                |model_s, ep| -> anyhow::Result<(Rollout, String, f32, Option<RewardBreakdown>)> {
                    let func_index = ep % n_funcs;

                    let worker_dir = base_work_dir.join(format!("worker-tfx-{ep}"));
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

                    let device  = Dev::default();
                    let mut rng = rand::thread_rng();

                    let mut rollout        = Rollout::new();
                    let mut episode_reward = 0.0f32;

                    // Sequence history: flat feature buffer + prev_action per step.
                    // feat_history[t * feat_dim .. (t+1) * feat_dim] = features at step t.
                    // act_history[t] = prev_action entering step t (0 for t=0).
                    let feat_dim = state.features.len();
                    let mut feat_history: Vec<f32> = Vec::new();
                    let mut act_history:  Vec<i64> = Vec::new();
                    let mut prev_action:  i64      = 0;

                    let terminal_breakdown = loop {
                        // Append current step to sequence.
                        feat_history.extend_from_slice(&state.features);
                        act_history.push(prev_action);
                        let seq_len = act_history.len();

                        let features_seq = Tensor::<Inner, 3>::from_data(
                            TensorData::new(feat_history.clone(), [1, seq_len, feat_dim]),
                            &device,
                        );
                        let actions_seq = Tensor::<Inner, 2, Int>::from_data(
                            TensorData::new(act_history.clone(), [1, seq_len]),
                            &device,
                        );

                        let (logits, value_tensor) = model_s.forward(features_seq, actions_seq);
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
                            break step.breakdown;
                        }

                        prev_action = action as i64;
                        state = step.state;
                    };

                    Ok((rollout, func_name, episode_reward, terminal_breakdown))
                },
            )
            .collect::<anyhow::Result<Vec<_>>>()?;

        // ── Collect rollouts ──────────────────────────────────────────────────
        let mut rollouts: Vec<Rollout> = Vec::with_capacity(total_episodes);
        let mut rollout_funcs: Vec<String> = Vec::with_capacity(total_episodes);
        let mut episode_g0s: Vec<f32> = Vec::with_capacity(total_episodes);
        let mut episode_v0s: Vec<f32> = Vec::with_capacity(total_episodes);
        // Per-episode baseline = fn_ema value BEFORE this iteration's episodes
        // are incorporated — avoids using the current episode in its own baseline.
        let mut episode_baselines: Vec<f32> = Vec::with_capacity(total_episodes);
        let mut fn_bd_sum: HashMap<String, (f32, f32, f32, usize)> = HashMap::new();

        for (rollout, func_name, _episode_reward, bd) in episode_results {
            let g0: f32 = rollout.rewards.iter().enumerate()
                .map(|(t, &r)| r * config.ppo.gamma.powi(t as i32))
                .sum();
            let v0 = rollout.values.first().copied().unwrap_or(0.0);
            episode_g0s.push(g0);
            episode_v0s.push(v0);
            // Snapshot baseline before updating EMA so each episode is compared
            // against the function's historical average, not the current batch.
            let baseline = fn_ema.get(&func_name).copied().unwrap_or(g0);
            episode_baselines.push(baseline);
            let ema = fn_ema.entry(func_name.clone()).or_insert(g0);
            *ema = 0.9 * *ema + 0.1 * g0;
            if let Some(b) = bd {
                let e = fn_bd_sum.entry(func_name.clone()).or_insert((0.0, 0.0, 0.0, 0));
                e.0 += b.vs_o0; e.1 += b.vs_o2; e.2 += b.vs_o3; e.3 += 1;
            }
            rollout_funcs.push(func_name);
            rollouts.push(rollout);
        }

        // ── Compute advantages (episode-level REINFORCE with baseline) ────────
        // Every step in an episode receives the same advantage: g0 - baseline.
        // We don't try to decompose credit to individual steps — the speedup
        // comes from the combination of passes, not any single one.  The
        // transformer's self-attention over the full sequence history is what
        // learns which combinations are good; the policy gradient here just
        // says "sequences above baseline = reinforce, below = suppress".
        // The value function target is g0 broadcast to all steps so it learns
        // to predict episode-level quality from each intermediate state.
        step_pb.set_message("computing advantages");
        let mut all_returns: Vec<f32> = Vec::new();
        let mut all_advantages: Vec<f32> = Vec::new();
        for ((rollout, &g0), &baseline) in rollouts.iter()
            .zip(episode_g0s.iter())
            .zip(episode_baselines.iter())
        {
            let advantage = g0 - baseline;
            let n = rollout.len();
            for _ in 0..n {
                all_advantages.push(advantage);
                all_returns.push(g0);
            }
        }

        let combined = Rollout::merge(&rollouts);

        // ── PPO update ────────────────────────────────────────────────────────
        step_pb.set_message("ppo update");
        let ev = explained_variance(&all_returns, &combined.values);

        let stats;
        (model, stats) = ppo_update_tfx(
            model,
            &mut optim,
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

            let delta = |cur: f32, prev: Option<f32>| -> String {
                match prev {
                    None => "    ".to_string(),
                    Some(p) => {
                        let d = cur - p;
                        if d.abs() < 0.005 { "    ".to_string() }
                        else if d > 0.0    { format!(" \x1b[32m↑{:.2}\x1b[0m", d.abs()) }
                        else               { format!(" \x1b[31m↓{:.2}\x1b[0m", d.abs()) }
                    }
                }
            };

            let ev_s = if ev > 0.3      { format!("\x1b[36m{ev:+.2}\x1b[0m") }
                       else if ev > 0.0 { format!("\x1b[33m{ev:+.2}\x1b[0m") }
                       else             { format!("\x1b[31m{ev:+.2}\x1b[0m") };

            let ent_s = if ent_frac > 0.4      { format!("\x1b[35m{ent_frac:.2}\x1b[0m") }
                        else if ent_frac > 0.2 { format!("\x1b[33m{ent_frac:.2}\x1b[0m") }
                        else                   { format!("\x1b[31m{ent_frac:.2}\x1b[0m") };

            let kl_s = if kl < 0.05     { format!("\x1b[2m{kl:.4}\x1b[0m") }
                       else if kl < 0.1 { format!("\x1b[33m{kl:.4}\x1b[0m") }
                       else             { format!("\x1b[31m{kl:.4}\x1b[0m") };

            let clip_s = if clip < 0.2      { format!("\x1b[2m{clip:.2}\x1b[0m") }
                         else if clip < 0.4 { format!("\x1b[33m{clip:.2}\x1b[0m") }
                         else               { format!("\x1b[31m{clip:.2}\x1b[0m") };

            let dev  = delta(ev,       prev_ev);
            let dent = delta(ent_frac, prev_ent);
            let dkl  = delta(kl,       prev_kl);

            let mean_ema = if fn_ema.is_empty() { 0.0 }
                           else { fn_ema.values().sum::<f32>() / fn_ema.len() as f32 };
            let ema_s = if mean_ema > 0.05       { format!("\x1b[1;36m{mean_ema:+.3}\x1b[0m") }
                        else if mean_ema > -0.05 { format!("\x1b[33m{mean_ema:+.3}\x1b[0m") }
                        else                     { format!("\x1b[31m{mean_ema:+.3}\x1b[0m") };

            let mut fn_summary: Vec<(&str, f32)> = fn_ema
                .iter()
                .map(|(k, &v)| (k.as_str(), v))
                .collect();
            fn_summary.sort_by_key(|(k, _)| *k);

            train_pb.println(format!(
                "  [{iteration:>4}] steps={:>4}  ev={ev_s}{dev}  ent={ent_s}{dent}  kl={kl_s}{dkl}  clip={clip_s}  ema={ema_s}",
                combined.len(),
            ));
            for (k, v) in &fn_summary {
                let bd_str = if let Some(&(s0, s2, s3, n)) = fn_bd_sum.get(*k) {
                    let n = n.max(1) as f32;
                    format!("  O0:{:+.0}%  O2:{:+.0}%  O3:{:+.0}%",
                        s0 / n * 100.0, s2 / n * 100.0, s3 / n * 100.0)
                } else {
                    String::new()
                };
                let s = format!("         {k:>24} = {v:+.3}{bd_str}");
                let colored = if *v > 0.05       { format!("\x1b[36m{s}\x1b[0m") }
                              else if *v > -0.05 { format!("\x1b[33m{s}\x1b[0m") }
                              else               { format!("\x1b[31m{s}\x1b[0m") };
                train_pb.println(colored);
            }

            // ── Diagnostics ──────────────────────────────────────────────────
            {
                let fmts = |v: &[f32]| -> String {
                    if v.is_empty() { return "  (empty)".to_string(); }
                    let mean = v.iter().sum::<f32>() / v.len() as f32;
                    let std  = (v.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / v.len() as f32).sqrt();
                    let min  = v.iter().cloned().fold(f32::INFINITY, f32::min);
                    let max  = v.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                    format!("mean={mean:+.3}  std={std:.3}  min={min:+.3}  max={max:+.3}")
                };
                train_pb.println(format!("          g0: {}", fmts(&episode_g0s)));
                train_pb.println(format!("          v0: {}", fmts(&episode_v0s)));
                train_pb.println(format!("    adv(gae): {}", fmts(&all_advantages)));
                train_pb.println(format!("        vloss: {:.4}  ploss: {:.4}",
                    stats.value_loss, stats.policy_loss));

                let mut fn_g0s: HashMap<&str, Vec<f32>> = HashMap::new();
                for (func, &g0) in rollout_funcs.iter().zip(episode_g0s.iter()) {
                    fn_g0s.entry(func.as_str()).or_default().push(g0);
                }
                let mut fn_g0_list: Vec<(&str, Vec<f32>)> = fn_g0s.into_iter().collect();
                fn_g0_list.sort_by_key(|(k, _)| *k);
                for (func, g0s) in &fn_g0_list {
                    let mean = g0s.iter().sum::<f32>() / g0s.len() as f32;
                    let min  = g0s.iter().cloned().fold(f32::INFINITY, f32::min);
                    let max  = g0s.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                    let v0   = episode_v0s.iter().zip(rollout_funcs.iter())
                        .filter(|(_, f)| f.as_str() == *func)
                        .map(|(&v, _)| v)
                        .next().unwrap_or(0.0);
                    train_pb.println(format!(
                        "    {func:>24}  g0 [{min:+.3} .. {max:+.3}] mean={mean:+.3}  v0={v0:+.3}  adv_spread={:.3}",
                        max - min,
                    ));
                }
            }

            if mean_ema > best_mean_ema {
                best_mean_ema = mean_ema;
                std::fs::create_dir_all(&config.checkpoint_dir).ok();
                let best = format!("{}/best_tfx", config.checkpoint_dir);
                if let Err(e) = model.valid().save_file(&best, &CompactRecorder::new()) {
                    train_pb.println(format!("  warn: checkpoint save failed: {e}"));
                } else {
                    train_pb.println(format!(
                        "  \x1b[1;36m★ new best (tfx)\x1b[0m  mean_ema={mean_ema:+.3}  → {best}.mpk"
                    ));
                }
            }

            prev_ev  = Some(ev);
            prev_ent = Some(ent_frac);
            prev_kl  = Some(kl);
        }

        if iteration % config.eval_interval == 0 && iteration > 0 {
            std::fs::create_dir_all(&config.checkpoint_dir).ok();
            let ckpt = format!("{}/tfx_iter_{:04}", config.checkpoint_dir, iteration);
            if let Err(e) = model.valid().save_file(&ckpt, &CompactRecorder::new()) {
                train_pb.println(format!("  warn: checkpoint save failed: {e}"));
            }
        }
    }

    train_pb.finish_with_message("complete");
    ep_pb.finish_and_clear();
    step_pb.finish_and_clear();
    Ok(())
}

fn sample_categorical(logits: &[f32], rng: &mut impl Rng) -> usize {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp: Vec<f32> = logits.iter().map(|x| (x - max).exp()).collect();
    let sum: f32 = exp.iter().sum();
    let u: f32 = rng.r#gen();
    let mut cumsum = 0.0f32;
    for (i, e) in exp.iter().enumerate() {
        cumsum += e / sum;
        if u <= cumsum { return i; }
    }
    logits.len() - 1
}

fn log_softmax_at(logits: &[f32], action: usize) -> f32 {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let log_sum_exp = logits.iter().map(|x| (x - max).exp()).sum::<f32>().ln() + max;
    logits[action] - log_sum_exp
}

fn explained_variance(returns: &[f32], values: &[f32]) -> f32 {
    let n = returns.len() as f32;
    let ret_mean = returns.iter().sum::<f32>() / n;
    let ret_var  = returns.iter().map(|r| (r - ret_mean).powi(2)).sum::<f32>() / n;
    if ret_var < 0.01 { return 0.0; }
    let residuals: Vec<f32> = returns.iter().zip(values.iter()).map(|(r, v)| r - v).collect();
    let res_mean = residuals.iter().sum::<f32>() / n;
    let res_var  = residuals.iter().map(|r| (r - res_mean).powi(2)).sum::<f32>() / n;
    1.0 - res_var / ret_var
}
