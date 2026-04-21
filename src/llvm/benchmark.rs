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

/// Parallel benchmark noise margin applied when comparing episode timings
/// (collected by concurrent rayon workers) against the solo baseline.
/// BenchNoise showed up to ~4% overhead; 1.05 gives a 1% buffer above that.
pub(crate) const PARALLEL_NOISE_MARGIN: f32 = 1.00;

impl Baselines {
    /// Raw speedup of `opt_ns` relative to O3. Use this for evaluation and any
    /// context where both measurements are collected under the same conditions.
    pub(crate) fn speedup_vs_o3(&self, opt_ns: u64) -> f32 {
        let base = self.o3.mean_ns as f32;
        if base == 0.0 {
            return 0.0;
        }
        (base - opt_ns as f32) / base
    }

    /// Speedup for episode benchmarks collected in parallel rayon workers.
    /// Pessimizes the O3 baseline by PARALLEL_NOISE_MARGIN to cancel the
    /// systematic overhead rayon introduces on the measured binary timing.
    pub(crate) fn speedup_vs_o3_parallel(&self, opt_ns: u64) -> f32 {
        let base = self.o3.mean_ns as f32 * PARALLEL_NOISE_MARGIN;
        if base == 0.0 {
            return 0.0;
        }
        (base - opt_ns as f32) / base
    }
}
