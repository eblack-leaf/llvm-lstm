use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::llvm::benchmark::Benchmark;
use crate::llvm::ir::{Bin, Ir, Source};
use crate::llvm::pass::Pass;

mod ir;
mod pass;
mod benchmark;

pub(crate) struct Llvm {
    pub(crate) clang: String,
    pub(crate) opt: String,
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
    pub(crate) fn benchmark(&self, bin: &Bin, runs: usize) -> Result<Benchmark> {
        todo!()
    }
    pub(crate) fn baseline(&self, src: &Source, opt_level: &str, runs: usize) -> Result<Benchmark> {
        // calls .benchmark after different compile step
        todo!()
    }
}