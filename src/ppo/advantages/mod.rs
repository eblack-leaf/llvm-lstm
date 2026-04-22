pub(crate) mod baseline;

use crate::ppo::episode::Results;

/// Produces per-slot advantages for metrics/logging.
/// Called once after episode collection, before the PPO update.
pub(crate) trait Advantages {
    fn compute(&self, returns: &[Vec<f32>], results: &[Results]) -> Vec<Vec<f32>>;
}
