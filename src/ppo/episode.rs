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
    pub(crate) probabilities: Vec<f32>,
}
impl Episode {
    pub fn new(idx: usize, llvm: Llvm, ir: Ir, cfg: Cfg) -> Self {
        Self {
            llvm,
            ir,
            cfg,
            steps: vec![],
            actions: vec![],
            probabilities: vec![],
        }
    }
    pub(crate) fn results(self) -> Results {
        todo!()
    }
}

pub(crate) struct Results {}
