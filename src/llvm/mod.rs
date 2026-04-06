use crate::llvm::benchmark::{Baselines, Benchmark};
use crate::llvm::ir::{Bin, Ir, Source};
use crate::llvm::pass::{Pass, opt_pipeline};
use anyhow::{Context, Result, bail};
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Shared cache mapping (func_name, pass_sequence) → speedup.
/// Keyed on the actual sequence of passes applied so repeated sequences across
/// episodes and epochs never re-benchmark the same combination.
/// Stop is stripped from the key since it produces no IR change.
pub(crate) type BenchCache = Arc<DashMap<(String, Vec<Pass>), f32>>;

/// Persist the cache to disk.
pub(crate) fn save_cache(cache: &BenchCache, path: &std::path::Path) -> Result<()> {
    let entries: Vec<((String, Vec<Pass>), f32)> =
        cache.iter().map(|e| (e.key().clone(), *e.value())).collect();
    let bytes = bincode::serialize(&entries).context("serialize bench cache")?;
    std::fs::write(path, bytes).context("write bench cache")?;
    Ok(())
}

/// Load a previously saved cache from disk. Returns an empty cache if the file
/// does not exist.
pub(crate) fn load_cache(path: &std::path::Path) -> Result<BenchCache> {
    let cache = Arc::new(DashMap::new());
    match std::fs::read(path) {
        Ok(bytes) => {
            let entries: Vec<((String, Vec<Pass>), f32)> =
                bincode::deserialize(&bytes).context("deserialize bench cache")?;
            for (key, value) in entries {
                cache.insert(key, value);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).context("read bench cache"),
    }
    Ok(cache)
}

pub(crate) mod benchmark;
pub(crate) mod functions;
pub(crate) mod ir;
pub(crate) mod pass;
pub(crate) mod top_sequences;

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
    pub(crate) fn ir(&self, src: &Source) -> Result<Ir> {
        let out = self.work_dir.join("base.ll");
        let status = std::process::Command::new(&self.clang)
            .args(["-O3", "-Xclang", "-disable-llvm-optzns", "-emit-llvm", "-S"])
            .arg(&src.file)
            .arg("-o")
            .arg(&out)
            .status()
            .context("failed to run clang (ir)")?;
        if !status.success() {
            bail!("clang exited with {status}");
        }
        Ok(Ir { file: out })
    }

    /// Apply the full pass sequence to `ir` in one opt invocation.
    #[allow(unused)]
    pub(crate) fn apply(&self, ir: &Ir, passes: &[Pass]) -> Result<Ir> {
        let pipeline = opt_pipeline(passes);
        let out = self.work_dir.join("optimized.ll");
        let status = std::process::Command::new(&self.opt)
            .arg(format!("-passes={pipeline}"))
            .arg(&ir.file)
            .arg("-S")
            .arg("-o")
            .arg(&out)
            .status()
            .context("failed to run opt (apply)")?;
        if !status.success() {
            bail!("opt exited with {status}");
        }
        Ok(Ir { file: out })
    }

    /// Apply a single pass to `ir` at the given step index.
    pub(crate) fn apply_one(&self, ir: &Ir, pass: Pass, step: usize) -> Result<Ir> {
        let out = self.work_dir.join(format!("step_{step}.ll"));
        let pipeline = opt_pipeline(&[pass]);
        let status = std::process::Command::new(&self.opt)
            .arg(format!("-passes={pipeline}"))
            .arg(&ir.file)
            .arg("-S")
            .arg("-o")
            .arg(&out)
            .status()
            .context("failed to run opt (apply_one)")?;
        if !status.success() {
            bail!("opt exited with {status}");
        }
        Ok(Ir { file: out })
    }

    /// Compile an IR file to a native binary, bypassing clang's own optimisations.
    pub(crate) fn compile(&self, ir: &Ir) -> Result<Bin> {
        let out = self.work_dir.join("compiled");
        let status = std::process::Command::new(&self.clang)
            .args(["-O3", "-Xclang", "-disable-llvm-passes"])
            .arg(&ir.file)
            .arg("-o")
            .arg(&out)
            .arg("-lm")
            .status()
            .context("failed to run clang (compile)")?;
        if !status.success() {
            bail!("clang exited with {status}");
        }
        Ok(Bin { file: out })
    }

    /// Run `bin` `runs` times. Returns the mean nanosecond time.
    pub(crate) fn benchmark(&self, bin: &Bin, runs: usize, iters: usize) -> Result<Benchmark> {
        let mut total_ns: u64 = 0;
        for _ in 0..runs {
            let output = std::process::Command::new(&bin.file)
                .arg(iters.to_string())
                .output()
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
    pub(crate) fn collect_baselines(&self, src: &Source, runs: usize, iters: usize) -> Result<Baselines> {
        let o0 = self.baseline(src, "-O0", runs, iters)?;
        let o1 = self.baseline(src, "-O1", runs, iters)?;
        let o2 = self.baseline(src, "-O2", runs, iters)?;
        let o3 = self.baseline(src, "-O3", runs, iters)?;
        Ok(Baselines { o0, o1, o2, o3 })
    }

    pub(crate) fn baseline(
        &self,
        src: &Source,
        opt_level: &str,
        runs: usize,
        iters: usize,
    ) -> Result<Benchmark> {
        let bin_path = self.work_dir.join("baseline");
        let status = std::process::Command::new(&self.clang)
            .arg(opt_level)
            .arg(&src.file)
            .arg("-o")
            .arg(&bin_path)
            .arg("-lm")
            .status()
            .context("failed to compile baseline")?;
        if !status.success() {
            bail!("clang baseline exited with {status}");
        }
        self.benchmark(&Bin { file: bin_path }, runs, iters)
    }
}
