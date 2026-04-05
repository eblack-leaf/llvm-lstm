use crate::llvm::pass::Pass;
use crate::ppo::episode::Results;
use crate::ppo::returns::{FuncStoreStats, Returns, StoreStats};
use std::collections::{HashMap, HashSet};

/// Per-step return based on ordered pass co-occurrence patterns in top-K episodes.
///
/// No lookahead benchmarking — the only signal needed is the episode-end benchmark.
/// Credit is assigned by matching each step against patterns found in the best
/// episodes seen so far for the same function.
///
/// Store: per function, top-K episodes by terminal speedup. Updated each batch
/// before returns are computed.
///
/// Per-step score for step t (curr = action chosen at t):
///   unigram:  rank-weighted frequency of curr in top-K store
///   pairwise: for each prior pass already chosen this episode, rank-weighted
///             frequency of (prior → curr) as an ordered subsequence in top-K
///             episodes. Prior must appear at an earlier index than curr — order
///             is respected. Non-contiguous: other passes may appear between them.
///   score = max(unigram, max_over_history(pairwise))
///
/// Trifecta credit: if A→B→C is the winning pattern, step B earns the (A→B) pair
/// score, step C earns the (A→C) and (B→C) pair scores. Each step gets credit
/// as it completes an ordered pair with any prior pass in the current episode.
///
/// Per-step return = pattern_score * terminal_scale. No backward accumulation —
/// pattern score is already a local credit signal; discounting would spread
/// future rewards into earlier steps including no-ops.
/// No-op: delta_features == 0 → reward 0.0 exactly, no hashing needed.
/// Cold start: empty store → uniform 1.0 so terminal scaling still provides signal.
pub(crate) struct EpisodicPatternReturn {
    /// How many top episodes to retain per function.
    pub(crate) top_k: usize,
    /// Keep entries within prune_threshold * best below the best (only when best > 0).
    pub(crate) prune_threshold: f32,
    /// func_name → [(speedup, pass_sequence)], sorted descending, capped at top_k.
    store: HashMap<String, Vec<(f32, Vec<Pass>)>>,
    /// Pre-normalisation return std from the most recent compute_batch call.
    last_raw_std: f32,
}

impl EpisodicPatternReturn {
    pub(crate) fn new(top_k: usize, prune_threshold: f32) -> Self {
        Self { top_k, prune_threshold, store: HashMap::new(), last_raw_std: 0.0 }
    }

    fn update_store(&mut self, results: &[Results]) {
        for ep in results {
            let Some(bm) = ep.steps.last().and_then(|s| s.benchmark.as_ref()) else { continue };
            let seq: Vec<Pass> = ep.actions.iter().copied()
                .filter(|&p| p != Pass::Start)
                .collect();
            let entry = self.store.entry(ep.func_name.clone()).or_default();
            entry.push((bm.speedup, seq));
            entry.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            entry.truncate(self.top_k);
            // Only prune once there is a positive best — before that, keep all entries
            // so the store fills freely without pushing the policy toward negative sequences.
            // cutoff = best - prune_threshold * best: keep within 20% below the best.
            if entry.len() >= 2 {
                let best = entry[0].0;
                if best > 0.0 {
                    let cutoff = best - self.prune_threshold * best;
                    entry.retain(|(sp, _)| *sp >= cutoff);
                }
            }
        }
    }

    /// Rank-weighted frequency of `pass` appearing anywhere in top-K episodes.
    fn unigram_score(&self, store: &[(f32, Vec<Pass>)], pass: Pass) -> f32 {
        let k = store.len();
        let total = (k * (k + 1) / 2) as f32;
        let matched: f32 = store.iter().enumerate()
            .filter(|(_, (_, seq))| seq.contains(&pass))
            .map(|(rank, _)| (k - rank) as f32)
            .sum();
        matched / total
    }

    /// Rank-weighted frequency of `first` appearing before `second` (non-contiguous
    /// ordered subsequence) in top-K episodes. Order respected: first must precede second.
    fn ordered_pair_score(&self, store: &[(f32, Vec<Pass>)], first: Pass, second: Pass) -> f32 {
        let k = store.len();
        let total = (k * (k + 1) / 2) as f32;
        let matched: f32 = store.iter().enumerate()
            .filter(|(_, (_, seq))| contains_ordered_pair(seq, first, second))
            .map(|(rank, _)| (k - rank) as f32)
            .sum();
        matched / total
    }

