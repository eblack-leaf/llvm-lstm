#[cfg(feature = "auto-gru")]
pub(crate) mod auto_gru;
#[cfg(feature = "auto-tfx")]
pub(crate) mod auto_tfx;
pub(crate) mod conclave;
pub(crate) mod seq;

use crate::config::Cfg;
use crate::config::{BurnBackend, BurnDevice};
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

/// Model input: chunked IR opcode histogram for N episodes.
pub(crate) struct Input<B: Backend> {
    /// [N, ir_chunks * IR_CATEGORY_COUNT] — pre-computed chunked opcode histogram.
    pub(crate) ir_features: Tensor<B, 2>,
}

impl Input<BurnBackend> {
    /// Build single-episode input (N=1) from a pre-computed feature vector.
    pub(crate) fn new_slots(dev: &BurnDevice, features: &[f32]) -> Self {
        let dim = features.len();
        Self {
            ir_features: Tensor::from_data(TensorData::new(features.to_vec(), [1, dim]), dev),
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

/// Autoregressive actor: processes one step at a time, sees the *actual updated IR*
/// after each applied pass instead of the fixed initial IR histogram.
///
/// Collection uses `infer_step_stateful` (O(K) for GRU, O(K²) for TFX — inherent).
/// PPO update uses `replay_batch` (one batched call per mini-batch for both models).
pub(crate) trait AutoActor<B: Backend<FloatElem = f32>>: Module<B> + Sized {
    type Config: Clone;
    fn init(cfg: Self::Config, device: &B::Device) -> Self;
    fn cfg(cfg: &Cfg) -> Self::Config;

    /// Stateful single-step inference for O(K) collection.
    ///
    /// * `ir_features_so_far` — IR histograms steps 0..=t; `[t]` is current IR.
    /// * `taken_actions`      — action indices chosen at steps 0..t-1 (length = t).
    /// * `hidden`             — opaque hidden state from the previous call, `None` at step 0.
    ///
    /// Returns `(logits[num_actions], value, new_hidden)`.
    ///
    /// Default: delegates to `infer_step` (correct but O(K²) for stateful models).
    /// GRU overrides with a single recurrent step (O(1)), threading `hidden` as the
    /// serialised GRU hidden vector.
    fn infer_step_stateful(
        &self,
        ir_features_so_far: &[Vec<f32>],
        taken_actions: &[usize],
        hidden: Option<Vec<f32>>,
        device: &B::Device,
    ) -> (Vec<f32>, f32, Option<Vec<f32>>) {
        let (logits, value) = self.infer_step(ir_features_so_far, taken_actions, device);
        (logits, value, None)
    }

    /// Single-episode inference (used internally and for debugging).
    fn infer_step(
        &self,
        ir_features_so_far: &[Vec<f32>],
        taken_actions: &[usize],
        device: &B::Device,
    ) -> (Vec<f32>, f32);

    /// Single-episode training replay (used internally by `replay_batch` default impl).
    fn replay_episode(
        &self,
        ir_features_per_step: &[Vec<f32>],
        taken_actions: &[usize],
        device: &B::Device,
    ) -> (Tensor<B, 2>, Tensor<B, 1>);

    /// Batched PPO replay for a full mini-batch.
    ///
    /// * `batch_ir_features`   — per-episode IR histograms; `[i][t]` = features before step t.
    /// * `batch_taken_actions` — per-episode action indices; `[i][t]` = action taken at step t.
    ///
    /// Returns `(logits [Σep_len, A], values [Σep_len])` in episode-contiguous flat order.
    ///
    /// Default: serial loop over `replay_episode` (correct, but no cross-episode batching).
    /// Both GRU and TFX override this with efficient batched implementations.
    fn replay_batch(
        &self,
        batch_ir_features: &[&[Vec<f32>]],
        batch_taken_actions: &[&[usize]],
        device: &B::Device,
    ) -> (Tensor<B, 2>, Tensor<B, 1>) {
        let mut logits_list = Vec::new();
        let mut values_list = Vec::new();
        for (&ir, &acts) in batch_ir_features.iter().zip(batch_taken_actions.iter()) {
            let (l, v) = self.replay_episode(ir, acts, device);
            logits_list.push(l);
            values_list.push(v);
        }
        (Tensor::cat(logits_list, 0), Tensor::cat(values_list, 0))
    }
}
