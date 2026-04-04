use crate::llvm::pass::Pass;
use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Index of `total_instruction_count` in the feature vector produced by `Features::to_vec()`.
const TOTAL_INSTR_IDX: usize = 17;

/// Credits the episode-level speedup to each step by how much that step changed
/// the total instruction count, measured as the log-ratio of consecutive counts:
///
///   weight[t] = |ln(instr[t]) - ln(instr[t-1])|
///             = |delta_features[t][17]|   (stored as marginal in train.rs)
///
/// This is already scale-stable (log-ratio is bounded regardless of IR size).
/// The most impactful step gets return = sign(adjusted_speedup); others proportionally
/// less; no-op steps (no instruction count change) get 0 or the noop penalty.
///
/// `length_coef`: penalty on episode length, applied before taking the sign.
///   adjusted = speedup − length_coef × (ep_len / max_seq_len)
///
/// `noop_penalty`: fixed negative return for non-Stop steps that changed no instructions.
///   Stop is exempt — the length penalty already rewards early termination.
pub(crate) struct DeltaWeightedReturn {
    pub(crate) length_coef: f32,
    pub(crate) max_seq_len: usize,
    pub(crate) noop_penalty: f32,
}

impl DeltaWeightedReturn {
    pub(crate) fn new(length_coef: f32, max_seq_len: usize, noop_penalty: f32) -> Self {
        Self { length_coef, max_seq_len, noop_penalty }
    }
}

impl Returns for DeltaWeightedReturn {
    fn compute(&self, results: &Results) -> Vec<f32> {
        let reward = results
            .steps
            .iter()
            .filter_map(|s| s.benchmark.as_ref())
            .last()
            .map(|b| b.speedup)
            .unwrap_or(0.0);

        let n = results.log_probs.len();
        if n == 0 {
            return vec![];
        }

        let adjusted = reward - self.length_coef * (n as f32 / self.max_seq_len as f32);

        let weights: Vec<f32> = results.steps.iter()
            .map(|s| s.delta_features[TOTAL_INSTR_IDX].abs())
            .collect();

        let max_w = weights.iter().cloned().fold(0.0f32, f32::max);
        if max_w < 1e-8 {
            return vec![adjusted.signum(); n];
        }

        let sign = adjusted.signum();
        weights.iter().enumerate().map(|(t, &w)| {
            if w < 1e-8 && results.steps[t].pass != Pass::Stop {
                -self.noop_penalty
            } else {
                sign * w / max_w
            }
        }).collect()
    }
}
