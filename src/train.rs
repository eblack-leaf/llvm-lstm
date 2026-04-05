use crate::config::{Arch, BurnAutoDiff, BurnBackend, BurnDevice, Cfg};
use crate::ppo::checkpoint::{Checkpoint, CheckpointMeta};
use crate::llvm::{Llvm, LookaheadCache, load_lookahead_cache, save_lookahead_cache};
use blake3;
use dashmap::DashMap;
use rayon::prelude::*;
use std::sync::Arc;
use crate::llvm::functions::Functions;
use crate::llvm::ir::Features;
use crate::llvm::pass::Pass;
use crate::ppo::Ppo;
use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Episode;
use crate::ppo::logging::{LogMode, Logger};
use crate::ppo::metrics::{Metrics, explained_variance};
use crate::ppo::model::transformer::TransformerActor;
use crate::ppo::model::{Actor, Input, ACTIONS};
use crate::ppo::returns::Returns;
use crate::ppo::step::Step;
use burn::lr_scheduler::LrScheduler;
use burn::lr_scheduler::cosine::CosineAnnealingLrSchedulerConfig;
use burn::module::AutodiffModule;
use burn::optim::{AdamW, AdamWConfig};
use std::path::PathBuf;
use std::time::Instant;

pub(crate) struct Trainer {
    cfg: Cfg,
    llvm: Llvm,
    device: BurnDevice,
    returns: Box<dyn Returns>,
    advantages: Box<dyn Advantages>,
    ppo: Ppo,
    log_mode: LogMode,
    log_path: Option<PathBuf>,
}

impl Trainer {
    pub(crate) fn new(
        cfg: Cfg,
        returns: Box<dyn Returns>,
        advantages: Box<dyn Advantages>,
        log_mode: LogMode,
        log_path: Option<PathBuf>,
    ) -> Self {
        let llvm = Llvm::new(&cfg.clang, &cfg.opt, cfg.work_dir.clone());
        let ppo = Ppo::new(&cfg);
        Self {
            cfg,
            llvm,
            device: Default::default(),
            returns,
            advantages,
            ppo,
            log_mode,
            log_path,
        }
    }

