use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use burn::config::Config;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

use crate::ir_features::IrFeatures;
use crate::pass_menu::Pass;
use crate::pipeline::CompilationPipeline;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RewardMode {
    /// Reward only at episode end: speedup vs baseline
    Sparse,
    /// Reward at each step: incremental improvement (expensive — benchmarks every step)
    PerStep,
    /// Per-step proxy from instruction count reduction + full benchmark at terminal.
    /// Free: total_instruction_count is already extracted each step anyway.
    InstructionProxy,
}

#[derive(Config, Debug)]
pub struct EnvConfig {
    /// Directory containing .c benchmark files.
    /// No inline default — PathBuf can't be expressed as a Config literal.
    pub functions_dir: PathBuf,
    /// Working directory for compiled IR and binaries.
    pub work_dir: PathBuf,
    /// Maximum number of passes per episode before forced termination.
    #[config(default = 40)]
    pub max_seq_length: usize,
    /// Whether to give reward at every step or only at episode end.
    /// No inline default — enum variants aren't Config literals.
    pub reward_mode: RewardMode,
    /// Number of benchmark process invocations to average per timing call.
    #[config(default = 3)]
    pub benchmark_runs: usize,
    /// Internal timing iterations inside each benchmark binary (passed as argv[1]).
    #[config(default = 51)]
    pub bench_iters: usize,
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

/// Breakdown of the terminal benchmark result into per-baseline margins.
/// Positive = faster than that baseline, negative = slower.
/// Only populated on the final step of an episode.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RewardBreakdown {
    /// (O0 - t) / O0
    pub vs_o0: f32,
    /// (O2 - t) / O2
    pub vs_o2: f32,
    /// (O3 - t) / O3
    pub vs_o3: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub state: State,
    pub reward: f32,
    pub done: bool,
    pub info: StepInfo,
    pub breakdown: Option<RewardBreakdown>,
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
    /// Instruction count at episode start — denominator for InstructionProxy reward.
    base_inst_count: Option<u32>,
    /// Instruction count after the previous step — numerator delta for InstructionProxy.
    previous_inst_count: Option<u32>,
    config: EnvConfig,
    /// Shared directory for caching IR by pass sequence. None = no caching.
    ir_cache_dir: Option<PathBuf>,
}

impl LlvmEnv {
    pub fn new(config: EnvConfig) -> Result<Self> {
        let functions = Self::discover_functions(&config.functions_dir)?;
        if functions.is_empty() {
            bail!("No .c files found in {}", config.functions_dir.display());
        }

        let pipeline = CompilationPipeline::new(config.work_dir.clone())
            .with_bench_iters(config.bench_iters);

        Ok(Self {
            pipeline,
            functions,
            baselines: HashMap::new(),
            current_function: None,
            current_base_ir: None,
            current_opt_ir: None,
            current_passes: Vec::new(),
            previous_time_ns: None,
            base_inst_count: None,
            previous_inst_count: None,
            config,
            ir_cache_dir: None,
        })
    }

    /// Construct a worker env with pre-computed baselines (skips baseline computation).
    /// Use a unique `config.work_dir` per worker to avoid file collisions.
    pub fn new_with_baselines(config: EnvConfig, baselines: HashMap<String, BaselineTimes>) -> Result<Self> {
        let functions = Self::discover_functions(&config.functions_dir)?;
        if functions.is_empty() {
            bail!("No .c files found in {}", config.functions_dir.display());
        }
        let pipeline = CompilationPipeline::new(config.work_dir.clone())
            .with_bench_iters(config.bench_iters);
        Ok(Self {
            pipeline,
            functions,
            baselines,
            current_function: None,
            current_base_ir: None,
            current_opt_ir: None,
            current_passes: Vec::new(),
            previous_time_ns: None,
            base_inst_count: None,
            previous_inst_count: None,
            config,
            ir_cache_dir: None,
        })
    }

    /// Enable the shared IR cache. Call before the first episode.
    /// `dir` should be the same path for all parallel workers (e.g. `work/ir_cache`).
    #[allow(unused)]
    pub fn with_ir_cache(mut self, dir: PathBuf) -> Self {
        std::fs::create_dir_all(&dir).ok();
        self.ir_cache_dir = Some(dir);
        self
    }

    pub fn baselines(&self) -> &HashMap<String, BaselineTimes> {
        &self.baselines
    }

    pub fn num_functions(&self) -> usize {
        self.functions.len()
    }

    /// Compute baselines for all functions. Call once before training.
    pub fn compute_baselines(&mut self) -> Result<()> {
        let pb = ProgressBar::new(self.functions.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  baselines  {bar:30.cyan}  {pos}/{len}  {elapsed}  {msg}")
                .unwrap(),
        );

        for func_path in &self.functions.clone() {
            let stem = func_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();

            pb.set_message(stem.clone());

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
            pb.inc(1);
        }
        pb.finish_with_message("done");
        Ok(())
    }

