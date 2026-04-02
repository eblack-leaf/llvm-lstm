use crate::config::Cfg;
use crate::llvm::Llvm;
use crate::llvm::ir::Ir;
use crate::llvm::pass::Pass;
use crate::ppo::model::transformer::TransformerActor;
use crate::ppo::step::Step;
use burn::prelude::Backend;

pub(crate) struct Episode<B: Backend> {
    pub(crate) actor: TransformerActor<B>,
    pub(crate) device: B::Device,
    pub(crate) llvm: Llvm,
    pub(crate) ir: Ir,
    pub(crate) cfg: Cfg,
    pub(crate) steps: Vec<Step>,
    pub(crate) actions: Vec<Pass>,
    pub(crate) probabilities: Vec<f32>,
}
impl<B: Backend> Episode<B> {
    pub fn new(
        actor: TransformerActor<B>,
        llvm: Llvm,
        ir: Ir,
        device: B::Device,
        cfg: Cfg,
    ) -> Self {
        Self {
            actor,
            device,
            llvm,
            ir,
            cfg,
            steps: vec![],
            actions: vec![],
            probabilities: vec![],
        }
    }
    pub(crate) fn results(self) -> () {
        todo!()
    }
}
