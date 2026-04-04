use crate::llvm::pass::Pass;
use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Credits the episode-level speedup to each step by how much that step
/// changed the IR, measured as the magnitude of the relative marginal delta.
///
/// weight[t] = Σ |normalised_marginal[t]|   (always ≥ 0)
///
/// The most impactful step gets return = sign(adjusted_speedup).
/// Other steps get a proportional fraction.  No-op steps (including Stop) get 0.
///
/// `length_coef` adds a length penalty to the speedup before taking the sign:
///   adjusted = speedup − length_coef × (ep_len / max_seq_len)
/// A barely-positive episode that ran too long gets its sign flipped to negative,
/// incentivising early stopping without overwhelming the speedup signal.
///
/// `noop_penalty` gives a fixed negative return to any non-Stop step that produced
/// no measurable IR change (weight ≈ 0).  Stop is exempt — its length benefit comes
/// from the length penalty above.  This distinguishes "did nothing useful" from
/// "intentionally ended the episode".
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

        let base = &results.base_features;

        let weights: Vec<f32> = (0..n)
            .map(|t| {
                let curr = &results.steps[t].delta_features;
                let prev: &[f32] = if t == 0 { &[] } else { &results.steps[t - 1].delta_features };
                curr.iter().enumerate().map(|(i, &c)| {
                    let p = if prev.is_empty() { 0.0 } else { prev[i] };
                    let marginal = c - p;
                    let scale = base[i].abs().max(0.1) + 1e-6;
                    (marginal / scale).abs()
                }).sum::<f32>()
            })
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
