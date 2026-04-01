use std::collections::HashMap;
use std::time::{Duration, Instant};

use rayon::prelude::*;

use anyhow::Result;
use burn::backend::Autodiff;

#[cfg(feature = "wgpu")]
use burn::backend::wgpu::{Wgpu, WgpuDevice};
// ── Backend selection ─────────────────────────────────────────────────────────
// Default: NdArray (CPU).  GPU: add `wgpu` to burn features in Cargo.toml and
// recompile.  No code changes needed — the type aliases below wire it through.
#[cfg(not(feature = "wgpu"))]
use burn::backend::{NdArray, ndarray::NdArrayDevice};

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
use burn::prelude::Module as _;
use burn::record::CompactRecorder;
use burn::tensor::{Int, Tensor, TensorData};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::Rng;

use crate::actor_critic_tfx::{TransformerActorCritic, TransformerActorCriticConfig};
use crate::baseline::{Baseline, BaselineMode, FnStats, broadcast_to_steps, build_advantages};
use crate::critic::{Critic, IrFilmCnnConfig, IrFilmCritic, NullCritic, PerFuncCritic};
use crate::env::{EnvConfig, LlvmEnv, RewardBreakdown};
use crate::episode_store::{BestEpisodeStore, Episode};
use crate::ppo::ppo_update_tfx;
use crate::returns::{ReturnMode, Returns};
use crate::rollout::Rollout;
use crate::tfx_critic::TransformerCritic;
use crate::training::TrainConfig;

type B = Autodiff<Inner>;

