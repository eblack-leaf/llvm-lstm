use crate::ppo::episode::Results;
use crate::ppo::model::ACTIONS;
use crate::ppo::returns::Returns;

/// Cumulative discounted lookahead return.
///
/// At each step t, the per-step reward is mean-centred and rescaled to [-1, +1]:
///   r_t = (la[chosen_t] - mean(la_t)) / max(|la_t[i] - mean(la_t)|)
///
/// The best available action always scores +1, the worst always scores -1.
/// Stop is included in the distribution so it scores positively only when it
/// beats the average of all available actions — guiding the policy toward the
/// best option regardless of absolute speedup level.
///
/// The return is the standard discounted cumulative sum:
///   R_t = r_t + γ·r_{t+1} + γ²·r_{t+2} + ...
///
/// Episode returns are then normalised by max(|R_t|) to keep them in [-1, +1].
///
/// Returns 0.0 for steps without lookahead data (lookahead disabled).
pub(crate) struct LookaheadCumulativeReturn {
    pub(crate) gamma: f32,
}

impl LookaheadCumulativeReturn {
    pub(crate) fn new(gamma: f32) -> Self {
        Self { gamma }
    }
}

impl Returns for LookaheadCumulativeReturn {
    fn compute_batch(&self, results: &[Results]) -> Vec<Vec<f32>> {
        let mut all_returns: Vec<Vec<f32>> = results.iter().map(|r| self.compute(r)).collect();
        let batch_max = all_returns.iter().flatten()
            .map(|r| r.abs())
            .fold(0.0f32, f32::max)
            .max(1e-4);
        for ep in &mut all_returns {
            for r in ep.iter_mut() {
                *r /= batch_max;
            }
        }
        all_returns
    }

    fn compute(&self, results: &Results) -> Vec<f32> {
        let n = results.log_probs.len();
        if n == 0 { return vec![]; }

        // Per-step reward: mean-centred, rescaled to [-1, +1].
        let rewards: Vec<f32> = results.steps.iter().map(|step| {
            let Some(la) = &step.lookahead else { return 0.0 };
            let chosen_idx = ACTIONS
                .iter()
                .position(|&p| p == step.pass)
                .expect("step pass not in ACTIONS");
            let mean = la.iter().sum::<f32>() / la.len() as f32;
            let norm = la.iter().map(|v| (v - mean).abs()).fold(0.0f32, f32::max).max(1e-4);
            (la[chosen_idx] - mean) / norm
        }).collect();

        // Discounted cumulative return, computed backwards.
        // No per-episode normalisation — caller normalises across the full batch
        // so V sees consistent scale across episodes of different quality.
        let mut returns = vec![0.0f32; n];
        let mut running = 0.0f32;
        for t in (0..n).rev() {
            running = rewards[t] + self.gamma * running;
            returns[t] = running;
        }
        returns
    }
}
