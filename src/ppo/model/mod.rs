pub(crate) mod gru;
pub(crate) mod transformer;

use crate::config::{BurnBackend, BurnDevice, Cfg};
use crate::llvm::ir::Ir;
use crate::llvm::pass::Pass;
use crate::ppo::tokens::Tokens;
use burn::nn::{Linear, LinearConfig};
use burn::prelude::Module;
use burn::tensor::TensorData;
use burn::tensor::activation::relu;
use burn::{Tensor, prelude::Int};

// Shared 2-layer MLP head used by both GRU and Transformer actors.
// forward: [batch, in_dim] → relu → [batch, out_dim]
#[derive(Module, Debug, Clone)]
pub(crate) struct MlpHead {
    fc1: Linear<BurnBackend>,
    fc2: Linear<BurnBackend>,
}

pub(crate) struct MlpHeadConfig {
    pub(crate) in_dim: usize,
    pub(crate) hidden_dim: usize,
    pub(crate) out_dim: usize,
}

impl MlpHeadConfig {
    pub(crate) fn new(in_dim: usize, hidden_dim: usize, out_dim: usize) -> Self {
        Self { in_dim, hidden_dim, out_dim }
    }
    pub(crate) fn init(&self, device: &BurnDevice) -> MlpHead {
        MlpHead {
            fc1: LinearConfig::new(self.in_dim, self.hidden_dim).init(device),
            fc2: LinearConfig::new(self.hidden_dim, self.out_dim).init(device),
        }
    }
}

impl MlpHead {
    pub(crate) fn forward(&self, x: Tensor<BurnBackend, 2>) -> Tensor<BurnBackend, 2> {
        let x = self.fc1.forward(x);
        let x = relu(x);
        self.fc2.forward(x)
    }
}

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
