pub(crate) mod best_step;
pub(crate) mod delta_weighted;
pub(crate) mod episode_return;
pub(crate) mod episodic_pattern;
pub(crate) mod lookahead;

use crate::ppo::episode::Results;

/// Per-function snapshot of the survivorship store for logging.
pub(crate) struct FuncStoreStats {
    pub(crate) func_name:  String,
    pub(crate) entries:    usize,
    pub(crate) best:       f32,
    /// Worst speedup still kept after pruning.
    pub(crate) worst:      f32,
    /// best - worst within the store.
    pub(crate) spread:     f32,
    /// Mean pairwise Jaccard distance between stored pass-sets (0=identical, 1=fully diverse).
    pub(crate) diversity:  f32,
}

pub(crate) struct StoreStats {
    pub(crate) total_entries: usize,
    pub(crate) per_func:      Vec<FuncStoreStats>,
}

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
    /// Takes `&mut self` so implementors can update internal state (e.g. episode
    /// stores) before computing returns. Default maps `compute` over each episode.
    fn compute_batch(&mut self, results: &[Results]) -> Vec<Vec<f32>> {
        results.iter().map(|r| self.compute(r)).collect()
    }

    /// Snapshot of the survivorship store for logging. Returns None for
    /// implementors that don't maintain a store (e.g. LookaheadCumulativeReturn).
    fn store_stats(&self) -> Option<StoreStats> { None }
}
