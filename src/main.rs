use std::path::PathBuf;
use clap::{Parser, Subcommand, ValueEnum};

mod llvm;
mod ppo;
#[derive(Parser)]
struct LlvmLstm {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Train,
    Evaluate,
    Plot {
        #[arg(long, default_value = "train")]
        variant: Plot,
        #[arg(long, default_value = "checkpoints")]
        dir: PathBuf,
    }
}
#[derive(ValueEnum, Clone)]
enum Plot {
    Train,
    Evaluate,
}
fn main() {
    let args = LlvmLstm::parse();
    match args.command {
        Command::Train => {}
        Command::Evaluate => {}
        Command::Plot { dir, variant } => {}
    }
}