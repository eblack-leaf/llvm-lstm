use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;
use crate::ppo::BatchStep;

/// Standard return-minus-baseline advantage.
///
/// `advantage[t] = returns[t] - V(s_t)`
///
/// V(s_t) is trained to predict the same per-step returns (from whichever Returns
/// implementor is in use), so it is a valid variance-reducing baseline here.
///
/// Use this with `DeltaWeightedReturn` when you want per-step credit attribution to
/// drive both the policy gradient and the value target consistently.
///
/// Optional global whitening (zero mean, unit variance across the batch) reduces
/// sensitivity to return scale and stabilises early training.
pub(crate) struct BaselineAdvantage {
    pub(crate) normalise: bool,
}

impl BaselineAdvantage {
    pub(crate) fn new(normalise: bool) -> Self {
        Self { normalise }
    }
}

impl Advantages for BaselineAdvantage {
    fn compute(&self, returns: &[Vec<f32>], results: &[Results]) -> Vec<Vec<f32>> {
        let mut all_advantages: Vec<Vec<f32>> = returns
            .iter()
            .enumerate()
            .map(|(ep_idx, ep_returns)| {
                ep_returns
                    .iter()
                    .zip(&results[ep_idx].values)
                    .map(|(r, v)| r - v)
                    .collect()
            })
            .collect();

        if self.normalise {
            let flat: Vec<f32> = all_advantages.iter().flatten().copied().collect();
            if flat.len() > 1 {
                let mean = flat.iter().sum::<f32>() / flat.len() as f32;
                let var =
                    flat.iter().map(|a| (a - mean).powi(2)).sum::<f32>() / flat.len() as f32;
                let std = var.sqrt().max(1e-8);
                // Scale only — do not subtract mean. This stabilises gradient magnitude
                // while preserving absolute sign: a step with return=0 (Stop, no-ops)
                // keeps its positive advantage relative to negative-return steps.
                // Full whitening would center everything to zero and destroy that signal.
                for ep in &mut all_advantages {
                    for a in ep.iter_mut() {
                        *a /= std;
                    }
                }
            }
        }

        all_advantages
    }

    fn compute_live(&self, steps: &[BatchStep], pred_v: &[f32]) -> Vec<f32> {
        let mut advs: Vec<f32> = steps.iter().zip(pred_v).map(|(s, &v)| s.ret - v).collect();
        if self.normalise && advs.len() > 1 {
            let var = advs.iter().map(|a| a.powi(2)).sum::<f32>() / advs.len() as f32;
            let std = var.sqrt().max(1e-8);
            for a in &mut advs { *a /= std; }
        }
        advs
    }
}
