use burn::backend::{Autodiff};
use std::path::PathBuf;
#[cfg(not(feature = "wgpu"))]
pub(crate) type BurnBackend = burn::backend::NdArray;
#[cfg(not(feature = "wgpu"))]
pub(crate) type BurnDevice = burn::backend::ndarray::NdArrayDevice;
#[cfg(feature = "wgpu")]
pub(crate) type BurnBackend = burn::backend::Wgpu;
#[cfg(feature = "wgpu")]
pub(crate) type BurnDevice = burn::backend::wgpu::WgpuDevice;
pub(crate) type BurnAutoDiff = Autodiff<BurnBackend>;
#[cfg(not(feature = "gru"))]
pub(crate) type Arch = crate::ppo::model::transformer::TransformerActor<BurnAutoDiff>;
#[cfg(feature = "gru")]
pub(crate) type Arch = crate::ppo::model::gru::GruActor<BurnAutoDiff>;
#[cfg(not(feature = "gru"))]
pub(crate) type ArchConfig = crate::ppo::model::transformer::TransformerActorConfig;
#[cfg(feature = "gru")]
pub(crate) type ArchConfig = crate::ppo::model::gru::GruActorConfig;
#[derive(Debug, Clone)]
pub(crate) struct Cfg {
    pub(crate) functions: PathBuf,
    pub(crate) clang: String,
    pub(crate) opt: String,
    pub(crate) epochs: usize,
    pub(crate) ppo_epochs: usize,
    pub(crate) episodes: usize,
    /// How many times to invoke the benchmark binary per measurement (outer average).
    pub(crate) benchmark_runs: usize,
    /// Iteration count passed to bench_timing.h during episode benchmarking.
    pub(crate) benchmark_iters: usize,
    pub(crate) baseline_runs: usize,
    /// Iteration count passed to bench_timing.h during baseline collection.
    /// Higher than benchmark_iters — baselines are the fixed reference so accuracy matters more.
    pub(crate) baseline_iters: usize,
    pub(crate) per_step_benchmark: bool,
    pub(crate) max_seq_len: usize,
    pub(crate) work_dir: PathBuf,
    pub(crate) checkpoint_dir: PathBuf,
    pub(crate) learning_rate: f64,
    pub(crate) clip_epsilon: f32,
    pub(crate) value_coef: f32,
    pub(crate) entropy_coef: f32,
    pub(crate) mini_batch_size: usize,
}
