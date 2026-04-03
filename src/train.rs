use crate::config::{BurnAutoDiff, BurnBackend, BurnDevice, Cfg};
use crate::llvm::Llvm;
use crate::llvm::functions::Functions;
use crate::llvm::pass::Pass;
use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Episode;
use crate::ppo::model::{Actor, Input};
use crate::ppo::returns::Returns;
use crate::ppo::step::Step;
use burn::lr_scheduler::cosine::CosineAnnealingLrSchedulerConfig;
use burn::module::AutodiffModule;
use burn::optim::{AdamW, AdamWConfig};
use tokio::task::JoinSet;

pub(crate) struct Trainer {
    cfg: Cfg,
    llvm: Llvm,
    functions: Functions,
    device: BurnDevice,
    returns: Box<dyn Returns>,
    advantages: Box<dyn Advantages>,
}

impl Trainer {
    pub(crate) fn new(
        cfg: Cfg,
        returns: Box<dyn Returns>,
        advantages: Box<dyn Advantages>,
    ) -> Self {
        let llvm = Llvm::new(&cfg.clang, &cfg.opt, cfg.work_dir.clone());
        let functions = Functions::new(&cfg.functions);
        Self {
            cfg,
            llvm,
            functions,
            device: Default::default(),
            returns,
            advantages,
        }
    }
    pub(crate) fn train<A: Actor + Clone + 'static + Send + AutodiffModule<BurnAutoDiff>>(mut self) {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            // Collect per-function baselines before any episode collection.
            // Run sequentially so timing is not polluted by parallel worker activity.
            for func in &mut self.functions.functions {
                func.baselines = Some(
                    self.llvm
                        .collect_baselines(&func.source, self.cfg.baseline_runs)
                        .await
                        .expect("collect_baselines"),
                );
            }

            let model = A::init(A::cfg(&self.cfg), &self.device);
            let policy_optimizer = AdamWConfig::new().init::<BurnAutoDiff, A>();
            let value_optimizer = AdamWConfig::new().init::<BurnAutoDiff, A>();
            let policy_scheduler =
                CosineAnnealingLrSchedulerConfig::new(self.cfg.policy_lr, self.cfg.epochs);
            let value_scheduler =
                CosineAnnealingLrSchedulerConfig::new(self.cfg.value_lr, self.cfg.epochs);
            for epoch in 0..self.cfg.epochs {
                let current = model.no_grads();
                let mut workers = JoinSet::new();
                for func in self.functions.functions.iter() {
                    let baselines = func.baselines.as_ref().expect("baselines not collected");
                    for ep in 0..self.cfg.episodes {
                        let mut episode = Episode::new(
                            ep,
                            self.llvm
                                .with_env(self.cfg.work_dir.join(format!("worker_{ep}"))),
                            func.ir.clone(),
                            self.cfg.clone(),
                            baselines.clone(),
                        );
                        let actor = current.clone();
                        workers.spawn(async move {
                            loop {
                                let input = Input::new(
                                    &self.device,
                                    &episode.ir,
                                    &episode.current_ir,
                                    &episode.actions,
                                ).await;
                                let output = actor.forward(&episode.cfg, input);
                                let action = output.action();
                                let log_prob = output.log_prob(action);
                                let value = output.value_scalar();
                                episode.actions.push(action);
                                episode.log_probs.push(log_prob);
                                episode.values.push(value);
                                let done = action == Pass::Stop
                                    || episode.actions.len() + 1 > episode.cfg.max_seq_len;
                                let step_idx = episode.steps.len();
                                // Apply the pass incrementally. Skip Stop — it terminates
                                // the episode without changing the IR.
                                if action != Pass::Stop {
                                    let out = episode.llvm.work_dir.join(
                                        format!("step_{step_idx}.ll")
                                    );
                                    episode.current_ir = episode
                                        .llvm
                                        .apply_one(&episode.current_ir, action, out)
                                        .await
                                        .expect("apply_one");
                                }
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
                                episode.steps.push(Step::new(action, step_idx, benchmark));
                                if done {
                                    break;
                                }
                            }
                            episode.results()
                        });
                    }
                }
                let results = workers.join_all().await;
                let all_returns: Vec<Vec<f32>> = results
                    .iter()
                    .map(|r| self.returns.compute(r))
                    .collect();
                let advantages = self.advantages.compute(&all_returns, &results);
                // TODO PPO update
                // metrics updating + using to check best => Checkpoint::save(best) + patience on EMA
                // logging update + every N epochs => plot train
            }
            // final cleanup + plot
            todo!()
        });
    }
}
