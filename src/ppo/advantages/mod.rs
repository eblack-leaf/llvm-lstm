pub(crate) mod baseline;
pub(crate) mod rank;

use crate::ppo::episode::Results;

/// Produces per-slot advantages for metrics/logging.
/// Called once after episode collection, before the PPO update.
/// The PPO update itself computes advantages inline as (ret − V(IR)).
pub(crate) trait Advantages {
    fn compute(&self, returns: &[Vec<f32>], results: &[Results]) -> Vec<Vec<f32>>;
}
