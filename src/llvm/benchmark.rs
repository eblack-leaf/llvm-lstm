pub(crate) struct Benchmark {
    /// Mean wall-clock time over the benchmark runs, in nanoseconds.
    pub(crate) mean_ns: u64,
    /// Relative speedup vs the baseline: (t_base - t_opt) / t_base.
    /// Set to 0.0 by Llvm::benchmark; filled in by the caller once a
    /// baseline measurement is available.
    pub(crate) speedup: f32,
}
