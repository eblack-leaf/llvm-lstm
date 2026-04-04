use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Credits the episode-level speedup to each step by how much that step
/// changed the IR, measured as the magnitude of the relative marginal delta.
///
/// `delta_features[t]` is cumulative: `current_IR[t] − base_IR`.
/// Marginal at step t = `delta[t] − delta[t−1]`  (delta[−1] = 0).
/// Each dimension is normalised by `|base| + ε` so count features
/// and ratio features are on the same scale.
///
/// weight[t] = Σ |normalised_marginal[t]|   (always ≥ 0)
///
/// The most impactful step gets return = episode_speedup.
/// Other steps get a proportional fraction.  No-op steps get 0.
/// If nothing changed at all, every step gets episode_speedup (uniform fallback).
///
/// The sign of the return comes entirely from the episode outcome —
/// bad episode → all returns ≤ 0, good episode → all returns ≥ 0.
pub(crate) struct DeltaWeightedReturn;

impl DeltaWeightedReturn {
    pub(crate) fn new() -> Self {
        Self
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

        let base = &results.base_features;

        let weights: Vec<f32> = (0..n)
            .map(|t| {
                let curr = &results.steps[t].delta_features;
                let prev: &[f32] = if t == 0 { &[] } else { &results.steps[t - 1].delta_features };
                results.steps[t].delta_features.iter().enumerate().map(|(i, &c)| {
                    let p = if prev.is_empty() { 0.0 } else { prev[i] };
                    let marginal = c - p;
                    let scale = base[i].abs().max(0.1) + 1e-6;
                    (marginal / scale).abs()
                }).sum::<f32>()
            })
            .collect();

        let max_w = weights.iter().cloned().fold(0.0f32, f32::max);
        if max_w < 1e-8 {
            return vec![reward.signum(); n];
        }

        // Normalise to [-1, 1]: most impactful step gets sign(speedup), others
        // proportionally less, no-ops get 0.  V learns a fraction in [-1, 1]
        // regardless of speedup magnitude, so value loss is always O(1).
        let sign = reward.signum();
        weights.iter().map(|&w| sign * w / max_w).collect()
    }
}
