use crate::llvm::pass::Pass;
use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Index of `total_instruction_count` in the feature vector produced by `Features::to_vec()`.
const TOTAL_INSTR_IDX: usize = 17;

/// Per-step return derived from per-step benchmark results.
/// Requires `--per-step-benchmark`.
///
/// Strategy:
///   1. Find `best_idx` — the step with the highest speedup.
///   2. Steps at or before `best_idx`: return = speedup[t] / norm
///      (credit proportional to how good each step was on the path to the peak)
///   3. Steps after `best_idx`: return = (speedup[t] - best_speedup) / norm
///      (penalty proportional to how far the episode regressed from the peak)
///   4. No-op nullification: non-Stop steps with |Δinstr| < noop_threshold get 0 —
///      changes are not attributed to passes that left the IR untouched.
///   5. Stop is exempt from no-op nullification: it represents a deliberate choice
///      to terminate and receives the natural return for its position.
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

        let best_idx = speedups
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);
        let best_speedup = speedups[best_idx];

        let norm = speedups.iter().map(|s| s.abs()).fold(0.0f32, f32::max).max(1e-4);

        results.steps.iter().enumerate().map(|(t, step)| {
            // No-op nullification: non-Stop passes that changed nothing get 0.
            if step.pass != Pass::Stop
                && step.delta_features[TOTAL_INSTR_IDX].abs() < self.noop_threshold
            {
                return 0.0;
            }

            if t <= best_idx {
                speedups[t] / norm
            } else {
                (speedups[t] - best_speedup) / norm
            }
        }).collect()
    }
}
