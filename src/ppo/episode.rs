use crate::config::Cfg;
use crate::llvm::Llvm;
use crate::llvm::ir::Ir;
use crate::ppo::model::transformer::TransformerActor;
use burn::backend::ndarray::NdArrayDevice;
use burn::prelude::Backend;
use std::sync::Arc;
use crate::ppo::step::Step;

pub(crate) struct Episode<B: Backend> {
    pub(crate) actor: TransformerActor<B>,
    pub(crate) device: B::Device,
    pub(crate) llvm: Llvm,
    pub(crate) ir: Ir,
    pub(crate) cfg: Cfg,
    pub(crate) steps: Vec<Step>,
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
        }
    }
    pub(crate) fn results(self) -> () {
        todo!()
    }
}
