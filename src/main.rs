use crate::config::Cfg;
use crate::train::Trainer;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
        #[arg(long, default_value = "clang-20")]
        clang: String,
        #[arg(long, default_value = "opt-20")]
        opt: String,
    },
    Evaluate {
        #[arg(long, default_value = "checkpoints/best.mpk")]
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
            clang,
            opt,
            ..
        } => {
            let mut cfg = Cfg::default();
            cfg.functions = directory;
            cfg.clang = clang;
            cfg.opt = opt;
            let trainer = Trainer::new(cfg);
            trainer.train();
        }
        Command::Evaluate { model } => {
            // load model
            // do baselines
            // do greedy
            // do random
            // do model
            // compare
        }
        Command::PlotTrain { dir } => {
            // read dir + run python plotting
        }
        Command::PlotEvaluate { dir } => {
            // read dir + run python plotting
        }
    }
}
