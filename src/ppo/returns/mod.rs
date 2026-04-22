pub(crate) mod episode_return;
pub(crate) mod ir_step_return;
pub(crate) mod weighted;

use crate::ppo::episode::Results;

/// Produces a per-slot return for each action taken in an episode.
pub(crate) trait Returns {
    fn compute(&self, results: &Results) -> Vec<f32>;

    fn compute_batch(&mut self, results: &[Results]) -> Vec<Vec<f32>> {
        results.iter().map(|r| self.compute(r)).collect()
    }
}
