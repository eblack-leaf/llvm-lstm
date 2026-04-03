use crate::llvm::benchmark::{Baselines, Benchmark};
use crate::llvm::ir::{Bin, Ir, Source};
use crate::llvm::pass::{Pass, to_opt_pipeline};
use anyhow::{Context, Result, bail};
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
    pub(crate) fn new(clang: &str, opt: &str, work_dir: PathBuf) -> Self {
        Self {
            clang: clang.to_string(),
            opt: opt.to_string(),
            work_dir,
        }
    }
    pub(crate) fn with_env(&self, env: PathBuf) -> Self {
        Self {
            clang: self.clang.clone(),
            opt: self.opt.clone(),
            work_dir: env,
        }
    }

    /// Emit unoptimised LLVM IR from a C source file.
    pub(crate) async fn ir(&self, src: &Source) -> Result<Ir> {
        let out = self.work_dir.join("base.ll");
        let status = tokio::process::Command::new(&self.clang)
            .args(["-O0", "-Xclang", "-disable-llvm-optzns", "-emit-llvm", "-S"])
            .arg(&src.file)
            .arg("-o")
            .arg(&out)
            .status()
            .await
            .context("failed to run clang (ir)")?;
        if !status.success() {
            bail!("clang exited with {status}");
        }
        Ok(Ir { file: out })
    }

    /// Apply the full pass sequence to `ir` in one opt invocation.
    /// Used when the complete pass list is known upfront.
    pub(crate) async fn apply(&self, ir: &Ir, passes: &[Pass]) -> Result<Ir> {
        let pipeline = to_opt_pipeline(passes);
        let out = self.work_dir.join("optimized.ll");
        let status = tokio::process::Command::new(&self.opt)
            .arg(format!("-passes={pipeline}"))
            .arg(&ir.file)
            .arg("-S")
            .arg("-o")
            .arg(&out)
            .status()
            .await
            .context("failed to run opt (apply)")?;
        if !status.success() {
            bail!("opt exited with {status}");
        }
        Ok(Ir { file: out })
    }

    /// Apply a single pass to `ir`, writing the result to `out`.
    /// Called every step for incremental IR: O(T) total invocations vs O(T²)
    /// for re-applying the full prefix each step. Also makes the current IR
    /// state available for feature extraction and delta computation.
    pub(crate) async fn apply_one(&self, ir: &Ir, pass: Pass, out: PathBuf) -> Result<Ir> {
        let pipeline = to_opt_pipeline(&[pass]);
        let status = tokio::process::Command::new(&self.opt)
            .arg(format!("-passes={pipeline}"))
            .arg(&ir.file)
            .arg("-S")
            .arg("-o")
            .arg(&out)
            .status()
            .await
            .context("failed to run opt (apply_one)")?;
        if !status.success() {
            bail!("opt exited with {status}");
        }
        Ok(Ir { file: out })
    }

    /// Compile an IR file to a native binary, bypassing clang's own optimisations
    /// so the benchmark reflects only the passes the model applied.
    pub(crate) async fn compile(&self, ir: &Ir) -> Result<Bin> {
        let out = self.work_dir.join("compiled");
        let status = tokio::process::Command::new(&self.clang)
            .args(["-O3", "-Xclang", "-disable-llvm-passes"])
            .arg(&ir.file)
            .arg("-o")
            .arg(&out)
            .arg("-lm")
            .status()
            .await
            .context("failed to run clang (compile)")?;
        if !status.success() {
            bail!("clang exited with {status}");
        }
        Ok(Bin { file: out })
    }

    /// Run `bin` for `runs` repetitions and return mean wall-clock time.
    /// Speedup relative to baseline is computed by the caller once a baseline
    /// measurement is available.
    pub(crate) async fn benchmark(&self, bin: &Bin, runs: usize) -> Result<Benchmark> {
        let mut total_ns: u64 = 0;
        for _ in 0..runs {
            let start = std::time::Instant::now();
            let status = tokio::process::Command::new(&bin.file)
                .status()
                .await
                .context("failed to run benchmark binary")?;
            if !status.success() {
                bail!("benchmark binary exited with {status}");
            }
            total_ns += start.elapsed().as_nanos() as u64;
        }
        Ok(Benchmark { mean_ns: total_ns / runs as u64, speedup: 0.0 })
    }

    /// Collect baselines at all four standard opt levels for a single function.
    /// Run sequentially — no worker contention, no cache pollution from parallel
    /// episode collection. Called once per function before the training epoch loop.
    pub(crate) async fn collect_baselines(&self, src: &Source, runs: usize) -> Result<Baselines> {
        let o0 = self.baseline(src, "-O0", runs).await?;
        let o1 = self.baseline(src, "-O1", runs).await?;
        let o2 = self.baseline(src, "-O2", runs).await?;
        let o3 = self.baseline(src, "-O3", runs).await?;
        Ok(Baselines { o0, o1, o2, o3 })
    }

    /// Compile `src` at `opt_level` (e.g. "-O0", "-O3") and benchmark it.
    /// Returns the raw timing used to compute speedup for model-optimised builds.
    pub(crate) async fn baseline(&self, src: &Source, opt_level: &str, runs: usize) -> Result<Benchmark> {
        let bin_path = self.work_dir.join("baseline");
        let status = tokio::process::Command::new(&self.clang)
            .arg(opt_level)
            .arg(&src.file)
            .arg("-o")
            .arg(&bin_path)
            .arg("-lm")
            .status()
            .await
            .context("failed to compile baseline")?;
        if !status.success() {
            bail!("clang baseline exited with {status}");
        }
        self.benchmark(&Bin { file: bin_path }, runs).await
    }
}
