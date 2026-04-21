use crate::ppo::advantages::Advantages;
use crate::ppo::episode::Results;
use std::collections::HashMap;

/// GRPO-style advantage: group episodes by function, normalise each episode's
/// terminal return by the group mean and std. All steps in an episode share
/// the same advantage. The value head is not consulted.
pub(crate) struct GroupRelativeAdvantage;

impl Advantages for GroupRelativeAdvantage {
    fn compute(&self, _returns: &[Vec<f32>], results: &[Results]) -> Vec<Vec<f32>> {
        // Collect terminal speedups per function group.
        let mut groups: HashMap<&str, Vec<f32>> = HashMap::new();
        for r in results {
            groups.entry(&r.func_name).or_default().push(r.episode_return);
        }

        // Compute mean and std per group.
        let stats: HashMap<&str, (f32, f32)> = groups
            .iter()
            .map(|(&name, vals)| {
                let n = vals.len() as f32;
                let mean = vals.iter().sum::<f32>() / n;
                let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
                let std = var.sqrt().max(1e-8);
                (name, (mean, std))
            })
            .collect();

        results
            .iter()
            .map(|r| {
                let (mean, std) = stats[r.func_name.as_str()];
                let adv = (r.episode_return - mean) / std;
                vec![adv; r.ep_len]
            })
            .collect()
    }
}
