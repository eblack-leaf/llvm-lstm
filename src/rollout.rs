use serde::{Deserialize, Serialize};

/// Experience buffer for PPO training.
/// Stores trajectory data collected during rollouts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Rollout {
    pub states: Vec<Vec<f32>>,
    pub actions: Vec<usize>,
    pub log_probs: Vec<f32>,
    pub rewards: Vec<f32>,
    pub values: Vec<f32>,
    pub dones: Vec<bool>,
}

impl Rollout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(
        &mut self,
        state: Vec<f32>,
        action: usize,
        log_prob: f32,
        reward: f32,
        value: f32,
        done: bool,
    ) {
        self.states.push(state);
        self.actions.push(action);
        self.log_probs.push(log_prob);
        self.rewards.push(reward);
        self.values.push(value);
        self.dones.push(done);
    }

    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Concatenate multiple rollouts into one flat buffer.
    /// Used to combine per-episode rollouts into a single batch for the PPO update.
    pub fn merge(rollouts: &[Rollout]) -> Self {
        let mut out = Self::new();
        for r in rollouts {
            out.states.extend_from_slice(&r.states);
            out.actions.extend_from_slice(&r.actions);
            out.log_probs.extend_from_slice(&r.log_probs);
            out.rewards.extend_from_slice(&r.rewards);
            out.values.extend_from_slice(&r.values);
            out.dones.extend_from_slice(&r.dones);
        }
        out
    }

    /// Compute GAE advantages and discounted returns.
    ///
    /// `last_value` — critic's V(s) for the state *after* the final step.
    /// Pass 0.0 if the last step was a terminal `done`, otherwise run the
    /// model one extra time and pass that scalar.
    ///
    /// Returns `(advantages, returns)` both length `self.len()`.
    /// `returns[t] = advantages[t] + values[t]` and is used as the value target.
    pub fn compute_advantages(
        &self,
        gamma: f32,
        lambda: f32,
        last_value: f32,
    ) -> (Vec<f32>, Vec<f32>) {
        let n = self.len();
        let mut advantages = vec![0f32; n];
        let mut gae = 0f32;

        // Bootstrap from the state after the final stored step.
        let mut next_value = last_value;

        for t in (0..n).rev() {
            let mask = if self.dones[t] { 0.0 } else { 1.0 };
            let delta = self.rewards[t] + gamma * next_value * mask - self.values[t];
            gae = delta + gamma * lambda * mask * gae;
            advantages[t] = gae;
            next_value = self.values[t];
        }

        let returns = advantages
            .iter()
            .zip(self.values.iter())
            .map(|(a, v)| a + v)
            .collect();

        (advantages, returns)
    }
}
