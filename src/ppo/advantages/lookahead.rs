use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;
use crate::ppo::model::ACTIONS;

/// Advantage derived from exhaustive one-step lookahead benchmarks.
///
/// At each step, all 29 ACTIONS were benchmarked from the pre-action IR state
/// during collection. The advantage is:
///
///   advantage[t] = (speedup(chosen_action) - mean(speedup(all_actions))) - value[t]
///
/// The lookahead term answers "was the chosen pass better than average from
/// this state?" The value baseline V(s_t) is subtracted on top so the critic
/// can learn to predict the expected lookahead signal and reduce variance over
/// time — otherwise the critic is trained but never influences the policy gradient.
///
/// Falls back to `return[t] - value[t]` for steps where lookahead data is
/// absent (lookahead disabled, or Stop on an uncollected step).
pub(crate) struct LookaheadAdvantage {
    pub(crate) normalise: bool,
}

impl LookaheadAdvantage {
    pub(crate) fn new(normalise: bool) -> Self {
        Self { normalise }
    }
}

impl Advantages for LookaheadAdvantage {
    fn compute(&self, returns: &[Vec<f32>], results: &[Results]) -> Vec<Vec<f32>> {
        let mut all_advantages: Vec<Vec<f32>> = returns
            .iter()
            .enumerate()
            .map(|(ep_idx, ep_returns)| {
                let result = &results[ep_idx];
                ep_returns
                    .iter()
                    .enumerate()
                    .map(|(t, &ret)| {
                        let step = &result.steps[t];
                        if let Some(la) = &step.lookahead {
                            let chosen_idx = ACTIONS
                                .iter()
                                .position(|&p| p == step.pass)
                                .expect("step pass not in ACTIONS");
                            let mean = la.iter().sum::<f32>() / la.len() as f32;
                            (la[chosen_idx] - mean) - result.values[t]
                        } else {
                            // Fallback: standard return-minus-baseline.
                            ret - result.values[t]
                        }
                    })
                    .collect()
            })
            .collect();

        if self.normalise {
            let flat: Vec<f32> = all_advantages.iter().flatten().copied().collect();
            if flat.len() > 1 {
                let var = flat.iter().map(|a| a.powi(2)).sum::<f32>() / flat.len() as f32;
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
