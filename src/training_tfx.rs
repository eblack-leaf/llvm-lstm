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

    // Rolling histories for trend sparklines (last HIST logged iterations).
    const HIST: usize = 20;
    let mut ema_mean_hist: Vec<f32> = Vec::new();
    let mut ent_frac_hist: Vec<f32> = Vec::new();
    let mut ploss_hist:    Vec<f32> = Vec::new();
    let mut fn_ema_hist:   HashMap<String, Vec<f32>> = HashMap::new();

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
        let mut fn_bd_sum: HashMap<String, (f32, f32, f32, usize)> = HashMap::new();

        for (rollout, func_name, _episode_reward, bd) in episode_results {
            let g0: f32 = rollout.rewards.iter().enumerate()
                .map(|(t, &r)| r * config.ppo.gamma.powi(t as i32))
                .sum();
            let v0 = rollout.values.first().copied().unwrap_or(0.0);
            episode_g0s.push(g0);
            episode_v0s.push(v0);
            let ema = fn_ema.entry(func_name.clone()).or_insert(g0);
            *ema = 0.9 * *ema + 0.1 * g0;
            if let Some(b) = bd {
                let e = fn_bd_sum.entry(func_name.clone()).or_insert((0.0, 0.0, 0.0, 0));
                e.0 += b.vs_o0; e.1 += b.vs_o2; e.2 += b.vs_o3; e.3 += 1;
            }
            rollout_funcs.push(func_name);
            rollouts.push(rollout);
        }

        // ── Compute advantages (episode-level REINFORCE, intra-batch baseline) ─
        // Baseline = per-function mean g0 within this batch.
        // This zero-centers advantages within each function every iteration —
        // no EMA lag, no inflation from early lucky episodes, works from iter 1.
        // Positive ploss with EMA baseline meant the historical average was
        // inflated above current policy performance; intra-batch mean fixes that.
        step_pb.set_message("computing advantages");
        let mut fn_g0_batch: HashMap<&str, (f32, usize)> = HashMap::new();
        for (func_name, &g0) in rollout_funcs.iter().zip(episode_g0s.iter()) {
            let e = fn_g0_batch.entry(func_name.as_str()).or_insert((0.0, 0));
            e.0 += g0; e.1 += 1;
        }
        let fn_batch_mean: HashMap<&str, f32> = fn_g0_batch.iter()
            .map(|(&k, &(sum, n))| (k, sum / n as f32))
            .collect();

        let mut all_returns: Vec<f32> = Vec::new();
        let mut all_advantages: Vec<f32> = Vec::new();
        for ((rollout, &g0), func_name) in rollouts.iter()
            .zip(episode_g0s.iter())
            .zip(rollout_funcs.iter())
        {
            let baseline = fn_batch_mean.get(func_name.as_str()).copied().unwrap_or(g0);
            let advantage = g0 - baseline;
            let n = rollout.len();
            for _ in 0..n {
                all_advantages.push(advantage);
                all_returns.push(g0);
            }
        }

        let combined = Rollout::merge(&rollouts);

        // ── PPO update ────────────────────────────────────────────────────────
        // With episode-level REINFORCE advantages the value function is not used
        // for bootstrapping, so its loss has no algorithmic purpose.  Keeping it
        // in the total loss sends contradictory gradients through the shared
        // transformer trunk (every step in an episode has the same return target
        // g0 but different IR-feature states, so the value head can't converge).
        // Zero the coefficient so only policy gradient + entropy drive training.
        step_pb.set_message("ppo update");
        let _ev = explained_variance(&all_returns, &combined.values);

        let mut ppo_cfg = config.ppo.clone();
        ppo_cfg.value_loss_coef = 0.0;

        let stats;
        (model, stats) = ppo_update_tfx(
            model,
            &mut optim,
            &combined,
            &all_advantages,
            &all_returns,
            &ppo_cfg,
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
            let ploss    = stats.policy_loss;

            let mean_ema = if fn_ema.is_empty() { 0.0 }
                           else { fn_ema.values().sum::<f32>() / fn_ema.len() as f32 };

            let adv_std = {
                let n = all_advantages.len().max(1) as f32;
                let m = all_advantages.iter().sum::<f32>() / n;
                (all_advantages.iter().map(|x| (x - m).powi(2)).sum::<f32>() / n).sqrt()
            };

            let g0_min = episode_g0s.iter().cloned().fold(f32::INFINITY, f32::min);
            let g0_max = episode_g0s.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let g0_spread = g0_max - g0_min;

            // Update rolling histories
            fn push(h: &mut Vec<f32>, v: f32, cap: usize) {
                h.push(v);
                if h.len() > cap { h.remove(0); }
            }
            push(&mut ema_mean_hist, mean_ema, HIST);
            push(&mut ent_frac_hist, ent_frac, HIST);
            push(&mut ploss_hist,    ploss,    HIST);
            for (k, &v) in &fn_ema {
                push(fn_ema_hist.entry(k.clone()).or_default(), v, HIST);
            }

            // ── Pre-diagnosed status words ────────────────────────────────

            // Policy trend: compare current ema vs oldest in history
            let ema_delta = mean_ema - ema_mean_hist[0];
            let enough_hist = ema_mean_hist.len() >= 5;
            let (trend_label, trend_c) = if !enough_hist {
                ("WARMUP  ", "\x1b[2m")
            } else if ema_delta > 0.01 {
                ("IMPROVING", "\x1b[32m")
            } else if ema_delta < -0.01 {
                ("DECLINING", "\x1b[31m")
            } else {
                ("FLAT     ", "\x1b[33m")
            };

            // Signal: how much variance between episodes (can the policy learn anything?)
            let (sig_label, sig_c) = if adv_std > 0.05 {
                ("STRONG", "\x1b[32m")
            } else if adv_std > 0.015 {
                ("OK    ", "\x1b[33m")
            } else if adv_std > 0.003 {
                ("WEAK  ", "\x1b[31m")
            } else {
                ("DEAD  ", "\x1b[1;31m")
            };

            // Exploration: entropy as fraction of maximum
            let (exp_label, exp_c) = if ent_frac > 0.75 {
                ("HIGH     ", "\x1b[33m")  // too random
            } else if ent_frac > 0.35 {
                ("OK       ", "\x1b[32m")
            } else if ent_frac > 0.15 {
                ("LOW      ", "\x1b[33m")
            } else {
                ("COLLAPSED", "\x1b[1;31m")
            };

            // Update health: skipped by KL > stale positive ploss > saturated > healthy
            let ploss_avg = ploss_hist.iter().sum::<f32>() / ploss_hist.len() as f32;
            let (upd_label, upd_c, upd_detail) = if kl > config.ppo.target_kl {
                ("SKIPPED ", "\x1b[31m", format!("kl {kl:.3}"))
            } else if ploss_avg > 0.10 {
                ("STALE   ", "\x1b[31m", format!("ploss-avg {ploss_avg:+.3}"))
            } else if clip > 0.4 {
                ("SATURATED", "\x1b[33m", format!("clip {:.0}%", clip * 100.0))
            } else {
                ("HEALTHY ", "\x1b[32m", format!("ploss {ploss:+.3}"))
            };

            let ema_c = if mean_ema > 0.05 { "\x1b[1;36m" }
                        else if mean_ema > 0.0 { "\x1b[36m" }
                        else if mean_ema > -0.05 { "\x1b[33m" }
                        else { "\x1b[31m" };

            // ── Line 1: overall status ────────────────────────────────────
            train_pb.println(format!(
                "  \x1b[2m[{iteration:>4}]\x1b[0m  \
                 {trend_c}{trend_label}\x1b[0m ema {ema_c}{mean_ema:+.3}\x1b[0m  \
                 signal {sig_c}{sig_label}\x1b[0m  \
                 explore {exp_c}{exp_label}\x1b[0m {:.0}%  \
                 update {upd_c}{upd_label}\x1b[0m {upd_detail}  \
                 \x1b[2m{iter_secs:.0}s\x1b[0m",
                ent_frac * 100.0,
            ));

            // ── Line 2: per-function — name + trend arrow + ema + O3 ──────
            // Trend arrow from per-function ema history (old half vs new half mean).
            let mut fn_list: Vec<&str> = fn_ema.keys().map(|s| s.as_str()).collect();
            fn_list.sort();
            let mut fn_parts: Vec<String> = Vec::new();
            for func in fn_list {
                let ema_val = fn_ema[func];
                let hist    = fn_ema_hist.get(func).map(|v| v.as_slice()).unwrap_or(&[]);
                let arrow = if hist.len() >= 4 {
                    let mid = hist.len() / 2;
                    let old_mean = hist[..mid].iter().sum::<f32>() / mid as f32;
                    let new_mean = hist[mid..].iter().sum::<f32>() / (hist.len() - mid) as f32;
                    let d = new_mean - old_mean;
                    if d > 0.008 { "↑" } else if d < -0.008 { "↓" } else { "→" }
                } else { "·" };

                let o3_str = if let Some(&(_, _, s3, n)) = fn_bd_sum.get(func) {
                    let pct = s3 / n.max(1) as f32 * 100.0;
                    let (pc, sign) = if pct >= 1.0 { ("\x1b[32m", "+") }
                                     else if pct > -1.0 { ("\x1b[2m", "+") }
                                     else { ("\x1b[31m", "") };
                    format!(" O3 {pc}{sign}{pct:.0}%\x1b[0m")
                } else { String::new() };

                let fc = if ema_val > 0.05 { "\x1b[36m" }
                         else if ema_val > -0.05 { "\x1b[33m" } else { "\x1b[31m" };
                fn_parts.push(format!("{func} {fc}{ema_val:+.3}\x1b[0m{arrow}{o3_str}"));
            }
            if !fn_parts.is_empty() {
                train_pb.println(format!("         {}", fn_parts.join("   ")));
            }

            // ── Signal detail (only printed when signal is not OK/STRONG) ─
            if adv_std <= 0.05 {
                train_pb.println(format!(
                    "         \x1b[2msignal detail: adv_std {adv_std:.4}  g0 spread {g0_spread:.3}  [{g0_min:+.3}..{g0_max:+.3}]\x1b[0m"
                ));
            }

            // Checkpoint
            if mean_ema > best_mean_ema {
                best_mean_ema = mean_ema;
                std::fs::create_dir_all(&config.checkpoint_dir).ok();
                let best = format!("{}/best_tfx", config.checkpoint_dir);
                if let Err(e) = model.valid().save_file(&best, &CompactRecorder::new()) {
                    train_pb.println(format!("  warn: checkpoint save failed: {e}"));
                } else {
                    train_pb.println(format!(
                        "  \x1b[1;36m★ new best\x1b[0m  ema={mean_ema:+.3}  → {best}.mpk"
                    ));
                }
            }
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