    pub(crate) fn train(mut self) {
        let mut functions = Functions::new(&self.cfg.functions);
        let mut logger = Logger::init(
            self.log_mode,
            self.log_path.as_deref(),
            self.cfg.epochs as u64,
            functions.functions.len() as u64,
        )
        .expect("logger init");
        let mut metrics = Metrics::new(0.05);

        // Compile IR and collect baselines for each function before any episode
        // collection. Run sequentially so timing is not polluted by parallel workers.
        for func in &mut functions.functions {
            let t0 = Instant::now();
            let func_llvm = self.llvm.with_env(self.cfg.work_dir.join(&func.name));
            std::fs::create_dir_all(&func_llvm.work_dir).expect("create func work dir");
            func.ir = func_llvm
                .ir(&func.source)
                .expect("ir");
            func.baselines = Some(
                func_llvm
                    .collect_baselines(&func.source, self.cfg.baseline_runs, self.cfg.baseline_iters)
                    .expect("collect_baselines"),
            );
            let elapsed = t0.elapsed().as_millis() as u64;
            metrics.record_func_ir_ms(elapsed);
            logger.log_baseline_progress(&func.name, elapsed);
        }
        logger.finish_baseline_phase();

        let arch_cfg = Arch::cfg(&self.cfg);
        let checkpoint_dir = self.cfg.checkpoint_dir.join("best");
        let mut best_mean = f32::NEG_INFINITY;
        // Persists across epochs — (func, ir_hash, pass_idx) → speedup is fully
        // deterministic. Optionally loaded from / saved to disk so restarts on
        // the same machine skip already-benchmarked states.
        let lookahead_cache: LookaheadCache = if let Some(ref p) = self.cfg.lookahead_cache_file {
            load_lookahead_cache(p).expect("load lookahead cache")
        } else {
            Arc::new(DashMap::new())
        };

        let mut model = Arch::init(arch_cfg.clone(), &self.device);
        let mut optimizer = AdamWConfig::new().init::<BurnAutoDiff, Arch>();
        let mut scheduler =
            CosineAnnealingLrSchedulerConfig::new(self.cfg.learning_rate, self.cfg.epochs)
                .init()
                .expect("scheduler init");

        for epoch in 0..self.cfg.epochs {
            let t_collect = Instant::now();
            let current = model.valid();

            // Build task list, then clone actor once per episode into owned tuples.
            // Separate steps avoid moving `current` inside an FnMut flat_map closure.
            // Each rayon worker gets exclusive actor ownership — no Sync bound needed.
            let tasks: Vec<_> = functions.functions.iter()
                .flat_map(|func| {
                    let func = func.clone();
                    (0..self.cfg.episodes).map(move |ep| (ep, func.clone()))
                })
                .collect();

            let total_episodes = tasks.len();
            let actors: Vec<_> = (0..total_episodes).map(|_| current.clone()).collect();
            let col_bar = logger.collection_bar(total_episodes as u64);

            let results: Vec<_> = tasks.into_par_iter().zip(actors).map(|((ep, func), actor)| {
                let baselines = func.baselines.as_ref().expect("baselines not collected");
                let mut episode = Episode::new(
                    ep,
                    self.llvm.with_env(self.cfg.work_dir.join(format!("worker_{}_{ep}", func.name))),
                    func.name.clone(),
                    func.ir.clone(),
                    self.cfg.clone(),
                    baselines.clone(),
                );
                let dev = self.device.clone();
                let cache = lookahead_cache.clone();

                let mut prev_features: Vec<f32> = episode.base_features.clone();
                // Step-level no-op cache: if the chosen pass didn't change the IR,
                // the next step's lookahead is identical — reuse instead of re-running.
                let mut prev_lookahead: Option<([u8; 32], [f32; 29])> = None;
                loop {
                    let input = Input::new(
                        &dev,
                        &episode.ir,
                        &episode.current_ir,
                        &episode.actions,
                    );
                    let output = actor.forward(&episode.cfg, input);
                    let action = output.action();
                    let log_prob = output.log_prob(action);
                    let value = output.value_scalar();
                    episode.actions.push(action);
                    episode.log_probs.push(log_prob);
                    episode.values.push(value);
                    let done = action == Pass::Stop
                        || episode.actions.len() > episode.cfg.max_seq_len;
                    let step_idx = episode.steps.len();

                    // Lookahead: bench all 29 ACTIONS from the pre-action IR.
                    // Happens before apply_one so the IR state is unmodified.
                    let lookahead: Option<[f32; 29]> = if episode.cfg.lookahead_benchmark {
                        let pre_ir = episode.current_ir.clone();
                        let baselines = episode.baselines.clone();
                        let runs = episode.cfg.lookahead_runs;
                        let iters = episode.cfg.lookahead_iters;
                        // Hash once — shared key prefix for all 29 passes.
                        let content = std::fs::read(&pre_ir.file).expect("read IR for hash");
                        let ir_hash: [u8; 32] = *blake3::hash(&content).as_bytes();
                        let speedups = if matches!(prev_lookahead, Some((h, _)) if h == ir_hash) {
                            // Same IR as last step (chosen pass was a no-op) — reuse.
                            episode.lookahead_hits += 29;
                            prev_lookahead.unwrap().1
                        } else {
                            let mut speedups = [0.0f32; 29];
                            for (pass_idx, &pass) in ACTIONS.iter().enumerate() {
                                let key = (episode.func_name.clone(), ir_hash, pass_idx as u8);
                                if let Some(cached) = cache.get(&key) {
                                    speedups[pass_idx] = *cached;
                                    episode.lookahead_hits += 1;
                                    continue;
                                }
                                let (speedup, noop_hit) = episode.llvm
                                    .bench_lookahead_cached(
                                        &pre_ir, pass, pass_idx, step_idx,
                                        &episode.func_name, &baselines, runs, iters, &cache,
                                    )
                                    .expect("bench_lookahead_cached");
                                if noop_hit {
                                    episode.lookahead_hits += 1;
                                } else {
                                    episode.lookahead_misses += 1;
                                }
                                speedups[pass_idx] = speedup;
                            }
                            speedups
                        };
                        prev_lookahead = Some((ir_hash, speedups));
                        Some(speedups)
                    } else {
                        None
                    };

                    // Apply the pass incrementally. Skip Stop — it terminates
                    // the episode without changing the IR.
                    if action != Pass::Stop {
                        episode.current_ir = episode
                            .llvm
                            .apply_one(&episode.current_ir, action, step_idx)
                            .expect("apply_one");
                    }
                    // Compute per-step marginal delta: features[t] - features[t-1].
                    // Zero for Stop (IR unchanged). Also capture the post-action
                    // IR hash for the benchmark cache check below.
                    let (delta_features, post_ir_hash) = {
                        let content =
                            std::fs::read(&episode.current_ir.file)
                                .expect("read current IR");
                        let post_hash: [u8; 32] = *blake3::hash(&content).as_bytes();
                        let content_str = String::from_utf8_lossy(&content);
                        let current = Features::from_ll_str(&content_str)
                            .expect("parse current IR features")
                            .to_vec();
                        let delta: Vec<f32> = prev_features
                            .iter()
                            .zip(&current)
                            .map(|(p, c)| c - p)
                            .collect();
                        prev_features = current;
                        (delta, post_hash)
                    };
                    let benchmark = if done || self.cfg.per_step_benchmark {
                        // Check if we already have this IR's benchmark in the
                        // lookahead cache — Stop entry gives bench(current_ir).
                        let cache_key = (episode.func_name.clone(), post_ir_hash, crate::llvm::STOP_PASS_IDX);
                        if let Some(&cached_speedup) = cache.get(&cache_key).as_deref() {
                            episode.lookahead_hits += 1;
                            Some(crate::llvm::benchmark::Benchmark { mean_ns: 0, speedup: cached_speedup })
                        } else {
                            let bin = episode
                                .llvm
                                .compile(&episode.current_ir)
                                .expect("compile");
                            let mut bm = episode
                                .llvm
                                .benchmark(&bin, episode.cfg.benchmark_runs, episode.cfg.benchmark_iters)
                                .expect("benchmark");
                            bm.speedup = episode.baselines.speedup_vs_o3_parallel(bm.mean_ns);
                            cache.insert(cache_key, bm.speedup);
                            Some(bm)
                        }
                    } else {
                        None
                    };
                    episode.steps.push(Step::new(
                        action,
                        step_idx,
                        benchmark,
                        delta_features,
                        lookahead,
                    ));
                    if done {
                        break;
                    }
                }
                col_bar.inc(1);
                episode.results()
            }).collect();

            col_bar.finish_and_clear();
            metrics.record_collection_ms(t_collect.elapsed().as_millis() as u64);
            let (total_hits, total_misses) = results.iter()
                .fold((0u64, 0u64), |(h, m), r| (h + r.lookahead_hits, m + r.lookahead_misses));
            metrics.record_lookahead_cache(total_hits, total_misses);
            metrics.update_episode(&results);

            let all_returns: Vec<Vec<f32>> = self.returns.compute_batch(&results);
            metrics.store_stats = self.returns.store_stats();

            // Explained variance from rollout values vs computed returns (pre-update).
            let ev_rets: Vec<f32> = all_returns.iter().flatten().copied().collect();
            let ev_vals: Vec<f32> = results.iter().flat_map(|r| r.values.iter().copied()).collect();
            metrics.update_explained_variance(explained_variance(&ev_rets, &ev_vals));

            let advantages = self.advantages.compute(&all_returns, &results);
            metrics.update_returns_advs(&all_returns, &advantages);
            let batch = Ppo::batch(&results, &all_returns);
            let lr = scheduler.step();

            let t_ppo = Instant::now();
            let num_chunks = batch.steps.len().div_ceil(self.cfg.mini_batch_size);
            let ppo_bar = logger.ppo_bar(self.cfg.ppo_epochs as u64 * num_chunks as u64);
            let (new_model, new_optimizer, losses) = self.ppo.update(
                model,
                optimizer,
                &batch,
                lr,
                &self.cfg,
                &self.device,
                &ppo_bar,
                self.advantages.as_ref(),
            );
            ppo_bar.finish_and_clear();
            model = new_model;
            optimizer = new_optimizer;
            metrics.record_ppo_ms(t_ppo.elapsed().as_millis() as u64);
            metrics.update_ppo(losses);

            logger.log_epoch(epoch, &metrics, lr);

            let epoch_mean = metrics.avg_final_speedup();
            if epoch_mean > best_mean {
                best_mean = epoch_mean;
                Checkpoint::save(
                    &model,
                    &arch_cfg,
                    CheckpointMeta {
                        epoch,
                        speedup_mean: best_mean,
                        max_seq_len: self.cfg.max_seq_len,
                    },
                    &checkpoint_dir,
                )
                .expect("checkpoint save");
                logger.log_best(epoch, best_mean);
            }

            if let Some(ref p) = self.cfg.lookahead_cache_file {
                save_lookahead_cache(&lookahead_cache, p).expect("save lookahead cache");
            }

            metrics.next_epoch();
        }

        logger.finish();
        // final cleanup + plot
    }
}
