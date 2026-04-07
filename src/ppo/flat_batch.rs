// Place this in your `ppo` module, e.g., in `batch.rs` or inside `model.rs`

use crate::ppo::BatchEpisode;
use burn::prelude::*;
use burn::tensor::Tensor;

pub(crate) struct FlatBatch<B: Backend> {
    /// Total number of valid steps across all episodes in this mini‑batch.
    pub total_steps: usize,

    /// IR features padded to max_k, shape [n_episodes, max_k, n_features]
    pub ir_features: Tensor<B, 3>,

    /// For each valid step, its episode index and position in padded sequence.
    /// Used to gather logits/values from padded output.
    /// Shape [total_steps, 2] where [ep_idx, step_idx]
    pub gather_indices: Tensor<B, 2, Int>,

    /// Taken action index for each step, shape [total_steps]
    pub taken_idx: Tensor<B, 1, Int>,

    /// Old log probability for each step, shape [total_steps]
    pub old_log_probs: Tensor<B, 1>,

    /// Advantage for each step, shape [total_steps]
    pub advantages: Tensor<B, 1>,

    /// Target value (episode return) for each step, shape [total_steps]
    pub targets: Tensor<B, 1>,
}

impl<B: Backend> FlatBatch<B> {
    /// Build a `FlatBatch` from a collection of episodes, moving all data to the given device.
    pub fn from_episodes(episodes: &[BatchEpisode], device: &B::Device) -> Self {
        let n_episodes = episodes.len();
        let max_k = episodes.iter().map(|e| e.steps.len()).max().unwrap_or(1);
        let n_features = episodes[0].ir_features.len();

        // Build padded IR features: [n_episodes, max_k, n_features]
        let mut feat_data: Vec<f32> = Vec::with_capacity(n_episodes * max_k * n_features);
        for ep in episodes {
            for _ in 0..max_k {
                feat_data.extend(&ep.ir_features);
            }
        }
        let ir_features = Tensor::<B, 3>::from_data(
            TensorData::new(feat_data, [n_episodes, max_k, n_features]),
            device,
        );

        // Build flat arrays for all steps
        let mut total_steps = 0;
        let mut gather_ep = Vec::new();
        let mut gather_step = Vec::new();
        let mut taken_idx = Vec::new();
        let mut old_log_probs = Vec::new();
        let mut advantages = Vec::new();
        let mut targets = Vec::new();

        for (ep_idx, ep) in episodes.iter().enumerate() {
            let k = ep.steps.len();
            for (step_idx, step) in ep.steps.iter().enumerate() {
                gather_ep.push(ep_idx as i64);
                gather_step.push(step_idx as i64);
                taken_idx.push(step.taken_action_idx as i64);
                old_log_probs.push(step.old_log_prob);
                advantages.push(step.advantage);
                targets.push(step.ret);
                total_steps += 1;
            }
        }

        let gather_indices = Tensor::<B, 2, Int>::from_data(
            TensorData::new(
                gather_ep
                    .into_iter()
                    .zip(gather_step)
                    .flat_map(|(e, s)| vec![e, s])
                    .collect(),
                [total_steps, 2],
            ),
            device,
        );
        let taken_idx =
            Tensor::<B, 1, Int>::from_data(TensorData::new(taken_idx, [total_steps]), device);
        let old_log_probs =
            Tensor::<B, 1>::from_data(TensorData::new(old_log_probs, [total_steps]), device);
        let advantages =
            Tensor::<B, 1>::from_data(TensorData::new(advantages, [total_steps]), device);
        let targets = Tensor::<B, 1>::from_data(TensorData::new(targets, [total_steps]), device);

        FlatBatch {
            total_steps,
            ir_features,
            gather_indices,
            taken_idx,
            old_log_probs,
            advantages,
            targets,
        }
    }
}
