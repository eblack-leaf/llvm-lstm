// TODO: Implement PPO (Proximal Policy Optimization) algorithm
//
// Reference: Schulman et al., "Proximal Policy Optimization Algorithms" (2017)
//
// Key components to implement:
// 1. Clipped surrogate objective
// 2. Value function loss (clipped or unclipped)
// 3. Entropy bonus for exploration
// 4. Mini-batch updates over collected rollouts
// 5. Advantage normalization

use burn::config::Config;

#[derive(Config, Debug)]
pub struct PpoConfig {
    /// Clipped surrogate objective epsilon.
    #[config(default = 0.2)]
    pub clip_epsilon: f32,
    /// Weight of the value function loss term.
    #[config(default = 0.5)]
    pub value_loss_coef: f32,
    /// Entropy bonus coefficient — encourages exploration.
    #[config(default = 0.01)]
    pub entropy_coef: f32,
    /// Adam learning rate.
    #[config(default = 3e-4)]
    pub learning_rate: f64,
    /// Discount factor.
    #[config(default = 0.99)]
    pub gamma: f32,
    /// GAE lambda for advantage estimation.
    #[config(default = 0.95)]
    pub gae_lambda: f32,
    /// Number of PPO epochs per rollout batch.
    #[config(default = 4)]
    pub num_epochs: usize,
    /// Mini-batch size for each PPO update step.
    #[config(default = 64)]
    pub mini_batch_size: usize,
    /// Maximum gradient norm for clipping.
    #[config(default = 0.5)]
    pub max_grad_norm: f32,
}

// TODO: Implement PPO update step
// pub fn ppo_update<B: Backend>(
//     policy: &mut LstmPolicy,
//     value_net: &mut ValueNetwork,
//     rollout: &Rollout,
//     config: &PpoConfig,
// ) -> PpoStats { ... }

#[derive(Debug, Clone, Default)]
pub struct PpoStats {
    pub policy_loss: f32,
    pub value_loss: f32,
    pub entropy: f32,
    pub approx_kl: f32,
    pub clip_fraction: f32,
}
