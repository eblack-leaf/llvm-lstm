use crate::llvm::benchmark::Benchmark;
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
    pub(crate) work_dir: PathBuf, // used for output paths
}

impl Llvm {
    pub(crate) fn new(clang: &str, opt: &str, work_dir: PathBuf) -> Self {
        Self {
            clang: clang.to_string(),
            opt: opt.to_string(),
            work_dir,
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
        // like below but tokio::process::Command
        // let output = Command::new(&self.clang)
        //     .args(["-O3", "-Xclang", "-disable-llvm-optzns", "-emit-llvm", "-S"])
        //     .arg(c_source)
        //     .arg("-o")
        //     .arg(&ll_path)
        //     .output()
        //     .context("failed to run clang")?;
        todo!()
    }
    pub(crate) async fn apply(&self, ir: &Ir, passes: &[Pass]) -> Result<Ir> {
        // like below but tokio::process::Command
        // let output = Command::new(&self.opt)
        //     .arg(format!("-passes={pipeline}"))
        //     .arg(ir)
        //     .arg("-S")
        //     .arg("-o")
        //     .arg(out)
        //     .output()
        //     .context("failed to run opt")?;
        todo!()
    }
    pub(crate) async fn compile(&self, ir: &Ir) -> Result<Bin> {
        // tokio::process::Command
        // tokio::fs::write
        // let output = Command::new(&self.clang)
        //     .args(["-O3", "-Xclang", "-disable-llvm-passes"])
        //     .arg(ir)
        //     .arg("-o")
        //     .arg(&bin_path)
        //     .arg("-lm")
        //     .output()
        //     .context("failed to compile IR")?;
        todo!()
    }
    pub(crate) async fn benchmark(&self, bin: &Bin, runs: usize) -> Result<Benchmark> {
        todo!()
    }
    pub(crate) fn baseline(&self, src: &Source, opt_level: &str, runs: usize) -> Result<Benchmark> {
        // calls .benchmark after different compile step
        // let output = Command::new(&self.clang)
        //             .arg(opt_level) // e.g., "-O0", "-O2", "-O3"
        //             .arg(c_source)
        //             .arg("-o")
        //             .arg(&bin_path)
        //             .arg("-lm")
        //             .output()
        //             .context("failed to compile baseline")?;
        todo!()
    }
}
