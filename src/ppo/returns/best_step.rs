use crate::llvm::pass::Pass;
use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Index of `total_instruction_count` in the feature vector produced by `Features::to_vec()`.
const TOTAL_INSTR_IDX: usize = 17;

/// Per-step return derived from per-step benchmark results.
/// Requires `--per-step-benchmark`.
///
/// Strategy:
///   For each step t, return = (speedup[t] - prev_peak) / norm, where
///   prev_peak is the running maximum speedup achieved before step t.
///   - Improvement over prev peak → positive return (reward for the gain)
///   - Regression below prev peak → negative return (penalty for the drop)
///   The running peak naturally handles both pre- and post-peak steps: once
///   the peak is reached prev_peak stays there, so all subsequent steps are
///   penalised relative to the best seen, not blanket-credited.
///
///   No-op nullification: non-Stop steps with |Δinstr| < noop_threshold get 0
///   but still advance prev_peak (the IR state they measured is real).
///   Stop is exempt from no-op nullification.
///
/// `norm` = max(|speedup[t]|) across all steps, keeping returns in [-1, 1].
/// Floor of 1e-4 on norm prevents divide-by-zero on fully-flat episodes.
pub(crate) struct BestStepReturn {
    pub(crate) noop_threshold: f32,
}

impl BestStepReturn {
    pub(crate) fn new(noop_threshold: f32) -> Self {
        Self { noop_threshold }
    }
}

impl Returns for BestStepReturn {
    fn compute(&self, results: &Results) -> Vec<f32> {
        let n = results.log_probs.len();
        if n == 0 {
            return vec![];
        }

        let speedups: Vec<f32> = results.steps.iter()
            .map(|s| s.benchmark.as_ref().map(|b| b.speedup).unwrap_or(0.0))
            .collect();

        let norm = speedups.iter().map(|s| s.abs()).fold(0.0f32, f32::max).max(1e-4);

        let mut prev_peak = 0.0f32;
        results.steps.iter().enumerate().map(|(t, step)| {
            // No-op nullification: non-Stop passes that changed nothing get 0,
            // but still advance prev_peak since the IR state is real.
            if step.pass != Pass::Stop
                && step.delta_features[TOTAL_INSTR_IDX].abs() < self.noop_threshold
            {
                prev_peak = prev_peak.max(speedups[t]);
                return 0.0;
            }

            let ret = (speedups[t] - prev_peak) / norm;
            prev_peak = prev_peak.max(speedups[t]);
            ret
        }).collect()
    }
}
