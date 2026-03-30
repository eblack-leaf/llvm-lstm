use std::collections::HashMap;

/// A single collected episode — full action sequence plus its discounted return.
pub struct Episode {
    pub func: String,
    pub actions: Vec<usize>,
    pub g0: f32,
}

/// Per-function episode memory with threshold-based pruning.
///
/// Survival-of-the-fittest: after each insert the store drops any episode
/// whose g0 falls more than `prune_threshold` below the new per-function best.
/// This keeps the distribution tight around the frontier and avoids
/// contaminating the critic signal with stale low-quality trajectories.
pub struct BestEpisodeStore {
    /// Maximum allowed gap below the best g0; episodes outside are pruned.
    pub prune_threshold: f32,
    store: HashMap<String, Vec<Episode>>,
}

impl BestEpisodeStore {
    pub fn new(prune_threshold: f32) -> Self {
        Self { prune_threshold, store: HashMap::new() }
    }

    /// Insert an episode.  Re-sorts the per-function list and prunes.
    pub fn insert(&mut self, ep: Episode) {
        let entries = self.store.entry(ep.func.clone()).or_default();
        entries.push(ep);
        entries.sort_by(|a, b| b.g0.partial_cmp(&a.g0).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(best_g0) = entries.first().map(|e| e.g0) {
            let cutoff = best_g0 - self.prune_threshold;
            entries.retain(|e| e.g0 >= cutoff);
        }
    }

    /// All surviving episodes for a function, sorted best-first.
    pub fn get(&self, func: &str) -> &[Episode] {
        self.store.get(func).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Best g0 seen for a function, or `None` if no episodes yet.
    pub fn best_g0(&self, func: &str) -> Option<f32> {
        self.store.get(func)?.first().map(|e| e.g0)
    }

    /// Number of surviving episodes across all functions.
    pub fn total_count(&self) -> usize {
        self.store.values().map(|v| v.len()).sum()
    }

    /// Whether we have at least one episode for a function.
    pub fn has(&self, func: &str) -> bool {
        self.store.get(func).map(|v| !v.is_empty()).unwrap_or(false)
    }

    /// Iterate over all (func_name, episodes) pairs.
    pub fn iter_funcs(&self) -> impl Iterator<Item = (&str, &[Episode])> {
        self.store.iter().map(|(k, v)| (k.as_str(), v.as_slice()))
    }
}
