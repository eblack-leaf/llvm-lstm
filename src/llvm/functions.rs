use std::path::PathBuf;
use crate::llvm::ir::{Ir, Source};
#[derive(Clone)]
pub(crate) struct Functions {
    pub(crate) functions: Vec<Function>,
}
impl Functions {
    pub(crate) fn new(dir: &PathBuf) -> Self {
        // read dir
        // parse into funcs with metadata
        let functions = vec![];
        Self {
            functions
        }
    }
}
#[derive(Clone)]
pub(crate) struct Function {
    pub(crate) name: String,
    pub(crate) source: Source,
    pub(crate) ir: Ir,
}