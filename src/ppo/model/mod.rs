pub(crate) mod conclave;
pub(crate) mod seq;

use crate::config::{BurnBackend, BurnDevice, Cfg};
use crate::llvm::pass::Pass;
use burn::Tensor;
use burn::nn::{Linear, LinearConfig};
use burn::prelude::{Backend, Module};
use burn::tensor::TensorData;
use burn::tensor::activation::{log_softmax, softmax};

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
        use burn::tensor::activation::relu;
        let x = self.fc1.forward(x);
        let x = relu(x);
        self.fc2.forward(x)
    }
}

/// Model input: base IR features tiled K times — one row per slot, N episodes in batch.
/// Slot positions are derived from the sequence dimension inside the forward.
pub(crate) struct Input<B: Backend> {
    /// [N, K, input_dim] — same IR features for every slot; N=1 during collection.
    pub(crate) ir_features: Tensor<B, 3>,
}

impl Input<BurnBackend> {
    /// Build single-episode input (N=1) for K slots.
    pub(crate) fn new_slots(dev: &BurnDevice, ir_features: &[f32], k: usize) -> Self {
        let dim = ir_features.len();
        let feat_data: Vec<f32> = ir_features.iter().copied().cycle().take(k * dim).collect();
        Self {
            ir_features: Tensor::from_data(TensorData::new(feat_data, [1, k, dim]), dev),
        }
    }
}

pub(crate) struct Output<B: Backend> {
    /// [N, K, 1, num_actions] — policy logits for every slot of every episode.
    pub(crate) policy: Tensor<B, 4>,
    /// [N, K, 1] — value estimate V(base_IR), same for every slot of every episode.
    pub(crate) value: Tensor<B, 3>,
}

impl Output<BurnBackend> {
    /// Sample the full pass sequence for episode 0 (N=1 collection path).
    /// Returns (actions[K], log_probs[K]).
    pub(crate) fn sample_sequence(&self) -> (Vec<Pass>, Vec<f32>) {
        let k = self.policy.dims()[1];
        let mut actions = Vec::with_capacity(k);
        let mut log_probs = Vec::with_capacity(k);
        for slot in 0..k {
            // policy[0, slot, 0, :] → logits [num_actions]
            let logits = self
                .policy
                .clone()
                .narrow(0, 0, 1)
                .narrow(1, slot, 1)
                .flatten::<1>(0, 3);
            let log_p = log_softmax(logits.clone(), 0);
            let probs = softmax(logits, 0);
            let cumsum = probs.cumsum(0);
            let u: f32 = rand::random();
            let idx = cumsum.lower_equal_elem(u).int().sum().into_scalar() as usize;
            let idx = idx.min(ACTIONS.len() - 1);
            actions.push(ACTIONS[idx]);
            log_probs.push(log_p.narrow(0, idx, 1).into_scalar());
        }
        (actions, log_probs)
    }

    /// Per-slot value estimates for episode 0, length = K (N=1 collection path).
    pub(crate) fn value_vec(&self) -> Vec<f32> {
        let k = self.value.dims()[1];
        (0..k)
            .map(|t| {
                self.value
                    .clone()
                    .narrow(0, 0, 1)
                    .narrow(1, t, 1)
                    .flatten::<1>(0, 2)
                    .into_scalar()
            })
            .collect()
    }
}

pub(crate) trait Actor<B: Backend>: Module<B> + Sized {
    type Config;
    fn init(cfg: Self::Config, device: &B::Device) -> Self;
    fn forward(&self, cfg: &Cfg, input: Input<B>) -> Output<B>;
    fn cfg(cfg: &Cfg) -> Self::Config;
}
