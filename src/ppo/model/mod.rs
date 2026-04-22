#[cfg(feature = "auto-gru")]
pub(crate) mod auto_gru;
#[cfg(feature = "auto-tfx")]
pub(crate) mod auto_tfx;

use crate::config::Cfg;
use crate::llvm::pass::Pass;
use burn::Tensor;
use burn::nn::{Linear, LinearConfig};
use burn::prelude::{Backend, Module};

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

/// Model input: IR feature vector for a batch of episodes.
pub(crate) struct Input<B: Backend> {
    pub(crate) ir_features: Tensor<B, 2>,
}

/// Autoregressive actor: processes one step at a time, sees the actual updated IR
/// after each applied pass instead of a fixed initial snapshot.
///
/// Collection uses `infer_step_stateful` (O(K) for GRU, O(K²) for TFX).
/// PPO update uses `replay_batch` (one batched call per mini-batch).
pub(crate) trait AutoActor<B: Backend<FloatElem = f32>>: Module<B> + Sized {
    type Config: Clone;
    fn init(cfg: Self::Config, device: &B::Device) -> Self;
    fn cfg(cfg: &Cfg) -> Self::Config;

    /// Stateful single-step inference for O(K) collection.
    ///
    /// * `ir_features_so_far` — IR histograms steps 0..=t; `[t]` is current IR.
    /// * `taken_actions`      — action indices chosen at steps 0..t-1.
    /// * `hidden`             — opaque hidden state from the previous call, `None` at step 0.
    ///
    /// Returns `(logits[num_actions], value, new_hidden)`.
    /// GRU overrides with a single recurrent step O(1); TFX uses the default O(K²) replay.
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

    fn infer_step(
        &self,
        ir_features_so_far: &[Vec<f32>],
        taken_actions: &[usize],
        device: &B::Device,
    ) -> (Vec<f32>, f32);

    fn replay_episode(
        &self,
        ir_features_per_step: &[Vec<f32>],
        taken_actions: &[usize],
        device: &B::Device,
    ) -> (Tensor<B, 2>, Tensor<B, 1>);

    /// Batched PPO replay for a full mini-batch.
    /// Default: serial loop over `replay_episode`.
    /// Both GRU and TFX override with efficient batched implementations.
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
