use crate::llvm::benchmark::Benchmark;
use crate::llvm::pass::Pass;

pub(crate) struct Step {
    pub(crate) pass: Pass,
    /// Zero-based position in the episode sequence.
    pub(crate) step_idx: usize,
    /// None between benchmarks; Some when per_step_benchmark is enabled or at episode end.
    pub(crate) benchmark: Option<Benchmark>,
    /// Element-wise difference of the 34-dim IR feature vectors: current - base.
    /// Zero at step 0 (no passes applied); grows as passes structurally change the IR.
    /// Encodes what the sequence of passes has accomplished so far — useful for
    /// attribution methods that want to correlate IR changes with episode reward.
    pub(crate) delta_features: Vec<f32>,
}
impl Step {
    pub(crate) fn new(
        pass: Pass,
        step_idx: usize,
        benchmark: Option<Benchmark>,
        delta_features: Vec<f32>,
    ) -> Self {
        Self {
            pass,
            step_idx,
            benchmark,
            delta_features,
        }
    }
}
