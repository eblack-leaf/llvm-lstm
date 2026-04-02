use std::path::PathBuf;

#[derive(Debug, Default)]
pub(crate) struct Cfg {
    // llvm
    pub(crate) functions: PathBuf,
    pub(crate) clang: String,
    pub(crate) opt: String,
    // ppo
}