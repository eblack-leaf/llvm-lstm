use crate::ppo::episode::Results;
use crate::ppo::model::ACTIONS;
use crate::ppo::returns::Returns;

/// Per-step return derived from exhaustive one-step lookahead benchmarks.
///
/// return[t] = (speedup(chosen_action) - mean(speedup(all_actions))) / norm
///
/// norm = max(|la[i] - mean(la)|) across all 29 candidates, keeping the
/// return in [-1, 1]. Floor of 1e-4 prevents divide-by-zero on flat episodes
/// where all passes produce identical speedup.
///
/// Provides the value training target that matches what LookaheadAdvantage
/// subtracts V(s) from — so the critic learns to predict the expected
/// per-step lookahead quality from a given IR state rather than a
/// path-dependent trajectory return it can't properly baseline against.
///
/// Returns 0.0 for steps without lookahead data (lookahead disabled).
pub(crate) struct LookaheadReturn;

impl Returns for LookaheadReturn {
    fn compute(&self, results: &Results) -> Vec<f32> {
        results.steps.iter().map(|step| {
            let Some(la) = &step.lookahead else { return 0.0 };
            let chosen_idx = ACTIONS
                .iter()
                .position(|&p| p == step.pass)
                .expect("step pass not in ACTIONS");
            let mean = la.iter().sum::<f32>() / la.len() as f32;
            let norm = la.iter().map(|&v| (v - mean).abs()).fold(0.0f32, f32::max).max(1e-4);
            (la[chosen_idx] - mean) / norm
        }).collect()
    }
}
