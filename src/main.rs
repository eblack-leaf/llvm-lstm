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
            };
            let log_path = checkpoint_dir.join("train.jsonl");
            let trainer = Trainer::new(
                cfg,
                Box::new(LookaheadCumulativeReturn::new(0.99)),
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
    }
}