    /// Reset to a specific function by index.
    pub fn reset_to(&mut self, func_index: usize) -> Result<State> {
        let func = self.functions[func_index % self.functions.len()].clone();
        let base_ir = self.pipeline.emit_ir(&func)?;
        let features = IrFeatures::from_ll_file(&base_ir)?;

        let base_insts = features.total_instruction_count;
        self.current_function = Some(func);
        self.current_base_ir = Some(base_ir.clone());
        self.current_opt_ir = Some(base_ir);
        self.current_passes.clear();
        self.previous_time_ns = None;
        self.base_inst_count = Some(base_insts);
        self.previous_inst_count = Some(base_insts);

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
        // Check the shared IR cache first; on miss, run opt and populate the cache.
        let opt_ir = if pass != Pass::Stop {
            let cache_path = self.ir_cache_path(&stem);
            if let Some(ref cp) = cache_path {
                if cp.exists() {
                    cp.clone()
                } else {
                    let result = self.pipeline.optimize_only(
                        &func,
                        self.current_opt_ir.as_deref(),
                        &[pass],
                    )?;
                    // Atomic write: copy to .tmp then rename so concurrent workers
                    // never see a half-written file.
                    let tmp = cp.with_extension("ll.tmp");
                    if std::fs::copy(&result, &tmp).is_ok() {
                        std::fs::rename(&tmp, cp).ok();
                    }
                    result
                }
            } else {
                self.pipeline.optimize_only(
                    &func,
                    self.current_opt_ir.as_deref(),
                    &[pass],
                )?
            }
        } else {
            self.current_opt_ir.clone().context("no current IR")?
        };
        let features = IrFeatures::from_ll_file(&opt_ir)?;

        // Compute reward
        let (reward, exec_time, binary_size, breakdown) = if done {
            // Final step: always benchmark and return full breakdown.
            let binary = self.pipeline.compile_ir(&opt_ir)?;
            let result = self.pipeline.benchmark(&binary, self.config.benchmark_runs)?;
            let (reward, bd) = self.compute_reward(&stem, result.median_ns);
            (reward, Some(result.median_ns), Some(result.binary_size_bytes), bd)
        } else {
            match self.config.reward_mode {
                RewardMode::Sparse => (0.0, None, None, None),
                RewardMode::PerStep => {
                    // Benchmark at each step for per-step reward
                    let binary = self.pipeline.compile_ir(&opt_ir)?;
                    let result = self
                        .pipeline
                        .benchmark(&binary, self.config.benchmark_runs)?;
                    let reward = self.compute_step_reward(&stem, result.median_ns);
                    self.previous_time_ns = Some(result.median_ns);
                    (reward, Some(result.median_ns), Some(result.binary_size_bytes), None)
                }
                RewardMode::InstructionProxy => {
                    const PROXY_SCALE: f32 = 0.2;
                    let curr = features.total_instruction_count;
                    let prev = self.previous_inst_count.unwrap_or(curr);
                    let base = self.base_inst_count.unwrap_or(prev.max(1));
                    let reward = (prev as f32 - curr as f32) / base.max(1) as f32 * PROXY_SCALE;
                    (reward, None, None, None)
                }
            }
        };

        self.current_opt_ir = Some(opt_ir);
        self.previous_inst_count = Some(features.total_instruction_count);

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
            breakdown,
        })
    }

    /// Path where the IR for the current accumulated pass sequence should be cached.
    /// Returns None if caching is disabled or no function is set.
    fn ir_cache_path(&self, func_stem: &str) -> Option<PathBuf> {
        self.ir_cache_dir.as_ref().map(|dir| {
            let key: String = self.current_passes
                .iter()
                .map(|p| p.to_index().to_string())
                .collect::<Vec<_>>()
                .join("-");
            dir.join(format!("{func_stem}__{key}.ll"))
        })
    }

    pub fn current_function_name(&self) -> Option<String> {
        self.current_function
            .as_ref()
            .map(|p| p.file_stem().unwrap().to_string_lossy().to_string())
    }

    fn compute_reward(&self, function: &str, time_ns: u64) -> (f32, Option<RewardBreakdown>) {
        if let Some(baselines) = self.baselines.get(function) {
            // Tiered reward: scaled points for beating each baseline tier,
            // plus a continuous gradient for margin above O3.
            //
            //   r = w0 * 1[t < O0] + w2 * 1[t < O2] + w3 * 1[t < O3]
            //       + s3 * (O3 - t) / O3
            //
            // Tiered reward: discrete bonuses for each baseline beaten, plus a
            // continuous bonus for the margin beyond O3.
            // All terms are non-negative — beating a faster baseline always
            // increases the reward; being slower than O3 is not penalised.
            const W0: f64 = 0.1;   // bonus for beating -O0
            const W2: f64 = 0.3;   // bonus for beating -O2
            const W3: f64 = 0.5;   // bonus for beating -O3
            const S3: f64 = 1.0;   // scale for continuous margin beyond O3

            let t = time_ns as f64;
            let o0 = baselines.o0_ns as f64;
            let o2 = baselines.o2_ns as f64;
            let o3 = baselines.o3_ns as f64;

            let vs_o0 = ((o0 - t) / o0) as f32;
            let vs_o2 = ((o2 - t) / o2) as f32;
            let vs_o3 = ((o3 - t) / o3) as f32;

            let tier = if t < o0 { W0 } else { 0.0 }
                     + if t < o2 { W2 } else { 0.0 }
                     + if t < o3 { W3 } else { 0.0 };
            // Margin only positive: extra credit for beating O3, no penalty for missing it.
            let margin = if t < o3 { S3 * (o3 - t) / o3 } else { 0.0 };

            let total = (tier + margin) as f32;
            let bd = RewardBreakdown { vs_o0, vs_o2, vs_o3 };
            (total, Some(bd))
        } else {
            (0.0, None)
        }
    }

    fn compute_step_reward(&self, function: &str, time_ns: u64) -> f32 {
        // Compare against the previous step's time, or the O0 baseline on the
        // first step so the agent gets a real signal immediately rather than 0.
        let prev = self.previous_time_ns.or_else(|| {
            self.baselines.get(function).map(|b| b.o0_ns)
        });
        if let Some(prev_ns) = prev {
            let improvement = (prev_ns as f64 - time_ns as f64) / prev_ns as f64;
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
