pub(crate) mod best_step;
pub(crate) mod delta_weighted;
pub(crate) mod episode_return;
pub(crate) mod lookahead;

use crate::ppo::episode::Results;

/// Produces a per-step return for each action taken in an episode.
/// The return is what gets compared against V(s_t) to form the advantage.
///
/// The hard problem here is credit assignment: the only signal we fully trust is
/// the episode-level benchmark. Step carries metadata to help future implementors
/// do finer attribution (IR feature deltas, pass no-op detection, correlation with
/// historical orderings, etc.) without being locked into a fixed strategy now.
pub(crate) trait Returns {
    /// Compute per-step returns for a single episode.
    fn compute(&self, results: &Results) -> Vec<f32>;

    /// Compute returns for a full batch of episodes.
    /// Default maps `compute` over each episode. Override when cross-episode
    /// context is needed (e.g. batch-level normalisation).
    fn compute_batch(&self, results: &[Results]) -> Vec<Vec<f32>> {
        results.iter().map(|r| self.compute(r)).collect()
    }
}
