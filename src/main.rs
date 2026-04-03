#![allow(unused)]
use crate::config::{Arch, BurnAutoDiff, Cfg};
use crate::ppo::advantages::rank::RankAdvantage;
use crate::ppo::model::gru::GruActor;
use crate::ppo::model::transformer::TransformerActor;
use crate::ppo::returns::episode_return::EpisodeReturn;
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
        #[arg(long, default_value = "tfx")]
        arch: Arch,
        #[arg(long, default_value = "40")]
        max_seq_len: usize,
        // ...
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
            clang,
            opt,
            arch,
            max_seq_len,
        } => {
            let mut cfg = Cfg::default();
            cfg.functions = directory;
            cfg.clang = clang;
            cfg.opt = opt;
            cfg.arch = arch;
            cfg.max_seq_len = max_seq_len;
            // ...
            let trainer = Trainer::new(
                cfg,
                Box::new(EpisodeReturn),
                Box::new(RankAdvantage::new(true)),
            );
            match arch {
                Arch::Tfx => trainer.train::<TransformerActor<BurnAutoDiff>>(),
                Arch::Gru => trainer.train::<GruActor<BurnAutoDiff>>(),
            }
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
