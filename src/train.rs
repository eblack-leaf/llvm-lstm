use crate::config::{Arch, BurnAutoDiff, BurnBackend, BurnDevice, Cfg};
use crate::llvm::Llvm;
use crate::llvm::functions::Functions;
use crate::llvm::ir::Features;
use crate::llvm::pass::Pass;
use crate::ppo::Ppo;
use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Episode;
use crate::ppo::logging::{LogMode, Logger};
use crate::ppo::metrics::Metrics;
use crate::ppo::model::transformer::TransformerActor;
use crate::ppo::model::{Actor, Input};
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
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
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
                        .collect_baselines(&func.source, self.cfg.baseline_runs)
                        .await
                        .expect("collect_baselines"),
                );
                let elapsed = t0.elapsed().as_millis() as u64;
                metrics.record_func_ir_ms(elapsed);
                logger.log_baseline_progress(&func.name, elapsed);
            }
            logger.finish_baseline_phase();

            let mut model = Arch::init(Arch::cfg(&self.cfg), &self.device);
            let mut optimizer = AdamWConfig::new().init::<BurnAutoDiff, Arch>();
            let mut scheduler =
                CosineAnnealingLrSchedulerConfig::new(self.cfg.policy_lr, self.cfg.epochs)
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
                            func.ir.clone(),
                            self.cfg.clone(),
                            baselines.clone(),
                        )
                        .await;
                        let actor = current.clone();
                        workers.spawn(async move {
                            loop {
                                let input = Input::new(
                                    &self.device,
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
                                // Apply the pass incrementally. Skip Stop — it terminates
                                // the episode without changing the IR.
                                if action != Pass::Stop {
                                    episode.current_ir = episode
                                        .llvm
                                        .apply_one(&episode.current_ir, action, step_idx)
                                        .await
                                        .expect("apply_one");
                                }
                                // Compute delta_features from the current IR state.
                                // Zero for Stop (IR unchanged); non-zero entries show
                                // which IR characteristics the pass sequence has moved.
                                let delta_features = {
                                    let content =
                                        tokio::fs::read_to_string(&episode.current_ir.file)
                                            .await
                                            .expect("read current IR");
                                    let current = Features::from_ll_str(&content)
                                        .expect("parse current IR features")
                                        .to_vec();
                                    episode
                                        .base_features
                                        .iter()
                                        .zip(&current)
                                        .map(|(b, c)| c - b)
                                        .collect()
                                };
                                let benchmark = if done || self.cfg.per_step_benchmark {
                                    let bin = episode
                                        .llvm
                                        .compile(&episode.current_ir)
                                        .await
                                        .expect("compile");
                                    let mut bm = episode
                                        .llvm
                                        .benchmark(&bin, episode.cfg.benchmark_runs)
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
                                ));
                                if done {
                                    break;
                                }
                            }
                            episode.results()
                        });
                    }
                }
                let results = workers.join_all().await;
                metrics.record_collection_ms(t_collect.elapsed().as_millis() as u64);
                metrics.update_episode(&results);

                let all_returns: Vec<Vec<f32>> =
                    results.iter().map(|r| self.returns.compute(r)).collect();
                let advantages = self.advantages.compute(&all_returns, &results);
                let batch = Ppo::batch(&results, &all_returns, &advantages);
                let lr = scheduler.step();

                let t_ppo = Instant::now();
                let (new_model, new_optimizer, losses) = self.ppo.update(
                    model,
                    optimizer,
                    &batch,
                    lr,
                    &self.cfg,
                    &self.device,
                );
                model = new_model;
                optimizer = new_optimizer;
                metrics.record_ppo_ms(t_ppo.elapsed().as_millis() as u64);
                metrics.update_ppo(losses);

                logger.log_epoch(epoch, &metrics, lr);
                metrics.next_epoch();

                // TODO checkpoint (best model by speedup_ema)
            }

            logger.finish();
            // final cleanup + plot
            todo!()
        });
    }
}
