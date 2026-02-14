use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Result;
use rand::Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ir_features::IrFeatures;
use crate::pass_menu::Pass;
use crate::pipeline::CompilationPipeline;

#[derive(Debug, Serialize, Deserialize)]
pub struct DataRecord {
    pub function: String,
    pub pass_sequence: Vec<String>,
    pub execution_time_ns: u64,
    pub binary_size_bytes: u64,
    pub ir_features: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BaselineRecord {
    pub function: String,
    pub opt_level: String,
    pub execution_time_ns: u64,
    pub binary_size_bytes: u64,
}

pub struct DataCollector {
    functions: Vec<PathBuf>,
    output_dir: PathBuf,
    num_sequences: usize,
    benchmark_runs: usize,
    baseline_runs: usize,
}

impl DataCollector {
    pub fn new(
        functions_dir: &Path,
        output_dir: &Path,
        num_sequences: usize,
        benchmark_runs: usize,
        baseline_runs: usize,
    ) -> Result<Self> {
        let mut functions = Vec::new();
        for entry in fs::read_dir(functions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "c") {
                functions.push(path);
            }
        }
        functions.sort();

        if functions.is_empty() {
            anyhow::bail!("No .c files found in {}", functions_dir.display());
        }

        fs::create_dir_all(output_dir)?;

        Ok(Self {
            functions,
            output_dir: output_dir.to_path_buf(),
            num_sequences,
            benchmark_runs,
            baseline_runs,
        })
    }

    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Collect exploratory data in parallel: one thread per function.
    pub fn collect(&self) -> Result<()> {
        let total = self.functions.len() * self.num_sequences;
        let progress = AtomicUsize::new(0);

        eprintln!(
            "Collecting {} sequences x {} functions ({} total) using {} threads",
            self.num_sequences,
            self.functions.len(),
            total,
            rayon::current_num_threads(),
        );

        // Each function runs in parallel with its own work dir and temp output file.
        let results: Vec<Result<PathBuf>> = self
            .functions
            .par_iter()
            .map(|func_path| {
                let stem = func_path
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();

                // Per-function work directory so files don't collide
                let work_dir = self.output_dir.join("_work").join(&stem);
                fs::create_dir_all(&work_dir)?;
                let pipeline = CompilationPipeline::new(work_dir);

                // Write to per-function temp file
                let tmp_path = self.output_dir.join(format!("_tmp_{stem}.jsonl"));
                let file = File::create(&tmp_path)?;
                let mut writer = BufWriter::new(file);
                let mut rng = rand::thread_rng();
                let transforms = Pass::all_transforms();

                eprintln!("[{stem}] Starting {num} sequences...", num = self.num_sequences);

                let mut func_count = 0usize;
                for seq_idx in 0..self.num_sequences {
                    let seq_len = rng.gen_range(1..=15);
                    let passes: Vec<Pass> = (0..seq_len)
                        .map(|_| transforms[rng.gen_range(0..transforms.len())])
                        .collect();

                    match pipeline.full_pipeline(func_path, &passes, self.benchmark_runs) {
                        Ok(result) => {
                            let features = IrFeatures::from_ll_file(&result.opt_ir)?;

                            let record = DataRecord {
                                function: stem.clone(),
                                pass_sequence: passes
                                    .iter()
                                    .map(|p| p.opt_name().to_string())
                                    .collect(),
                                execution_time_ns: result.benchmark.median_ns,
                                binary_size_bytes: result.benchmark.binary_size_bytes,
                                ir_features: features.to_vec(),
                            };

                            serde_json::to_writer(&mut writer, &record)?;
                            writeln!(writer)?;
                            func_count += 1;
                        }
                        Err(e) => {
                            eprintln!("  [{stem}] Warning: sequence {seq_idx} failed: {e}");
                            continue;
                        }
                    }

                    let done = progress.fetch_add(1, Ordering::Relaxed) + 1;
                    if done % 50 == 0 {
                        eprintln!("  Progress: {done}/{total}");
                    }
                }

                writer.flush()?;
                eprintln!("[{stem}] Done — {func_count} records");
                Ok(tmp_path)
            })
            .collect();

        // Merge per-function files into single output
        let data_path = self.output_dir.join("exploratory.jsonl");
        let mut out = BufWriter::new(File::create(&data_path)?);
        let mut total_records = 0usize;

        for result in results {
            let tmp_path = result?;
            let contents = fs::read_to_string(&tmp_path)?;
            for line in contents.lines() {
                if !line.trim().is_empty() {
                    writeln!(out, "{line}")?;
                    total_records += 1;
                }
            }
            fs::remove_file(&tmp_path)?;
        }

        out.flush()?;
        eprintln!("Wrote {total_records} records to {}", data_path.display());
        Ok(())
    }

    /// Collect baselines (-O0, -O2, -O3) for all functions.
    pub fn collect_baselines(&self) -> Result<()> {
        let baseline_path = self.output_dir.join("baselines.jsonl");

        // Baselines are quick — run sequentially to avoid timing interference
        let work_dir = self.output_dir.join("_work").join("_baselines");
        fs::create_dir_all(&work_dir)?;
        let pipeline = CompilationPipeline::new(work_dir);

        eprintln!("Computing baselines ({} runs per binary)...", self.baseline_runs);

        let file = File::create(&baseline_path)?;
        let mut writer = BufWriter::new(file);

        for func_path in &self.functions {
            let stem = func_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();

            eprintln!("  {stem}...");

            for opt_level in ["-O0", "-O2", "-O3"] {
                match pipeline.baseline(func_path, opt_level, self.baseline_runs) {
                    Ok(result) => {
                        let record = BaselineRecord {
                            function: stem.clone(),
                            opt_level: opt_level.to_string(),
                            execution_time_ns: result.median_ns,
                            binary_size_bytes: result.binary_size_bytes,
                        };
                        serde_json::to_writer(&mut writer, &record)?;
                        writeln!(writer)?;
                    }
                    Err(e) => {
                        eprintln!("  Warning: baseline {opt_level} for {stem} failed: {e}");
                    }
                }
            }
        }

        writer.flush()?;
        eprintln!("Wrote baselines to {}", baseline_path.display());
        Ok(())
    }
}
