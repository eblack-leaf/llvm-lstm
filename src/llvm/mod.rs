use crate::llvm::benchmark::{Baselines, Benchmark};
use crate::llvm::ir::{Bin, Ir, Source};
use crate::llvm::pass::{Pass, opt_pipeline};
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
    #[allow(unused)]
    pub(crate) async fn apply(&self, ir: &Ir, passes: &[Pass]) -> Result<Ir> {
        let pipeline = opt_pipeline(passes);
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

    /// Apply a single pass to `ir` at the given step index.
    /// Called every step for incremental IR: O(T) total invocations vs O(T²)
    /// for re-applying the full prefix each step. Also makes the current IR
    /// state available for feature extraction and delta computation.
    pub(crate) async fn apply_one(&self, ir: &Ir, pass: Pass, step: usize) -> Result<Ir> {
        let out = self.work_dir.join(format!("step_{step}.ll"));
        let pipeline = opt_pipeline(&[pass]);
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

    /// Run `bin` `runs` times, passing `iters` to the binary each invocation as
    /// the inner iteration count for `bench_timing.h`. Returns the mean of the
    /// per-invocation trimmed-mean nanosecond times reported to stdout.
    pub(crate) async fn benchmark(&self, bin: &Bin, runs: usize, iters: usize) -> Result<Benchmark> {
        let mut total_ns: u64 = 0;
        for _ in 0..runs {
            let output = tokio::process::Command::new(&bin.file)
                .arg(iters.to_string())
                .output()
                .await
                .context("failed to run benchmark binary")?;
            if !output.status.success() {
                bail!("benchmark binary exited with {}", output.status);
            }
            let stdout = std::str::from_utf8(&output.stdout)
                .context("benchmark output was not valid UTF-8")?
                .trim();
            let ns: u64 = stdout
                .parse()
                .with_context(|| format!("could not parse benchmark output as u64: {stdout:?}"))?;
            total_ns += ns;
        }
        Ok(Benchmark {
            mean_ns: total_ns / runs.max(1) as u64,
            speedup: 0.0,
        })
    }

    /// Collect baselines at all four standard opt levels for a single function.
    /// Run sequentially — no worker contention, no cache pollution from parallel
    /// episode collection. Called once per function before the training epoch loop.
    pub(crate) async fn collect_baselines(&self, src: &Source, runs: usize, iters: usize) -> Result<Baselines> {
        let o0 = self.baseline(src, "-O0", runs, iters).await?;
        let o1 = self.baseline(src, "-O1", runs, iters).await?;
        let o2 = self.baseline(src, "-O2", runs, iters).await?;
        let o3 = self.baseline(src, "-O3", runs, iters).await?;
        Ok(Baselines { o0, o1, o2, o3 })
    }

    /// Compile `src` at `opt_level` (e.g. "-O0", "-O3") and benchmark it.
    /// Returns the raw timing used to compute speedup for model-optimised builds.
    pub(crate) async fn baseline(
        &self,
        src: &Source,
        opt_level: &str,
        runs: usize,
        iters: usize,
    ) -> Result<Benchmark> {
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
        self.benchmark(&Bin { file: bin_path }, runs, iters).await
    }
}
