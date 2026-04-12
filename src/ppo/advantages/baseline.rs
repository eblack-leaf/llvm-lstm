use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;

/// Standard return-minus-baseline advantage: advantage[t] = return[t] − V(base_IR).
///
/// Advantages are batch-normalised (zero mean, unit variance) before being returned.
/// This removes value-function bias from the policy gradient — whether the critic
/// predicts 0.03 or 0.40, the normalised signal is always centred at zero so roughly
/// half of actions get reinforced and half penalised.
pub(crate) struct BaselineAdvantage;

impl Advantages for BaselineAdvantage {
    fn compute(&self, returns: &[Vec<f32>], results: &[Results]) -> Vec<Vec<f32>> {
        // Compute raw advantages.
        let raw: Vec<Vec<f32>> = returns
            .iter()
            .enumerate()
            .map(|(i, ep_returns)| {
                ep_returns
                    .iter()
                    .enumerate()
                    .map(|(t, &r)| {
                        let v = results[i].values.get(t).copied().unwrap_or(0.0);
                        r - v
                    })
                    .collect()
            })
            .collect();

        // Batch-normalise: compute mean and std across all steps.
        let all: Vec<f32> = raw.iter().flatten().copied().collect();
        let n = all.len() as f32;
        if n == 0.0 {
            return raw;
        }
        let mean = all.iter().sum::<f32>() / n;
        let var = all.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n;
        let std = var.sqrt();
        if std < 1e-8 {
            // All advantages identical — zero them out rather than divide by ~0.
            return raw.iter().map(|ep| vec![0.0; ep.len()]).collect();
        }

        raw.into_iter()
            .map(|ep| ep.into_iter().map(|a| (a - mean) / std).collect())
            .collect()
    }
}
