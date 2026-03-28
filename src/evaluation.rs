use std::collections::HashMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use anyhow::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::pass_menu::Pass;
use crate::pipeline::CompilationPipeline;

#[derive(Debug, Serialize, Deserialize)]
pub struct EvalResult {
    pub function: String,
    pub method: String,
    pub pass_sequence: Vec<String>,
    pub execution_time_ns: u64,
    pub binary_size_bytes: u64,
    pub speedup_vs_o0: f64,
    pub speedup_vs_o3: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvalSummary {
    pub results: Vec<EvalResult>,
    pub avg_speedup_vs_o0: f64,
    pub avg_speedup_vs_o3: f64,
    pub beat_o3_count: usize,
    pub total_functions: usize,
}

pub struct Evaluator {
    pipeline: CompilationPipeline,
    functions: Vec<PathBuf>,
    benchmark_runs: usize,
}

impl Evaluator {
    pub fn new(functions_dir: &Path, work_dir: &Path, benchmark_runs: usize) -> Result<Self> {
        let mut functions = Vec::new();
        for entry in fs::read_dir(functions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "c") {
                functions.push(path);
            }
        }
        functions.sort();

        fs::create_dir_all(work_dir)?;

        Ok(Self {
            pipeline: CompilationPipeline::new(work_dir.to_path_buf()),
            functions,
            benchmark_runs,
        })
    }

    /// Evaluate baselines (-O0, -O2, -O3) for all functions.
    pub fn eval_baselines(&self) -> Result<Vec<EvalResult>> {
        let mut results = Vec::new();

        for func_path in &self.functions {
            let stem = func_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();

            for opt in ["-O0", "-O2", "-O3"] {
                let bench = self.pipeline.baseline(func_path, opt, self.benchmark_runs)?;
                results.push(EvalResult {
                    function: stem.clone(),
                    method: opt.to_string(),
                    pass_sequence: vec![],
                    execution_time_ns: bench.median_ns,
                    binary_size_bytes: bench.binary_size_bytes,
                    speedup_vs_o0: 0.0, // filled in later
                    speedup_vs_o3: 0.0,
                });
            }
        }

        Ok(results)
    }

    /// Evaluate random search: best of N random pass sequences.
    pub fn eval_random_search(&self, num_trials: usize) -> Result<Vec<EvalResult>> {
        let mut results = Vec::new();
        let mut rng = rand::thread_rng();
        let transforms = Pass::all_transforms();

        for func_path in &self.functions {
            let stem = func_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let base_ir = self.pipeline.emit_ir(func_path)?;
            let mut best_time = u64::MAX;
            let mut best_passes: Vec<Pass> = Vec::new();
            let mut best_size = 0u64;

            for _ in 0..num_trials {
                let seq_len = rng.gen_range(1..=20);
                let passes: Vec<Pass> = (0..seq_len)
                    .map(|_| transforms[rng.gen_range(0..transforms.len())])
                    .collect();

                match self
                    .pipeline
                    .full_pipeline(func_path, Some(&base_ir), &passes, self.benchmark_runs)
                {
                    Ok(result) => {
                        if result.benchmark.median_ns < best_time {
                            best_time = result.benchmark.median_ns;
                            best_passes = passes;
                            best_size = result.benchmark.binary_size_bytes;
                        }
                    }
                    Err(_) => continue,
                }
            }

            if best_time < u64::MAX {
                results.push(EvalResult {
                    function: stem,
                    method: format!("random_search_{num_trials}"),
                    pass_sequence: best_passes.iter().map(|p| p.opt_name().to_string()).collect(),
                    execution_time_ns: best_time,
                    binary_size_bytes: best_size,
                    speedup_vs_o0: 0.0,
                    speedup_vs_o3: 0.0,
                });
            }
        }

        Ok(results)
    }

    /// Evaluate greedy single-step: try each pass individually, pick the best.
    pub fn eval_greedy(&self) -> Result<Vec<EvalResult>> {
        let mut results = Vec::new();
        let transforms = Pass::all_transforms();

        for func_path in &self.functions {
            let stem = func_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let base_ir = self.pipeline.emit_ir(func_path)?;
            let mut best_time = u64::MAX;
            let mut best_pass = Pass::Instcombine;
            let mut best_size = 0u64;

            for &pass in transforms {
                match self
                    .pipeline
                    .full_pipeline(func_path, Some(&base_ir), &[pass], self.benchmark_runs)
                {
                    Ok(result) => {
                        if result.benchmark.median_ns < best_time {
                            best_time = result.benchmark.median_ns;
                            best_pass = pass;
                            best_size = result.benchmark.binary_size_bytes;
                        }
                    }
                    Err(_) => continue,
                }
            }

            if best_time < u64::MAX {
                results.push(EvalResult {
                    function: stem,
                    method: "greedy_single".to_string(),
                    pass_sequence: vec![best_pass.opt_name().to_string()],
                    execution_time_ns: best_time,
                    binary_size_bytes: best_size,
                    speedup_vs_o0: 0.0,
                    speedup_vs_o3: 0.0,
                });
            }
        }

        Ok(results)
    }

