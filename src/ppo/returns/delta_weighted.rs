use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Credits the episode-level speedup to each step proportionally to how much
/// it changed the IR, measured as the *relative* marginal delta at each step.
///
/// `delta_features[t]` is cumulative: `current_IR[t] − base_IR`.
/// Marginal at step t = `delta[t] − delta[t−1]`  (with `delta[−1] = 0`).
///
/// Each marginal dimension is divided by the corresponding base-IR value
/// (plus ε) so that large-scale ratio features (avg_bb_size, load_store_ratio)
/// and small log-count changes are treated on the same relative footing.
///
/// Two modes controlled by `penalize_growth`:
///
/// `false` — unsigned: weight[t] = Σ |normalised_marginal[t]|
///   Any impactful pass gets credit/blame; direction of change is ignored.
///
/// `true` — signed: weight[t] = −Σ normalised_marginal[t]
///   IR shrinkage → positive weight (credit).
///   IR growth    → negative weight (penalty).
///
/// Weights are then normalised by their L1 norm so they sum to ±1.
///
/// If nothing changed at all, falls back to uniform credit.
pub(crate) struct DeltaWeightedReturn {
    pub(crate) penalize_growth: bool,
}

impl DeltaWeightedReturn {
    pub(crate) fn new(penalize_growth: bool) -> Self {
        Self { penalize_growth }
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
                let prev: &[f32] = if t == 0 {
                    // delta[-1] is zero, so marginal = curr itself
                    &[]
                } else {
                    &results.steps[t - 1].delta_features
                };

                // Relative marginal: (curr - prev) / (|base| + ε) per dimension.
                let rel_sum: f32 = curr.iter().enumerate().map(|(i, &c)| {
                    let p = if prev.is_empty() { 0.0 } else { prev[i] };
                    let marginal = c - p;
                    let scale = base[i].abs().max(0.1) + 1e-6;
                    marginal / scale
                }).sum();

                if self.penalize_growth {
                    -rel_sum  // shrinkage (rel_sum < 0) → positive weight
                } else {
                    rel_sum.abs()
                }
            })
            .collect();

        let l1: f32 = weights.iter().map(|w| w.abs()).sum();
        if l1 < 1e-8 {
            return vec![reward / n as f32; n];
        }

        weights.iter().map(|&w| reward * w / l1).collect()
    }
}
