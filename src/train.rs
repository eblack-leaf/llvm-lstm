use crate::config::{Arch, BurnAutoDiff, BurnBackend, BurnDevice, Cfg};
use crate::ppo::checkpoint::{Checkpoint, CheckpointMeta};
use crate::llvm::{Llvm, LookaheadCache};
use blake3;
use dashmap::DashMap;
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
use tokio::task::JoinSet;

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
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4))
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(async move {
            let mut functions = Functions::new(&self.cfg.functions).await;
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
                tokio::fs::create_dir_all(&func_llvm.work_dir)
                    .await
                    .expect("create func work dir");
                func.ir = func_llvm
                    .ir(&func.source)
                    .await
                    .expect("ir");
                func.baselines = Some(
                    func_llvm
                        .collect_baselines(&func.source, self.cfg.baseline_runs, self.cfg.baseline_iters)
                        .await
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
            // Persists across epochs — (ir_content_hash, pass_idx) → speedup is fully
            // deterministic, policy changes only affect which IR states get explored.
            let lookahead_cache: LookaheadCache = Arc::new(DashMap::new());

            let mut model = Arch::init(arch_cfg.clone(), &self.device);
            let mut optimizer = AdamWConfig::new().init::<BurnAutoDiff, Arch>();
            let mut scheduler =
                CosineAnnealingLrSchedulerConfig::new(self.cfg.learning_rate, self.cfg.epochs)
                    .init()
                    .expect("scheduler init");

            for epoch in 0..self.cfg.epochs {
                let t_collect = Instant::now();
                let current = model.valid();
                let mut workers = JoinSet::new();
                for func in functions.functions.iter() {
                    let baselines = func.baselines.as_ref().expect("baselines not collected");
                    for ep in 0..self.cfg.episodes {
                        let mut episode = Episode::new(
                            ep,
                            self.llvm
                                .with_env(self.cfg.work_dir.join(format!("worker_{}_{ep}", func.name))),
                            func.name.clone(),
                            func.ir.clone(),
                            self.cfg.clone(),
                            baselines.clone(),
                        )
                        .await;
                        let actor = current.clone();
                        let dev = self.device.clone();
                        let cache = lookahead_cache.clone();
                        workers.spawn(async move {
                            let mut prev_features: Vec<f32> = episode.base_features.clone();
                            loop {
                                let input = Input::new(
                                    &dev,
                                    &episode.ir,
                                    &episode.current_ir,
                                    &episode.actions,
                                )
                                .await;
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
                                let lookahead: Option<Box<[f32; 29]>> = if episode.cfg.lookahead_benchmark {
                                    let pre_ir = episode.current_ir.clone();
                                    let llvm = episode.llvm.clone();
                                    let baselines = episode.baselines.clone();
                                    let runs = episode.cfg.lookahead_runs;
                                    let iters = episode.cfg.lookahead_iters;
                                    let mut speedups = Box::new([0.0f32; 29]);
                                    // Hash once — shared key prefix for all 29 passes.
                                    let content = tokio::fs::read(&pre_ir.file).await.expect("read IR for hash");
                                    let ir_hash: [u8; 32] = *blake3::hash(&content).as_bytes();
                                    let mut tasks = JoinSet::new();
                                    let mut step_hits: u64 = 0;
                                    let mut step_misses: u64 = 0;
                                    for (pass_idx, &pass) in ACTIONS.iter().enumerate() {
                                        let key = (ir_hash, pass_idx as u8);
                                        if let Some(cached) = cache.get(&key) {
                                            speedups[pass_idx] = *cached;
                                            step_hits += 1;
                                            continue;
                                        }
                                        step_misses += 1;
                                        let llvm = llvm.clone();
                                        let pre_ir = pre_ir.clone();
                                        let baselines = baselines.clone();
                                        let cache = cache.clone();
                                        tasks.spawn(async move {
                                            let speedup = llvm
                                                .bench_lookahead_cached(
                                                    &pre_ir, pass, pass_idx, step_idx,
                                                    &baselines, runs, iters, &cache,
                                                )
                                                .await
                                                .expect("bench_lookahead_cached");
                                            (pass_idx, speedup)
                                        });
                                    }
                                    while let Some(res) = tasks.join_next().await {
                                        let (idx, speedup) = res.expect("lookahead task panicked");
                                        speedups[idx] = speedup;
                                    }
                                    episode.lookahead_hits += step_hits;
                                    episode.lookahead_misses += step_misses;
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
                                        .await
                                        .expect("apply_one");
                                }
                                // Compute per-step marginal delta: features[t] - features[t-1].
                                // Zero for Stop (IR unchanged).
                                let delta_features = {
                                    let content =
                                        tokio::fs::read_to_string(&episode.current_ir.file)
                                            .await
                                            .expect("read current IR");
                                    let current = Features::from_ll_str(&content)
                                        .expect("parse current IR features")
                                        .to_vec();
                                    let delta: Vec<f32> = prev_features
                                        .iter()
                                        .zip(&current)
                                        .map(|(p, c)| c - p)
                                        .collect();
                                    prev_features = current;
                                    delta
                                };
                                let benchmark = if done || self.cfg.per_step_benchmark {
                                    let bin = episode
                                        .llvm
                                        .compile(&episode.current_ir)
                                        .await
                                        .expect("compile");
                                    let mut bm = episode
                                        .llvm
                                        .benchmark(&bin, episode.cfg.benchmark_runs, episode.cfg.benchmark_iters)
                                        .await
                                        .expect("benchmark");
                                    bm.speedup = episode.baselines.speedup_vs_o3(bm.mean_ns);
                                    Some(bm)
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
                            episode.results()
                        });
                    }
                }
                let total_episodes = workers.len();
                let col_bar = logger.collection_bar(total_episodes as u64);
                let mut results = Vec::with_capacity(total_episodes);
                while let Some(res) = workers.join_next().await {
                    results.push(res.expect("worker panicked"));
                    col_bar.inc(1);
                }
                col_bar.finish_and_clear();
                metrics.record_collection_ms(t_collect.elapsed().as_millis() as u64);
                let (total_hits, total_misses) = results.iter()
                    .fold((0u64, 0u64), |(h, m), r| (h + r.lookahead_hits, m + r.lookahead_misses));
                metrics.record_lookahead_cache(total_hits, total_misses);
                metrics.update_episode(&results);

                let all_returns: Vec<Vec<f32>> =
                    results.iter().map(|r| self.returns.compute(r)).collect();

                // Explained variance from rollout values vs computed returns (pre-update).
                let ev_rets: Vec<f32> = all_returns.iter().flatten().copied().collect();
                let ev_vals: Vec<f32> = results.iter().flat_map(|r| r.values.iter().copied()).collect();
                metrics.update_explained_variance(explained_variance(&ev_rets, &ev_vals));

                let advantages = self.advantages.compute(&all_returns, &results);
                metrics.update_returns_advs(&all_returns, &advantages);
                let batch = Ppo::batch(&results, &all_returns, &advantages);
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

                metrics.next_epoch();
            }

            logger.finish();
            // final cleanup + plot
            todo!()
        });
    }
}
