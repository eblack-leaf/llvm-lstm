use crate::config::{BurnAutoDiff, BurnBackend, BurnDevice, Cfg};
use crate::llvm::Llvm;
use crate::llvm::functions::{Function, Functions};
use crate::llvm::pass::Pass;
use crate::ppo::episode::Episode;
use crate::ppo::model::{Actor, Input};
use crate::ppo::step::Step;
use crate::ppo::tokens::Tokens;
use burn::lr_scheduler::cosine::{CosineAnnealingLrScheduler, CosineAnnealingLrSchedulerConfig};
use burn::module::AutodiffModule;
use burn::optim::adaptor::OptimizerAdaptor;
use burn::optim::{AdamW, AdamWConfig};
use tokio::task::JoinSet;

pub(crate) struct Trainer {
    cfg: Cfg,
    llvm: Llvm,
    functions: Functions,
    device: BurnDevice,
}

impl Trainer {
    pub(crate) fn new(cfg: Cfg) -> Self {
        let llvm = Llvm::new(&cfg.clang, &cfg.opt, cfg.work_dir.clone());
        let functions = Functions::new(&cfg.functions);
        Self {
            cfg,
            llvm,
            functions,
            device: Default::default(),
        }
    }
    pub(crate) fn train<A: Actor + Clone + 'static + Send + AutodiffModule<BurnAutoDiff>>(self) {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
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
                    for ep in 0..self.cfg.episodes {
                        let mut episode = Episode::new(
                            ep,
                            self.llvm
                                .with_env(self.cfg.work_dir.join(format!("worker_{}", ep))),
                            func.ir.clone(),
                            self.cfg.clone(),
                        );
                        let actor = current.clone();
                        workers.spawn(async move {
                            loop {
                                // let tokens = Tokens::new(&episode.ir, &episode.actions); // TODO is this needed?
                                let input = Input::new(&self.device, &episode.ir, &episode.actions); // TODO tokenize first?
                                let output = actor.forward(&episode.cfg, input);
                                let action = output.action(); // TODO derive from output.policy
                                let prob = output.probability(action); // TODO log probability using action?
                                episode.actions.push(action);
                                episode.probabilities.push(prob);
                                // TODO value stuff
                                let done = action == Pass::Stop
                                    || episode.actions.len() + 1 > episode.cfg.max_seq_len;
                                if done || self.cfg.per_step_benchmark {
                                    let optimized = episode
                                        .llvm
                                        .apply(&episode.ir, &episode.actions)
                                        .await
                                        .expect("apply passes");
                                    let bin =
                                        episode.llvm.compile(&optimized).await.expect("compile");
                                    let benchmark = episode
                                        .llvm
                                        .benchmark(&bin, episode.cfg.benchmark_runs)
                                        .await
                                        .expect("benchmark");
                                    let step = Step::new(benchmark); // TODO add meta-data
                                    episode.steps.push(step);
                                    if done {
                                        break;
                                    }
                                }
                            }
                            episode.results()
                        });
                    }
                }
                let results = workers.join_all().await;
                let returns = (); // step attributed returns
                let advantages = (); // step based adv
                // TODO PPO update
                // metrics updating + using to check best => Checkpoint::save(best) + patience on EMA
                // logging update + every N epochs => plot train
            }
            // final cleanup + plot
            todo!()
        });
    }
}
