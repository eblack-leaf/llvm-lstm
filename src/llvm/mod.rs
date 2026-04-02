use crate::llvm::benchmark::Benchmark;
use crate::llvm::functions::Functions;
use crate::llvm::ir::{Bin, Ir, Source};
use crate::llvm::pass::Pass;
use anyhow::Result;
use std::path::PathBuf;

pub(crate) mod benchmark;
pub(crate) mod functions;
pub(crate) mod ir;
pub(crate) mod pass;
#[derive(Clone)]
pub(crate) struct Llvm {
    pub(crate) clang: String,
    pub(crate) opt: String,
}

impl Llvm {
    pub(crate) fn new(clang: &str, opt: &str) -> Self {
        Self {
            clang: clang.to_string(),
            opt: opt.to_string(),
        }
    }
}

impl Llvm {
    pub(crate) fn ir(&self, src: &Source) -> Result<Ir> {
        todo!()
    }
    pub(crate) fn compile(&self, ir: &Ir) -> Result<Bin> {
        todo!()
    }
    pub(crate) fn apply(&self, ir: &Ir, passes: &[Pass]) -> Result<Ir> {
        todo!()
    }
    pub(crate) async fn benchmark(&self, bin: &Bin, runs: usize) -> Result<Benchmark> {
        todo!()
    }
    pub(crate) fn baseline(&self, src: &Source, opt_level: &str, runs: usize) -> Result<Benchmark> {
        // calls .benchmark after different compile step
        todo!()
    }
}
