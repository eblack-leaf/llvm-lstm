use crate::config::BurnAutoDiff;
use crate::predictor::data::Sample;
use crate::predictor::model::{SpeedupPredictor, SpeedupPredictorConfig};
use anyhow::Result;
use burn::lr_scheduler::LrScheduler;
use burn::optim::{Adam, AdamConfig, GradientsParams, Optimizer};
use burn::prelude::{Backend, Module, Tensor};
use burn::tensor::TensorData;
use indicatif::{ProgressBar, ProgressStyle};
use rand::seq::SliceRandom;
use rand::Rng;
use std::io::Write;
use std::path::Path;
use crate::config::BurnDevice;

/// A single sample for the predictor.
#[derive(Clone)]
pub struct PredictorSample {
    pub ir_features: Vec<f32>,
    pub passes: Vec<crate::llvm::pass::Pass>,
    pub mask: Vec<bool>,
    pub speedup: f32,
}

/// Convert a batch of samples into batched tensors.
fn batch_to_tensors<B: Backend>(
    batch: &[PredictorSample],
    device: &B::Device,
    max_seq_len: usize,
) -> (Tensor<B, 2>, Tensor<B, 2, burn::tensor::Int>, Tensor<B, 2, burn::tensor::Bool>, Tensor<B, 1>) {
    let batch_size = batch.len();
    let feat_dim = batch[0].ir_features.len();
    let mut ir_data: Vec<f32> = Vec::with_capacity(batch_size * feat_dim);
    let mut pass_data: Vec<i64> = Vec::with_capacity(batch_size * max_seq_len);
    let mut mask_data: Vec<u8> = Vec::with_capacity(batch_size * max_seq_len);
    let mut target_data: Vec<f32> = Vec::with_capacity(batch_size);

    for sample in batch {
        let mut padded_passes = sample.passes.clone();
        padded_passes.resize(max_seq_len, crate::llvm::pass::Pass::Start);
        let mut padded_mask = sample.mask.clone();
        padded_mask.resize(max_seq_len, false);

        ir_data.extend(&sample.ir_features);
        pass_data.extend(padded_passes.iter().map(|&p| p as i64));
        mask_data.extend(padded_mask.iter().map(|&b| b as u8));
        target_data.push(sample.speedup);
    }

    let ir = Tensor::from_data(TensorData::new(ir_data, [batch_size, feat_dim]), device);
    let passes = Tensor::from_data(TensorData::new(pass_data, [batch_size, max_seq_len]), device);
    let mask = Tensor::from_data(TensorData::new(mask_data, [batch_size, max_seq_len]), device);
    let targets = Tensor::from_data(TensorData::new(target_data, [batch_size]), device);
    (ir, passes, mask, targets)
}

