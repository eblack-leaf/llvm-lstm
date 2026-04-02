use crate::config::Cfg;
use crate::llvm::functions::Functions;
use crate::llvm::pass::Pass;
use crate::llvm::Llvm;
use crate::ppo::episode::Episode;
use crate::ppo::model::transformer::{TransformerActor, TransformerActorConfig};
use crate::ppo::model::{Actor, Input};
use crate::ppo::step::Step;
use burn::backend::ndarray::NdArrayDevice;
use burn::backend::{Autodiff, NdArray};
use burn::module::AutodiffModule;
use tokio::task::JoinSet;

type Backend = NdArray;
type Dev = NdArrayDevice;
type Diff = Autodiff<Backend>;
pub(crate) struct Trainer {
    cfg: Cfg,
}

impl Trainer {
    pub(crate) fn new(cfg: Cfg) -> Self {
        Self { cfg }
    }
    pub(crate) fn train(self) {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            let llvm = Llvm::new(&self.cfg.clang, &self.cfg.opt);
            let functions = Functions::new(&self.cfg.functions);
            let device = Dev::default();
            let model =
                TransformerActor::<Diff>::init::<Diff>(TransformerActorConfig::new(), &device);
            // create parallel episode collection
            for epoch in 0..self.cfg.epochs {
                let current = model.valid();
                let mut workers = JoinSet::new();
                for func in functions.functions.iter() {
                    for ep in 0..self.cfg.episodes {
                        let mut episode = Episode::new(
                            current.clone(),
                            llvm.clone(),
                            func.ir.clone(),
                            device.clone(),
                            self.cfg.clone(),
                        );
                        workers.spawn(async move {
                            loop {
                                let input = Input::<Diff>::new(&episode.device);
                                let output = episode.actor.forward(&episode.cfg, input);
                                let action = Pass::Stop; // TODO derive from output.policy
                                let prob = (); // TODO log probability using action?
                                // TODO value stuff
                                let done = action == Pass::Stop;
                                // TODO no skip if per-step bench? what is most concise flow of branch?
                                if done {
                                    let optimized =
                                        episode.llvm.apply(&episode.ir, &[]).expect("apply passes");
                                    let bin = episode.llvm.compile(&optimized).expect("compile");
                                    let benchmark = episode
                                        .llvm
                                        .benchmark(&bin, episode.cfg.benchmark_runs)
                                        .await
                                        .expect("benchmark");
                                    let step = Step::new(benchmark); // TODO add meta-data
                                    episode.steps.push(step);
                                    break;
                                }
                            }
                            episode.results()
                        });
                    }
                }
                let results = workers.join_all().await;
                // reward attribution
                // advantages
                // ppo.update
            }
            todo!()
        });
    }
}
