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

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub fn clear(&mut self) {
        self.states.clear();
        self.actions.clear();
        self.log_probs.clear();
        self.rewards.clear();
        self.values.clear();
        self.dones.clear();
    }

    // TODO: Implement GAE (Generalized Advantage Estimation)
    // pub fn compute_advantages(&self, gamma: f32, lambda: f32) -> (Vec<f32>, Vec<f32>) { ... }
}