pub fn train(config: TrainConfig) -> Result<()> {
    let device = Dev::default();

    let worker_functions_dir = config.env.functions_dir.clone();
    let worker_work_dir = config.env.work_dir.clone();
    let worker_reward_mode = config.env.reward_mode.clone();
    let worker_max_seq_length = config.env.max_seq_length;
    let worker_benchmark_runs = config.env.benchmark_runs;
    let worker_ir_mode = config.ir_mode.clone();

    let mut env = LlvmEnv::new(config.env)?;
    env.compute_baselines()?;

    // "base+current" concatenates base and current IR → input_dim doubles to 68.
    let input_dim = if config.ir_mode == "base+current" {
        68
    } else {
        34
    };
    // pos_embed table must exceed max episode length + 1 (IR token) + action zero-pad.
    let pos_table = worker_max_seq_length + 4;
    let mut model = TransformerActorCriticConfig::new()
        .with_input_dim(input_dim)
        .with_max_seq_len(pos_table)
        .init::<B>(&device);
    let grad_clip = Some(GradientClippingConfig::Norm(0.5));
    let mut optim = AdamConfig::new()
        .with_grad_clipping(grad_clip)
        .init::<B, TransformerActorCritic<B>>();

    let multi = MultiProgress::new();
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

    let return_mode = ReturnMode::from_str(&config.return_mode);
    let baseline_mode = BaselineMode::from_str(&config.baseline_mode);
    let mut fn_stats = FnStats::new();
    let mut store = BestEpisodeStore::new(config.prune_threshold, config.store_max_per_func);
    let mut best_mean_ema: f32 = -f32::MAX;

    let num_actions = TransformerActorCriticConfig::new().num_actions;
    let ir_dim = if config.ir_mode == "base+current" {
        68
    } else {
        34
    };

    let film_cfg = || IrFilmCnnConfig::new(num_actions).with_ir_dim(ir_dim);
    let mut critic: Box<dyn Critic> = match config.critic_arch.as_str() {
        "ir-film" => Box::new(IrFilmCritic::<B>::new(
            film_cfg(),
            config.ppo.learning_rate,
            device.clone(),
        )),
        "per-func" => Box::new(PerFuncCritic::<B>::new(
            film_cfg(),
            config.ppo.learning_rate,
            device.clone(),
        )),
        "transformer" => {
            let actor_cfg = TransformerActorCriticConfig::new()
                .with_input_dim(input_dim)
                .with_max_seq_len(pos_table);
            Box::new(TransformerCritic::<B>::new(
                actor_cfg,
                config.ppo.learning_rate,
                &device,
            ))
        }
        _ => Box::new(NullCritic),
    };

    // Rolling histories for trend detection (last HIST logged iterations).
    const HIST: usize = 20;
    let mut ema_mean_hist: Vec<f32> = Vec::new();
    let mut ent_frac_hist: Vec<f32> = Vec::new();
    let mut ploss_hist: Vec<f32> = Vec::new();
    let mut kl_hist: Vec<f32> = Vec::new();
    let mut fn_ema_hist: HashMap<String, Vec<f32>> = HashMap::new();

    let max_entropy = (TransformerActorCriticConfig::new().num_actions as f32).ln();

    let baselines = env.baselines().clone();
    let n_funcs = env.num_functions();

    // Build a name→index map so episode allocation can look up EMAs by function index.
    let fn_index_names: Vec<String> = (0..n_funcs).map(|i| env.function_name(i)).collect();

    // Metrics log: one JSON record per logged iteration, written incrementally.
    std::fs::create_dir_all(&config.checkpoint_dir).ok();
    let metrics_path = format!("{}/train_metrics.json", config.checkpoint_dir);
    let mut metric_records: Vec<serde_json::Value> = Vec::new();

    let total_episodes = n_funcs * config.episodes_per_function;
    ep_pb.set_length(total_episodes as u64);

    for iteration in 0..config.total_iterations {
        let iter_start = Instant::now();
        ep_pb.set_position(0);

        let model_inf = model.valid();
        let base_work_dir = &worker_work_dir;

        // Episode→function mapping: even by default, dynamic when --dynamic-alloc.
        // Dynamic mode allocates a minimum floor to solved functions and routes
        // remaining episodes to functions still below O3, proportional to their
        // distance below it. Even mode is safer when all functions are near-solved
        // (fewer episodes destabilises the per-function batch-mean baseline).
        let episode_func_map: Vec<usize> = if config.dynamic_alloc {
            let min_per_func = (config.episodes_per_function / 4).max(2);
            let mut alloc = vec![min_per_func; n_funcs];
            let spare = total_episodes.saturating_sub(min_per_func * n_funcs);
            let weights: Vec<f32> = fn_index_names
                .iter()
                .map(|name| (-fn_stats.ema(name.as_str()).unwrap_or(0.0)).max(0.0))
                .collect();
            let weight_sum: f32 = weights.iter().sum();
            if weight_sum > 0.0 {
                for (i, &w) in weights.iter().enumerate() {
                    alloc[i] += (spare as f32 * w / weight_sum).round() as usize;
                }
            } else {
                for i in 0..n_funcs {
                    alloc[i] += spare / n_funcs;
                }
            }
            alloc
                .iter()
                .enumerate()
                .flat_map(|(fi, &count)| std::iter::repeat(fi).take(count))
                .collect()
        } else {
            (0..total_episodes).map(|ep| ep % n_funcs).collect()
        };
        let actual_episodes = episode_func_map.len();
        ep_pb.set_length(actual_episodes as u64);

        // ── Collect episodes in parallel ──────────────────────────────────────
        // Transformer rollout: no hidden state threading. Instead each worker
        // accumulates the full (features, prev_action) sequence for the episode
        // and re-runs the Transformer over the growing sequence at each step.
        // Compute cost is O(t²) per step vs O(t) for GRU, but entirely negligible
        // compared to compilation + benchmarking time.
        let episode_results: Vec<(Rollout, String, f32, Option<RewardBreakdown>)> = (0
            ..actual_episodes)
            .into_par_iter()
            .map_with(
                model_inf,
                |model_s, ep| -> anyhow::Result<(Rollout, String, f32, Option<RewardBreakdown>)> {
                    let func_index = episode_func_map[ep];

                    let worker_dir = base_work_dir.join(format!("worker-tfx-{ep}"));
                    let worker_config = EnvConfig::new(
                        worker_functions_dir.clone(),
                        worker_dir,
                        worker_reward_mode.clone(),
                    )
                    .with_max_seq_length(worker_max_seq_length)
                    .with_benchmark_runs(worker_benchmark_runs);

                    let mut worker_env =
                        LlvmEnv::new_with_baselines(worker_config, baselines.clone())?;
                    let mut state = worker_env.reset_to(func_index)?;
                    let func_name = worker_env
                        .current_function_name()
                        .unwrap_or_else(|| "?".into());

                    let device = Dev::default();
                    let mut rng = rand::thread_rng();

                    let mut rollout = Rollout::new();
                    let mut episode_reward = 0.0f32;
                    let feat_dim = state.features.len();
                    let ir_mode = worker_ir_mode.as_str();

                    // Shared across all modes: base IR captured once at episode start.
                    let base_features: Vec<f32> = state.features.clone();
                    // Action history: grows each step; index 0 = zero-pad "no prior action".
                    let mut act_history: Vec<i64> = vec![0i64];

                    let terminal_breakdown = loop {
                        let logits = match ir_mode {
                            "base+current" => {
                                // IR token = concat(base, current) — 68-d input.
                                let mut concat = base_features.clone();
                                concat.extend_from_slice(&state.features);
                                let base_t = Tensor::<Inner, 2>::from_data(
                                    TensorData::new(concat, [1, feat_dim * 2]),
                                    &device,
                                );
                                let acts_t = Tensor::<Inner, 2, Int>::from_data(
                                    TensorData::new(act_history.clone(), [1, act_history.len()]),
                                    &device,
                                );
                                model_s.forward(base_t, acts_t)
                            }
                            _ => {
                                // "base" (default): fixed base IR token + action sequence.
                                let base_t = Tensor::<Inner, 2>::from_data(
                                    TensorData::new(base_features.clone(), [1, feat_dim]),
                                    &device,
                                );
                                let acts_t = Tensor::<Inner, 2, Int>::from_data(
                                    TensorData::new(act_history.clone(), [1, act_history.len()]),
                                    &device,
                                );
                                model_s.forward(base_t, acts_t)
                            }
                        };

                        let logits_vec: Vec<f32> = logits.into_data().to_vec()?;
                        let action = sample_categorical(&logits_vec, &mut rng);
                        let log_prob = log_softmax_at(&logits_vec, action);

                        let step = worker_env.step(action)?;
                        episode_reward += step.reward;

                        // Store in rollout: per-step and base both use current state.features;
                        // base+current stores the 68-d concat so PPO uses step-0 vector correctly.
                        let stored_features = if ir_mode == "base+current" {
                            let mut concat = base_features.clone();
                            concat.extend_from_slice(&state.features);
                            concat
                        } else {
                            state.features.clone()
                        };

                        rollout.push(
                            stored_features,
                            action,
                            log_prob,
                            step.reward,
                            0.0,
                            step.done,
                        );

                        if step.done {
                            ep_pb.inc(1);
                            break step.breakdown;
                        }

                        act_history.push(action as i64);
                        state = step.state;
                    };

                    Ok((rollout, func_name, episode_reward, terminal_breakdown))
                },
            )
            .collect::<anyhow::Result<Vec<_>>>()?;

        // ── Collect rollouts ──────────────────────────────────────────────────
        let mut rollouts: Vec<Rollout> = Vec::with_capacity(total_episodes);
        let mut rollout_funcs: Vec<String> = Vec::with_capacity(total_episodes);
        let mut fn_bd_sum: HashMap<String, (f32, f32, f32, usize)> = HashMap::new();

        for (rollout, func_name, _episode_reward, bd) in episode_results {
            if let Some(b) = bd {
                let e = fn_bd_sum
                    .entry(func_name.clone())
                    .or_insert((0.0, 0.0, 0.0, 0));
                e.0 += b.vs_o0;
                e.1 += b.vs_o2;
                e.2 += b.vs_o3;
                e.3 += 1;
            }
            rollout_funcs.push(func_name);
            rollouts.push(rollout);
        }

        // ── Compute returns, update state, compute advantages ─────────────────
        step_pb.set_message("computing advantages");

        // 1. Compute returns
        let returns = Returns::compute(
            &rollouts,
            &return_mode,
            if return_mode == ReturnMode::Episode {
                1.0
            } else {
                config.ppo.gamma
            },
        );
        let episode_g0s = returns.g0_per_ep.clone();

        // 2. Update fn_stats (EMA + best) and BestEpisodeStore
        for (ep_idx, (func, rollout)) in rollout_funcs.iter().zip(rollouts.iter()).enumerate() {
            let g0 = returns.g0_per_ep[ep_idx];
            fn_stats.update(func, g0);
            // Step 0 state = base IR features for this episode
            let ir_features = rollout.states.first().cloned().unwrap_or_default();
            store.insert(Episode {
                func: func.clone(),
                actions: rollout.actions.clone(),
                g0,
                ir_features,
            });
        }

        // After store.insert loops...
        let adaptive_baseline_mode =
            if config.baseline_mode == "critic" && store.total_count() < config.warmup_threshold {
                &BaselineMode::Retrieval
            } else {
                &baseline_mode
            };

        let start = std::time::Instant::now();
        let critic_loss = critic.update(&store); // now returns Option<f32>
        let critic_time = start.elapsed().as_secs_f64();

        // 4. Compute baselines
        let baseline = Baseline::select(
            &rollout_funcs,
            &rollouts,
            &returns,
            adaptive_baseline_mode,
            &fn_stats,
            critic.as_ref(),
            &store,
        );

        // 5. Per-episode advantage weights (downweight solved functions)
        let any_unsolved = fn_stats.fn_ema.values().any(|&e| e < 0.0);
        let ep_weights: Vec<f32> = rollout_funcs
            .iter()
            .map(|func| {
                if config.adv_weighting && any_unsolved {
                    let ema_val = fn_stats.ema(func).unwrap_or(0.0);
                    (1.0 - ema_val.max(0.0) / 0.2).max(0.1)
                } else {
                    1.0
                }
            })
            .collect();
        let ep_lens: Vec<usize> = rollouts.iter().map(|r| r.len()).collect();
        let step_weights = broadcast_to_steps(&ep_weights, &ep_lens);

        // 6. Build normalised advantages
        let all_advantages = build_advantages(&returns.values, &baseline.values, &step_weights);

        let combined = Rollout::merge(&rollouts);

        // ── PPO update ────────────────────────────────────────────────────────
        step_pb.set_message("ppo update");

        let stats;
        (model, stats) = ppo_update_tfx(
            model,
            &mut optim,
            &combined,
            &all_advantages,
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
            let kl = stats.approx_kl;
            let clip = stats.clip_fraction;
            let ploss = stats.policy_loss;

            let mean_ema = if fn_stats.fn_ema.is_empty() {
                0.0
            } else {
                fn_stats.fn_ema.values().sum::<f32>() / fn_stats.fn_ema.len() as f32
            };

            // Raw advantage std (return - baseline, before normalisation).
            // Measures how much outcome variance the baseline leaves unexplained.
            // Near 0 = all episodes had similar returns → weak learning signal.
            let adv_std = {
                let raw: Vec<f32> = returns
                    .values
                    .iter()
                    .zip(baseline.values.iter())
                    .map(|(r, b)| r - b)
                    .collect();
                let n = raw.len().max(1) as f32;
                let m = raw.iter().sum::<f32>() / n;
                (raw.iter().map(|x| (x - m).powi(2)).sum::<f32>() / n).sqrt()
            };

            let g0_min = episode_g0s.iter().cloned().fold(f32::INFINITY, f32::min);
            let g0_max = episode_g0s
                .iter()
                .cloned()
                .fold(f32::NEG_INFINITY, f32::max);
            let g0_spread = g0_max - g0_min;

            // ── Save metrics record ───────────────────────────────────────────
            {
                let fn_ema_map: serde_json::Map<String, serde_json::Value> = fn_stats
                    .fn_ema
                    .iter()
                    .map(|(k, &v)| (k.clone(), serde_json::Value::from(v as f64)))
                    .collect();
                let fn_vs_o0: serde_json::Map<String, serde_json::Value> = fn_bd_sum
                    .iter()
                    .map(|(k, &(s0, _, _, n))| {
                        (
                            k.clone(),
                            serde_json::Value::from(s0 as f64 / n.max(1) as f64 * 100.0),
                        )
                    })
                    .collect();
                let fn_vs_o2: serde_json::Map<String, serde_json::Value> = fn_bd_sum
                    .iter()
                    .map(|(k, &(_, s2, _, n))| {
                        (
                            k.clone(),
                            serde_json::Value::from(s2 as f64 / n.max(1) as f64 * 100.0),
                        )
                    })
                    .collect();
                let fn_vs_o3: serde_json::Map<String, serde_json::Value> = fn_bd_sum
                    .iter()
                    .map(|(k, &(_, _, s3, n))| {
                        (
                            k.clone(),
                            serde_json::Value::from(s3 as f64 / n.max(1) as f64 * 100.0),
                        )
                    })
                    .collect();
                metric_records.push(serde_json::json!({
                    "iteration":    iteration,
                    "ema_mean":     mean_ema as f64,
                    "policy_loss":  ploss as f64,
                    "entropy_frac": ent_frac as f64,
                    "kl":           kl as f64,
                    "clip_fraction":clip as f64,
                    "adv_std":      adv_std as f64,
                    "g0_spread":    g0_spread as f64,
                    "critic_loss":  critic_loss.unwrap_or(1.0) as f64,
                    "iter_secs":    iter_secs as f64,
                    "fn_ema":       fn_ema_map,
                    "fn_vs_o0":     fn_vs_o0,
                    "fn_vs_o2":     fn_vs_o2,
                    "fn_vs_o3":     fn_vs_o3,
                }));
                if let Ok(json) = serde_json::to_string_pretty(&metric_records) {
                    std::fs::write(&metrics_path, json).ok();
                }
            }

            // Update rolling histories
            fn push(h: &mut Vec<f32>, v: f32, cap: usize) {
                h.push(v);
                if h.len() > cap {
                    h.remove(0);
                }
            }
            push(&mut ema_mean_hist, mean_ema, HIST);
            push(&mut ent_frac_hist, ent_frac, HIST);
            push(&mut ploss_hist, ploss, HIST);
            push(&mut kl_hist, kl, HIST);
            for (k, &v) in &fn_stats.fn_ema {
                push(fn_ema_hist.entry(k.clone()).or_default(), v, HIST);
            }

            // colour helpers: c = higher is better, cr = lower is better
            let c = |v: f32, lo: f32, hi: f32| -> &'static str {
                if v >= hi {
                    "\x1b[32m"
                } else if v >= lo {
                    "\x1b[33m"
                } else {
                    "\x1b[31m"
                }
            };
            let cr = |v: f32, lo: f32, hi: f32| -> &'static str {
                if v <= lo {
                    "\x1b[32m"
                } else if v <= hi {
                    "\x1b[33m"
                } else {
                    "\x1b[31m"
                }
            };

            let ema_delta = mean_ema - ema_mean_hist[0];
            let ploss_avg = ploss_hist.iter().sum::<f32>() / ploss_hist.len() as f32;

            let ema_c = if mean_ema > 0.05 {
                "\x1b[1;36m"
            } else if mean_ema > 0.0 {
                "\x1b[36m"
            } else if mean_ema > -0.05 {
                "\x1b[33m"
            } else {
                "\x1b[31m"
            };
            let delta_c = if ema_delta > 0.005 {
                "\x1b[32m"
            } else if ema_delta < -0.005 {
                "\x1b[31m"
            } else {
                "\x1b[2m"
            };
            let ploss_c = if ploss.abs() < 0.05 {
                "\x1b[32m"
            } else if ploss.abs() < 0.2 {
                "\x1b[33m"
            } else {
                "\x1b[31m"
            };
            let pavg_c = if ploss_avg.abs() < 0.05 {
                "\x1b[32m"
            } else if ploss_avg.abs() < 0.15 {
                "\x1b[33m"
            } else {
                "\x1b[31m"
            };

            // ── Line 1: policy progress ───────────────────────────────────
            // ema = smoothed return quality. delta = change over history window.
            // ploss-avg trending positive = baseline too high or policy degrading.
            train_pb.println(format!(
                "  \x1b[2m[{iteration:>4}]\x1b[0m  ema {ema_c}{mean_ema:+.4}\x1b[0m  \
                 Δ {delta_c}{ema_delta:+.4}/{}\x1b[0m  \
                 ploss {ploss_c}{ploss:+.4}\x1b[0m  ploss-avg {pavg_c}{ploss_avg:+.4}\x1b[0m  \
                 \x1b[2m{iter_secs:.0}s\x1b[0m",
                ema_mean_hist.len(),
            ));

            // ── Line 2: update diagnostics ────────────────────────────────
            // kl>0.15 = epoch skipped. clip>30% = steps too large. ent<35% = collapsing.
            train_pb.println(format!(
                "         update  kl {}{kl:.4}\x1b[0m  clip {}{:.1}%\x1b[0m  ent {}{:.1}%\x1b[0m  critic={}",
                cr(kl,   0.05, 0.15),
                cr(clip, 0.3,  0.5),  clip * 100.0,
                c(ent_frac, 0.35, 0.55), ent_frac * 100.0,
                critic.name(),
            ));

            // ── Line 3: signal diagnostics ────────────────────────────────
            // adv_std = raw (return-baseline) std, reward units.
            // <0.02 = weak signal. g0 spread = batch return range.
            train_pb.println(format!(
                "         signal  adv± {}{adv_std:.4}\x1b[0m  g0 {}{g0_spread:.4}\x1b[0m [{g0_min:+.4}..{g0_max:+.4}]  n={}",
                c(adv_std,   0.02, 0.08),
                c(g0_spread, 0.05,  0.12),
                combined.len(),
            ));

            // ── Per-function lines ────────────────────────────────────────
            let mut fn_list: Vec<&str> = fn_stats.fn_ema.keys().map(|s| s.as_str()).collect();
            fn_list.sort();
            for func in fn_list {
                let ema_val = fn_stats.fn_ema[func];
                let hist = fn_ema_hist.get(func).map(|v| v.as_slice()).unwrap_or(&[]);

                // trend arrow: compare old half vs new half of ema history
                let arrow = if hist.len() >= 4 {
                    let mid = hist.len() / 2;
                    let d = hist[mid..].iter().sum::<f32>() / (hist.len() - mid) as f32
                        - hist[..mid].iter().sum::<f32>() / mid as f32;
                    if d > 0.008 {
                        "↑"
                    } else if d < -0.008 {
                        "↓"
                    } else {
                        "→"
                    }
                } else {
                    "·"
                };

                let g0s: Vec<f32> = rollout_funcs
                    .iter()
                    .zip(episode_g0s.iter())
                    .filter(|(f, _)| f.as_str() == func)
                    .map(|(_, &g)| g)
                    .collect();
                let fn_spread = if g0s.len() > 1 {
                    g0s.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
                        - g0s.iter().cloned().fold(f32::INFINITY, f32::min)
                } else {
                    0.0
                };

                let bd_str = if let Some(&(s0, s2, s3, n)) = fn_bd_sum.get(func) {
                    let n = n.max(1) as f32;
                    let (p0, p2, p3) = (s0 / n * 100.0, s2 / n * 100.0, s3 / n * 100.0);
                    let c0 = if p0 >= 1.0 {
                        "\x1b[32m"
                    } else if p0 > -1.0 {
                        "\x1b[2m"
                    } else {
                        "\x1b[31m"
                    };
                    let c2 = if p2 >= 1.0 {
                        "\x1b[32m"
                    } else if p2 > -1.0 {
                        "\x1b[2m"
                    } else {
                        "\x1b[31m"
                    };
                    let c3 = if p3 >= 1.0 {
                        "\x1b[32m"
                    } else if p3 > -1.0 {
                        "\x1b[2m"
                    } else {
                        "\x1b[31m"
                    };
                    format!(
                        "  O0 {c0}{p0:+.0}%\x1b[0m  O2 {c2}{p2:+.0}%\x1b[0m  O3 {c3}{p3:+.0}%\x1b[0m"
                    )
                } else {
                    String::new()
                };

                let ec = if ema_val > 0.05 {
                    "\x1b[36m"
                } else if ema_val > -0.05 {
                    "\x1b[33m"
                } else {
                    "\x1b[31m"
                };
                let sc = c(fn_spread, 0.03, 0.08);
                train_pb.println(format!(
                    "  {func:>22}  ema {ec}{ema_val:+.4}\x1b[0m{arrow}  spread {sc}{fn_spread:.4}\x1b[0m{bd_str}",
                ));
            }

            if let Some(loss) = critic_loss {
                let count = store.total_count();
                let max = config.store_max_per_func * n_funcs;
                let prune = config.prune_threshold;
                train_pb.println(format!("         critic loss = {loss:.4}    critic update: {critic_time:.1}s    store: {count:.1} / {max:.1} @ {prune:.1}%"));
            }

            // ── Pattern flags (detected across rolling history) ───────────
            // Only printed when there is something to call out.
            if ema_mean_hist.len() >= 5 {
                let mut flags: Vec<String> = Vec::new();

                // ploss stuck positive: baseline too high or policy degrading
                let ploss_mean = ploss_hist.iter().sum::<f32>() / ploss_hist.len() as f32;
                let ploss_std = {
                    let m = ploss_mean;
                    (ploss_hist.iter().map(|x| (x - m).powi(2)).sum::<f32>()
                        / ploss_hist.len() as f32)
                        .sqrt()
                };
                if ploss_mean > 0.08 {
                    let pos = ploss_hist.iter().filter(|&&x| x > 0.0).count();
                    if pos > ploss_hist.len() / 2 {
                        flags.push(format!("\x1b[31mploss stuck positive (avg {ploss_mean:+.3}, {pos}/{} iters)\x1b[0m", ploss_hist.len()));
                    }
                }
                // ploss oscillating: only flag when mean is near zero (no consistent direction)
                // negative mean = policy genuinely improving, not a problem
                if ploss_std > 0.10 && ploss_mean.abs() < 0.05 {
                    flags.push(format!("\x1b[33mploss no clear direction (avg {ploss_mean:+.3} std {ploss_std:.3})\x1b[0m"));
                }

                // ema flat or declining over window
                let ema_delta = mean_ema - ema_mean_hist[0];
                if ema_delta.abs() < 0.005 {
                    flags.push(format!(
                        "\x1b[33mema flat (Δ{ema_delta:+.4} over {} iters)\x1b[0m",
                        ema_mean_hist.len()
                    ));
                } else if ema_delta < -0.01 {
                    flags.push(format!(
                        "\x1b[31mema declining (Δ{ema_delta:+.4} over {} iters)\x1b[0m",
                        ema_mean_hist.len()
                    ));
                }

                // entropy collapsing: new half of history significantly lower than old half
                if ent_frac_hist.len() >= 4 {
                    let mid = ent_frac_hist.len() / 2;
                    let old_e = ent_frac_hist[..mid].iter().sum::<f32>() / mid as f32;
                    let new_e = ent_frac_hist[mid..].iter().sum::<f32>()
                        / (ent_frac_hist.len() - mid) as f32;
                    if old_e - new_e > 0.08 {
                        flags.push(format!(
                            "\x1b[31ment collapsing ({:.1}% → {:.1}%)\x1b[0m",
                            old_e * 100.0,
                            new_e * 100.0
                        ));
                    } else if new_e < 0.2 {
                        flags.push(format!(
                            "\x1b[31ment critically low ({:.1}%)\x1b[0m",
                            new_e * 100.0
                        ));
                    }
                }

                // kl repeatedly skipping updates
                if kl_hist.len() >= 4 {
                    let skipped = kl_hist
                        .iter()
                        .filter(|&&k| k > config.ppo.target_kl)
                        .count();
                    if skipped > kl_hist.len() / 3 {
                        flags.push(format!(
                            "\x1b[31mkl skipping updates ({skipped}/{} iters)\x1b[0m",
                            kl_hist.len()
                        ));
                    }
                }

                // signal dead: raw adv_std in reward units; < 0.01 means all
                // episodes got nearly identical returns → no learning gradient
                if adv_std < 0.01 {
                    flags.push(format!(
                        "\x1b[1;31msignal dead (adv_std {adv_std:.5})\x1b[0m"
                    ));
                }

                if !flags.is_empty() {
                    train_pb.println(format!("         \x1b[2m!\x1b[0m  {}", flags.join("  ")));
                }
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

    if std::path::Path::new(&metrics_path).exists() {
        if let Err(e) = crate::plots::plot_train(std::path::Path::new(&config.checkpoint_dir)) {
            eprintln!("Warning: plot_train failed: {e}");
        }
    }

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
        if u <= cumsum {
            return i;
        }
    }
    logits.len() - 1
}

fn log_softmax_at(logits: &[f32], action: usize) -> f32 {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let log_sum_exp = logits.iter().map(|x| (x - max).exp()).sum::<f32>().ln() + max;
    logits[action] - log_sum_exp
}
