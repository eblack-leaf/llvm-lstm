use crate::llvm::pass::Pass;
use anyhow::{Context, Result};
use std::path::Path;

/// Keeps the top-K pass sequences seen across all training episodes.
/// Persisted alongside the bench cache so the Diagnose command can re-benchmark them.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct TopSequences {
    top_k: usize,
    /// Sorted descending by speedup.
    pub(crate) entries: Vec<TopEntry>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(crate) struct TopEntry {
    pub(crate) speedup: f32,
    pub(crate) func_name: String,
    pub(crate) passes: Vec<Pass>,
}

impl TopSequences {
    pub(crate) fn new(top_k: usize) -> Self {
        Self {
            top_k,
            entries: Vec::new(),
        }
    }

    pub(crate) fn update(&mut self, speedup: f32, func_name: &str, passes: &[Pass]) {
        if self.entries.len() < self.top_k
            || speedup
                > self
                    .entries
                    .last()
                    .map(|e| e.speedup)
                    .unwrap_or(f32::NEG_INFINITY)
        {
            self.entries.push(TopEntry {
                speedup,
                func_name: func_name.to_string(),
                passes: passes.to_vec(),
            });
            self.entries
                .sort_by(|a, b| b.speedup.partial_cmp(&a.speedup).unwrap());
            self.entries.truncate(self.top_k);
        }
    }

    pub(crate) fn save(&self, path: &Path) -> Result<()> {
        let bytes = bincode::serialize(self).context("serialize top sequences")?;
        std::fs::write(path, bytes).context("write top sequences")?;
        Ok(())
    }

    pub(crate) fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).context("read top sequences")?;
        bincode::deserialize(&bytes).context("deserialize top sequences")
    }
}
