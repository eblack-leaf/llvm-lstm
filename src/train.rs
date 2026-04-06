use crate::config::{Arch, BurnAutoDiff, BurnBackend, BurnDevice, Cfg};
use crate::ppo::checkpoint::{Checkpoint, CheckpointMeta};
use crate::llvm::{Llvm, BenchCache, load_cache, save_cache};
use crate::llvm::ir::Features;
use crate::llvm::functions::Functions;
use crate::llvm::pass::Pass;
use crate::ppo::Ppo;
use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;
use crate::ppo::logging::{LogMode, Logger};
use crate::ppo::metrics::{Metrics, explained_variance};
use crate::ppo::model::{Actor, Input};
use crate::ppo::returns::Returns;
use dashmap::DashMap;
use rayon::prelude::*;
use std::sync::Arc;
use std::path::PathBuf;
use std::time::Instant;
use burn::lr_scheduler::LrScheduler;
use burn::lr_scheduler::cosine::CosineAnnealingLrSchedulerConfig;
use burn::module::AutodiffModule;
use burn::optim::{AdamW, AdamWConfig};

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
        let ppo  = Ppo::new(&cfg);
        Self { cfg, llvm, device: Default::default(), returns, advantages, ppo, log_mode, log_path }
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

        // Compile IR and collect baselines before the training loop.
        for func in &mut functions.functions {
            let t0       = Instant::now();
            let func_llvm = self.llvm.with_env(self.cfg.work_dir.join(&func.name));
            std::fs::create_dir_all(&func_llvm.work_dir).expect("create func work dir");

            func.ir = func_llvm.ir(&func.source).expect("ir");
            func.baselines = Some(
                func_llvm
                    .collect_baselines(&func.source, self.cfg.baseline_runs, self.cfg.baseline_iters)
                    .expect("collect_baselines"),
            );

            // Extract base IR features once per function (34-dim log-transformed).
            let content = std::fs::read_to_string(&func.ir.file).expect("read base IR");
            func.ir_features = Some(
                Features::from_ll_str(&content)
                    .expect("parse IR features")
                    .to_vec(),
            );

            metrics.record_func_ir_ms(t0.elapsed().as_millis() as u64);
            logger.log_baseline_progress(&func.name, t0.elapsed().as_millis() as u64);
        }
        logger.finish_baseline_phase();

        let arch_cfg      = Arch::cfg(&self.cfg);
        let checkpoint_dir = self.cfg.checkpoint_dir.join("best");
        let mut best_mean = f32::NEG_INFINITY;

        let bench_cache: BenchCache = if let Some(ref p) = self.cfg.cache_file {
            load_cache(p).expect("load bench cache")
        } else {
            Arc::new(DashMap::new()) as BenchCache
        };

        let mut model     = Arch::init(arch_cfg.clone(), &self.device);
        let mut optimizer = AdamWConfig::new().init::<BurnAutoDiff, Arch>();
        let mut scheduler =
            CosineAnnealingLrSchedulerConfig::new(self.cfg.learning_rate, self.cfg.epochs)
                .init()
                .expect("scheduler init");

        for epoch in 0..self.cfg.epochs {
            let t_collect = Instant::now();
            let current = model.valid();

            let tasks: Vec<_> = functions.functions.iter()
                .flat_map(|func| {
                    let func = func.clone();
                    (0..self.cfg.episodes).map(move |ep| (ep, func.clone()))
                })
                .collect();

            let total_episodes = tasks.len();
            let actors: Vec<_> = (0..total_episodes).map(|_| current.clone()).collect();
            let col_bar = logger.collection_bar(total_episodes as u64);

            let results: Vec<Results> = tasks
                .into_par_iter()
                .zip(actors)
                .map(|((ep, func), actor)| {
                    let baselines    = func.baselines.as_ref().expect("baselines not collected");
                    let ir_features  = func.ir_features.as_ref().expect("ir_features not collected");
                    let dev          = self.device.clone();
                    let cache        = bench_cache.clone();
                    let func_name    = func.name.clone();
                    let k            = self.cfg.max_seq_len;

                    let llvm = self.llvm.with_env(
                        self.cfg.work_dir.join(format!("worker_{}_{ep}", func.name)),
                    );
                    std::fs::create_dir_all(&llvm.work_dir).expect("create worker dir");

                    // Single forward pass over all K slots simultaneously.
                    let input  = Input::new_slots(&dev, ir_features, k);
                    let output = actor.forward(&self.cfg, input);

                    let value                  = output.value_scalar();
                    let (all_actions, all_lps) = output.sample_sequence();

                    // ep_len = first Stop index + 1, or K if no Stop was chosen.
                    // Only slots 0..ep_len are executed and trained.
                    let ep_len = all_actions.iter()
                        .position(|&a| a == Pass::Stop)
                        .map(|t| t + 1)
                        .unwrap_or(k);

                    let actions   = all_actions[..ep_len].to_vec();
                    let log_probs = all_lps[..ep_len].to_vec();

                    // Apply non-Stop actions in 0..ep_len to build the terminal IR.
                    let mut current_ir = func.ir.clone();
                    let mut bench_cache_hits   = 0u64;
                    let mut bench_cache_misses = 0u64;

                    for (step, &action) in actions.iter().enumerate() {
                        if action != Pass::Stop {
                            current_ir = llvm
                                .apply_one(&current_ir, action, step)
                                .expect("apply_one");
                        }
                    }

                    // Benchmark terminal IR (with cache keyed by IR content hash).
                    let content = std::fs::read(&current_ir.file).expect("read terminal IR");
                    let ir_hash: [u8; 32] = *blake3::hash(&content).as_bytes();
                    let cache_key = (func_name.clone(), ir_hash, crate::llvm::STOP_PASS_IDX);

                    let speedup = if let Some(&cached) = cache.get(&cache_key).as_deref() {
                        bench_cache_hits += 1;
                        cached
                    } else {
                        bench_cache_misses += 1;
                        let bin = llvm.compile(&current_ir).expect("compile");
                        let mut bm = llvm
                            .benchmark(&bin, self.cfg.benchmark_runs, self.cfg.benchmark_iters)
                            .expect("benchmark");
                        bm.speedup = baselines.speedup_vs_o3_parallel(bm.mean_ns);
                        cache.insert(cache_key, bm.speedup);
                        bm.speedup
                    };

                    col_bar.inc(1);

                    Results {
                        func_name,
                        bench_cache_hits,
                        bench_cache_misses,
                        ir_features: ir_features.clone(),
                        actions,
                        log_probs,
                        ep_len,
                        value,
                        episode_return: speedup,
                        baselines: baselines.clone(),
                    }
                })
                .collect();

            col_bar.finish_and_clear();
            metrics.record_collection_ms(t_collect.elapsed().as_millis() as u64);

            let (bench_hits, bench_misses) = results.iter()
                .fold((0u64, 0u64), |(h, m), r| (h + r.bench_cache_hits, m + r.bench_cache_misses));
            metrics.record_bench_cache(bench_hits, bench_misses);
            metrics.update_episode(&results);

            let all_returns = self.returns.compute_batch(&results);

            // Explained variance from rollout values vs computed returns (pre-update).
            let ev_rets: Vec<f32> = all_returns.iter().flatten().copied().collect();
            let ev_vals: Vec<f32> = results.iter()
                .flat_map(|r| std::iter::repeat(r.value).take(r.ep_len))
                .collect();
            metrics.update_explained_variance(explained_variance(&ev_rets, &ev_vals));

            let advantages = self.advantages.compute(&all_returns, &results);
            metrics.update_returns_advs(&all_returns, &advantages);

            let batch = Ppo::batch(&results, &all_returns);
            let lr    = scheduler.step();

            let t_ppo    = Instant::now();
            let num_chunks = batch.episodes.len().div_ceil(self.cfg.mini_batch_size);
            let ppo_bar  = logger.ppo_bar(self.cfg.ppo_epochs as u64 * num_chunks as u64);

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
            model     = new_model;
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

            if let Some(ref p) = self.cfg.cache_file {
                save_cache(&bench_cache, p).expect("save bench cache");
            }

            metrics.next_epoch();
        }

        logger.finish();
    }
}
