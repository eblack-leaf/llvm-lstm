pub(crate) mod gru;
pub(crate) mod transformer;

use crate::config::Cfg;
use crate::llvm::ir::Ir;
use crate::llvm::pass::Pass;
use burn::Tensor;
use burn::prelude::{Backend, Int};
use burn::tensor::backend::AutodiffBackend;

pub(crate) struct Input<B: Backend> {
    pub(crate) features: Tensor<B, 2>,
    pub(crate) actions: Tensor<B, 2, Int>,
}
impl<B: Backend> Input<B> {
    pub(crate) fn new(dev: &B::Device, ir: &Ir, actions: &[Pass]) -> Self {
        Self {
            features: Tensor::from_data([[1]], dev),
            actions: Tensor::from_data([[1]], dev),
        }
    }
}
pub(crate) struct Output<B: Backend> {
    pub(crate) policy: Tensor<B, 3>,
    pub(crate) value: Tensor<B, 2>,
}
pub(crate) trait Actor {
    type Config;
    fn init<B: Backend>(cfg: Self::Config, device: &B::Device) -> Self;
    fn forward<B: Backend>(&self, cfg: &Cfg, input: Input<B>) -> Output<B>;
    fn cfg(cfg: &Cfg) -> Self::Config;
    fn no_grads(&self) -> Self;
}
