#[derive(Clone)]
pub(crate) struct Benchmark {
    /// Mean wall-clock time over the benchmark runs, in nanoseconds.
    pub(crate) mean_ns: u64,
    /// Relative speedup vs the chosen baseline: (t_base - t_opt) / t_base.
    /// Set to 0.0 by Llvm::benchmark; filled in by the train loop once the
    /// per-function baselines are available.
    pub(crate) speedup: f32,
}

/// Per-function baseline timings at each standard optimisation level.
/// Collected once before training so episode rewards are comparable across
/// functions and hardware state — no worker contention, no cache effects
/// from interleaving with episode collection.
#[derive(Clone)]
pub(crate) struct Baselines {
    pub(crate) o0: Benchmark,
    pub(crate) o1: Benchmark,
    pub(crate) o2: Benchmark,
    pub(crate) o3: Benchmark,
}

impl Baselines {
    /// Speedup of `opt_ns` relative to O3 — the standard compiler's best effort.
    /// Positive = model beat clang -O3; negative = model was worse.
    pub(crate) fn speedup_vs_o3(&self, opt_ns: u64) -> f32 {
        let base = self.o3.mean_ns as f32;
        if base == 0.0 {
            return 0.0;
        }
        (base - opt_ns as f32) / base
    }
}
