// TODO: Implement training loop
//
// High-level flow:
// 1. Initialize environment, policy, value network
// 2. For each iteration:
//    a. Collect rollouts by running policy in environment
//    b. Compute advantages (GAE)
//    c. Run PPO update for K epochs
//    d. Log metrics
//    e. Periodically evaluate and save checkpoints
//
// Key decisions for the human:
// - Number of parallel environments (if any)
// - Rollout length vs episode length
// - Checkpoint strategy
// - Curriculum learning (start with easy functions?)

use burn::config::Config;
use crate::env::EnvConfig;
use crate::ppo::PpoConfig;

#[derive(Config, Debug)]
pub struct TrainConfig {
    /// Environment settings (benchmark paths, episode length, reward mode).
    /// Required — contains PathBuf fields that have no Config literal default.
    pub env: EnvConfig,
    /// PPO hyperparameters. Defaults to PpoConfig::new() if not overridden.
    #[config(default = "PpoConfig::new()")]
    pub ppo: PpoConfig,
    /// Total number of rollout-collect + PPO-update iterations.
    #[config(default = 1000)]
    pub total_iterations: usize,
    /// Number of environment steps collected per rollout batch.
    #[config(default = 128)]
    pub rollout_steps: usize,
    /// Run full evaluation every N iterations.
    #[config(default = 50)]
    pub eval_interval: usize,
    /// Directory to write model checkpoints.
    /// Required — String defaults aren't supported as Config literals.
    pub checkpoint_dir: String,
    /// Print training stats every N iterations.
    #[config(default = 10)]
    pub log_interval: usize,
}

// TODO: Implement training loop
// pub fn train(config: TrainConfig) -> Result<()> {
//     let mut env = LlvmEnv::new(config.env)?;
//     env.compute_baselines()?;
//
//     // Initialize policy and value network
//     // ...
//
//     for iteration in 0..config.total_iterations {
//         // Collect rollouts
//         // Compute advantages
//         // PPO update
//         // Log and evaluate
//     }
//
//     Ok(())
// }
