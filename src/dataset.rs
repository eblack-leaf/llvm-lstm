use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use rand::Rng;
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
    pipeline: CompilationPipeline,
    functions: Vec<PathBuf>,
    output_dir: PathBuf,
    num_sequences: usize,
    benchmark_runs: usize,
}

impl DataCollector {
    pub fn new(
        functions_dir: &Path,
        output_dir: &Path,
        num_sequences: usize,
        benchmark_runs: usize,
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
        let work_dir = output_dir.join("_work");
        fs::create_dir_all(&work_dir)?;

        Ok(Self {
            pipeline: CompilationPipeline::new(work_dir),
            functions,
            output_dir: output_dir.to_path_buf(),
            num_sequences,
            benchmark_runs,
        })
    }

    /// Collect exploratory data: random pass sequences for each function.
    pub fn collect(&self) -> Result<()> {
        let data_path = self.output_dir.join("exploratory.jsonl");
        let file = File::create(&data_path)?;
        let mut writer = BufWriter::new(file);
        let mut rng = rand::thread_rng();

        let transforms = Pass::all_transforms();
        let total = self.functions.len() * self.num_sequences;
        let mut count = 0;

        for func_path in &self.functions {
            let stem = func_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();

            eprintln!("Collecting data for {stem}...");

            for seq_idx in 0..self.num_sequences {
                // Generate random pass sequence (length 1-15)
                let seq_len = rng.gen_range(1..=15);
                let passes: Vec<Pass> = (0..seq_len)
                    .map(|_| transforms[rng.gen_range(0..transforms.len())])
                    .collect();

                match self.pipeline.full_pipeline(func_path, &passes, self.benchmark_runs) {
                    Ok(result) => {
                        // Get IR features of optimized code
                        let opt_ir = self.pipeline.optimize_only(func_path, &passes)?;
                        let features = IrFeatures::from_ll_file(&opt_ir)?;

                        let record = DataRecord {
                            function: stem.clone(),
                            pass_sequence: passes.iter().map(|p| p.opt_name().to_string()).collect(),
                            execution_time_ns: result.benchmark.median_ns,
                            binary_size_bytes: result.benchmark.binary_size_bytes,
                            ir_features: features.to_vec(),
                        };

                        serde_json::to_writer(&mut writer, &record)?;
                        writeln!(writer)?;
                    }
                    Err(e) => {
                        eprintln!(
                            "  Warning: sequence {seq_idx} for {stem} failed: {e}"
                        );
                        continue;
                    }
                }

                count += 1;
                if count % 10 == 0 {
                    eprintln!("  Progress: {count}/{total}");
                }
            }
        }

        writer.flush()?;
        eprintln!("Wrote {count} records to {}", data_path.display());
        Ok(())
    }

    /// Collect baselines (-O0, -O2, -O3) for all functions.
    pub fn collect_baselines(&self) -> Result<()> {
        let baseline_path = self.output_dir.join("baselines.jsonl");
        let file = File::create(&baseline_path)?;
        let mut writer = BufWriter::new(file);

        for func_path in &self.functions {
            let stem = func_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();

            eprintln!("Computing baselines for {stem}...");

            for opt_level in ["-O0", "-O2", "-O3"] {
                match self.pipeline.baseline(func_path, opt_level) {
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
