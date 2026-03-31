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
}
