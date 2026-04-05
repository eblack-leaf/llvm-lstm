pub(crate) mod gru;
pub(crate) mod transformer;

use crate::config::{BurnAutoDiff, BurnBackend, BurnDevice, Cfg};
use crate::llvm::ir::Ir;
use crate::llvm::pass::Pass;
use crate::ppo::tokens::Tokens;
use burn::Tensor;
use burn::module::AutodiffModule;
use burn::nn::{Linear, LinearConfig};
use burn::prelude::{Backend, Bool, Int, Module};
use burn::tensor::TensorData;
use burn::tensor::activation::{log_softmax, relu, softmax};

pub(crate) const ACTIONS: [Pass; 29] = [
    Pass::Instcombine,
    Pass::Mem2reg,
    Pass::Adce,
    Pass::Dse,
    Pass::Sccp,
    Pass::Reassociate,
    Pass::JumpThreading,
    Pass::Gvn,
    Pass::Sroa,
    Pass::SroaModifyCfg,
    Pass::Memcpyopt,
    Pass::Simplifycfg,
    Pass::Inline,
    Pass::EarlyCseMemssa,
    Pass::LoopRotate,
    Pass::LoopRotateHeaderDup,
    Pass::Licm,
    Pass::LicmAllowSpeculation,
    Pass::IndVars,
    Pass::LoopIdiom,
    Pass::LoopDeletion,
    Pass::SimpleLoopUnswitch,
    Pass::SimpleLoopUnswitchNontrivial,
    Pass::LoopUnroll,
    Pass::LoopUnrollO3,
    Pass::LoopVectorize,
    Pass::SlpVectorizer,
    Pass::Tailcallelim,
    Pass::Stop,
];

#[derive(Module, Debug)]
pub(crate) struct MlpHead<B: Backend> {
    fc1: Linear<B>,
    fc2: Linear<B>,
}

pub(crate) struct MlpHeadConfig {
    pub(crate) in_dim: usize,
    pub(crate) hidden_dim: usize,
    pub(crate) out_dim: usize,
}

impl MlpHeadConfig {
    pub(crate) fn new(in_dim: usize, hidden_dim: usize, out_dim: usize) -> Self {
        Self {
            in_dim,
            hidden_dim,
            out_dim,
        }
    }
    pub(crate) fn init<B: Backend>(&self, device: &B::Device) -> MlpHead<B> {
        MlpHead {
            fc1: LinearConfig::new(self.in_dim, self.hidden_dim).init(device),
            fc2: LinearConfig::new(self.hidden_dim, self.out_dim).init(device),
        }
    }
}

impl<B: Backend> MlpHead<B> {
    pub(crate) fn forward(&self, x: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = self.fc1.forward(x);
        let x = relu(x);
        self.fc2.forward(x)
    }
}

pub(crate) struct Input<B: Backend> {
    pub(crate) features: Tensor<B, 2>,
    pub(crate) actions: Tensor<B, 2, Int>,
    /// Padding mask [batch, seq_len+1] — True where the position is padding (action tokens
    /// beyond the actual sequence length). Position 0 (IR token) is always False.
    /// None during inference (single step, no padding needed).
    pub(crate) mask_pad: Option<Tensor<B, 2, Bool>>,
    /// Actual action-sequence length for each batch item, needed by GRU to gather the
    /// correct last hidden state when sequences have been padded to a common length.
    /// None during inference.
    pub(crate) action_lens: Option<Vec<usize>>,
}

impl Input<BurnBackend> {
    pub(crate) fn new(dev: &BurnDevice, ir: &Ir, current_ir: &Ir, actions: &[Pass]) -> Self {
        let tokens = Tokens::new(ir, current_ir, actions);
        let n_features = tokens.features.len();
        let seq_len = tokens.actions.len();
        let features = Tensor::from_data(TensorData::new(tokens.features, [1, n_features]), dev);
        let actions = Tensor::from_data(TensorData::new(tokens.actions, [1, seq_len]), dev);
        Self { features, actions, mask_pad: None, action_lens: None }
    }
}

pub(crate) struct Output<B: Backend> {
    pub(crate) policy: Tensor<B, 3>,
    pub(crate) value: Tensor<B, 2>,
}

// Inference-only methods — sampling and scalar extraction require a concrete non-autodiff backend.
impl Output<BurnBackend> {
    pub(crate) fn action(&self) -> Pass {
        let logits = self.policy.clone().flatten::<1>(0, 2);
        let probs = softmax(logits, 0);
        let cumsum = probs.cumsum(0);
        let u: f32 = rand::random();
        let idx = cumsum.lower_equal_elem(u).int().sum().into_scalar() as usize;
        ACTIONS[idx.min(ACTIONS.len() - 1)]
    }

    pub(crate) fn value_scalar(&self) -> f32 {
        self.value.clone().flatten::<1>(0, 1).into_scalar()
    }

    pub(crate) fn log_prob(&self, action: Pass) -> f32 {
        let logits = self.policy.clone().flatten::<1>(0, 2);
        let log_probs = log_softmax(logits, 0);
        let idx = ACTIONS
            .iter()
            .position(|&p| p == action)
            .expect("pass not in action space");
        log_probs.narrow(0, idx, 1).into_scalar()
    }
}

pub(crate) trait Actor<B: Backend>: Module<B> + Sized {
    type Config;
    fn init(cfg: Self::Config, device: &B::Device) -> Self;
    fn forward(&self, cfg: &Cfg, input: Input<B>) -> Output<B>;
    fn cfg(cfg: &Cfg) -> Self::Config;
}
