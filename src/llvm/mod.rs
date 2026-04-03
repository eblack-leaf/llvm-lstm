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
    pub(crate) work_dir: PathBuf,
}

impl Llvm {
    pub(crate) fn new(clang: &str, opt: &str) -> Self {
        Self {
            clang: clang.to_string(),
            opt: opt.to_string(),
            work_dir: Default::default(),
        }
    }
}

impl Llvm {
    pub(crate) fn with_env(&self, env: PathBuf) -> Self {
        Self {
            clang: self.clang.clone(),
            opt: self.opt.clone(),
            work_dir: env,
        }
    }
    pub(crate) fn ir(&self, src: &Source) -> Result<Ir> {
        todo!()
    }
    pub(crate) async fn apply(&self, ir: &Ir, passes: &[Pass]) -> Result<Ir> {
        todo!()
    }
    pub(crate) async fn compile(&self, ir: &Ir) -> Result<Bin> {
        // tokio::process::Command
        // tokio::fs::write
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
