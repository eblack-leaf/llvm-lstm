use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::ir_features::IrFeatures;
use crate::pass_menu::Pass;
use crate::pipeline::CompilationPipeline;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RewardMode {
    /// Reward only at episode end: speedup vs baseline
    Sparse,
    /// Reward at each step: incremental improvement
    PerStep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvConfig {
    pub functions_dir: PathBuf,
    pub work_dir: PathBuf,
    pub max_seq_length: usize,
    pub reward_mode: RewardMode,
    pub benchmark_runs: usize,
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self {
            functions_dir: PathBuf::from("benchmarks"),
            work_dir: PathBuf::from("/tmp/llvm-lstm-env"),
            max_seq_length: 40,
            reward_mode: RewardMode::Sparse,
            benchmark_runs: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineTimes {
    pub o0_ns: u64,
    pub o2_ns: u64,
    pub o3_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub features: Vec<f32>,
    pub pass_history: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInfo {
    pub execution_time_ns: Option<u64>,
    pub binary_size_bytes: Option<u64>,
    pub pass_applied: String,
    pub sequence_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub state: State,
    pub reward: f32,
    pub done: bool,
    pub info: StepInfo,
}

pub struct LlvmEnv {
    pipeline: CompilationPipeline,
    functions: Vec<PathBuf>,
    baselines: HashMap<String, BaselineTimes>,
    current_function: Option<PathBuf>,
    /// Base (unoptimized) IR emitted once at reset — never modified during episode.
    current_base_ir: Option<PathBuf>,
    /// Current optimized IR state — updated incrementally each step.
    current_opt_ir: Option<PathBuf>,
    current_passes: Vec<Pass>,
    previous_time_ns: Option<u64>,
    config: EnvConfig,
    rng_index: usize,
}

impl LlvmEnv {
    pub fn new(config: EnvConfig) -> Result<Self> {
        let functions = Self::discover_functions(&config.functions_dir)?;
        if functions.is_empty() {
            bail!("No .c files found in {}", config.functions_dir.display());
        }

        let pipeline = CompilationPipeline::new(config.work_dir.clone());

        Ok(Self {
            pipeline,
            functions,
            baselines: HashMap::new(),
            current_function: None,
            current_base_ir: None,
            current_opt_ir: None,
            current_passes: Vec::new(),
            previous_time_ns: None,
            config,
            rng_index: 0,
        })
    }

    /// Compute baselines for all functions. Call once before training.
    pub fn compute_baselines(&mut self) -> Result<()> {
        for func_path in &self.functions.clone() {
            let stem = func_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();

            eprintln!("Computing baselines for {stem}...");

            let o0 = self.pipeline.baseline(func_path, "-O0", 5)?;
            let o2 = self.pipeline.baseline(func_path, "-O2", 5)?;
            let o3 = self.pipeline.baseline(func_path, "-O3", 5)?;

            self.baselines.insert(
                stem,
                BaselineTimes {
                    o0_ns: o0.median_ns,
                    o2_ns: o2.median_ns,
                    o3_ns: o3.median_ns,
                },
            );
        }
        Ok(())
    }

    /// Reset environment: pick a function, extract initial IR features.
    pub fn reset(&mut self) -> Result<State> {
        // Round-robin function selection
        let func = self.functions[self.rng_index % self.functions.len()].clone();
        self.rng_index += 1;

        let base_ir = self.pipeline.emit_ir(&func)?;
        let features = IrFeatures::from_ll_file(&base_ir)?;

        self.current_function = Some(func);
        self.current_base_ir = Some(base_ir.clone());
        self.current_opt_ir = Some(base_ir); // no passes yet — opt IR == base IR
        self.current_passes.clear();
        self.previous_time_ns = None;

        Ok(State {
            features: features.to_vec(),
            pass_history: Vec::new(),
        })
    }

    /// Take a step: apply a pass (or STOP), return new state + reward.
    pub fn step(&mut self, action: usize) -> Result<StepResult> {
        let pass = Pass::from_index(action);
        let func = self
            .current_function
            .as_ref()
            .context("call reset() before step()")?
            .clone();

        let stem = func.file_stem().unwrap().to_string_lossy().to_string();

        // Check if done (STOP action or max length reached)
        let done = pass == Pass::Stop || self.current_passes.len() + 1 >= self.config.max_seq_length;

        if pass != Pass::Stop {
            self.current_passes.push(pass);
        }

        // Apply only the new pass to the current optimized IR (incremental).
        // This avoids re-running all accumulated passes from scratch each step.
        let opt_ir = if pass != Pass::Stop {
            self.pipeline.optimize_only(
                &func,
                self.current_opt_ir.as_deref(),
                &[pass],
            )?
        } else {
            self.current_opt_ir.clone().context("no current IR")?
        };
        let features = IrFeatures::from_ll_file(&opt_ir)?;

        // Compute reward
        let (reward, exec_time, binary_size) = if done {
            // Final step: actually benchmark
            let binary = self.pipeline.compile_ir(&opt_ir)?;
            let result = self
                .pipeline
                .benchmark(&binary, self.config.benchmark_runs)?;

            let reward = self.compute_reward(&stem, result.median_ns);
            (reward, Some(result.median_ns), Some(result.binary_size_bytes))
        } else {
            match self.config.reward_mode {
                RewardMode::Sparse => (0.0, None, None),
                RewardMode::PerStep => {
                    // Benchmark at each step for per-step reward
                    let binary = self.pipeline.compile_ir(&opt_ir)?;
                    let result = self
                        .pipeline
                        .benchmark(&binary, self.config.benchmark_runs)?;
                    let reward = self.compute_step_reward(result.median_ns);
                    self.previous_time_ns = Some(result.median_ns);
                    (reward, Some(result.median_ns), Some(result.binary_size_bytes))
                }
            }
        };

        self.current_opt_ir = Some(opt_ir);

        Ok(StepResult {
            state: State {
                features: features.to_vec(),
                pass_history: self.current_passes.iter().map(|p| p.to_index()).collect(),
            },
            reward,
            done,
            info: StepInfo {
                execution_time_ns: exec_time,
                binary_size_bytes: binary_size,
                pass_applied: pass.opt_name().to_string(),
                sequence_length: self.current_passes.len(),
            },
        })
    }

    pub fn baseline_time(&self, function: &str) -> Option<&BaselineTimes> {
        self.baselines.get(function)
    }

    pub fn current_function_name(&self) -> Option<String> {
        self.current_function
            .as_ref()
            .map(|p| p.file_stem().unwrap().to_string_lossy().to_string())
    }

    fn compute_reward(&self, function: &str, time_ns: u64) -> f32 {
        if let Some(baselines) = self.baselines.get(function) {
            // Reward = speedup over O0, normalized by O3
            // Positive if faster than O0, bonus if faster than O3
            let o0 = baselines.o0_ns as f64;
            let o3 = baselines.o3_ns as f64;
            let t = time_ns as f64;

            // Speedup ratio: how much faster than O0
            let speedup = (o0 - t) / o0;

            // Bonus for beating O3
            let o3_bonus = if t < o3 as u64 as f64 {
                0.5 * ((o3 - t) / o3)
            } else {
                0.0
            };

            (speedup + o3_bonus) as f32
        } else {
            0.0
        }
    }

    fn compute_step_reward(&self, time_ns: u64) -> f32 {
        if let Some(prev) = self.previous_time_ns {
            // Reward proportional to improvement from previous step
            let improvement = (prev as f64 - time_ns as f64) / prev as f64;
            improvement as f32
        } else {
            0.0
        }
    }

    fn discover_functions(dir: &Path) -> Result<Vec<PathBuf>> {
        let mut functions = Vec::new();
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "c") {
                    functions.push(path);
                }
            }
        }
        functions.sort();
        Ok(functions)
    }
}
