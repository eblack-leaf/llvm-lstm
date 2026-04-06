pub(crate) mod seq;

use crate::config::{BurnBackend, BurnDevice, Cfg};
use crate::llvm::pass::Pass;
use burn::Tensor;
use burn::nn::{Linear, LinearConfig};
use burn::prelude::{Backend, Int, Module};
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
        Self { in_dim, hidden_dim, out_dim }
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

/// Model input: base IR features tiled K times + slot indices [0..K].
pub(crate) struct Input<B: Backend> {
    /// [K, input_dim] — same IR features for every slot.
    pub(crate) ir_features: Tensor<B, 2>,
    /// [K] — slot positions [0, 1, ..., K-1].
    pub(crate) slot_idx: Tensor<B, 1, Int>,
}

impl Input<BurnBackend> {
    /// Build input for all K=max_seq_len slots in one episode.
    pub(crate) fn new_slots(dev: &BurnDevice, ir_features: &[f32], k: usize) -> Self {
        let dim = ir_features.len();
        let feat_data: Vec<f32> = ir_features.iter().copied().cycle().take(k * dim).collect();
        let slot_data: Vec<i64> = (0..k as i64).collect();
        Self {
            ir_features: Tensor::from_data(TensorData::new(feat_data, [k, dim]), dev),
            slot_idx:    Tensor::from_data(TensorData::new(slot_data, [k]), dev),
        }
    }
}

pub(crate) struct Output<B: Backend> {
    /// [K, 1, num_actions] — policy logits for every slot.
    pub(crate) policy: Tensor<B, 3>,
    /// [K, 1] — value estimate V(base_IR), same for every slot.
    pub(crate) value: Tensor<B, 2>,
}

impl Output<BurnBackend> {
    /// Sample the full pass sequence in one call.
    /// Returns (actions[K], log_probs[K]) for all slots.
    pub(crate) fn sample_sequence(&self) -> (Vec<Pass>, Vec<f32>) {
        let k = self.policy.dims()[0];
        let mut actions  = Vec::with_capacity(k);
        let mut log_probs = Vec::with_capacity(k);
        for slot in 0..k {
            let logits   = self.policy.clone().narrow(0, slot, 1).flatten::<1>(0, 2); // [num_actions]
            let log_p    = log_softmax(logits.clone(), 0);
            let probs    = softmax(logits, 0);
            let cumsum   = probs.cumsum(0);
            let u: f32   = rand::random();
            let idx      = cumsum.lower_equal_elem(u).int().sum().into_scalar() as usize;
            let idx      = idx.min(ACTIONS.len() - 1);
            let action   = ACTIONS[idx];
            let lp       = log_p.narrow(0, idx, 1).into_scalar();
            actions.push(action);
            log_probs.push(lp);
        }
        (actions, log_probs)
    }

    pub(crate) fn value_scalar(&self) -> f32 {
        self.value.clone().narrow(0, 0, 1).flatten::<1>(0, 1).into_scalar()
    }
}

pub(crate) trait Actor<B: Backend>: Module<B> + Sized {
    type Config;
    fn init(cfg: Self::Config, device: &B::Device) -> Self;
    fn forward(&self, cfg: &Cfg, input: Input<B>) -> Output<B>;
    fn cfg(cfg: &Cfg) -> Self::Config;
}
