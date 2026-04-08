use crate::llvm::pass::Pass;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sample {
    /// Name of the source function — used to look up the base IR at training time.
    pub func_name: String,
    pub passes: Vec<Pass>,
    /// Normalised instruction-count delta per step.
    /// step_deltas[t] = tanh((instr[t] - instr[t+1]) / instr[t]).
    /// len == passes.len().
    pub step_deltas: Vec<f32>,
    pub speedup: f32,
}

pub fn load_dataset(path: &Path) -> Result<Vec<Sample>> {
    let file = File::open(path).context("open dataset")?;
    let reader = BufReader::new(file);
    let mut samples = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let sample: Sample = serde_json::from_str(&line).context("parse sample")?;
        samples.push(sample);
    }
    Ok(samples)
}
