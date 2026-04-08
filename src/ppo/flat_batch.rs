use crate::ppo::episode::Results;
use crate::ppo::model::ACTIONS;
use burn::prelude::*;
use burn::tensor::Tensor;

pub(crate) struct FlatBatch<B: Backend> {
    /// Total number of valid steps across all episodes in this mini-batch.
    pub total_steps: usize,

    /// Chunked IR histogram, shape [n_episodes, ir_feature_dim].
    pub ir_features: Tensor<B, 2>,

    /// For each valid step, its episode index and position in the episode.
    /// Shape [total_steps, 2] where col 0 = ep_idx (global), col 1 = step_idx.
    pub gather_indices: Tensor<B, 2, Int>,

    /// Taken action index for each step, shape [total_steps]
    pub taken_idx: Tensor<B, 1, Int>,

    /// Old log probability for each step, shape [total_steps]
    pub old_log_probs: Tensor<B, 1>,

    /// Advantage for each step, shape [total_steps]
    pub advantages: Tensor<B, 1>,

    /// Per-step return (target for value head) for each step, shape [total_steps]
    pub targets: Tensor<B, 1>,
}

impl<B: Backend> FlatBatch<B> {
    pub fn from_results(
        results: &[Results],
        returns: &[Vec<f32>],
        advantages: &[Vec<f32>],
        device: &B::Device,
    ) -> Self {
        let n_episodes = results.len();
        let ir_feature_dim = results.first().map(|r| r.ir_features.len()).unwrap_or(0);

        // Stack IR feature vectors: one row per episode.
        let mut feat_data: Vec<f32> = Vec::with_capacity(n_episodes * ir_feature_dim);
        for r in results {
            feat_data.extend_from_slice(&r.ir_features);
        }
        let ir_features = Tensor::<B, 2>::from_data(
            TensorData::new(feat_data, [n_episodes, ir_feature_dim]),
            device,
        );

        let mut total_steps = 0usize;
        let mut gather_ep: Vec<i64> = Vec::new();
        let mut gather_step: Vec<i64> = Vec::new();
        let mut taken_idx: Vec<i64> = Vec::new();
        let mut old_log_probs: Vec<f32> = Vec::new();
        let mut adv_data: Vec<f32> = Vec::new();
        let mut target_data: Vec<f32> = Vec::new();

        for (ep_idx, ((r, ep_rets), ep_advs)) in results
            .iter()
            .zip(returns.iter())
            .zip(advantages.iter())
            .enumerate()
        {
            for t in 0..r.ep_len {
                let action_idx = ACTIONS
                    .iter()
                    .position(|&p| p == r.actions[t])
                    .expect("action not in ACTIONS");
                gather_ep.push(ep_idx as i64);
                gather_step.push(t as i64);
                taken_idx.push(action_idx as i64);
                old_log_probs.push(r.log_probs[t]);
                adv_data.push(ep_advs[t]);
                target_data.push(ep_rets[t]);
                total_steps += 1;
            }
        }

        let gather_indices = Tensor::<B, 2, Int>::from_data(
            TensorData::new(
                gather_ep
                    .into_iter()
                    .zip(gather_step)
                    .flat_map(|(e, s)| [e, s])
                    .collect::<Vec<_>>(),
                [total_steps, 2],
            ),
            device,
        );
        let taken_idx =
            Tensor::<B, 1, Int>::from_data(TensorData::new(taken_idx, [total_steps]), device);
        let old_log_probs =
            Tensor::<B, 1>::from_data(TensorData::new(old_log_probs, [total_steps]), device);
        let advantages =
            Tensor::<B, 1>::from_data(TensorData::new(adv_data, [total_steps]), device);
        let targets =
            Tensor::<B, 1>::from_data(TensorData::new(target_data, [total_steps]), device);

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
