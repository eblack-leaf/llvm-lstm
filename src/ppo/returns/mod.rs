pub(crate) mod episode_return;

use crate::ppo::episode::Results;

/// Produces a per-step return for each action taken in an episode.
/// The return is what gets compared against V(s_t) to form the advantage.
///
/// The hard problem here is credit assignment: the only signal we fully trust is
/// the episode-level benchmark. Step carries metadata to help future implementors
/// do finer attribution (IR feature deltas, pass no-op detection, correlation with
/// historical orderings, etc.) without being locked into a fixed strategy now.
pub(crate) trait Returns {
    fn compute(&self, results: &Results) -> Vec<f32>;
}
