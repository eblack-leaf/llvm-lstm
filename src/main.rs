#![recursion_limit = "256"]
mod actor_critic_tfx;
mod baseline;
mod critic;
mod dataset;
mod eda;
mod env;
mod episode_store;
mod evaluation;
mod ir_features;
mod pass_menu;
mod pipeline;
mod plots;
mod ppo;
mod returns;
mod rollout;
mod tfx_critic;
mod training;
mod training_tfx;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use pass_menu::Pass;

#[derive(Parser)]
#[command(name = "llvm-lstm", about = "LSTM+PPO agent for LLVM pass ordering")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Collect exploratory data with random pass sequences
    Collect {
        /// Directory containing benchmark .c files
        #[arg(long, default_value = "benchmarks")]
        functions: PathBuf,

        /// Number of random sequences per function
        #[arg(long, default_value = "200")]
        num_sequences: usize,

        /// Output directory for collected data
        #[arg(long, default_value = "data/exploratory")]
        output: PathBuf,

        /// Number of benchmark runs per sequence
        #[arg(long, default_value = "3")]
        runs: usize,

        /// Number of benchmark runs for baselines (each run internally does bench-iters trimmed-mean iterations)
        #[arg(long, default_value = "5")]
        baseline_runs: usize,

        /// Number of internal timing iterations per benchmark run (passed as argv[1] to C binary)
        #[arg(long, default_value = "51")]
        bench_iters: usize,

        /// Number of parallel threads (0 = use all cores)
        #[arg(long, default_value = "0")]
        threads: usize,
    },

    /// Run exploratory data analysis on collected data
    Eda {
        /// Input directory with exploratory data
        #[arg(long, default_value = "data/exploratory")]
        input: PathBuf,

        /// Output directory for analysis results
        #[arg(long, default_value = "eda_output")]
        output: PathBuf,

        /// Benchmark directory for IR feature extraction
        #[arg(long, default_value = "benchmarks")]
        functions: Option<PathBuf>,
    },

    /// Compute baselines (-O0, -O2, -O3) for all functions
    Baseline {
        /// Directory containing benchmark .c files
        #[arg(long, default_value = "benchmarks")]
        functions: PathBuf,

        /// Output directory for baseline data
        #[arg(long, default_value = "data/baselines")]
        output: PathBuf,

        /// Number of benchmark runs per baseline
        #[arg(long, default_value = "5")]
        baseline_runs: usize,

        /// Number of internal timing iterations per benchmark run
        #[arg(long, default_value = "201")]
        bench_iters: usize,
    },

    /// Train the actor-critic PPO agent
    Train {
        /// Directory containing benchmark .c files
        #[arg(long, default_value = "achievable")]
        functions: PathBuf,
        /// Working directory for compiled IR and binaries
        #[arg(long, default_value = "work")]
        work_dir: PathBuf,
        /// Directory to write model checkpoints
        #[arg(long, default_value = "checkpoints")]
        checkpoint_dir: String,
        /// Total number of collect+update iterations
        #[arg(long, default_value = "1000")]
        iterations: usize,
        /// Episodes to collect per function per iteration (total = episodes * num_functions)
        #[arg(long, default_value = "128")]
        episodes: usize,
        /// Entropy bonus coefficient (higher = more exploration)
        #[arg(long, default_value = "0.05")]
        entropy_coef: f32,
        /// Benchmark invocations per episode final step (1 is enough for training)
        #[arg(long, default_value = "3")]
        benchmark_runs: usize,
        /// Internal timing iterations inside each benchmark binary
        #[arg(long, default_value = "200")]
        bench_iters: usize,
        /// Max pass sequence length per episode
        #[arg(long, default_value = "100")]
        max_seq_length: usize,
        /// Reward mode: sparse | per-step
        #[arg(long, default_value = "sparse")]
        reward_mode: String,
        /// Allocate more episodes to functions still below O3, fewer to solved ones
        #[arg(long, default_value = "true")]
        dynamic_alloc: bool,
        /// IR featurisation mode: base | base+current
        #[arg(long, default_value = "base")]
        ir_mode: String,
        /// Downweight solved functions' advantages when batch mixes solved/unsolved
        #[arg(long, default_value = "false")]
        adv_weighting: bool,
        /// Return computation mode: episode | per-step
        #[arg(long, default_value = "episode")]
        return_mode: String,
        /// Baseline mode: intra-batch | best | critic | retrieval
        #[arg(long, default_value = "best")]
        baseline_mode: String,
        /// Critic architecture: null | ir-film | per-func | transformer
        #[arg(long, default_value = "null")]
        critic_arch: String,
        /// BestEpisodeStore prune threshold: drop episodes below (best_g0 - threshold)
        #[arg(long, default_value = "0.2")]
        prune_threshold: f32,
        /// Hard cap on episodes kept per function in store (best-first)
        #[arg(long, default_value = "256")]
        store_max_per_func: usize,
        /// How big the store needs to be before switching to critic scoring
        #[arg(long, default_value = "300")]
        warmup_threshold: usize,
    },

    /// Evaluate agent against baselines
    Evaluate {
        /// Directory containing benchmark .c files
        #[arg(long, default_value = "achievable")]
        functions: PathBuf,

        /// Output directory for evaluation results
        #[arg(long, default_value = "results")]
        output: PathBuf,

        /// Number of random search trials for comparison
        #[arg(long, default_value = "50")]
        random_trials: usize,

        /// Path to trained model checkpoint (omit to run baselines only)
        #[arg(long)]
        model: Option<PathBuf>,

        /// Reuse cached baselines from a previous run
        #[arg(long, default_value = "false")]
        rerun_baselines: bool,
    },

    /// Test the compilation pipeline on a single file
    TestPipeline {
        /// C source file to test
        #[arg(long)]
        file: PathBuf,

        /// Comma-separated list of pass names
        #[arg(long, default_value = "instcombine,sroa,simplifycfg")]
        passes: String,
    },

    /// Extract and print IR features from a .c or .ll file
    Features {
        /// Source file (.c or .ll)
        #[arg(long)]
        file: PathBuf,
    },

    /// Plot training metrics from a previous transformer train run
    PlotTrain {
        /// Checkpoint directory containing train_metrics.json
        #[arg(long, default_value = "checkpoints")]
        checkpoint_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Collect {
            functions,
            num_sequences,
            output,
            runs,
            baseline_runs,
            bench_iters,
            threads,
        } => {
            if threads > 0 {
                rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build_global()
                    .ok();
            }

            let wall_start = std::time::Instant::now();

            let collector = dataset::DataCollector::new(
                &functions,
                &output,
                num_sequences,
                runs,
                baseline_runs,
                bench_iters,
            )?;

            let t0 = std::time::Instant::now();
            collector.collect_baselines()?;
            let baseline_secs = t0.elapsed().as_secs_f64();
            eprintln!("Baselines completed in {baseline_secs:.1}s");

            let t1 = std::time::Instant::now();
            collector.collect()?;
            let collect_secs = t1.elapsed().as_secs_f64();

            let wall_secs = wall_start.elapsed().as_secs_f64();
            let wall_min = wall_secs / 60.0;
            let seq_per_min = (num_sequences as f64 * collector.function_count() as f64) / wall_min;
            eprintln!("Collection completed in {collect_secs:.1}s");
            eprintln!("Total wall time: {wall_min:.1} min ({seq_per_min:.0} sequences/min)");
        }

        Commands::Eda {
            input,
            output,
            functions,
        } => {
            let analyzer = eda::EdaAnalyzer::load(&input)?;
            analyzer.write_all(&output, functions.as_deref())?;
        }

        Commands::Baseline {
            functions,
            output,
            baseline_runs,
            bench_iters,
        } => {
            let collector =
                dataset::DataCollector::new(&functions, &output, 0, 1, baseline_runs, bench_iters)?;
            collector.collect_baselines()?;
        }

        Commands::Train {
            functions,
            work_dir,
            checkpoint_dir,
            iterations,
            episodes,
            entropy_coef,
            benchmark_runs,
            bench_iters,
            max_seq_length,
            reward_mode,
            dynamic_alloc,
            ir_mode,
            adv_weighting,
            return_mode,
            baseline_mode,
            critic_arch,
            prune_threshold,
            store_max_per_func,
            warmup_threshold,
        } => {
            use env::{EnvConfig, RewardMode};
            use ppo::PpoConfig;
            use training::TrainConfig;

            let mode = match reward_mode.as_str() {
                "per-step" => RewardMode::PerStep,
                _ => RewardMode::Sparse,
            };

            let config = TrainConfig::new(
                EnvConfig::new(functions, work_dir, mode)
                    .with_benchmark_runs(benchmark_runs)
                    .with_bench_iters(bench_iters)
                    .with_max_seq_length(max_seq_length),
                checkpoint_dir,
            )
            .with_total_iterations(iterations)
            .with_episodes_per_function(episodes)
            .with_ppo(PpoConfig::new().with_entropy_coef(entropy_coef))
            .with_dynamic_alloc(dynamic_alloc)
            .with_ir_mode(ir_mode)
            .with_adv_weighting(adv_weighting)
            .with_return_mode(return_mode)
            .with_baseline_mode(baseline_mode)
            .with_critic_arch(critic_arch)
            .with_prune_threshold(prune_threshold)
            .with_store_max_per_func(store_max_per_func)
            .with_warmup_threshold(warmup_threshold);

            training_tfx::train(config)?;
        }

        Commands::Evaluate {
            functions,
            output,
            random_trials,
            model,
            rerun_baselines,
        } => {
            let work_dir = output.join("_work");
            let evaluator = evaluation::Evaluator::new(&functions, &work_dir, 3)?;

            let agent_results = if let Some(model_path) = model {
                use actor_critic_tfx::{TransformerActorCritic, TransformerActorCriticConfig};
                use burn::backend::{NdArray, ndarray::NdArrayDevice};
                use burn::prelude::Module as _;
                use burn::record::CompactRecorder;
                use burn::tensor::{Int, Tensor, TensorData};
                use env::{EnvConfig, LlvmEnv, RewardMode};
                use evaluation::EvalResult;

                eprintln!("Loading model from {}...", model_path.display());
                let device = NdArrayDevice::default();
                let model: TransformerActorCritic<NdArray> = TransformerActorCriticConfig::new()
                    .init::<NdArray>(&device)
                    .load_file(&model_path, &CompactRecorder::new(), &device)?;

                let inf_config = EnvConfig::new(
                    functions.clone(),
                    work_dir.join("inference"),
                    RewardMode::Sparse,
                );
                let mut env = LlvmEnv::new(inf_config)?;
                eprintln!("Computing baselines for inference...");
                env.compute_baselines()?;
                let baselines = env.baselines().clone();

                let mut results: Vec<EvalResult> = Vec::new();
                for func_idx in 0..env.num_functions() {
                    let state = env.reset_to(func_idx)?;
                    let func_name = env.current_function_name().unwrap_or_else(|| "?".into());
                    eprintln!("  inference: {func_name}");

                    let feat_dim = state.features.len();
                    let base_feats = state.features.clone();
                    let mut act_history: Vec<i64> = vec![0i64];
                    let mut passes: Vec<Pass> = Vec::new();

                    let (time_ns, size_bytes) = loop {
                        let base_t = Tensor::<NdArray, 2>::from_data(
                            TensorData::new(base_feats.clone(), [1, feat_dim]),
                            &device,
                        );
                        let acts_t = Tensor::<NdArray, 2, Int>::from_data(
                            TensorData::new(act_history.clone(), [1, act_history.len()]),
                            &device,
                        );

                        let logits = model.forward(base_t, acts_t);
                        let logits_vec: Vec<f32> = logits.into_data().to_vec::<f32>()?;

                        // Greedy: argmax
                        let action = logits_vec
                            .iter()
                            .enumerate()
                            .max_by(|(_, a): &(usize, &f32), (_, b)| {
                                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .map(|(i, _)| i)
                            .unwrap_or(0);

                        let pass = Pass::from_index(action);
                        let step = env.step(action)?;

                        if pass != Pass::Stop {
                            passes.push(pass);
                        }

                        if step.done {
                            break (
                                step.info.execution_time_ns.unwrap_or(u64::MAX),
                                step.info.binary_size_bytes.unwrap_or(0),
                            );
                        }

                        act_history.push(action as i64);
                        let _ = step.state;
                    };

                    let bl = baselines.get(&func_name);
                    results.push(EvalResult {
                        function: func_name,
                        method: "agent_greedy".to_string(),
                        pass_sequence: passes.iter().map(|p| p.opt_name().to_string()).collect(),
                        execution_time_ns: time_ns,
                        binary_size_bytes: size_bytes,
                        speedup_vs_o0: bl.map_or(0.0, |b| b.o0_ns as f64 / time_ns.max(1) as f64),
                        speedup_vs_o3: bl.map_or(0.0, |b| b.o3_ns as f64 / time_ns.max(1) as f64),
                    });
                }
                Some(results)
            } else {
                None
            };

            evaluator.full_evaluation(random_trials, &output, rerun_baselines, agent_results)?;
        }

        Commands::TestPipeline { file, passes } => {
            let pass_list: Vec<Pass> = passes
                .split(',')
                .map(|s| {
                    let s = s.trim();
                    Pass::all_transforms()
                        .iter()
                        .find(|p| p.opt_name() == s)
                        .copied()
                        .unwrap_or_else(|| panic!("Unknown pass: {s}"))
                })
                .collect();

            let work_dir = PathBuf::from("/tmp/llvm-lstm-test");
            let pipe = pipeline::CompilationPipeline::new(work_dir);

            eprintln!("Emitting IR...");
            let ir = pipe.emit_ir(&file)?;
            eprintln!("  IR: {}", ir.display());

            eprintln!("Applying passes: {}", Pass::to_opt_pipeline(&pass_list));
            let opt_ir = ir.with_extension("opt.ll");
            pipe.apply_passes(&ir, &pass_list, &opt_ir)?;
            eprintln!("  Optimized IR: {}", opt_ir.display());

            eprintln!("Extracting IR features...");
            let features = ir_features::IrFeatures::from_ll_file(&opt_ir)?;
            eprintln!("  Features: {features:?}");

            eprintln!("Compiling to binary...");
            let binary = pipe.compile_ir(&opt_ir)?;
            eprintln!("  Binary: {}", binary.display());

            eprintln!("Benchmarking (3 runs)...");
            let result = pipe.benchmark(&binary, 3)?;
            eprintln!("  Median: {} ns", result.median_ns);
            eprintln!("  All times: {:?}", result.all_times_ns);
            eprintln!("  Binary size: {} bytes", result.binary_size_bytes);
        }

        Commands::PlotTrain { checkpoint_dir } => {
            plots::plot_train(&checkpoint_dir)?;
        }

        Commands::Features { file } => {
            let features = if file.extension().is_some_and(|e| e == "ll") {
                ir_features::IrFeatures::from_ll_file(&file)?
            } else {
                let work_dir = PathBuf::from("/tmp/llvm-lstm-features");
                let pipe = pipeline::CompilationPipeline::new(work_dir);
                let ir = pipe.emit_ir(&file)?;
                ir_features::IrFeatures::from_ll_file(&ir)?
            };

            println!("{}", serde_json::to_string_pretty(&features)?);
            eprintln!(
                "Feature vector ({} dims): {:?}",
                features.to_vec().len(),
                features.to_vec()
            );
        }
    }

    Ok(())
}
