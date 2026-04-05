use crate::llvm::pass::Pass;
use crate::ppo::episode::Results;
use crate::ppo::model::ACTIONS;
use crate::ppo::returns::Returns;

/// Cumulative discounted lookahead return.
///
/// At each step t, the per-step reward is:
///   r_t = la[chosen_t] / max(|la_t[i]|)   — normalised to [-1, 1]
///
/// The return is the discounted sum from t to episode end:
///   R_t = r_t + γ·r_{t+1} + γ²·r_{t+2} + ...
///
/// Episode returns are then normalised by max(|R_t|) across the episode
/// to keep them in [-1, 1] regardless of episode length or γ.
///
/// V(s_t) trains on R_t — a non-zero-centred, state-dependent, multi-step
/// target that captures the cumulative quality of the IR state from this
/// point forwards. Use with BaselineAdvantage: A_t = R_t - V(s_t).
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
    fn compute(&self, results: &Results) -> Vec<f32> {
        let n = results.log_probs.len();
        if n == 0 { return vec![]; }

        // Per-step reward: chosen pass speedup normalised by spread of la.
        let rewards: Vec<f32> = results.steps.iter().map(|step| {
            let Some(la) = &step.lookahead else { return 0.0 };
            let chosen_idx = ACTIONS
                .iter()
                .position(|&p| p == step.pass)
                .expect("step pass not in ACTIONS");
            let norm = la.iter().map(|v| v.abs()).fold(0.0f32, f32::max).max(1e-4);
            la[chosen_idx] / norm
        }).collect();

        // Discounted cumulative return, computed backwards.
        // Stop's reward does not flow into prior steps — each non-Stop step
        // accumulates only from its own pass quality forward, not from the
        // quality of the terminal state. Stop still gets its own lookahead
        // return for the PPO update on the Stop action itself.
        let mut returns = vec![0.0f32; n];
        let mut running = 0.0f32;
        for t in (0..n).rev() {
            running = rewards[t] + self.gamma * running;
            returns[t] = running;
            if results.steps[t].pass == Pass::Stop {
                running = 0.0;
            }
        }

        // Normalise episode returns to [-1, 1].
        let norm = returns.iter().map(|r| r.abs()).fold(0.0f32, f32::max).max(1e-4);
        returns.iter().map(|r| r / norm).collect()
    }
}
