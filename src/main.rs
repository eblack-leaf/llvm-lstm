mod dataset;
mod eda;
mod env;
mod evaluation;
mod ir_features;
mod model;
mod pass_menu;
mod pipeline;
mod plots;
mod ppo;
mod rollout;
mod training;

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

        /// Benchmark directory (optional, enables IR feature extraction)
        #[arg(long)]
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

    /// Train the LSTM+PPO agent
    Train {
        /// Training config file (TOML)
        #[arg(long, default_value = "configs/train.toml")]
        config: PathBuf,
    },

    /// Evaluate agent against baselines
    Evaluate {
        /// Directory containing benchmark .c files
        #[arg(long, default_value = "benchmarks")]
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

            let collector =
                dataset::DataCollector::new(&functions, &output, num_sequences, runs, baseline_runs, bench_iters)?;

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

        Commands::Eda { input, output, functions } => {
            let analyzer = eda::EdaAnalyzer::load(&input)?;
            analyzer.write_all(&output, functions.as_deref())?;
        }

        Commands::Baseline { functions, output, baseline_runs, bench_iters } => {
            let collector =
                dataset::DataCollector::new(&functions, &output, 0, 1, baseline_runs, bench_iters)?;
            collector.collect_baselines()?;
        }

        Commands::Train { config } => {
            eprintln!("Training not yet implemented.");
            eprintln!("Config file: {}", config.display());
            eprintln!("TODO: Human implements LSTM policy + PPO training loop");
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
                // TODO: Load trained model and generate pass sequences
                // 1. Load checkpoint from model_path
                // 2. For each function, run inference to produce Vec<Pass>
                // 3. Call evaluator.eval_sequence(&passes)
                eprintln!("Loading model from {}...", model_path.display());
                eprintln!("TODO: Implement model loading + inference");
                None
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
