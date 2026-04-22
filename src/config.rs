use burn::backend::Autodiff;
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

// ── Autoregressive architectures ─────────────────────────────────────────────
// auto-gru takes precedence when both features are active (e.g. --no-default-features --features auto-gru).
#[cfg(all(feature = "auto-tfx", not(feature = "auto-gru")))]
pub(crate) type Arch = crate::ppo::model::auto_tfx::AutoTfxActor<BurnAutoDiff>;
#[cfg(all(feature = "auto-tfx", not(feature = "auto-gru")))]
pub(crate) type ArchConfig = crate::ppo::model::auto_tfx::AutoTfxConfig;

#[cfg(feature = "auto-gru")]
pub(crate) type Arch = crate::ppo::model::auto_gru::AutoGruActor<BurnAutoDiff>;
#[cfg(feature = "auto-gru")]
pub(crate) type ArchConfig = crate::ppo::model::auto_gru::AutoGruConfig;

// ── Unified init/cfg helpers ──────────────────────────────────────────────────

pub(crate) fn arch_init(cfg: ArchConfig, device: &BurnDevice) -> Arch {
    use crate::ppo::model::AutoActor;
    <Arch as AutoActor<BurnAutoDiff>>::init(cfg, device)
}

pub(crate) fn arch_cfg(cfg: &Cfg) -> ArchConfig {
    use crate::ppo::model::AutoActor;
    <Arch as AutoActor<BurnAutoDiff>>::cfg(cfg)
}

#[derive(Debug, Clone)]
pub(crate) struct Cfg {
    pub(crate) functions: PathBuf,
    pub(crate) clang: String,
    pub(crate) opt: String,
    pub(crate) epochs: usize,
    pub(crate) ppo_epochs: usize,
    pub(crate) episodes: usize,
    pub(crate) benchmark_runs: usize,
    pub(crate) benchmark_iters: usize,
    pub(crate) baseline_runs: usize,
    pub(crate) baseline_iters: usize,
    pub(crate) max_seq_len: usize,
    pub(crate) work_dir: PathBuf,
    pub(crate) checkpoint_dir: PathBuf,
    pub(crate) learning_rate: f64,
    pub(crate) clip_epsilon: f32,
    pub(crate) value_coef: f32,
    pub(crate) entropy_coef: f32,
    pub(crate) mini_batch_size: usize,
    pub(crate) cache_file: Option<PathBuf>,
    pub(crate) noop: crate::ppo::noop::NoOp,
    pub(crate) ir_chunks: usize,
    pub(crate) skip_benchmark: bool,
    pub(crate) kl_target: f32,
}
