use clap::{Parser, Subcommand};
use std::path::PathBuf;
use crate::config::Cfg;
use crate::train::Trainer;

mod llvm;
mod ppo;
mod config;
mod train;

#[derive(Parser)]
struct LlvmLstm {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Train {
        // options to cfg here\
        directory: PathBuf,
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
        Command::Train { directory} => {
            let mut cfg = Cfg::default();
            cfg.functions = directory;
            let mut trainer = Trainer::new(cfg);
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
