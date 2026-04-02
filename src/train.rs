use crate::config::Cfg;
use crate::llvm::functions::Functions;
use crate::llvm::Llvm;

pub(crate) struct Trainer {
    cfg: Cfg,
    llvm: Llvm,
    functions: Functions,
}

impl Trainer {
    pub(crate) fn new(cfg: Cfg) -> Self {
        let llvm = Llvm::new(&cfg.clang, &cfg.opt);
        let functions = Functions::new(&cfg.functions);
        Self {
            cfg,
            llvm,
            functions
        }
    }
    pub(crate) fn train(&mut self) {
        // create parallel episode collection
        // reward attribution
        // advantages
        // ppo.update
        todo!()
    }
}