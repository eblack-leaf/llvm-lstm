use std::path::PathBuf;

#[derive(Debug, Default, Clone)]
pub(crate) struct Cfg {
    // llvm
    pub(crate) functions: PathBuf,
    pub(crate) clang: String,
    pub(crate) opt: String,
    // ppo
    pub(crate) epochs: usize,
    pub(crate) episodes: usize,
    pub(crate) benchmark_runs: usize,
}