    fn pattern_score(&self, func_name: &str, actions: &[Pass], step_t: usize) -> f32 {
        let Some(store) = self.store.get(func_name) else { return 1.0 };
        if store.is_empty() { return 1.0 }

        // actions[0] = Start sentinel; actions[step_t + 1] = pass chosen at step t.
        let curr = actions[step_t + 1];
        let history = &actions[1..=step_t]; // passes chosen before this step, in order

        let uni = self.unigram_score(store, curr);

        // Best ordered-pair score: for each prior pass, check how often
        // (prior → curr) appears as an ordered subsequence in top-K store.
        let best_pair = history.iter()
            .map(|&prior| self.ordered_pair_score(store, prior, curr))
            .fold(0.0f32, f32::max);

        uni.max(best_pair)
    }
}

/// True if `first` appears at some index i and `second` at some index j > i in `seq`.
/// Non-contiguous — other passes may appear between them. Order respected.
/// Handles first == second: requires two separate occurrences.
fn contains_ordered_pair(seq: &[Pass], first: Pass, second: Pass) -> bool {
    let mut found_first = false;
    for &p in seq {
        if found_first && p == second { return true; }
        if p == first { found_first = true; }
    }
    false
}

/// Mean pairwise Jaccard distance across all pairs of pass-set sequences in the store.
/// Jaccard similarity(A,B) = |A∩B| / |A∪B|. Distance = 1 - similarity.
/// Returns 0.0 when there are fewer than 2 entries (can't measure diversity).
fn store_diversity(episodes: &[(f32, Vec<Pass>)]) -> f32 {
    if episodes.len() < 2 { return 0.0; }
    let sets: Vec<HashSet<Pass>> = episodes.iter()
        .map(|(_, seq)| seq.iter().copied().collect())
        .collect();
    let mut total = 0.0f32;
    let mut pairs = 0u32;
    for i in 0..sets.len() {
        for j in i + 1..sets.len() {
            let inter = sets[i].intersection(&sets[j]).count() as f32;
            let union = sets[i].union(&sets[j]).count() as f32;
            let sim = if union == 0.0 { 1.0 } else { inter / union };
            total += 1.0 - sim;
            pairs += 1;
        }
    }
    total / pairs as f32
}

impl Returns for EpisodicPatternReturn {
    fn compute(&self, results: &Results) -> Vec<f32> {
        let n = results.log_probs.len();
        if n == 0 { return vec![]; }

        let terminal_speedup = results.steps.last()
            .and_then(|s| s.benchmark.as_ref())
            .map(|b| b.speedup)
            .unwrap_or(0.0);
        let scale = (1.0 + terminal_speedup).max(0.1);

        let mut rewards: Vec<f32> = (0..n).map(|t| {
            // No-op: delta_features is zero when IR unchanged → exact zero reward.
            let changed = results.steps[t].delta_features.iter().any(|&d| d.abs() > 1e-6);
            if !changed { return 0.0; }
            self.pattern_score(&results.func_name, &results.actions, t)
        }).collect();

        // Per-step return = pattern_score * terminal_scale. No backward accumulation —
        // pattern score is already a local credit signal; discounting would spread
        // future rewards into earlier steps including no-ops.
        rewards.iter().map(|&r| r * scale).collect()
    }

    fn compute_batch(&mut self, results: &[Results]) -> Vec<Vec<f32>> {
        // Update store before computing — good episodes this batch immediately
        // shape the pattern weights used for returns below.
        self.update_store(results);

        let mut all_returns: Vec<Vec<f32>> = results.iter().map(|r| self.compute(r)).collect();

        // Batch std normalisation — capture pre-norm std for logging before dividing.
        let flat: Vec<f32> = all_returns.iter().flatten().copied().collect();
        let n = flat.len() as f32;
        if n > 0.0 {
            let mean = flat.iter().sum::<f32>() / n;
            let var = flat.iter().map(|r| (r - mean).powi(2)).sum::<f32>() / n;
            let std = var.sqrt().max(1e-4);
            self.last_raw_std = std;
            for ep in &mut all_returns {
                for r in ep.iter_mut() { *r /= std; }
            }
        }
        all_returns
    }

    fn pre_norm_ret_std(&self) -> Option<f32> {
        if self.last_raw_std == 0.0 { None } else { Some(self.last_raw_std) }
    }

    fn store_stats(&self) -> Option<StoreStats> {
        if self.store.is_empty() { return None; }
        let mut per_func: Vec<FuncStoreStats> = self.store.iter().map(|(func, episodes)| {
            let entries = episodes.len();
            let best  = episodes.first().map(|(s, _)| *s).unwrap_or(0.0);
            let worst = episodes.last().map(|(s, _)| *s).unwrap_or(0.0);
            FuncStoreStats {
                func_name: func.clone(),
                entries,
                best,
                worst,
                spread:    best - worst,
                diversity: store_diversity(episodes),
            }
        }).collect();
        per_func.sort_by(|a, b| a.func_name.cmp(&b.func_name));
        let total_entries = per_func.iter().map(|f| f.entries).sum();
        Some(StoreStats { total_entries, per_func })
    }
}
