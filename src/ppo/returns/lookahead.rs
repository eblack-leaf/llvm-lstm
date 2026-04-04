use crate::ppo::episode::Results;
use crate::ppo::model::ACTIONS;
use crate::ppo::returns::Returns;

/// Per-step return derived from exhaustive one-step lookahead benchmarks.
///
/// return[t] = speedup(chosen_action) - mean(speedup(all_actions))
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
            la[chosen_idx] - mean
        }).collect()
    }
}
