use burn::backend::ndarray::NdArrayDevice;
use burn::backend::{Autodiff, NdArray};
use clap::ValueEnum;
use std::path::PathBuf;
pub(crate) type Backend = NdArray;
pub(crate) type Dev = NdArrayDevice;
pub(crate) type Diff = Autodiff<Backend>;
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
    pub(crate) actor_arch: ActorArch,
}
#[derive(Debug, Default, Clone, ValueEnum)]
pub(crate) enum ActorArch {
    Gru,
    #[default]
    Tfx,
}