    /// Run full evaluation and compute speedups.
    /// `agent_results` can be passed in from model inference; they get included in the summary.
    pub fn full_evaluation(
        &self,
        random_trials: usize,
        output_dir: &Path,
        rerun_baselines: bool,
        agent_results: Option<Vec<EvalResult>>,
    ) -> Result<EvalSummary> {
        fs::create_dir_all(output_dir)?;

        let baselines_cache = output_dir.join("baselines_cache.json");
        let baselines: Vec<EvalResult> = if !rerun_baselines && baselines_cache.exists() {
            eprintln!("Loading cached baselines...");
            let file = File::open(&baselines_cache)?;
            serde_json::from_reader(file)?
        } else {
            eprintln!("Evaluating baselines...");
            let b = self.eval_baselines()?;
            let file = File::create(&baselines_cache)?;
            serde_json::to_writer_pretty(file, &b)?;
            eprintln!("Cached baselines to {}", baselines_cache.display());
            b
        };

        // Build baseline lookup
        let mut o0_times: HashMap<String, u64> = HashMap::new();
        let mut o3_times: HashMap<String, u64> = HashMap::new();
        for r in &baselines {
            match r.method.as_str() {
                "-O0" => { o0_times.insert(r.function.clone(), r.execution_time_ns); }
                "-O3" => { o3_times.insert(r.function.clone(), r.execution_time_ns); }
                _ => {}
            }
        }

        let random_cache = output_dir.join(format!("random_{random_trials}_cache.json"));
        let random: Vec<EvalResult> = if !rerun_baselines && random_cache.exists() {
            eprintln!("Loading cached random search ({random_trials} trials)...");
            let file = File::open(&random_cache)?;
            serde_json::from_reader(file)?
        } else {
            eprintln!("Evaluating random search ({random_trials} trials)...");
            let r = self.eval_random_search(random_trials)?;
            let file = File::create(&random_cache)?;
            serde_json::to_writer_pretty(file, &r)?;
            eprintln!("Cached random results to {}", random_cache.display());
            r
        };

        let greedy_cache = output_dir.join("greedy_cache.json");
        let greedy: Vec<EvalResult> = if !rerun_baselines && greedy_cache.exists() {
            eprintln!("Loading cached greedy results...");
            let file = File::open(&greedy_cache)?;
            serde_json::from_reader(file)?
        } else {
            eprintln!("Evaluating greedy single-step...");
            let g = self.eval_greedy()?;
            let file = File::create(&greedy_cache)?;
            serde_json::to_writer_pretty(file, &g)?;
            eprintln!("Cached greedy results to {}", greedy_cache.display());
            g
        };

        // Combine all results and compute speedups
        let mut all_results: Vec<EvalResult> = Vec::new();
        let mut chains: Vec<EvalResult> = baselines
            .into_iter()
            .chain(random)
            .chain(greedy)
            .collect();
        if let Some(agent) = agent_results {
            chains.extend(agent);
        }

        for mut r in chains {
            if let Some(&o0) = o0_times.get(&r.function) {
                r.speedup_vs_o0 = o0 as f64 / r.execution_time_ns.max(1) as f64;
            }
            if let Some(&o3) = o3_times.get(&r.function) {
                r.speedup_vs_o3 = o3 as f64 / r.execution_time_ns.max(1) as f64;
            }
            all_results.push(r);
        }

        // Per-method summary
        let methods: Vec<String> = {
            let mut seen = Vec::new();
            for r in &all_results {
                if !seen.contains(&r.method) {
                    seen.push(r.method.clone());
                }
            }
            seen
        };

        eprintln!("\n=== Evaluation Summary ===");
        eprintln!("{:<25} {:>12} {:>12} {:>10}", "Method", "Avg vs -O0", "Avg vs -O3", "Beat -O3");
        eprintln!("{}", "-".repeat(62));

        for method in &methods {
            let method_results: Vec<&EvalResult> = all_results
                .iter()
                .filter(|r| r.method == *method)
                .collect();
            if method_results.is_empty() {
                continue;
            }
            let avg_o0 = method_results.iter().map(|r| r.speedup_vs_o0).sum::<f64>()
                / method_results.len() as f64;
            let avg_o3 = method_results.iter().map(|r| r.speedup_vs_o3).sum::<f64>()
                / method_results.len() as f64;
            let beat = method_results.iter().filter(|r| r.speedup_vs_o3 > 1.0).count();
            eprintln!(
                "{:<25} {:>11.2}x {:>11.2}x {:>5}/{:<4}",
                method, avg_o0, avg_o3, beat, method_results.len()
            );
        }

        let (avg_speedup_vs_o0, avg_speedup_vs_o3, beat_o3_count) = {
            let non_baseline: Vec<&EvalResult> = all_results
                .iter()
                .filter(|r| !r.method.starts_with('-'))
                .collect();
            let beat = non_baseline.iter().filter(|r| r.speedup_vs_o3 > 1.0).count();
            if non_baseline.is_empty() {
                (0.0, 0.0, 0usize)
            } else {
                let avg_o0 = non_baseline.iter().map(|r| r.speedup_vs_o0).sum::<f64>()
                    / non_baseline.len() as f64;
                let avg_o3 = non_baseline.iter().map(|r| r.speedup_vs_o3).sum::<f64>()
                    / non_baseline.len() as f64;
                (avg_o0, avg_o3, beat)
            }
        };

        let summary = EvalSummary {
            results: all_results,
            avg_speedup_vs_o0,
            avg_speedup_vs_o3,
            beat_o3_count,
            total_functions: self.functions.len(),
        };

        let file = File::create(output_dir.join("evaluation.json"))?;
        serde_json::to_writer_pretty(file, &summary)?;
        eprintln!("\nWrote evaluation.json");

        Ok(summary)
    }
}
