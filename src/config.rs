use crate::ppo::model::transformer::TransformerActor;
use burn::backend::ndarray::NdArrayDevice;
use burn::backend::{Autodiff, NdArray};
use clap::ValueEnum;
use std::path::PathBuf;

pub(crate) type BurnBackend = NdArray;
pub(crate) type BurnDevice = NdArrayDevice;
pub(crate) type BurnAutoDiff = Autodiff<BurnBackend>;
#[cfg(not(feature = "gru"))]
pub(crate) type Arch = TransformerActor<BurnAutoDiff>;
#[cfg(feature = "gru")]
pub(crate) type Arch = GruActor<BurnAutoDiff>;
#[derive(Debug, Default, Clone)]
pub(crate) struct Cfg {
    pub(crate) functions: PathBuf,
    pub(crate) clang: String,
    pub(crate) opt: String,
    pub(crate) epochs: usize,
    pub(crate) ppo_epochs: usize,
    pub(crate) episodes: usize,
    pub(crate) benchmark_runs: usize,
    pub(crate) baseline_runs: usize,
    pub(crate) per_step_benchmark: bool,
    pub(crate) max_seq_len: usize,
    pub(crate) work_dir: PathBuf,
    pub(crate) policy_lr: f64,
    pub(crate) value_lr: f64,
    pub(crate) clip_epsilon: f32,
    pub(crate) value_coef: f32,
    pub(crate) entropy_coef: f32,
}
