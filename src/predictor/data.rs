use crate::llvm::pass::Pass;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sample {
    pub ir_features: Vec<f32>,
    pub passes: Vec<Pass>,
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