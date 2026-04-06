use crate::llvm::benchmark::Baselines;
use crate::llvm::pass::Pass;

/// Results produced by one episode — consumed by Returns, Advantages, and metrics.
pub(crate) struct Results {
    pub(crate) func_name: String,
    pub(crate) bench_cache_hits: u64,
    pub(crate) bench_cache_misses: u64,
    /// 34-dim log-transformed IR feature vector for the base (unoptimised) IR.
    pub(crate) ir_features: Vec<f32>,
    /// K actions sampled (one per slot). Stop = no-op, doesn't change IR.
    pub(crate) actions: Vec<Pass>,
    /// K log-probabilities, one per sampled action.
    pub(crate) log_probs: Vec<f32>,
    /// V(base_IR) — single value estimate for the whole episode.
    pub(crate) value: f32,
    /// Terminal speedup (vs -O3) after applying all non-Stop actions.
    pub(crate) episode_return: f32,
    pub(crate) baselines: Baselines,
}
