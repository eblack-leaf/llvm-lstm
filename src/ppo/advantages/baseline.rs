use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;

/// Standard return-minus-baseline advantage: advantage[t] = return[t] − V(base_IR).
///
/// With the whole-sequence approach, return[t] is the same terminal speedup for all
/// slots, and V(base_IR) is the same scalar for the episode. The raw difference is
/// used without normalisation — when episodes cluster, the gradient is honestly small.
pub(crate) struct BaselineAdvantage;

impl Advantages for BaselineAdvantage {
    fn compute(&self, returns: &[Vec<f32>], results: &[Results]) -> Vec<Vec<f32>> {
        returns
            .iter()
            .enumerate()
            .map(|(i, ep_returns)| {
                ep_returns.iter().enumerate().map(|(t, &r)| {
                    let v = results[i].values.get(t).copied().unwrap_or(0.0);
                    r - v
                }).collect()
            })
            .collect()
    }
}
