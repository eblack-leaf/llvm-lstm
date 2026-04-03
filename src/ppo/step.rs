use crate::llvm::benchmark::Benchmark;
use crate::llvm::pass::Pass;

pub(crate) struct Step {
    pub(crate) pass: Pass,
    /// Zero-based position in the episode sequence.
    pub(crate) step_idx: usize,
    /// None between benchmarks; Some when per_step_benchmark is enabled or at episode end.
    pub(crate) benchmark: Option<Benchmark>,
    // metadata about the step, to inform later steps
}
impl Step {
    pub(crate) fn new(pass: Pass, step_idx: usize, benchmark: Option<Benchmark>) -> Self {
        Self { pass, step_idx, benchmark }
    }
}
