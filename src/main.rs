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
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use burn::module::AutodiffModule;

mod config;
mod llvm;
mod ppo;
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
        #[arg(long, default_value = "32")]
        episodes: usize,
        #[arg(long, default_value = "1")]
        benchmark_runs: usize,
        #[arg(long, default_value = "100")]
        benchmark_iters: usize,
        #[arg(long, default_value = "3")]
        baseline_runs: usize,
        #[arg(long, default_value = "200")]
        baseline_iters: usize,
        #[arg(long, default_value = "40")]
        max_seq_len: usize,
        #[arg(long, default_value = "3e-4")]
        learning_rate: f64,
        #[arg(long, default_value = "0.1")]
        clip_epsilon: f32,
        #[arg(long, default_value = "0.75")]
        value_coef: f32,
        #[arg(long, default_value = "0.003")]
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
        /// weighted (terminal weighted by per-slot instr reduction; no-ops get 0).
        #[arg(long, default_value = "weighted")]
        returns: String,
        /// Steps with |instr_delta| <= this value are reported as no-ops in metrics (default 0 = exact no-op).
        #[arg(long, default_value = "0")]
        noop_threshold: usize,
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
}

fn print_stats(label: &str, workers: usize, solo_ns: u64, results: &mut Vec<u64>) {
    results.sort_unstable();
    let mean   = results.iter().sum::<u64>() / results.len() as u64;
    let median = results[results.len() / 2];
    let min    = results[0];
    let max    = results[results.len() - 1];
    let solo   = solo_ns as f64;
    let ratio  = mean as f64 / solo;
    let pct    = (ratio - 1.0) * 100.0;
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
            };
            let log_path = checkpoint_dir.join("train.jsonl");
            let seq_path = sequences_file
                .or_else(|| Some(checkpoint_dir.join("top_sequences.bin")));
            let returns_impl: Box<dyn crate::ppo::returns::Returns> = match returns.as_str() {
                "proxy"    => Box::new(InstructionProxyReturn { alpha: proxy_alpha }),
                "weighted" => Box::new(InstructionWeightedTerminal),
                _          => Box::new(EpisodeReturn),
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
        Command::Diagnose { sequences, directory, work_dir, clang, opt, top, runs, iters, baseline_runs, baseline_iters } => {
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
                    func_llvm.collect_baselines(&func.source, baseline_runs, baseline_iters)
                        .expect("baselines"),
                );
                println!("  {} O3={} ns", func.name,
                    func.baselines.as_ref().unwrap().o3.mean_ns);
            }

            let candidates: Vec<_> = top_seqs.entries.iter().take(top).collect();
            println!("\nRe-benchmarking top {} sequences ({} runs each):\n", candidates.len(), runs);

            for (rank, entry) in candidates.iter().enumerate() {
                let func = match functions.functions.iter().find(|f| f.name == entry.func_name) {
                    Some(f) => f,
                    None => {
                        println!("  #{} [{}] func '{}' not found — skipping",
                            rank + 1, entry.speedup, entry.func_name);
                        continue;
                    }
                };
                let baselines = func.baselines.as_ref().unwrap();
                let func_llvm = llvm.with_env(work_dir.join(format!("diag_{}", rank)));
                std::fs::create_dir_all(&func_llvm.work_dir).expect("create diag work dir");

                // Apply passes (skip Stop).
                let mut current_ir = func.ir.clone();
                let pass_strs: Vec<&str> = entry.passes.iter()
                    .filter(|&&p| p != Pass::Stop)
                    .map(|p| p.to_opt())
                    .collect();
                for (step, &pass) in entry.passes.iter().enumerate() {
                    if pass != Pass::Stop {
                        current_ir = func_llvm.apply_one(&current_ir, pass, step)
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
                let var  = speedups.iter().map(|&x| (x - mean).powi(2)).sum::<f32>()
                    / speedups.len() as f32;
                let std  = var.sqrt();
                let med  = speedups[speedups.len() / 2];

                println!("  #{:2}  func={}  cached={:+.4}  mean={:+.4}  std={:.4}  med={:+.4}  [{:+.4}, {:+.4}]",
                    rank + 1, entry.func_name, entry.speedup,
                    mean, std, med,
                    speedups[0], speedups[speedups.len() - 1]);
                println!("       passes: [{}]", pass_strs.join(", "));
            }
        }
        Command::BenchNoise { source, clang, work_dir, runs, iters, workers } => {
            use rayon::prelude::*;
            use crate::llvm::Llvm;
            use crate::llvm::ir::Source;

            std::fs::create_dir_all(&work_dir).expect("create work dir");
            let llvm = Llvm::new(&clang, "opt-20", work_dir.clone());
            let src  = Source { file: source };

            println!("Emitting IR...");
            let ir  = llvm.ir(&src).expect("emit IR");
            println!("Compiling IR...");
            let bin = llvm.compile(&ir).expect("compile IR");

            let solo = llvm.benchmark(&bin, runs, iters).expect("solo benchmark");
            println!("\n=== Serial (solo) ===");
            println!("  mean: {} ns", solo.mean_ns);

            let mut rayon_ns: Vec<u64> = (0..workers).into_par_iter().map(|_| {
                let llvm2 = llvm.clone();
                let bin2  = crate::llvm::ir::Bin { file: bin.file.clone() };
                llvm2.benchmark(&bin2, runs, iters).expect("rayon worker bench").mean_ns
            }).collect();
            print_stats("Rayon parallel", workers, solo.mean_ns, &mut rayon_ns);
        }
    }
}
