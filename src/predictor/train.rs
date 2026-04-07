use crate::config::BurnAutoDiff;
use crate::predictor::data::Sample;
use crate::predictor::model::{SpeedupPredictor, SpeedupPredictorConfig};
use anyhow::Result;
use burn::lr_scheduler::LrScheduler;
use burn::optim::{AdamConfig, GradientsParams, Optimizer};
use burn::module::AutodiffModule;
use burn::prelude::{Backend, Module, Tensor};
use burn::tensor::TensorData;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::Instant;
use crate::config::BurnDevice;

/// A single sample for the predictor.
#[derive(Clone)]
pub struct PredictorSample {
    pub ir_features: Vec<f32>,
    pub passes: Vec<crate::llvm::pass::Pass>,
    pub mask: Vec<bool>,
    pub speedup: f32,
}

/// Stable hash of ir_features to identify which function a sample came from.
/// Samples from the same function always have byte-identical feature vectors.
fn ir_features_key(features: &[f32]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for &f in features {
        f.to_bits().hash(&mut h);
    }
    h.finish()
}

/// Convert a batch of samples into batched tensors.
fn batch_to_tensors<B: Backend>(
    batch: &[PredictorSample],
    device: &B::Device,
    max_seq_len: usize,
    clip_min: f32,
) -> (Tensor<B, 2>, Tensor<B, 2, burn::tensor::Int>, Tensor<B, 2, burn::tensor::Bool>, Tensor<B, 1>) {
    let batch_size = batch.len();
    let feat_dim = batch[0].ir_features.len();
    let mut ir_data: Vec<f32> = Vec::with_capacity(batch_size * feat_dim);
    let mut pass_data: Vec<i64> = Vec::with_capacity(batch_size * max_seq_len);
    let mut mask_data: Vec<bool> = Vec::with_capacity(batch_size * max_seq_len);
    let mut target_data: Vec<f32> = Vec::with_capacity(batch_size);

    for sample in batch {
        let mut padded_passes = sample.passes.clone();
        padded_passes.resize(max_seq_len, crate::llvm::pass::Pass::Start);
        let mut padded_mask = sample.mask.clone();
        padded_mask.resize(max_seq_len, false);

        ir_data.extend(&sample.ir_features);
        pass_data.extend(padded_passes.iter().map(|&p| p as i64));
        mask_data.extend(padded_mask.iter().copied());
        target_data.push(sample.speedup.max(clip_min));
    }

    let ir = Tensor::from_data(TensorData::new(ir_data, [batch_size, feat_dim]), device);
    let passes = Tensor::from_data(TensorData::new(pass_data, [batch_size, max_seq_len]), device);
    let mask = Tensor::<B, 2, burn::tensor::Bool>::from_data(
        TensorData::new(mask_data, [batch_size, max_seq_len]),
        device,
    );
    let targets = Tensor::from_data(TensorData::new(target_data, [batch_size]), device);
    (ir, passes, mask, targets)
}

/// Huber loss: quadratic for |diff| ≤ delta, linear beyond — robust to outliers.
fn huber_loss<B: Backend<FloatElem = f32>>(diff: Tensor<B, 1>, delta: f32) -> Tensor<B, 1> {
    let abs_diff = diff.clone().abs();
    let quadratic = diff.clone() * diff * 0.5;
    let linear = abs_diff.clone() * delta - 0.5 * delta * delta;
    let big = abs_diff.greater_elem(delta);
    quadratic.mask_where(big, linear)
}

struct Metrics {
    mse: f32,
    rmse: f32,
    mae: f32,
    r2: f32,
    /// Mean signed error — positive means model over-predicts speedup.
    bias: f32,
    /// Std-dev of predictions — near zero means the model has collapsed to a constant.
    pred_std: f32,
}

fn compute_metrics<B: Backend<FloatElem = f32>>(
    predictions: &Tensor<B, 1>,
    targets: &Tensor<B, 1>,
) -> Metrics {
    let diff = predictions.clone() - targets.clone();
    let sq = diff.clone() * diff.clone();
    let mse: f32 = sq.clone().mean().into_scalar();
    let mae: f32 = diff.clone().abs().mean().into_scalar();
    let ss_res: f32 = sq.sum().into_scalar();
    let bias: f32 = diff.mean().into_scalar();

    let target_mean: f32 = targets.clone().mean().into_scalar();
    let dev = targets.clone() - target_mean;
    let ss_tot: f32 = (dev.clone() * dev).sum().into_scalar();
    let r2 = if ss_tot.abs() < 1e-10 { 0.0 } else { 1.0 - ss_res / ss_tot };

    let pred_mean: f32 = predictions.clone().mean().into_scalar();
    let pred_dev = predictions.clone() - pred_mean;
    let pred_var: f32 = (pred_dev.clone() * pred_dev).mean().into_scalar();
    let pred_std = pred_var.sqrt();

    Metrics { mse, rmse: mse.sqrt(), mae, r2, bias, pred_std }
}

