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

// ── Parallel architectures ────────────────────────────────────────────────────
#[cfg(not(any(feature = "conclave", feature = "auto-tfx", feature = "auto-gru")))]
pub(crate) type Arch = crate::ppo::model::seq::SeqActor<BurnAutoDiff>;
#[cfg(not(any(feature = "conclave", feature = "auto-tfx", feature = "auto-gru")))]
pub(crate) type ArchConfig = crate::ppo::model::seq::SeqActorConfig;

#[cfg(all(feature = "conclave", not(any(feature = "auto-tfx", feature = "auto-gru"))))]
pub(crate) type Arch = crate::ppo::model::conclave::ConclaveActor<BurnAutoDiff>;
#[cfg(all(feature = "conclave", not(any(feature = "auto-tfx", feature = "auto-gru"))))]
pub(crate) type ArchConfig = crate::ppo::model::conclave::ConclaveActorConfig;

// ── Autoregressive architectures ─────────────────────────────────────────────
#[cfg(feature = "auto-tfx")]
pub(crate) type Arch = crate::ppo::model::auto_tfx::AutoTfxActor<BurnAutoDiff>;
#[cfg(feature = "auto-tfx")]
pub(crate) type ArchConfig = crate::ppo::model::auto_tfx::AutoTfxConfig;

#[cfg(feature = "auto-gru")]
pub(crate) type Arch = crate::ppo::model::auto_gru::AutoGruActor<BurnAutoDiff>;
#[cfg(feature = "auto-gru")]
pub(crate) type ArchConfig = crate::ppo::model::auto_gru::AutoGruConfig;

// ── Unified init/cfg helpers (called from train.rs and checkpoint.rs) ─────────

/// Initialise the Arch model. Dispatches to the correct trait (Actor vs AutoActor).
pub(crate) fn arch_init(cfg: ArchConfig, device: &BurnDevice) -> Arch {
    #[cfg(not(any(feature = "auto-tfx", feature = "auto-gru")))]
    {
        use crate::ppo::model::Actor;
        <Arch as Actor<BurnAutoDiff>>::init(cfg, device)
    }
    #[cfg(any(feature = "auto-tfx", feature = "auto-gru"))]
    {
        use crate::ppo::model::AutoActor;
        <Arch as AutoActor<BurnAutoDiff>>::init(cfg, device)
    }
}

/// Build the ArchConfig from the training Cfg.
pub(crate) fn arch_cfg(cfg: &Cfg) -> ArchConfig {
    #[cfg(not(any(feature = "auto-tfx", feature = "auto-gru")))]
    {
        use crate::ppo::model::Actor;
        <Arch as Actor<BurnAutoDiff>>::cfg(cfg)
    }
    #[cfg(any(feature = "auto-tfx", feature = "auto-gru"))]
    {
        use crate::ppo::model::AutoActor;
        <Arch as AutoActor<BurnAutoDiff>>::cfg(cfg)
    }
}

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
    pub(crate) baseline_iters: usize,
    /// Fixed number of pass slots per episode. Stop = no-op (doesn't change IR).
    pub(crate) max_seq_len: usize,
    pub(crate) work_dir: PathBuf,
    pub(crate) checkpoint_dir: PathBuf,
    pub(crate) learning_rate: f64,
    pub(crate) clip_epsilon: f32,
    pub(crate) value_coef: f32,
    pub(crate) entropy_coef: f32,
    /// Number of episodes per PPO mini-batch.
    pub(crate) mini_batch_size: usize,
    /// Path to load/save the benchmark cache across runs.
    pub(crate) cache_file: Option<PathBuf>,
    pub(crate) noop: crate::ppo::noop::NoOp,
    /// Number of positional chunks used for the IR feature vector.
    /// Each chunk holds a normalised opcode-frequency histogram (IR_CATEGORY_COUNT = 12 bins).
    /// Total IR feature dim = ir_chunks * 64.  Default 4 → 256-dim vector.
    pub(crate) ir_chunks: usize,
    /// When true, skip per-episode benchmarking and use IR-count reduction as `episode_return`.
    pub(crate) skip_benchmark: bool,
    /// PPO inner-loop KL early-stop threshold (0 = disabled).
    pub(crate) kl_target: f32,
}
