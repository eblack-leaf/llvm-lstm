use crate::config::{Arch, ArchConfig, BurnAutoDiff, BurnDevice};
use crate::ppo::model::Actor;
use anyhow::Result;
use burn::module::AutodiffModule;
use burn::prelude::{Config, Module};
use burn::record::{FullPrecisionSettings, NamedMpkFileRecorder, Recorder};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Metadata saved alongside the model weights. Captures the training context
/// needed to interpret the checkpoint and reproduce the evaluation setup.
#[derive(Serialize, Deserialize)]
pub(crate) struct CheckpointMeta {
    pub(crate) epoch: usize,
    pub(crate) speedup_ema: f32,
    /// cfg.max_seq_len at training time — the episode rollout length limit.
    pub(crate) max_seq_len: usize,
}

pub(crate) struct Checkpoint;

impl Checkpoint {
    /// Save model weights, arch config, and metadata to `dir`.
    ///
    /// Files written:
    ///   `model.mpk`        — NamedMpk weights (full precision)
    ///   `arch_config.json` — arch hypers needed to re-init the model
    ///   `meta.json`        — epoch, speedup_ema, max_seq_len
    pub(crate) fn save(
        model: &Arch,
        arch_cfg: &ArchConfig,
        meta: CheckpointMeta,
        dir: &Path,
    ) -> Result<()> {
        std::fs::create_dir_all(dir)?;

        // Weights — burn adds .mpk extension to the path
        let recorder = NamedMpkFileRecorder::<FullPrecisionSettings>::new();
        model
            .clone()
            .save_file(dir.join("model"), &recorder)
            .map_err(|e| anyhow::anyhow!("save_file: {e:?}"))?;

        // Arch config — burn's Config trait serializes to JSON
        arch_cfg
            .save(dir.join("arch_config.json"))
            .map_err(|e| anyhow::anyhow!("arch_config save: {e:?}"))?;

        // Training metadata
        std::fs::write(dir.join("meta.json"), serde_json::to_string_pretty(&meta)?)?;

        Ok(())
    }

    /// Load a checkpoint from `dir`.
    ///
    /// Returns the model initialized with the saved arch config and loaded with
    /// the saved weights, plus the training metadata. Call `.valid()` on the
    /// returned model to get the inner non-autodiff model for inference.
    pub(crate) fn load(dir: &Path, device: &BurnDevice) -> Result<(Arch, CheckpointMeta)> {
        let meta: CheckpointMeta =
            serde_json::from_str(&std::fs::read_to_string(dir.join("meta.json"))?)?;

        let arch_cfg = ArchConfig::load(dir.join("arch_config.json"))
            .map_err(|e| anyhow::anyhow!("arch_config load: {e:?}"))?;

        let recorder = NamedMpkFileRecorder::<FullPrecisionSettings>::new();
        let record = recorder
            .load(dir.join("model").into(), device)
            .map_err(|e| anyhow::anyhow!("load record: {e:?}"))?;

        let model = <Arch as Actor<BurnAutoDiff>>::init(arch_cfg, device).load_record(record);

        Ok((model, meta))
    }
}
