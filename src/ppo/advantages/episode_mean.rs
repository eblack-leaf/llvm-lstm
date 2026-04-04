use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;

/// Episode-mean deviation advantage.
///
/// `advantage[t] = return[t] - mean(episode_returns)`
///
/// Designed for returns where the sign is determined by position relative to
/// some episode-level event (e.g. BestStepReturn's best_idx), making the
/// learned value baseline V(s_t) circular: if V correctly predicts a negative
/// return, advantage → 0 and the policy gets no gradient to change behaviour.
///
/// This implementor avoids that by using the episode mean as a constant
/// baseline instead of a learned one. The advantage question becomes:
/// "was this step better or worse than the average step in this episode?"
///
/// - Steps before the peak: positive return above a mixed-sign mean → strong positive adv
/// - No-op steps (return = 0): penalised relative to any positive mean
/// - Regression steps: doubly penalised (negative return minus positive mean)
///
/// The value head is still trained against the raw returns for EV monitoring,
/// but it does not feed into the policy gradient here.
///
/// Optional batch-level scale normalisation divides all advantages by the
/// batch standard deviation (scale only — mean is already removed per episode).
pub(crate) struct EpisodeMeanAdvantage {
    pub(crate) normalise: bool,
}

impl EpisodeMeanAdvantage {
    pub(crate) fn new(normalise: bool) -> Self {
        Self { normalise }
    }
}

impl Advantages for EpisodeMeanAdvantage {
    fn compute(&self, returns: &[Vec<f32>], _results: &[Results]) -> Vec<Vec<f32>> {
        let mut advantages: Vec<Vec<f32>> = returns
            .iter()
            .map(|ep| {
                let mean = ep.iter().sum::<f32>() / ep.len().max(1) as f32;
                ep.iter().map(|r| r - mean).collect()
            })
            .collect();

        if self.normalise {
            let flat: Vec<f32> = advantages.iter().flatten().copied().collect();
            if flat.len() > 1 {
                let var =
                    flat.iter().map(|a| a.powi(2)).sum::<f32>() / flat.len() as f32;
                let std = var.sqrt().max(1e-8);
                for ep in &mut advantages {
                    for a in ep.iter_mut() {
                        *a /= std;
                    }
                }
            }
        }

        advantages
    }
}
