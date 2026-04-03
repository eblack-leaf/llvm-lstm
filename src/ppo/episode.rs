use crate::config::Cfg;
use crate::llvm::Llvm;
use crate::llvm::ir::Ir;
use crate::llvm::pass::Pass;
use crate::ppo::step::Step;

pub(crate) struct Episode {
    pub(crate) llvm: Llvm,
    pub(crate) ir: Ir,
    pub(crate) cfg: Cfg,
    pub(crate) steps: Vec<Step>,
    pub(crate) actions: Vec<Pass>,
    pub(crate) log_probs: Vec<f32>,
    // V(s_t) estimate from the critic at each step; used to compute advantages
    pub(crate) values: Vec<f32>,
}
impl Episode {
    pub(crate) fn new(idx: usize, llvm: Llvm, ir: Ir, cfg: Cfg) -> Self {
        Self {
            llvm,
            ir,
            cfg,
            steps: vec![],
            actions: vec![],
            log_probs: vec![],
            values: vec![],
        }
    }
    pub(crate) fn results(self) -> Results {
        todo!()
    }
}

pub(crate) struct Results {
    // instead of all the data from episode, only what needs to be reported back
}
