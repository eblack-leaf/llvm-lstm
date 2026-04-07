use crate::config::{BurnBackend, BurnDevice};
use crate::llvm::ir::{PAD_OPCODE, step_delta};
use crate::llvm::pass::Pass;
use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;
use crate::predictor::model::{SpeedupPredictor, SpeedupPredictorConfig};
use burn::prelude::{Module, Tensor};
use burn::record::{FullPrecisionSettings, NamedMpkFileRecorder, Recorder};
use burn::tensor::{Bool, Int, TensorData};
use std::path::Path;

/// Per-step returns driven by the offline-trained SpeedupPredictor.
///
/// For each prefix [a_0, ..., a_t] in an episode the model predicts the
/// expected final speedup.  The return assigned to step t is the *marginal*
/// contribution of action t:
///
///   r_t = pred(prefix_t) - pred(prefix_{t-1})
///
/// with pred(prefix_{-1}) = 0 for the first step.
pub(crate) struct PredictorReturn {
    model: SpeedupPredictor<BurnBackend>,
    device: BurnDevice,
    max_seq_len: usize,
    max_ir_len: usize,
    noop_threshold: f32,
    scale: f32,
}

impl PredictorReturn {
    pub(crate) fn load(
        checkpoint_dir: &Path,
        noop_threshold: f32,
        scale: f32,
    ) -> anyhow::Result<Self> {
        let device = BurnDevice::default();

        let config: SpeedupPredictorConfig = serde_json::from_str(&std::fs::read_to_string(
            checkpoint_dir.join("config.json"),
        )?)?;

        let recorder = NamedMpkFileRecorder::<FullPrecisionSettings>::new();
        let record = recorder
            .load(checkpoint_dir.join("best_model").into(), &device)
            .map_err(|e| anyhow::anyhow!("predictor load: {e:?}"))?;

        let model = config.init::<BurnBackend>(&device).load_record(record);

        Ok(Self {
            model,
            device,
            max_seq_len: config.max_seq_len,
            max_ir_len: config.max_ir_len,
            noop_threshold,
            scale,
        })
    }
}

impl Returns for PredictorReturn {
    fn compute(&self, results: &Results) -> Vec<f32> {
        let ep_len = results.ep_len;
        if ep_len == 0 {
            return Vec::new();
        }

        // Build batch of ep_len rows: one per prefix length.
        // IR opcodes are the same for all rows — repeat the padded sequence.
        let raw_ir_len = results.ir_opcodes.len().min(self.max_ir_len);
        let mut ir_opcode_data: Vec<i64> = Vec::with_capacity(ep_len * self.max_ir_len);
        let mut ir_mask_data: Vec<bool> = Vec::with_capacity(ep_len * self.max_ir_len);
        for _ in 0..ep_len {
            for i in 0..self.max_ir_len {
                if i < raw_ir_len {
                    ir_opcode_data.push(results.ir_opcodes[i] as i64);
                    ir_mask_data.push(false);
                } else {
                    ir_opcode_data.push(PAD_OPCODE as i64);
                    ir_mask_data.push(true);
                }
            }
        }

        let mut pass_data: Vec<i64> = Vec::with_capacity(ep_len * self.max_seq_len);
        let mut delta_data: Vec<f32> = Vec::with_capacity(ep_len * self.max_seq_len);
        let mut mask_data: Vec<bool> = Vec::with_capacity(ep_len * self.max_seq_len);

        for t in 0..ep_len {
            for slot in 0..self.max_seq_len {
                if slot <= t {
                    let pass = results.actions.get(slot).copied().unwrap_or(Pass::Start);
                    pass_data.push(pass as i64);
                    mask_data.push(true);
                    let instr_before = results.instr_counts.get(slot).copied().unwrap_or(1);
                    let instr_after = results.instr_counts.get(slot + 1).copied().unwrap_or(0);
                    delta_data.push(step_delta(instr_before, instr_after));
                } else {
                    pass_data.push(Pass::Start as i64);
                    mask_data.push(false);
                    delta_data.push(0.0);
                }
            }
        }

        let ir_opcodes = Tensor::<BurnBackend, 2, Int>::from_data(
            TensorData::new(ir_opcode_data, [ep_len, self.max_ir_len]),
            &self.device,
        );
        let ir_mask = Tensor::<BurnBackend, 2, Bool>::from_data(
            TensorData::new(ir_mask_data, [ep_len, self.max_ir_len]),
            &self.device,
        );
        let passes = Tensor::<BurnBackend, 2, Int>::from_data(
            TensorData::new(pass_data, [ep_len, self.max_seq_len]),
            &self.device,
        );
        let mask = Tensor::<BurnBackend, 2, Bool>::from_data(
            TensorData::new(mask_data, [ep_len, self.max_seq_len]),
            &self.device,
        );
        let deltas = Tensor::<BurnBackend, 2>::from_data(
            TensorData::new(delta_data, [ep_len, self.max_seq_len]),
            &self.device,
        );

        let preds: Vec<f32> = self
            .model
            .forward(ir_opcodes, ir_mask, passes, mask, deltas)
            .squeeze::<1>()
            .into_data()
            .to_vec::<f32>()
            .unwrap();

        let mut returns = Vec::with_capacity(ep_len);
        let mut prev = 0.0_f32;
        for (t, pred) in preds.into_iter().enumerate() {
            let delta = pred - prev;
            let instr_before = results.instr_counts.get(t).copied().unwrap_or(1);
            let instr_after = results.instr_counts.get(t + 1).copied().unwrap_or(0);
            let r = if step_delta(instr_before, instr_after).abs() < self.noop_threshold {
                0.0
            } else {
                delta
            };
            returns.push(r * self.scale);
            prev = pred;
        }
        returns
    }
}