/// Train the predictor model using the autodiff backend.
pub fn train_predictor(
    dataset_path: &Path,
    checkpoint_dir: &Path,
    epochs: usize,
    batch_size: usize,
    learning_rate: f64,
    val_split: f32,
    max_seq_len: usize,
) -> Result<()> {
    let device = BurnDevice::default();

    let all_samples = crate::predictor::data::load_dataset(dataset_path)?;
    if all_samples.is_empty() {
        anyhow::bail!("No samples found in dataset");
    }

    // Convert to PredictorSample with masks
    let all_predictor_samples: Vec<PredictorSample> = all_samples
        .iter()
        .map(|s| PredictorSample {
            ir_features: s.ir_features.clone(),
            passes: s.passes.clone(),
            mask: (0..s.passes.len()).map(|_| true).collect(),
            speedup: s.speedup,
        })
        .collect();

    // Shuffle & split
    let mut rng = rand::rng();
    let mut indices: Vec<usize> = (0..all_predictor_samples.len()).collect();
    indices.shuffle(&mut rng);
    let split_idx = ((1.0 - val_split) * all_predictor_samples.len() as f32) as usize;

    let train_samples: Vec<_> = indices[..split_idx]
        .iter()
        .map(|&i| all_predictor_samples[i].clone())
        .collect();
    let val_samples: Vec<_> = indices[split_idx..]
        .iter()
        .map(|&i| all_predictor_samples[i].clone())
        .collect();

    // Model
    let config = SpeedupPredictorConfig {
        num_passes: 29,
        pass_embed_dim: 32,
        ir_feature_dim: 40,
        hidden_dim: 64,
        output_dim: 1,
    };
    let mut model: SpeedupPredictor<BurnAutoDiff> = config.init(&device);

    let mut optimizer = AdamConfig::new().init::<BurnAutoDiff, _>();
    let mut scheduler = burn::lr_scheduler::cosine::CosineAnnealingLrSchedulerConfig::new(
        learning_rate,
        epochs,
    )
        .init().expect("Failed to initialize optimizer");

    let mut best_val_loss = f32::INFINITY;
    let mut best_model = model.clone();

    // Progress bar
    let pb = ProgressBar::new(epochs as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} epochs {msg}")
            .unwrap()
            .progress_chars("=> "),
    );

    for epoch in 0..epochs {
        // ---------------- Training ----------------
        let mut train_loss_sum = 0.0;
        let mut train_steps = 0;

        let mut train_indices_shuffled: Vec<usize> = (0..train_samples.len()).collect();
        train_indices_shuffled.shuffle(&mut rng);

        let mut batch_start = 0;
        while batch_start < train_samples.len() {
            let end = (batch_start + batch_size).min(train_samples.len());
            let batch: Vec<_> = train_indices_shuffled[batch_start..end]
                .iter()
                .map(|&i| train_samples[i].clone())
                .collect();

            let (ir, passes, mask, targets) = batch_to_tensors(&batch, &device, max_seq_len);

            let output = model.forward(ir, passes, mask); // [B, 1]
            let output_flat = output.squeeze::<1>(); // [B]

            let diff = output_flat.clone() - targets;
            let loss = (diff.clone() * diff).mean();

            let grads = loss.backward();
            let grads = GradientsParams::from_grads(grads, &model);
            model = optimizer.step(learning_rate, model, grads);

            train_loss_sum += loss.into_scalar();
            train_steps += 1;
            batch_start = end;
        }
        let avg_train_loss = train_loss_sum / train_steps as f32;

        // ---------------- Validation ----------------
        let mut val_loss_sum = 0.0;
        let mut val_steps = 0;
        let mut batch_start = 0;
        while batch_start < val_samples.len() {
            let end = (batch_start + batch_size).min(val_samples.len());
            let batch = val_samples[batch_start..end].to_vec();
            let (ir, passes, mask, targets) = batch_to_tensors(&batch, &device, max_seq_len);

            let output = model.forward(ir, passes, mask);
            let output_flat = output.squeeze::<1>();

            let diff = output_flat.clone() - targets;
            let loss = (diff.clone() * diff).mean();

            val_loss_sum += loss.into_scalar();
            val_steps += 1;
            batch_start = end;
        }
        let avg_val_loss = val_loss_sum / val_steps as f32;

        if avg_val_loss < best_val_loss {
            best_val_loss = avg_val_loss;
            best_model = model.clone();

            std::fs::create_dir_all(checkpoint_dir)?;
            let recorder =
                burn::record::NamedMpkFileRecorder::<burn::record::FullPrecisionSettings>::new();
            best_model.save_file(&checkpoint_dir.join("best_model"), &recorder)?;
            std::fs::write(
                checkpoint_dir.join("config.json"),
                serde_json::to_string_pretty(&config)?,
            )?;
        }

        scheduler.step();
        pb.set_message(format!(
            "epoch {} | train_loss={:.5} val_loss={:.5} best={:.5}",
            epoch, avg_train_loss, avg_val_loss, best_val_loss
        ));
        pb.inc(1);
    }

    pb.finish_with_message("Training completed");
    Ok(())
}