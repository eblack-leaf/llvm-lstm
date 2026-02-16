use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::pass_menu::Pass;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub median_ns: u64,
    pub all_times_ns: Vec<u64>,
    pub binary_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub benchmark: BenchmarkResult,
    pub passes: Vec<String>,
    pub function_name: String,
    pub opt_ir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CompilationPipeline {
    clang: String,
    opt: String,
    llc: String,
    work_dir: PathBuf,
    timeout_secs: u64,
    /// Number of internal timing iterations each benchmark binary runs.
    /// Passed as argv[1] to the compiled C benchmark.
    pub bench_iters: usize,
}

impl CompilationPipeline {
    pub fn new(work_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&work_dir).ok();
        Self {
            clang: "clang-20".to_string(),
            opt: "opt-20".to_string(),
            llc: "llc-20".to_string(),
            work_dir,
            timeout_secs: 60,
            bench_iters: 201,
        }
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn with_bench_iters(mut self, iters: usize) -> Self {
        self.bench_iters = iters;
        self
    }

    /// Emit unoptimized LLVM IR from C source.
    pub fn emit_ir(&self, c_source: &Path) -> Result<PathBuf> {
        let stem = c_source
            .file_stem()
            .context("no file stem")?
            .to_string_lossy();
        let ll_path = self.work_dir.join(format!("{stem}.ll"));

        let output = Command::new(&self.clang)
            .args([
                "-emit-llvm",
                "-S",
                "-O0",
                "-Xclang",
                "-disable-O0-optnone",
            ])
            .arg(c_source)
            .arg("-o")
            .arg(&ll_path)
            .output()
            .context("failed to run clang")?;

        if !output.status.success() {
            bail!(
                "clang emit-ir failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(ll_path)
    }

    /// Apply optimization passes to an IR file using opt.
    pub fn apply_passes(&self, ir: &Path, passes: &[Pass], out: &Path) -> Result<()> {
        let pipeline = Pass::to_opt_pipeline(passes);
        if pipeline.is_empty() {
            // No passes to apply; just copy
            std::fs::copy(ir, out).context("failed to copy IR")?;
            return Ok(());
        }

        let output = Command::new(&self.opt)
            .arg(format!("-passes={pipeline}"))
            .arg(ir)
            .arg("-S")
            .arg("-o")
            .arg(out)
            .output()
            .context("failed to run opt")?;

        if !output.status.success() {
            bail!(
                "opt failed with passes '{pipeline}':\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Compile optimized IR to a native binary.
    pub fn compile_ir(&self, ir: &Path) -> Result<PathBuf> {
        let stem = ir
            .file_stem()
            .context("no file stem")?
            .to_string_lossy();
        let obj_path = self.work_dir.join(format!("{stem}.o"));
        let bin_path = self.work_dir.join(format!("{stem}.bin"));

        // llc: IR -> object file
        let output = Command::new(&self.llc)
            .arg(ir)
            .arg("-filetype=obj")
            .arg("-relocation-model=pic")
            .arg("-o")
            .arg(&obj_path)
            .output()
            .context("failed to run llc")?;

        if !output.status.success() {
            bail!(
                "llc failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // clang: link object file
        let output = Command::new(&self.clang)
            .arg(&obj_path)
            .arg("-o")
            .arg(&bin_path)
            .arg("-lm")
            .output()
            .context("failed to link")?;

        if !output.status.success() {
            bail!(
                "link failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(bin_path)
    }

    /// Run a compiled binary and collect timing results.
    pub fn benchmark(&self, binary: &Path, runs: usize) -> Result<BenchmarkResult> {
        let binary_size = std::fs::metadata(binary)
            .map(|m| m.len())
            .unwrap_or(0);

        let mut times: Vec<u64> = Vec::with_capacity(runs);
        let timeout = Duration::from_secs(self.timeout_secs);

        for _ in 0..runs {
            let mut child = Command::new(binary)
                .arg(self.bench_iters.to_string())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("failed to spawn benchmark binary")?;

            // Poll-based timeout: check every 100ms
            let start = std::time::Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        if !status.success() {
                            let stderr = child.stderr.take().map(|mut e| {
                                let mut s = String::new();
                                std::io::Read::read_to_string(&mut e, &mut s).ok();
                                s
                            }).unwrap_or_default();
                            bail!("benchmark binary failed:\n{stderr}");
                        }
                        break;
                    }
                    Ok(None) => {
                        if start.elapsed() > timeout {
                            child.kill().ok();
                            bail!("benchmark binary timed out after {}s", self.timeout_secs);
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    Err(e) => bail!("failed to wait on benchmark: {e}"),
                }
            }

            let stdout = child.stdout.take().map(|mut o| {
                let mut s = String::new();
                std::io::Read::read_to_string(&mut o, &mut s).ok();
                s
            }).unwrap_or_default();

            let ns: u64 = stdout
                .trim()
                .parse()
                .with_context(|| format!("failed to parse benchmark output: '{}'", stdout.trim()))?;
            times.push(ns);
        }

        times.sort();
        let median_ns = if times.len() % 2 == 1 {
            times[times.len() / 2]
        } else {
            (times[times.len() / 2 - 1] + times[times.len() / 2]) / 2
        };

        Ok(BenchmarkResult {
            median_ns,
            all_times_ns: times,
            binary_size_bytes: binary_size,
        })
    }

    /// Full pipeline: C source → IR → optimize → compile → benchmark.
    pub fn full_pipeline(
        &self,
        c_source: &Path,
        passes: &[Pass],
        runs: usize,
    ) -> Result<PipelineResult> {
        let stem = c_source
            .file_stem()
            .context("no file stem")?
            .to_string_lossy()
            .to_string();

        let ir = self.emit_ir(c_source)?;
        let opt_ir = self.work_dir.join(format!("{stem}_opt.ll"));
        self.apply_passes(&ir, passes, &opt_ir)?;
        let binary = self.compile_ir(&opt_ir)?;
        let benchmark = self.benchmark(&binary, runs)?;

        Ok(PipelineResult {
            benchmark,
            passes: passes.iter().map(|p| p.opt_name().to_string()).collect(),
            function_name: stem,
            opt_ir,
        })
    }

    /// Get the optimized IR path for feature extraction (without benchmarking).
    pub fn optimize_only(
        &self,
        c_source: &Path,
        passes: &[Pass],
    ) -> Result<PathBuf> {
        let stem = c_source
            .file_stem()
            .context("no file stem")?
            .to_string_lossy()
            .to_string();

        let ir = self.emit_ir(c_source)?;
        let opt_ir = self.work_dir.join(format!("{stem}_opt.ll"));
        self.apply_passes(&ir, passes, &opt_ir)?;
        Ok(opt_ir)
    }

    /// Run baseline at a standard optimization level (-O0, -O2, -O3).
    pub fn baseline(&self, c_source: &Path, opt_level: &str, runs: usize) -> Result<BenchmarkResult> {
        let stem = c_source
            .file_stem()
            .context("no file stem")?
            .to_string_lossy()
            .to_string();
        let bin_path = self.work_dir.join(format!("{stem}_baseline.bin"));

        let output = Command::new(&self.clang)
            .arg(opt_level) // e.g., "-O0", "-O2", "-O3"
            .arg(c_source)
            .arg("-o")
            .arg(&bin_path)
            .arg("-lm")
            .output()
            .context("failed to compile baseline")?;

        if !output.status.success() {
            bail!(
                "baseline compile failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        self.benchmark(&bin_path, runs)
    }
}
