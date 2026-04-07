#![recursion_limit = "256"]
#![allow(unused)]
use crate::config::{BurnAutoDiff, BurnDevice, Cfg};
use crate::llvm::Llvm;
use crate::llvm::functions::Functions;
use crate::llvm::pass::Pass;
use crate::llvm::top_sequences::TopSequences;
use crate::ppo::advantages::baseline::BaselineAdvantage;
use crate::ppo::checkpoint::Checkpoint;
use crate::ppo::logging::LogMode;
use crate::ppo::returns::episode_return::EpisodeReturn;
use crate::ppo::returns::instruction_proxy::InstructionProxyReturn;
use crate::ppo::returns::instruction_weighted_terminal::InstructionWeightedTerminal;
use crate::train::Trainer;
use burn::module::AutodiffModule;
use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;

mod config;
mod llvm;
mod ppo;
mod predictor;
mod train;

#[derive(Parser)]
struct LlvmLstm {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Train {
        #[arg(long, default_value = "benchmarks")]
        directory: PathBuf,
        #[arg(long, default_value = "work")]
        work_dir: PathBuf,
        #[arg(long, default_value = "checkpoints")]
        checkpoint_dir: PathBuf,
        #[arg(long, default_value = "clang-20")]
        clang: String,
        #[arg(long, default_value = "opt-20")]
        opt: String,
        #[arg(long, default_value = "100")]
        epochs: usize,
        #[arg(long, default_value = "4")]
        ppo_epochs: usize,
        #[arg(long, default_value = "256")]
        episodes: usize,
        #[arg(long, default_value = "2")]
        benchmark_runs: usize,
        #[arg(long, default_value = "100")]
        benchmark_iters: usize,
        #[arg(long, default_value = "3")]
        baseline_runs: usize,
        #[arg(long, default_value = "200")]
        baseline_iters: usize,
        #[arg(long, default_value = "30")]
        max_seq_len: usize,
        #[arg(long, default_value = "3e-4")]
        learning_rate: f64,
        #[arg(long, default_value = "0.1")]
        clip_epsilon: f32,
        #[arg(long, default_value = "0.5")]
        value_coef: f32,
        #[arg(long, default_value = "0.02")]
        entropy_coef: f32,
        /// Number of episodes per PPO mini-batch.
        #[arg(long, default_value = "64")]
        mini_batch_size: usize,
        #[arg(long)]
        cache_file: Option<PathBuf>,
        /// Path to save/load the top-sequences file for the Diagnose command.
        #[arg(long)]
        sequences_file: Option<PathBuf>,
        /// Blend weight for terminal speedup in instruction proxy returns.
        /// 1.0 = pure speedup (default), 0.0 = pure instruction-count delta.
        /// Values in (0, 1) blend both signals for denser credit assignment.
        /// Only used when --returns=proxy.
        #[arg(long, default_value = "1.0")]
        proxy_alpha: f32,
        /// Return signal: episode (uniform terminal), proxy (blended instr+terminal),
        /// weighted (terminal weighted by per-slot instr reduction; no-ops get 0),
        /// predictor (per-step marginal from pretrained SpeedupPredictor).
        #[arg(long, default_value = "weighted")]
        returns: String,
        /// Path to predictor checkpoint directory. Required when --returns=predictor.
        #[arg(long)]
        predictor_checkpoint: Option<PathBuf>,
        /// Instruction-count delta below which a step is considered a no-op and gets zero return (predictor mode).
        #[arg(long, default_value = "0.01")]
        predictor_noop_threshold: f32,
        /// Scale factor applied to all predictor returns.
        #[arg(long, default_value = "1.0")]
        predictor_scale: f32,
        /// Steps with |instr_delta| <= this value are reported as no-ops in metrics (default 0 = exact no-op).
        #[arg(long, default_value = "0.01")]
        noop_threshold: f32,
        #[arg(long, default_value = "0.01")]
        delta_threshold: f32,
    },
    Evaluate {
        #[arg(long, default_value = "checkpoints/best")]
        model: PathBuf,
    },
    PlotTrain {
        #[arg(long, default_value = "checkpoints")]
        dir: PathBuf,
    },
    /// Re-benchmark the top sequences from training to check if speedups are reproducible.
    Diagnose {
        #[arg(long, default_value = "checkpoints/top_sequences.bin")]
        sequences: PathBuf,
        #[arg(long, default_value = "benchmarks")]
        directory: PathBuf,
        #[arg(long, default_value = "work/diagnose")]
        work_dir: PathBuf,
        #[arg(long, default_value = "clang-20")]
        clang: String,
        #[arg(long, default_value = "opt-20")]
        opt: String,
        /// How many top sequences to re-test.
        #[arg(long, default_value = "10")]
        top: usize,
        /// Benchmark runs per sequence per trial (for mean/std).
        #[arg(long, default_value = "20")]
        runs: usize,
        #[arg(long, default_value = "200")]
        iters: usize,
        #[arg(long, default_value = "3")]
        baseline_runs: usize,
        #[arg(long, default_value = "200")]
        baseline_iters: usize,
    },
    /// Measure parallel-worker benchmark timing contention.
    BenchNoise {
        #[arg(long)]
        source: PathBuf,
        #[arg(long, default_value = "clang-20")]
        clang: String,
        #[arg(long, default_value = "work/bench_noise")]
        work_dir: PathBuf,
        #[arg(long, default_value = "5")]
        runs: usize,
        #[arg(long, default_value = "200")]
        iters: usize,
        #[arg(long, default_value = "16")]
        workers: usize,
    },
    // Inside Command enum, add:
    Collect {
        #[arg(long, default_value = "checkpoints/selected-6.cache")]
        cache_file: PathBuf,
        #[arg(long, default_value = "benchmarks")]
        functions_dir: PathBuf,
        #[arg(long, default_value = "clang-20")]
        clang: String,
        #[arg(long, default_value = "opt-20")]
        opt: String,
        #[arg(long, default_value = "work/collect")]
        work_dir: PathBuf,
        #[arg(long, default_value = "dataset.jsonl")]
        output: PathBuf,
    },
    TrainPredictor {
        #[arg(long, default_value = "dataset.jsonl")]
        data: PathBuf,
        #[arg(long, default_value = "predictor_checkpoints")]
        checkpoint_dir: PathBuf,
        #[arg(long, default_value = "300")]
        epochs: usize,
        #[arg(long, default_value = "4096")]
        batch_size: usize,
        #[arg(long, default_value = "1e-3")]
        learning_rate: f64,
        #[arg(long, default_value = "0.2")]
        val_split: f32,
        #[arg(long, default_value = "40")]
        max_seq_len: usize,
        #[arg(long, default_value = "128")]
        d_model: usize,
        #[arg(long, default_value = "8")]
        n_heads: usize,
        #[arg(long, default_value = "4")]
        n_layers: usize,
        #[arg(long, default_value = "512")]
        d_ff: usize,
        #[arg(long, default_value = "0.1")]
        dropout: f64,
        /// Clip target speedups below this value — removes measurement-noise outliers
        /// while keeping genuinely bad sequences.
        #[arg(long, default_value = "-3.0")]
        clip_min: f32,
        /// Huber loss delta — quadratic within ±delta of target, linear beyond.
        #[arg(long, default_value = "3.0")]
        huber_delta: f32,
        /// Cap total samples by taking this many evenly across all functions.
        /// Omit to use the full dataset.
        #[arg(long)]
        max_samples: Option<usize>,
    },
}

