pub(crate) mod gru;
pub(crate) mod transformer;

use crate::config::{BurnBackend, BurnDevice, Cfg};
use crate::llvm::ir::Ir;
use crate::llvm::pass::Pass;
use crate::ppo::tokens::Tokens;
use burn::nn::{Linear, LinearConfig};
use burn::prelude::Module;
use burn::tensor::TensorData;
use burn::tensor::activation::{log_softmax, relu, softmax};
use burn::{Tensor, prelude::Int};

// Maps policy-head output index (0-based) to the corresponding Pass.
// The head covers all actionable passes: Instcombine (index 0) through Stop (index 28).
// Pass::Start is excluded — it is never a model output, only a sequence prefix.
const ACTIONS: [Pass; 29] = [
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
    pub(crate) async fn new(dev: &BurnDevice, ir: &Ir, current_ir: &Ir, actions: &[Pass]) -> Self {
        let tokens = Tokens::new(ir, current_ir, actions).await;
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
    // Samples an action from the policy logits using categorical sampling.
    // policy is [batch=1, 1, num_actions]; we squeeze to [num_actions] for ops.
    pub(crate) fn action(&self) -> Pass {
        let logits = self.policy.clone().squeeze::<1>(); // [num_actions]
        let probs = softmax(logits, 0);
        let cumsum = probs.cumsum(0); // [num_actions]
        // Count how many cumulative probabilities are ≤ u; that index is our sample.
        let u: f32 = rand::random();
        let idx = cumsum
            .lower_equal_elem(u)
            .int()
            .sum()
            .into_scalar() as usize;
        ACTIONS[idx.min(ACTIONS.len() - 1)]
    }

    // Critic's state-value estimate V(s_t); stored per step to compute advantages.
    // value is [1, 1]; flatten to 1-D before extracting the scalar.
    pub(crate) fn value_scalar(&self) -> f32 {
        self.value.clone().flatten::<1>(0, 1).into_scalar()
    }

    // Log probability of `action` under the current policy (used for the PPO ratio).
    // Returns a negative f32; log_softmax is numerically stable and avoids a separate
    // softmax → log pass.
    pub(crate) fn log_prob(&self, action: Pass) -> f32 {
        let logits = self.policy.clone().squeeze::<1>(); // [num_actions]
        let log_probs = log_softmax(logits, 0);
        // Reverse the ACTIONS mapping to get the logit index for this pass.
        let idx = ACTIONS.iter().position(|&p| p == action).expect("pass not in action space");
        log_probs
            .narrow(0, idx, 1)
            .into_scalar()
    }
}
pub(crate) trait Actor {
    type Config;
    fn init(cfg: Self::Config, device: &BurnDevice) -> Self;
    fn forward(&self, cfg: &Cfg, input: Input) -> Output;
    fn cfg(cfg: &Cfg) -> Self::Config;
    fn no_grads(&self) -> Self;
}
