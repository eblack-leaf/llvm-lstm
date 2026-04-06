use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;

/// Rank-normalised advantage. Episodes are ranked by terminal speedup and mapped to
/// [-1, 1]. V(base_IR) is subtracted to reduce variance within each episode.
///
/// Ranking makes the signal ordinal rather than cardinal, reducing sensitivity to
/// speedup scale differences across functions.
pub(crate) struct RankAdvantage;

impl Advantages for RankAdvantage {
    fn compute(&self, returns: &[Vec<f32>], results: &[Results]) -> Vec<Vec<f32>> {
        let n = returns.len();
        if n == 0 {
            return vec![];
        }

        let episode_scores: Vec<f32> = returns
            .iter()
            .map(|r| r.iter().sum::<f32>() / r.len().max(1) as f32)
            .collect();

        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| episode_scores[a].partial_cmp(&episode_scores[b]).unwrap());
        let mut ranks = vec![0usize; n];
        for (rank, &idx) in order.iter().enumerate() {
            ranks[idx] = rank;
        }

        let rank_score = |ep_idx: usize| -> f32 {
            if n == 1 { 0.0 } else { 2.0 * ranks[ep_idx] as f32 / (n - 1) as f32 - 1.0 }
        };

        returns
            .iter()
            .enumerate()
            .map(|(ep_idx, ep_returns)| {
                let score = rank_score(ep_idx);
                let v = results[ep_idx].value;
                ep_returns.iter().map(|_| score - v).collect()
            })
            .collect()
    }
}
