use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;

/// Generalized Advantage Estimation (GAE-λ).
///
/// Computes advantages from per-step TD errors rather than full Monte Carlo returns,
/// breaking the circular dependency that arises when V is trained on the same
/// hindsight-determined targets it is supposed to baseline.
///
///   delta[t]      = r[t] + γ · V(s_{t+1}) - V(s_t)
///   advantage[t]  = Σ_{k≥0} (γλ)^k · delta[t+k]
///
/// V is compared against one-step predictions (r_t + γ·V_next), not the full
/// episode return. This makes V's target Markovian: it asks "given the current
/// IR state, what is the immediate reward plus discounted next-state value?"
/// rather than "what is the full hindsight return determined by which step turns
/// out to be best?"
///
/// With λ=1, γ=1 this reduces to standard MC advantage (return - V).
/// With λ=0, γ=1 this reduces to TD(0): r_t + V(s_{t+1}) - V(s_t).
/// λ ∈ (0.9, 0.97) gives the standard bias-variance trade-off.
///
/// Terminal state: V(s_T) = 0 (episode ends, no future value).
///
/// Optional batch-level scale normalisation (divide by batch std, do not subtract
/// mean) stabilises gradient magnitude across episodes of varying return scale.
pub(crate) struct GaeAdvantage {
    pub(crate) gamma: f32,
    pub(crate) lambda: f32,
    pub(crate) normalise: bool,
}

impl GaeAdvantage {
    pub(crate) fn new(gamma: f32, lambda: f32, normalise: bool) -> Self {
        Self { gamma, lambda, normalise }
    }
}

impl Advantages for GaeAdvantage {
    fn compute(&self, returns: &[Vec<f32>], results: &[Results]) -> Vec<Vec<f32>> {
        let mut all_advantages: Vec<Vec<f32>> = returns
            .iter()
            .enumerate()
            .map(|(ep_idx, ep_returns)| {
                let values = &results[ep_idx].values;
                let n = ep_returns.len();

                // TD errors: delta[t] = r[t] + γ·V(s_{t+1}) - V(s_t)
                // V(s_T) = 0 for terminal step.
                let deltas: Vec<f32> = (0..n)
                    .map(|t| {
                        let v_next = if t + 1 < n { values[t + 1] } else { 0.0 };
                        ep_returns[t] + self.gamma * v_next - values[t]
                    })
                    .collect();

                // GAE: backward pass accumulating discounted TD errors.
                let mut advantages = vec![0.0f32; n];
                let mut gae = 0.0f32;
                for t in (0..n).rev() {
                    gae = deltas[t] + self.gamma * self.lambda * gae;
                    advantages[t] = gae;
                }
                advantages
            })
            .collect();

        if self.normalise {
            let flat: Vec<f32> = all_advantages.iter().flatten().copied().collect();
            if flat.len() > 1 {
                let var =
                    flat.iter().map(|a| a.powi(2)).sum::<f32>() / flat.len() as f32;
                let std = var.sqrt().max(1e-8);
                for ep in &mut all_advantages {
                    for a in ep.iter_mut() {
                        *a /= std;
                    }
                }
            }
        }

        all_advantages
    }
}
