use crate::config::{Cfg, Dev, Diff};
use crate::llvm::Llvm;
use crate::llvm::functions::Functions;
use crate::llvm::pass::Pass;
use crate::ppo::episode::Episode;
use crate::ppo::model::{Actor, Input};
use crate::ppo::step::Step;
use tokio::task::JoinSet;

pub(crate) struct Trainer {
    cfg: Cfg,
}

impl Trainer {
    pub(crate) fn new(cfg: Cfg) -> Self {
        Self { cfg }
    }
    pub(crate) fn train<A: Actor + Clone + 'static + Send>(self) {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            let llvm = Llvm::new(&self.cfg.clang, &self.cfg.opt);
            let functions = Functions::new(&self.cfg.functions);
            let device = Dev::default();
            let model = A::init::<Diff>(A::cfg(&self.cfg), &device);
            for epoch in 0..self.cfg.epochs {
                let current = model.no_grads();
                let mut workers = JoinSet::new();
                for func in functions.functions.iter() {
                    for ep in 0..self.cfg.episodes {
                        let mut episode = Episode::new(
                            ep,
                            llvm.with_env(self.cfg.work_dir.join(format!("worker_{}", ep))),
                            func.ir.clone(),
                            self.cfg.clone(),
                        );
                        let actor = current.clone();
                        workers.spawn(async move {
                            loop {
                                let input =
                                    Input::<Diff>::new(&device, &episode.ir, &episode.actions); // TODO tokenize first?
                                let output = actor.forward(&episode.cfg, input);
                                let action = Pass::Stop; // TODO derive from output.policy
                                let prob = 1.0; // TODO log probability using action?
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
            }
            todo!()
        });
    }
}
