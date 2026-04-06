use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Assigns the terminal episode speedup uniformly to every executed slot.
pub(crate) struct EpisodeReturn;

impl Returns for EpisodeReturn {
    fn compute(&self, results: &Results) -> Vec<f32> {
        vec![results.episode_return; results.ep_len]
    }
}
