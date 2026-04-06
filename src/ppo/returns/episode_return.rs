use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Assigns the terminal episode speedup uniformly to every slot.
///
/// With the whole-sequence approach, every slot contributed to the final IR state
/// in some way (or chose not to via Stop), so uniform credit is the honest baseline.
pub(crate) struct EpisodeReturn;

impl Returns for EpisodeReturn {
    fn compute(&self, results: &Results) -> Vec<f32> {
        vec![results.episode_return; results.log_probs.len()]
    }
}
