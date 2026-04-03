pub(crate) struct Benchmark {
    // Relative speedup vs the unoptimised baseline: (t_base - t_opt) / t_base.
    // Positive = faster, negative = pessimisation.
    pub(crate) speedup: f32,
}
