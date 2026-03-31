use burn::config::Config;

use crate::env::EnvConfig;
use crate::ppo::PpoConfig;

#[derive(Config, Debug)]
pub struct TrainConfig {
    /// Environment settings (benchmark paths, episode length, reward mode).
    pub env: EnvConfig,
    /// PPO hyperparameters.
    #[config(default = "PpoConfig::new()")]
    pub ppo: PpoConfig,
    /// Total number of rollout-collect + PPO-update iterations.
    #[config(default = 1000)]
    pub total_iterations: usize,
    /// Number of episodes to collect per function per iteration.
    /// Total episodes per iteration = episodes_per_function * num_functions.
    #[config(default = 8)]
    pub episodes_per_function: usize,
    /// Run full evaluation every N iterations.
    #[config(default = 50)]
    pub eval_interval: usize,
    /// Directory to write model checkpoints.
    pub checkpoint_dir: String,
    /// Print training stats every N iterations.
    #[config(default = 1)]
    pub log_interval: usize,
    /// Dynamically allocate episodes toward unsolved functions (EMA < 0).
    /// Solved functions get a minimum floor; remaining episodes go to hard ones.
    #[config(default = false)]
    pub dynamic_alloc: bool,
    /// Downweight solved functions' advantages when the batch mixes solved/unsolved.
    /// Disable to use uniform weighting always.
    #[config(default = true)]
    pub adv_weighting: bool,
    /// IR featurisation mode for the transformer: "base" | "base+current".
    /// "base": fixed base IR token + action sequence (default).
    /// "base+current": concat(base, current) 68-d IR token at each step.
    #[config(default = "\"base\".to_string()")]
    pub ir_mode: String,
    /// Return computation mode: "episode" | "per-step".
    /// "episode": G0 broadcast to all steps; "per-step": G_t = r_t + γ·G_{t+1}.
    #[config(default = "\"episode\".to_string()")]
    pub return_mode: String,
    /// Baseline subtracted from returns before normalisation.
    /// "intra-batch": mean G0 of current batch.
    /// "best": running best G0 per function (regret signal).
    /// "critic": learned PatternCNN baseline.
    /// "retrieval": k-NN Jaccard lookup in BestEpisodeStore.
    #[config(default = "\"best\".to_string()")]
    pub baseline_mode: String,
    /// Critic architecture used when baseline_mode = "critic" or "hybrid".
    /// "null": always 0.0.  "pattern-cnn": 1D CNN.  "ir-film": IR-conditioned CNN.
    /// "hybrid": retrieval until store fills, then ir-film CNN.
    #[config(default = "\"null\".to_string()")]
    pub critic_arch: String,
    /// Prune threshold for BestEpisodeStore.
    /// Episodes with g0 < (best_g0 - threshold) are dropped.
    #[config(default = 0.3)]
    pub prune_threshold: f32,
}

