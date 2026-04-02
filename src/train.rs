use crate::config::Cfg;
use crate::llvm::Llvm;
use crate::llvm::functions::Functions;
use std::sync::Arc;
use tokio::task::JoinSet;

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
            let llvm = Arc::new(Llvm::new(&self.cfg.clang, &self.cfg.opt));
            let functions = Functions::new(&self.cfg.functions);
            // create parallel episode collection
            for epoch in 0..self.cfg.epochs {
                let model = Arc::new(());
                let mut workers = JoinSet::new();
                for func in functions.functions.iter() {
                    for ep in 0..self.cfg.episodes {
                        let actor = model.clone();
                        let runner = llvm.clone();
                        let ir = func.ir.clone();
                        workers.spawn(async move {
                            // actor.forward();
                            let optimized = runner.apply(&ir, &[]).expect("apply passes");
                            let bin = runner.compile(&optimized).expect("compile");
                            let result = runner
                                .benchmark(&bin, self.cfg.benchmark_runs)
                                .await
                                .expect("benchmark");
                            result
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
