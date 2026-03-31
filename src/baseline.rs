use std::collections::HashMap;

use crate::critic::{retrieval_score, Critic};
use crate::episode_store::BestEpisodeStore;
use crate::returns::Returns;
use crate::rollout::Rollout;

/// How to compute the baseline that advantages are measured against.
#[derive(Debug, Clone, PartialEq)]
pub enum BaselineMode {
    /// Mean G0 of the current batch — simple but noisy with few episodes.
    IntraBatch,
    /// Running best G0 per function — directional regret signal.
    /// Advantages are negative (or zero) unless the episode beats the best.
    Best,
    /// Ask the Critic module for a learned baseline estimate.
    Critic,
    /// Non-parametric k-NN lookup: Jaccard similarity on action sets,
    /// weighted average of top-k G0 values from BestEpisodeStore.
    Retrieval,
}

impl BaselineMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "intra-batch" => BaselineMode::IntraBatch,
            "critic"      => BaselineMode::Critic,
            "retrieval"   => BaselineMode::Retrieval,
            _             => BaselineMode::Best,
        }
    }
}

/// Per-function running statistics updated every iteration.
pub struct FnStats {
    /// Running best episode G0 per function.
    pub fn_best: HashMap<String, f32>,
    /// Exponential moving average of G0 per function (α=0.1).
    pub fn_ema:  HashMap<String, f32>,
}

impl FnStats {
    pub fn new() -> Self {
        Self { fn_best: HashMap::new(), fn_ema: HashMap::new() }
    }

    /// Update both EMA and running best from a single episode's G0.
    pub fn update(&mut self, func: &str, g0: f32) {
        let best = self.fn_best.entry(func.to_string()).or_insert(g0);
        if g0 > *best { *best = g0; }

        let ema = self.fn_ema.entry(func.to_string()).or_insert(g0);
        *ema = 0.9 * *ema + 0.1 * g0;
    }

    pub fn best(&self, func: &str) -> Option<f32> {
        self.fn_best.get(func).copied()
    }

    pub fn ema(&self, func: &str) -> Option<f32> {
        self.fn_ema.get(func).copied()
    }
}

/// Per-step baseline values aligned to the same flat indexing as `Returns`.
pub struct Baseline {
    pub values: Vec<f32>,
}

impl Baseline {
    /// Compute baselines for all (function, rollout) pairs.
    ///
    /// The baseline is the same scalar for every step in an episode (episode-level
    /// subtraction).  Critic and Retrieval modes call their scorer once per episode.
    pub fn select(
        rollout_funcs: &[String],
        rollouts:      &[Rollout],
        returns:       &Returns,
        mode:          &BaselineMode,
        fn_stats:      &FnStats,
        critic:        &dyn Critic,
        store:         &BestEpisodeStore,
    ) -> Self {
        let intra_mean = if returns.g0_per_ep.is_empty() {
            0.0
        } else {
            returns.g0_per_ep.iter().sum::<f32>() / returns.g0_per_ep.len() as f32
        };

        let mut values = Vec::new();

        for (func, rollout) in rollout_funcs.iter().zip(rollouts.iter()) {
            // Use step 0's features as the IR feature vector for this episode.
            let ir_feats = rollout.states.first().map(|s| s.as_slice()).unwrap_or(&[]);

            let ep_baseline = match mode {
                BaselineMode::IntraBatch => intra_mean,
                BaselineMode::Best       => fn_stats.best(func).unwrap_or(0.0),
                BaselineMode::Critic     => critic.score(func, &rollout.actions, ir_feats),
                BaselineMode::Retrieval  => retrieval_score(store, func, &rollout.actions),
            };
            for _ in 0..rollout.len() {
                values.push(ep_baseline);
            }
        }

        Self { values }
    }
}

/// Compute normalised advantages from returns, baselines, and per-step weights.
///
/// Raw advantage: `(return - baseline) * weight`
/// Normalised:    `(raw - mean(raw)) / (std(raw) + 1e-8)`
///
/// `weights` is per-step; all steps in an episode share the episode's weight.
/// Use [`broadcast_to_steps`] to build the weight vector from per-episode values.
pub fn build_advantages(returns: &[f32], baselines: &[f32], weights: &[f32]) -> Vec<f32> {
    let raw: Vec<f32> = returns.iter()
        .zip(baselines.iter())
        .zip(weights.iter())
        .map(|((&r, &b), &w)| (r - b) * w)
        .collect();

    let n = raw.len() as f32;
    if n < 2.0 {
        return raw;
    }
    let mean = raw.iter().sum::<f32>() / n;
    let std  = (raw.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n).sqrt();
    raw.iter().map(|x| (x - mean) / (std + 1e-8)).collect()
}

/// Broadcast per-episode weights to per-step weights by repeating each weight
/// for the length of its episode.
pub fn broadcast_to_steps(ep_weights: &[f32], ep_lens: &[usize]) -> Vec<f32> {
    ep_weights.iter().zip(ep_lens.iter())
        .flat_map(|(&w, &len)| std::iter::repeat(w).take(len))
        .collect()
}
