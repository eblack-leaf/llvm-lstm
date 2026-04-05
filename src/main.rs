#![recursion_limit = "256"]
#![allow(unused)]
use crate::config::{BurnAutoDiff, BurnDevice, Cfg};
use burn::module::AutodiffModule;
use crate::ppo::advantages::rank::RankAdvantage;
use crate::ppo::checkpoint::Checkpoint;
use crate::ppo::logging::{LogMode, Logger};
use crate::ppo::model::gru::GruActor;
use crate::ppo::model::transformer::TransformerActor;
use crate::ppo::returns::delta_weighted::DeltaWeightedReturn;
use crate::ppo::returns::episode_return::EpisodeReturn;
use crate::train::Trainer;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use crate::ppo::advantages::baseline::BaselineAdvantage;
use crate::ppo::advantages::gae::GaeAdvantage;
use crate::ppo::advantages::lookahead::LookaheadAdvantage;
use crate::ppo::returns::best_step::BestStepReturn;
use crate::ppo::returns::episodic_pattern::EpisodicPatternReturn; // EpisodicPatternReturn::new(0.99, 32, 0.2)
use crate::ppo::returns::lookahead::LookaheadCumulativeReturn;

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
        #[arg(long, default_value = "16")]
        episodes: usize,
        #[arg(long, default_value = "1")]
        benchmark_runs: usize,
        #[arg(long, default_value = "100")]
        benchmark_iters: usize,
        #[arg(long, default_value = "3")]
        baseline_runs: usize,
        #[arg(long, default_value = "200")]
        baseline_iters: usize,
        #[arg(long)]
        per_step_benchmark: bool,
        #[arg(long)]
        lookahead_benchmark: bool,
        #[arg(long, default_value = "1")]
        lookahead_runs: usize,
        #[arg(long, default_value = "50")]
        lookahead_iters: usize,
        #[arg(long, default_value = "40")]
        max_seq_len: usize,
        #[arg(long, default_value = "3e-4")]
        learning_rate: f64,
        #[arg(long, default_value = "0.1")]
        clip_epsilon: f32,
        #[arg(long, default_value = "0.5")]
        value_coef: f32,
        #[arg(long, default_value = "0.03")]
        entropy_coef: f32,
        #[arg(long, default_value = "64")]
        mini_batch_size: usize,
        /// Optional path to persist the benchmark cache across runs (episode-end
        /// and lookahead results). Omit to start fresh each run.
        #[arg(long)]
        cache_file: Option<PathBuf>,
    },
    Evaluate {
        #[arg(long, default_value = "checkpoints/best")]
        model: PathBuf,
    },
    PlotTrain {
        #[arg(long, default_value = "checkpoints")]
        dir: PathBuf,
    },
    PlotEvaluate {
        #[arg(long, default_value = "evaluation")]
        dir: PathBuf,
    },
    /// Measure how much parallel worker contention affects benchmark timing.
    /// Runs a single baseline benchmark alone, then 16 workers benchmarking
    /// the same binary concurrently, and reports the timing difference.
    BenchNoise {
        /// Path to the C source file to benchmark.
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
            per_step_benchmark,
            lookahead_benchmark,
            lookahead_runs,
            lookahead_iters,
            max_seq_len,
            learning_rate,
            clip_epsilon,
            value_coef,
            entropy_coef,
            mini_batch_size,
            cache_file,
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
                per_step_benchmark,
                lookahead_benchmark,
                lookahead_runs,
                lookahead_iters,
                max_seq_len,
                work_dir,
                checkpoint_dir: checkpoint_dir.clone(),
                learning_rate,
                clip_epsilon,
                value_coef,
                entropy_coef,
                mini_batch_size,
                cache_file,
            };
            let log_path = checkpoint_dir.join("train.jsonl");
            let trainer = Trainer::new(
                cfg,
                Box::new(EpisodicPatternReturn::new(300, 0.2)),
                Box::new(BaselineAdvantage::new(true)),
                LogMode::FileAndStdout,
                Some(log_path),
            );
            trainer.train();
        }
        Command::Evaluate { model } => {
            let device = BurnDevice::default();
            let (loaded_model, meta) =
                Checkpoint::load(&model, &device).expect("failed to load checkpoint");
            let inference_model = loaded_model.valid();
            // meta.max_seq_len, meta.speedup_ema available for eval setup
            // TODO: do baselines / greedy / random / model / compare
        }
        Command::PlotTrain { dir } => {
            // read dir + run python plotting
        }
        Command::PlotEvaluate { dir } => {
            // read dir + run python plotting
        }
        Command::BenchNoise { source, clang, work_dir, runs, iters, workers } => {
            use rayon::prelude::*;
            use crate::llvm::Llvm;
            use crate::llvm::ir::Source;

            std::fs::create_dir_all(&work_dir).expect("create work dir");
            let llvm = Llvm::new(&clang, "opt-20", work_dir.clone());
            let src = Source { file: source };

            // Emit IR then compile through the same pipeline as a training episode.
            println!("Emitting IR...");
            let ir = llvm.ir(&src).expect("emit IR");
            println!("Compiling IR...");
            let bin = llvm.compile(&ir).expect("compile IR");

            // ── 1. Serial solo run ──────────────────────────────────────────
            let solo = llvm.benchmark(&bin, runs, iters).expect("solo benchmark");
            println!("\n=== Serial (solo) ===");
            println!("  mean: {} ns", solo.mean_ns);

            // ── 2. Rayon parallel: `workers` threads, same binary ───────────
            let mut rayon_ns: Vec<u64> = (0..workers).into_par_iter().map(|_| {
                let llvm2 = llvm.clone();
                let bin2 = crate::llvm::ir::Bin { file: bin.file.clone() };
                llvm2.benchmark(&bin2, runs, iters).expect("rayon worker bench").mean_ns
            }).collect();
            print_stats("Rayon parallel", workers, solo.mean_ns, &mut rayon_ns);
        }
    }
}