fn print_stats(label: &str, workers: usize, solo_ns: u64, results: &mut Vec<u64>) {
    results.sort_unstable();
    let mean = results.iter().sum::<u64>() / results.len() as u64;
    let median = results[results.len() / 2];
    let min = results[0];
    let max = results[results.len() - 1];
    let solo = solo_ns as f64;
    let ratio = mean as f64 / solo;
    let pct = (ratio - 1.0) * 100.0;
    let spread = (max - min) as f64 / solo * 100.0;
    println!("\n=== {} ({} workers) ===", label, workers);
    println!("  mean:   {} ns  ({:+.1}% vs solo)", mean, pct);
    println!("  median: {} ns", median);
    println!("  min:    {} ns", min);
    println!("  max:    {} ns", max);
    println!("  spread: {:.1}% of solo  (max-min / solo)", spread);
}

fn main() {
    let args = LlvmLstm::parse();
    match args.command {
        Command::Train {
            directory,
            work_dir,
            checkpoint_dir,
            clang,
            opt,
            epochs,
            ppo_epochs,
            episodes,
            benchmark_runs,
            benchmark_iters,
            baseline_runs,
            baseline_iters,
            max_seq_len,
            learning_rate,
            clip_epsilon,
            value_coef,
            entropy_coef,
            mini_batch_size,
            cache_file,
            sequences_file,
            proxy_alpha,
            returns,
            noop_threshold,
            delta_threshold,
            predictor_checkpoint,
            predictor_noop_threshold,
            predictor_scale,
        } => {
            let cfg = Cfg {
                functions: directory,
                clang,
                opt,
                epochs,
                ppo_epochs,
                episodes,
                benchmark_runs,
                benchmark_iters,
                baseline_runs,
                baseline_iters,
                max_seq_len,
                work_dir,
                checkpoint_dir: checkpoint_dir.clone(),
                learning_rate,
                clip_epsilon,
                value_coef,
                entropy_coef,
                mini_batch_size,
                cache_file,
                noop_threshold,
                delta_threshold,
            };
            let log_path = checkpoint_dir.join("train.jsonl");
            let seq_path =
                sequences_file.or_else(|| Some(checkpoint_dir.join("top_sequences.bin")));
            let returns_impl: Box<dyn crate::ppo::returns::Returns> = match returns.as_str() {
                "proxy" => Box::new(InstructionProxyReturn { alpha: proxy_alpha }),
                "weighted" => Box::new(InstructionWeightedTerminal {
                    threshold: delta_threshold,
                }),
                "predictor" => {
                    let ckpt = predictor_checkpoint
                        .expect("--predictor-checkpoint required when --returns=predictor");
                    Box::new(
                        crate::ppo::returns::predictor_return::PredictorReturn::load(
                            &ckpt,
                            predictor_noop_threshold,
                            predictor_scale,
                        )
                        .expect("failed to load predictor checkpoint"),
                    )
                }
                _ => Box::new(EpisodeReturn),
            };
            let trainer = Trainer::new(
                cfg,
                returns_impl,
                Box::new(BaselineAdvantage),
                LogMode::FileAndStdout,
                Some(log_path),
                seq_path,
            );
            trainer.train();
        }
        Command::Evaluate { model } => {
            let device = BurnDevice::default();
            let (loaded_model, meta) =
                Checkpoint::load(&model, &device).expect("failed to load checkpoint");
            let _inference_model = loaded_model.valid();
        }
        Command::PlotTrain { dir } => {
            // TODO: read dir + run python plotting
        }
        Command::Diagnose {
            sequences,
            directory,
            work_dir,
            clang,
            opt,
            top,
            runs,
            iters,
            baseline_runs,
            baseline_iters,
        } => {
            use crate::llvm::ir::Source;

            let top_seqs = TopSequences::load(&sequences).expect("load top sequences");
            if top_seqs.entries.is_empty() {
                println!("No sequences recorded yet.");
                return;
            }

            std::fs::create_dir_all(&work_dir).expect("create work dir");
            let mut functions = Functions::new(&directory);

            // Collect serial baselines for each function.
            let llvm = Llvm::new(&clang, &opt, work_dir.clone());
            println!("Collecting baselines...");
            for func in &mut functions.functions {
                let func_llvm = llvm.with_env(work_dir.join(&func.name));
                std::fs::create_dir_all(&func_llvm.work_dir).expect("create func work dir");
                func.ir = func_llvm.ir(&func.source).expect("emit ir");
                func.baselines = Some(
                    func_llvm
                        .collect_baselines(&func.source, baseline_runs, baseline_iters)
                        .expect("baselines"),
                );
                println!(
                    "  {} O3={} ns",
                    func.name,
                    func.baselines.as_ref().unwrap().o3.mean_ns
                );
            }

            let candidates: Vec<_> = top_seqs.entries.iter().take(top).collect();
            println!(
                "\nRe-benchmarking top {} sequences ({} runs each):\n",
                candidates.len(),
                runs
            );

            for (rank, entry) in candidates.iter().enumerate() {
                let func = match functions
                    .functions
                    .iter()
                    .find(|f| f.name == entry.func_name)
                {
                    Some(f) => f,
                    None => {
                        println!(
                            "  #{} [{}] func '{}' not found — skipping",
                            rank + 1,
                            entry.speedup,
                            entry.func_name
                        );
                        continue;
                    }
                };
                let baselines = func.baselines.as_ref().unwrap();
                let func_llvm = llvm.with_env(work_dir.join(format!("diag_{}", rank)));
                std::fs::create_dir_all(&func_llvm.work_dir).expect("create diag work dir");

                // Apply passes (skip Stop).
                let mut current_ir = func.ir.clone();
                let pass_strs: Vec<&str> = entry
                    .passes
                    .iter()
                    .filter(|&&p| p != Pass::Stop)
                    .map(|p| p.to_opt())
                    .collect();
                for (step, &pass) in entry.passes.iter().enumerate() {
                    if pass != Pass::Stop {
                        current_ir = func_llvm
                            .apply_one(&current_ir, pass, step)
                            .expect("apply pass");
                    }
                }
                let bin = func_llvm.compile(&current_ir).expect("compile");

                // Benchmark `runs` times individually for std.
                let mut speedups: Vec<f32> = Vec::with_capacity(runs);
                for _ in 0..runs {
                    let bm = func_llvm.benchmark(&bin, 1, iters).expect("benchmark");
                    speedups.push(baselines.speedup_vs_o3(bm.mean_ns));
                }
                speedups.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let mean = speedups.iter().sum::<f32>() / speedups.len() as f32;
                let var = speedups.iter().map(|&x| (x - mean).powi(2)).sum::<f32>()
                    / speedups.len() as f32;
                let std = var.sqrt();
                let med = speedups[speedups.len() / 2];

                println!(
                    "  #{:2}  func={}  cached={:+.4}  mean={:+.4}  std={:.4}  med={:+.4}  [{:+.4}, {:+.4}]",
                    rank + 1,
                    entry.func_name,
                    entry.speedup,
                    mean,
                    std,
                    med,
                    speedups[0],
                    speedups[speedups.len() - 1]
                );
                println!("       passes: [{}]", pass_strs.join(", "));
            }
        }
        Command::BenchNoise {
            source,
            clang,
            work_dir,
            runs,
            iters,
            workers,
        } => {
            use crate::llvm::Llvm;
            use crate::llvm::ir::Source;
            use rayon::prelude::*;

            std::fs::create_dir_all(&work_dir).expect("create work dir");
            let llvm = Llvm::new(&clang, "opt-20", work_dir.clone());
            let src = Source { file: source };

            println!("Emitting IR...");
            let ir = llvm.ir(&src).expect("emit IR");
            println!("Compiling IR...");
            let bin = llvm.compile(&ir).expect("compile IR");

            let solo = llvm.benchmark(&bin, runs, iters).expect("solo benchmark");
            println!("\n=== Serial (solo) ===");
            println!("  mean: {} ns", solo.mean_ns);

            let mut rayon_ns: Vec<u64> = (0..workers)
                .into_par_iter()
                .map(|_| {
                    let llvm2 = llvm.clone();
                    let bin2 = crate::llvm::ir::Bin {
                        file: bin.file.clone(),
                    };
                    llvm2
                        .benchmark(&bin2, runs, iters)
                        .expect("rayon worker bench")
                        .mean_ns
                })
                .collect();
            print_stats("Rayon parallel", workers, solo.mean_ns, &mut rayon_ns);
        }
        Command::Collect {
            cache_file,
            functions_dir,
            clang,
            opt,
            work_dir,
            output,
        } => {
            use crate::llvm::Llvm;
            use crate::llvm::functions::Functions;
            use crate::llvm::ir::{Features, Source};
            use crate::llvm::pass::Pass;
            use std::collections::HashMap;

            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent).expect("create output dir");
            }

            // Load cache
            let cache_data: Vec<((String, Vec<Pass>), (f32, Vec<f32>))> = {
                let bytes = std::fs::read(&cache_file).expect("read cache file");
                bincode::deserialize(&bytes).expect("deserialize cache")
            };

            println!("loaded cache w/ {} samples", cache_data.len());
            // Group by func_name
            let mut func_cache: HashMap<String, Vec<(Vec<Pass>, f32, Vec<f32>)>> = HashMap::new();
            for ((func_name, passes), (speedup, step_deltas)) in cache_data {
                func_cache
                    .entry(func_name)
                    .or_default()
                    .push((passes, speedup, step_deltas));
            }

            // Setup LLVM to compute IR features for each function
            let llvm = Llvm::new(&clang, &opt, work_dir.clone());
            let mut functions = Functions::new(&functions_dir);
            std::fs::create_dir_all(&work_dir).expect("create work dir");

            let mut out_file = std::fs::File::create(&output).expect("create output file");
            for func in &mut functions.functions {
                println!("Processing {}...", func.name);
                let func_llvm = llvm.with_env(work_dir.join(&func.name));
                std::fs::create_dir_all(&func_llvm.work_dir).expect("create func work dir");
                let ir = func_llvm.ir(&func.source).expect("emit IR");
                let content = std::fs::read_to_string(&ir.file).expect("read IR");
                let features = Features::from_ll_str(&content).expect("parse features");
                let ir_features = features.to_vec();

                if let Some(entries) = func_cache.get(&func.name) {
                    for (passes, speedup, step_deltas) in entries {
                        for len in 1..=passes.len() {
                            let sample = crate::predictor::data::Sample {
                                ir_features: ir_features.clone(),
                                passes: passes[0..len].to_vec(),
                                step_deltas: step_deltas[0..len].to_vec(),
                                speedup: *speedup,
                            };
                            serde_json::to_writer(&mut out_file, &sample).expect("write sample");
                            writeln!(&mut out_file).expect("newline");
                        }
                    }
                }
            }
            println!("Dataset written to {:?}", output);
        }
        Command::TrainPredictor {
            data,
            checkpoint_dir,
            epochs,
            batch_size,
            learning_rate,
            val_split,
            max_seq_len,
            d_model,
            n_heads,
            n_layers,
            d_ff,
            dropout,
            clip_min,
            huber_delta,
            max_samples,
        } => {
            let config = crate::predictor::model::SpeedupPredictorConfig {
                num_passes: 29,
                ir_feature_dim: 40,
                output_dim: 1,
                d_model,
                n_heads,
                n_layers,
                d_ff,
                dropout,
                max_seq_len,
            };
            crate::predictor::train::train_predictor(
                &data,
                &checkpoint_dir,
                epochs,
                batch_size,
                learning_rate,
                val_split,
                clip_min,
                huber_delta,
                max_samples,
                config,
            )
            .expect("training failed");
        }
    }
}
