use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;

/// Rank-normalises returns across the batch before subtracting the value baseline.
///
/// Why rank instead of raw returns:
/// Episode-level speedups have poor cardinal meaning — 1.1x vs 1.2x vs 2.0x rewards
/// are not on a stable scale across functions, compiler versions, or hardware states.
/// Ranking makes the signal ordinal: we only claim "this episode did better than that
/// one", not by how much. This reduces sensitivity to outliers and avoids the policy
/// chasing a single lucky high-speedup episode.
///
/// Rank scores are mapped to [-1, 1] so the gradient signal is zero-centered.
/// The value baseline V(s_t) is still subtracted to further reduce variance within
/// an episode — steps where the critic already expected a good outcome get a smaller
/// advantage than steps that beat expectations.
pub(crate) struct RankAdvantage {
    /// If true, normalise final advantages to zero mean unit variance across the batch.
    /// Useful early in training; can be disabled once value estimates stabilise.
    pub(crate) normalise: bool,
}

impl RankAdvantage {
    pub(crate) fn new(normalise: bool) -> Self {
        Self { normalise }
    }
}

impl Advantages for RankAdvantage {
    fn compute(&self, returns: &[Vec<f32>], results: &[Results]) -> Vec<Vec<f32>> {
        let n = returns.len();
        if n == 0 {
            return vec![];
        }

        // Each episode's "episode return" is the mean of its per-step returns.
        // For EpisodeReturn all steps carry the same value, so mean == that value.
        let episode_scores: Vec<f32> = returns
            .iter()
            .map(|r| r.iter().sum::<f32>() / r.len().max(1) as f32)
            .collect();

        // Rank episodes by score (0 = worst, n-1 = best).
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| episode_scores[a].partial_cmp(&episode_scores[b]).unwrap());
        let mut ranks = vec![0usize; n];
        for (rank, &idx) in order.iter().enumerate() {
            ranks[idx] = rank;
        }

        // Map rank to [-1, 1]: score = 2 * rank / (n - 1) - 1.
        // With a single episode the score is 0 (no relative information).
        let rank_score = |ep_idx: usize| -> f32 {
            if n == 1 {
                0.0
            } else {
                2.0 * ranks[ep_idx] as f32 / (n - 1) as f32 - 1.0
            }
        };

        // Per-step advantage: rank score of the episode minus value estimate at that step.
        // results[i].values gives V(s_t); Step metadata is available via results[i].steps
        // for future implementors that want finer attribution.
        let mut all_advantages: Vec<Vec<f32>> = returns
            .iter()
            .enumerate()
            .map(|(ep_idx, ep_returns)| {
                let score = rank_score(ep_idx);
                ep_returns
                    .iter()
                    .zip(&results[ep_idx].values)
                    .map(|(_, v)| score - v)
                    .collect()
            })
            .collect();

        if self.normalise {
            let flat: Vec<f32> = all_advantages.iter().flatten().copied().collect();
            let mean = flat.iter().sum::<f32>() / flat.len() as f32;
            let var = flat.iter().map(|a| (a - mean).powi(2)).sum::<f32>() / flat.len() as f32;
            let std = var.sqrt().max(1e-8);
            for ep in &mut all_advantages {
                for a in ep.iter_mut() {
                    *a = (*a - mean) / std;
                }
            }
        }

        all_advantages
    }
}
