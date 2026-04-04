use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Credits the episode-level speedup to each step proportionally to how much
/// it changed the IR, measured as the L1 norm of its *marginal* delta.
///
/// `delta_features[t]` is cumulative: `current_IR[t] − base_IR`.
/// Marginal at step t = `delta[t] − delta[t−1]`  (with `delta[−1] = 0`).
/// Weight[t] = Σ|marginal[t]|
/// Return[t] = episode_speedup × weight[t] / Σ weights
///
/// Consequences:
/// - No-ops and Stop leave the IR unchanged → marginal = 0 → return = 0.
///   They receive neither credit for good episodes nor blame for bad ones.
/// - Passes that structurally restructure the IR receive the bulk of credit/blame.
/// - If nothing changed at all (degenerate episode), falls back to uniform credit
///   so the value head still gets a learning signal.
pub(crate) struct DeltaWeightedReturn;

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

        // Marginal IR change at each step: delta[t] − delta[t−1].
        // For t=0 the "previous" delta is the zero vector (unoptimised base).
        let mut weights: Vec<f32> = (0..n)
            .map(|t| {
                let curr = &results.steps[t].delta_features;
                if t == 0 {
                    curr.iter().map(|c| c.abs()).sum()
                } else {
                    let prev = &results.steps[t - 1].delta_features;
                    curr.iter().zip(prev).map(|(c, p)| (c - p).abs()).sum()
                }
            })
            .collect();

        let total: f32 = weights.iter().sum();
        if total < 1e-8 {
            // Nothing changed — uniform credit so value head still trains.
            return vec![reward / n as f32; n];
        }

        weights.iter().map(|&w| reward * w / total).collect()
    }
}
