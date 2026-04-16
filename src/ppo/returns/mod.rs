pub(crate) mod episode_return;
pub(crate) mod instruction_proxy;
pub(crate) mod instruction_weighted_terminal;
pub(crate) mod ir_count_return;
pub(crate) mod ir_step_return;
pub(crate) mod predictor_return;

use crate::ppo::episode::Results;

/// Produces a per-slot return for each action taken in an episode.
/// The return is compared against V(base_IR) to form the advantage.
pub(crate) trait Returns {
    fn compute(&self, results: &Results) -> Vec<f32>;

    fn compute_batch(&mut self, results: &[Results]) -> Vec<Vec<f32>> {
        results.iter().map(|r| self.compute(r)).collect()
    }
}
