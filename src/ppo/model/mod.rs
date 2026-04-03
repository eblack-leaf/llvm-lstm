pub(crate) mod gru;
pub(crate) mod transformer;

use crate::config::{BurnBackend, BurnDevice, Cfg};
use crate::llvm::ir::Ir;
use crate::llvm::pass::Pass;
use crate::ppo::tokens::Tokens;
use burn::Tensor;
use burn::prelude::Int;
use burn::tensor::TensorData;
use burn::tensor::backend::AutodiffBackend;

pub(crate) struct Input {
    pub(crate) features: Tensor<BurnBackend, 2>,
    pub(crate) actions: Tensor<BurnBackend, 2, Int>,
}
impl Input {
    pub(crate) async fn new(dev: &BurnDevice, ir: &Ir, actions: &[Pass]) -> Self {
        let tokens = Tokens::new(ir, actions).await;
        let n_features = tokens.features.len();
        let seq_len = tokens.actions.len();
        let features = Tensor::from_data(
            TensorData::new(tokens.features, [1, n_features]),
            dev,
        );
        let actions = Tensor::from_data(
            TensorData::new(tokens.actions, [1, seq_len]),
            dev,
        );
        Self { features, actions }
    }
}
pub(crate) struct Output {
    pub(crate) policy: Tensor<BurnBackend, 3>,
    pub(crate) value: Tensor<BurnBackend, 2>,
}
impl Output {
    pub(crate) fn action(&self) -> Pass {
        todo!()
    }
    pub(crate) fn probability(&self, action: Pass) -> f32 {
        todo!()
    }
}
pub(crate) trait Actor {
    type Config;
    fn init(cfg: Self::Config, device: &BurnDevice) -> Self;
    fn forward(&self, cfg: &Cfg, input: Input) -> Output;
    fn cfg(cfg: &Cfg) -> Self::Config;
    fn no_grads(&self) -> Self;
}
