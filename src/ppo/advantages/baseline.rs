use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;

/// Standard return-minus-baseline advantage: advantage[t] = return[t] − V(base_IR).
///
/// With the whole-sequence approach, return[t] is the same terminal speedup for all
/// slots, and V(base_IR) is the same scalar. So the advantage is constant per episode:
/// speedup − V(IR). Used here only for metrics display; the PPO update computes this
/// inline with the current model's V estimate.
pub(crate) struct BaselineAdvantage;

impl Advantages for BaselineAdvantage {
    fn compute(&self, returns: &[Vec<f32>], results: &[Results]) -> Vec<Vec<f32>> {
        returns
            .iter()
            .enumerate()
            .map(|(i, ep_returns)| {
                let v = results[i].value;
                ep_returns.iter().map(|&r| r - v).collect()
            })
            .collect()
    }
}
