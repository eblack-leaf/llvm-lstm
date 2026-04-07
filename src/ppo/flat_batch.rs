use crate::llvm::ir::PAD_OPCODE;
use crate::ppo::episode::Results;
use crate::ppo::model::ACTIONS;
use burn::prelude::*;
use burn::tensor::Tensor;

pub(crate) struct FlatBatch<B: Backend> {
    /// Total number of valid steps across all episodes in this mini-batch.
    pub total_steps: usize,

    /// Padded opcode-ID sequences, shape [n_episodes, max_ir_len].
    pub ir_opcodes: Tensor<B, 2, Int>,

    /// Padding mask for the IR encoder, shape [n_episodes, max_ir_len].
    /// `true` = PAD position (excluded from attention and mean-pool).
    pub ir_padding_mask: Tensor<B, 2, Bool>,

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
    /// Build a `FlatBatch` directly from episode results, per-step returns, and advantages.
    pub fn from_results(
        results: &[Results],
        returns: &[Vec<f32>],
        advantages: &[Vec<f32>],
        max_ir_len: usize,
        device: &B::Device,
    ) -> Self {
        let n_episodes = results.len();

        // Opcode sequences: pad each episode's raw sequence to max_ir_len.
        let mut opcode_data: Vec<i64> = Vec::with_capacity(n_episodes * max_ir_len);
        let mut mask_data: Vec<bool> = Vec::with_capacity(n_episodes * max_ir_len);
        for r in results {
            let raw_len = r.ir_opcodes.len().min(max_ir_len);
            for i in 0..max_ir_len {
                if i < raw_len {
                    opcode_data.push(r.ir_opcodes[i] as i64);
                    mask_data.push(false);
                } else {
                    opcode_data.push(PAD_OPCODE as i64);
                    mask_data.push(true);
                }
            }
        }
        let ir_opcodes = Tensor::<B, 2, Int>::from_data(
            TensorData::new(opcode_data, [n_episodes, max_ir_len]),
            device,
        );
        let ir_padding_mask = Tensor::<B, 2, Bool>::from_data(
            TensorData::new(mask_data, [n_episodes, max_ir_len]),
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
            ir_opcodes,
            ir_padding_mask,
            gather_indices,
            taken_idx,
            old_log_probs,
            advantages,
            targets,
        }
    }
}
