use crate::llvm::benchmark::{Baselines, Benchmark};
use crate::llvm::ir::{Bin, Ir, Source};
use crate::llvm::pass::{Pass, opt_pipeline};
use anyhow::{Context, Result, bail};
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Shared cache mapping (blake3 hash of IR content, pass index) to speedup.
/// Keyed on content hash so episodes that reach the same IR state via different
/// paths still get cache hits. Pass index is the position in ACTIONS.
pub(crate) type LookaheadCache = Arc<DashMap<([u8; 32], u8), f32>>;

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
            .args(["-O3", "-Xclang", "-disable-llvm-optzns", "-emit-llvm", "-S"])
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

    /// Apply a single pass for lookahead purposes. Writes to a temp file named
    /// `lookahead_{step}_{pass_idx}.ll` so it doesn't collide with episode step files.
    pub(crate) async fn apply_one_lookahead(&self, ir: &Ir, pass: Pass, step: usize, pass_idx: usize) -> Result<Ir> {
        let out = self.work_dir.join(format!("lookahead_{step}_{pass_idx}.ll"));
        let pipeline = opt_pipeline(&[pass]);
        let status = tokio::process::Command::new(&self.opt)
            .arg(format!("-passes={pipeline}"))
            .arg(&ir.file)
            .arg("-S")
            .arg("-o")
            .arg(&out)
            .status()
            .await
            .context("failed to run opt (apply_one_lookahead)")?;
        if !status.success() {
            bail!("opt exited with {status}");
        }
        Ok(Ir { file: out })
    }

    /// Bench a lookahead candidate, returning a cached speedup if the same
    /// (IR content, pass_idx) has been seen before this epoch.
    /// On a miss: applies the pass, compiles, benches, stores in cache, returns speedup.
    pub(crate) async fn bench_lookahead_cached(
        &self,
        ir: &Ir,
        pass: Pass,
        pass_idx: usize,
        step: usize,
        baselines: &Baselines,
        runs: usize,
        iters: usize,
        cache: &LookaheadCache,
    ) -> Result<f32> {
        // Hash the IR content — same content == same state regardless of path.
        let content = tokio::fs::read(&ir.file).await.context("read IR for hash")?;
        let hash: [u8; 32] = *blake3::hash(&content).as_bytes();
        let key = (hash, pass_idx as u8);

        if let Some(cached) = cache.get(&key) {
            return Ok(*cached);
        }

        let out_ir = if pass == Pass::Stop {
            ir.clone()
        } else {
            self.apply_one_lookahead(ir, pass, step, pass_idx).await?
        };
        let bin = self.compile_lookahead(&out_ir, step, pass_idx).await?;
        let mut bm = self.benchmark(&bin, runs, iters).await?;
        bm.speedup = baselines.speedup_vs_o3(bm.mean_ns);
        cache.insert(key, bm.speedup);
        Ok(bm.speedup)
    }

    /// Compile a lookahead IR to a binary. Output named `lookahead_{step}_{pass_idx}_bin`
    /// to avoid colliding with the episode's compiled binary.
    pub(crate) async fn compile_lookahead(&self, ir: &Ir, step: usize, pass_idx: usize) -> Result<Bin> {
        let out = self.work_dir.join(format!("lookahead_{step}_{pass_idx}_bin"));
        let status = tokio::process::Command::new(&self.clang)
            .args(["-O3", "-Xclang", "-disable-llvm-passes"])
            .arg(&ir.file)
            .arg("-o")
            .arg(&out)
            .arg("-lm")
            .status()
            .await
            .context("failed to run clang (compile_lookahead)")?;
        if !status.success() {
            bail!("clang exited with {status}");
        }
        Ok(Bin { file: out })
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
