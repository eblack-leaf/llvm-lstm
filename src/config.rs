use burn::backend::ndarray::NdArrayDevice;
use burn::backend::{Autodiff, NdArray};
use clap::ValueEnum;
use std::path::PathBuf;
pub(crate) type BurnBackend = NdArray;
pub(crate) type BurnDevice = NdArrayDevice;
pub(crate) type BurnAutoDiff = Autodiff<BurnBackend>;
#[derive(Debug, Default, Clone)]
pub(crate) struct Cfg {
    pub(crate) functions: PathBuf,
    pub(crate) clang: String,
    pub(crate) opt: String,
    pub(crate) epochs: usize,
    pub(crate) ppo_epochs: usize,
    pub(crate) episodes: usize,
    pub(crate) benchmark_runs: usize,
    pub(crate) per_step_benchmark: bool,
    pub(crate) max_seq_len: usize,
    pub(crate) work_dir: PathBuf,
    pub(crate) arch: Arch,
    pub(crate) policy_lr: f64,
    pub(crate) value_lr: f64,
}
#[derive(Debug, Default, Copy, Clone, ValueEnum)]
pub(crate) enum Arch {
    Gru,
    #[default]
    Tfx,
}