/// Train the predictor model.
pub fn train_predictor(
    dataset_path: &Path,
    checkpoint_dir: &Path,
    epochs: usize,
    batch_size: usize,
    learning_rate: f64,
    val_split: f32,
    clip_min: f32,
    huber_delta: f32,
    max_samples: Option<usize>,
    config: SpeedupPredictorConfig,
) -> Result<()> {
    let device = BurnDevice::default();

    let all_samples = crate::predictor::data::load_dataset(dataset_path)?;
    if all_samples.is_empty() {
        anyhow::bail!("No samples found in dataset");
    }

    let all_predictor_samples: Vec<PredictorSample> = all_samples
        .iter()
        .map(|s| PredictorSample {
            ir_features: s.ir_features.clone(),
            passes: s.passes.clone(),
            mask: (0..s.passes.len()).map(|_| true).collect(),
            speedup: s.speedup,
        })
        .collect();

    let mut rng = rand::rng();

    // ---- Group samples by function (identical ir_features → same function) ----
    let mut func_groups: HashMap<u64, Vec<usize>> = HashMap::new();
    for (i, s) in all_predictor_samples.iter().enumerate() {
        func_groups.entry(ir_features_key(&s.ir_features)).or_default().push(i);
    }
    let n_funcs = func_groups.len();
    let n_total = all_predictor_samples.len();

    // ---- Apply per-function cap if requested ----
    let working_samples: Vec<PredictorSample> = if let Some(cap) = max_samples {
        if n_total > cap {
            // Distribute the cap evenly across functions; small functions contribute
            // fewer samples so larger ones fill the gap via a second pass.
            let mut groups: Vec<Vec<usize>> = func_groups.into_values().collect();
            for g in &mut groups {
                g.shuffle(&mut rng);
            }
            // Sort by size ascending so small functions donate their full set first
            // and the remaining budget goes to larger ones.
            groups.sort_by_key(|g| g.len());

            let mut selected: Vec<usize> = Vec::with_capacity(cap);
            let mut remaining = cap;
            for (i, group) in groups.iter().enumerate() {
                let funcs_left = groups.len() - i;
                let allot = (remaining / funcs_left).max(1).min(group.len());
                selected.extend_from_slice(&group[..allot]);
                remaining = remaining.saturating_sub(allot);
                if remaining == 0 {
                    break;
                }
            }

            let mut keep = vec![false; n_total];
            for &idx in &selected {
                keep[idx] = true;
            }
            all_predictor_samples.into_iter().zip(keep)
                .filter_map(|(s, k)| if k { Some(s) } else { None })
                .collect()
        } else {
            all_predictor_samples
        }
    } else {
        all_predictor_samples
    };

    // ---- Dataset summary ----
    {
        let n = working_samples.len();
        let mut speedups: Vec<f32> = working_samples.iter().map(|s| s.speedup).collect();
        speedups.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let sp_mean = speedups.iter().sum::<f32>() / n as f32;
        let sp_std = (speedups.iter().map(|&x| (x - sp_mean).powi(2)).sum::<f32>() / n as f32).sqrt();
        let p25 = speedups[n / 4];
        let p50 = speedups[n / 2];
        let p75 = speedups[n * 3 / 4];
        let pct_positive = speedups.iter().filter(|&&x| x > 0.0).count() as f32 / n as f32 * 100.0;

        let seq_lens: Vec<usize> = working_samples.iter().map(|s| s.passes.len()).collect();
        let sl_mean = seq_lens.iter().sum::<usize>() as f32 / n as f32;
        let sl_min = seq_lens.iter().min().copied().unwrap_or(0);
        let sl_max = seq_lens.iter().max().copied().unwrap_or(0);

        println!("=== Dataset ===");
        if let Some(cap) = max_samples.filter(|&c| c < n_total) {
            let per_func = cap / n_funcs;
            println!("  samples : {} → capped to {}  ({} functions, ~{}/func)",
                n_total, n, n_funcs, per_func);
        } else {
            println!("  samples : {}  ({} functions)", n_total, n_funcs);
        }
        println!("  speedup : min={:.4}  p25={:.4}  p50={:.4}  p75={:.4}  max={:.4}  mean={:.4}  std={:.4}  positive={:.1}%",
            speedups[0], p25, p50, p75, speedups[n - 1], sp_mean, sp_std, pct_positive);
        println!("  seq_len : min={}  max={}  mean={:.1}", sl_min, sl_max, sl_mean);
        println!("  split   : train={:.0}%  val={:.0}%", (1.0 - val_split) * 100.0, val_split * 100.0);
        println!("  model   : d_model={}  n_layers={}  n_heads={}  d_ff={}  dropout={}",
            config.d_model, config.n_layers, config.n_heads, config.d_ff, config.dropout);
        println!("  optim   : lr={:.2e}  epochs={}  batch={}", learning_rate, epochs, batch_size);
        println!("  loss    : huber(delta={})  clip_min={}", huber_delta, clip_min);
        println!();
    }

    let mut indices: Vec<usize> = (0..working_samples.len()).collect();
    indices.shuffle(&mut rng);
    let split_idx = ((1.0 - val_split) * working_samples.len() as f32) as usize;

    let train_samples: Vec<_> = indices[..split_idx]
        .iter()
        .map(|&i| working_samples[i].clone())
        .collect();
    let val_samples: Vec<_> = indices[split_idx..]
        .iter()
        .map(|&i| working_samples[i].clone())
        .collect();

    let mut model: SpeedupPredictor<BurnAutoDiff> = config.init(&device);

    let mut optimizer = AdamConfig::new().init::<BurnAutoDiff, _>();
    let mut scheduler = burn::lr_scheduler::cosine::CosineAnnealingLrSchedulerConfig::new(
        learning_rate,
        epochs,
    )
        .init()
        .expect("Failed to initialize scheduler");

    let mut best_val_loss = f32::INFINITY;
    let mut best_epoch = 0usize;
    let mut best_model = model.clone();

    std::fs::create_dir_all(checkpoint_dir)?;
    std::fs::write(
        checkpoint_dir.join("config.json"),
        serde_json::to_string_pretty(&config)?,
    )?;

    let multi = MultiProgress::new();

    let batch_pb = multi.add(ProgressBar::new(0));
    batch_pb.set_style(
        ProgressStyle::default_bar()
            .template("  {prefix:.bold} [{bar:45.yellow/white}] {pos:>6}/{len} batches  {msg}")
            .unwrap()
            .progress_chars("=> "),
    );

    let epoch_pb = multi.add(ProgressBar::new(epochs as u64));
    epoch_pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] epoch [{bar:30.cyan/blue}] {pos}/{len}  {msg}")
            .unwrap()
            .progress_chars("=> "),
    );

    epoch_pb.println(format!(
        "{:>6}  {:>10}  {:>8} {:>7} {:>6} {:>+7} {:>6}  |  {:>8} {:>7} {:>6} {:>+7}  {:>6}",
        "epoch", "lr",
        "tr_rmse", "tr_mae", "tr_r²", "tr_bias", "pstd",
        "va_rmse", "va_mae", "va_r²", "va_bias",
        "gap"
    ));

    for epoch in 0..epochs {
        let lr = scheduler.step();

        // ---------------- Training ----------------
        let n_train_batches = (train_samples.len() + batch_size - 1) / batch_size;
        batch_pb.reset();
        batch_pb.set_length(n_train_batches as u64);
        batch_pb.set_prefix("train");

        let mut train_loss_sum = 0.0f32;
        let mut train_steps = 0;
        let mut train_preds = Vec::new();
        let mut train_targs = Vec::new();

        let mut train_indices_shuffled: Vec<usize> = (0..train_samples.len()).collect();
        train_indices_shuffled.shuffle(&mut rng);

        let phase_start = Instant::now();
        let mut samples_done = 0usize;

        let mut batch_start = 0;
        while batch_start < train_samples.len() {
            let end = (batch_start + batch_size).min(train_samples.len());
            let batch: Vec<_> = train_indices_shuffled[batch_start..end]
                .iter()
                .map(|&i| train_samples[i].clone())
                .collect();

            let (ir, passes, mask, targets) = batch_to_tensors(&batch, &device, config.max_seq_len, clip_min);

            let output = model.forward(ir, passes, mask); // [B, 1]
            let output_flat = output.squeeze::<1>(); // [B]

            // Collect predictions before consuming tensors in the graph
            let pred_vec: Vec<f32> = output_flat.clone().into_data().to_vec::<f32>().unwrap();
            let targ_vec: Vec<f32> = targets.clone().into_data().to_vec::<f32>().unwrap();

            let diff = output_flat - targets;
            let loss = huber_loss(diff, huber_delta).mean();
            let loss_val: f32 = loss.clone().into_scalar();

            let grads = loss.backward();
            let grads = GradientsParams::from_grads(grads, &model);
            model = optimizer.step(lr, model, grads);

            samples_done += end - batch_start;
            train_loss_sum += loss_val;
            train_steps += 1;
            train_preds.extend(pred_vec);
            train_targs.extend(targ_vec);

            let samp_per_sec = samples_done as f32 / phase_start.elapsed().as_secs_f32().max(1e-6);
            batch_pb.set_message(format!(
                "loss={:.5}  {:.1}k samp/s",
                loss_val, samp_per_sec / 1000.0,
            ));
            batch_pb.inc(1);

            batch_start = end;
        }
        let avg_train_loss = train_loss_sum / train_steps as f32;

        let n_train = train_preds.len();
        let train_pred_tensor = Tensor::<BurnAutoDiff, 1>::from_data(
            TensorData::new(train_preds, [n_train]),
            &device,
        );
        let train_targ_tensor = Tensor::<BurnAutoDiff, 1>::from_data(
            TensorData::new(train_targs, [n_train]),
            &device,
        );
        let tr = compute_metrics(&train_pred_tensor, &train_targ_tensor);

        // ---------------- Validation ----------------
        let n_val_batches = (val_samples.len() + batch_size - 1) / batch_size;
        batch_pb.reset();
        batch_pb.set_length(n_val_batches as u64);
        batch_pb.set_prefix("val  ");
        batch_pb.set_message(String::new());

        let valid_model = model.valid();
        let mut val_loss_sum = 0.0f32;
        let mut val_steps = 0;
        let mut val_preds = Vec::new();
        let mut val_targs = Vec::new();

        let val_start = Instant::now();
        let mut val_samples_done = 0usize;

        let mut batch_start = 0;
        while batch_start < val_samples.len() {
            let end = (batch_start + batch_size).min(val_samples.len());
            let batch = val_samples[batch_start..end].to_vec();
            let (ir, passes, mask, targets) =
                batch_to_tensors::<crate::config::BurnBackend>(&batch, &device, config.max_seq_len, clip_min);

            let output = valid_model.forward(ir, passes, mask);
            let output_flat = output.squeeze::<1>();

            let pred_vec: Vec<f32> = output_flat.clone().into_data().to_vec::<f32>().unwrap();
            let targ_vec: Vec<f32> = targets.clone().into_data().to_vec::<f32>().unwrap();

            let diff = output_flat - targets;
            let loss = huber_loss(diff, huber_delta).mean();
            val_loss_sum += loss.into_scalar();
            val_steps += 1;
            val_samples_done += end - batch_start;

            val_preds.extend(pred_vec);
            val_targs.extend(targ_vec);

            let samp_per_sec = val_samples_done as f32 / val_start.elapsed().as_secs_f32().max(1e-6);
            batch_pb.set_message(format!("{:.1}k samp/s", samp_per_sec / 1000.0));
            batch_pb.inc(1);

            batch_start = end;
        }
        batch_pb.finish_and_clear();
        let avg_val_loss = val_loss_sum / val_steps as f32;

        let n_val = val_preds.len();
        let val_pred_tensor = Tensor::<BurnAutoDiff, 1>::from_data(
            TensorData::new(val_preds, [n_val]),
            &device,
        );
        let val_targ_tensor = Tensor::<BurnAutoDiff, 1>::from_data(
            TensorData::new(val_targs, [n_val]),
            &device,
        );
        let va = compute_metrics(&val_pred_tensor, &val_targ_tensor);

        let is_best = avg_val_loss < best_val_loss;
        if is_best {
            best_val_loss = avg_val_loss;
            best_epoch = epoch;
            best_model = model.clone();

            let recorder =
                burn::record::NamedMpkFileRecorder::<burn::record::FullPrecisionSettings>::new();
            best_model.save_file(&checkpoint_dir.join("best_model"), &recorder)?;
        }

        let gap = if avg_train_loss > 0.0 { avg_val_loss / avg_train_loss } else { f32::NAN };
        let best_marker = if is_best { " ★" } else { "" };
        epoch_pb.println(format!(
            "{:>6}  {:>10.3e}  {:>8.5} {:>7.5} {:>7.3} {:>+6.4} {:>6.4}  |  {:>8.5} {:>7.5} {:>7.3} {:>+6.4}  {:>5.2}x{}",
            epoch, lr,
            tr.rmse, tr.mae, tr.r2, tr.bias, tr.pred_std,
            va.rmse, va.mae, va.r2, va.bias,
            gap, best_marker,
        ));
        epoch_pb.inc(1);
    }

    epoch_pb.finish_with_message(format!(
        "done — best val rmse={:.5} at epoch {}",
        best_val_loss.sqrt(), best_epoch
    ));
    Ok(())
}