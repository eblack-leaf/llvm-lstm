use crate::llvm::benchmark::Benchmark;

pub(crate) struct Step {
    benchmark: Benchmark,
    // metadata about the step, to inform later steps
}
impl Step {
    pub(crate) fn new(benchmark: Benchmark) -> Self {
        Self { benchmark }
    }
}
