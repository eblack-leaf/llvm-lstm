use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Assigns the episode-level benchmark reward uniformly to every step.
///
/// This is the honest baseline when causality is unclear: we know the sequence
/// as a whole produced a speedup (or pessimisation) but we cannot reliably say
/// which individual pass was responsible, so every step shares the credit equally.
///
/// In episode-level mode (steps.len() == 1), the single measurement is used directly.
/// In per-step mode (steps.len() > 1), the cumulative measurements are available
/// via Step metadata, but this implementor deliberately ignores them — it still uses
/// the final trusted speedup to avoid the last-pass attribution bias.
pub(crate) struct EpisodeReturn;

impl Returns for EpisodeReturn {
    fn compute(&self, results: &Results) -> Vec<f32> {
        // Use the final step's speedup as the episode reward regardless of mode.
        // Per-step measurements in steps[0..T-1] are cumulative states, not
        // marginal contributions, so we do not try to read causality into them here.
        let reward = results
            .steps
            .last()
            .map(|s| s.benchmark.speedup)
            .unwrap_or(0.0);
        // One return per logged action (excluding the Start prefix).
        vec![reward; results.log_probs.len()]
    }
}
