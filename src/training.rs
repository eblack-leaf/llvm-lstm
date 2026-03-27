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

// pub fn train(config: TrainConfig) -> Result<()> {
//
//     // ── 1. One env per benchmark, all sharing the same model ─────────────
//     //
//     //   Each worker owns its LlvmEnv (separate compile/benchmark state).
//     //   Baselines are computed once up front — expensive but only done once.
//     //
//     let benchmarks: Vec<PathBuf> = vec![...]; // the 6 selected benchmarks
//     let mut envs: Vec<LlvmEnv> = benchmarks
//         .iter()
//         .map(|b| {
//             let cfg = EnvConfig::new(b.clone(), work_dir.clone(), RewardMode::Sparse);
//             let mut env = LlvmEnv::new(cfg)?;
//             env.compute_baselines()?;
//             Ok(env)
//         })
//         .collect::<Result<_>>()?;
//
//     // ── 2. Single model, one optimizer ───────────────────────────────────
//     let device = Default::default(); // NdArray CPU device
//     let model = ActorCriticConfig::new().init::<NdArray>(&device);
//     let mut optim = AdamConfig::new().init::<NdArray, ActorCritic<NdArray>>(&model);
//
//     // ── 3. Training loop ─────────────────────────────────────────────────
//     for iteration in 0..config.total_iterations {
//
//         // ── 3a. Collect rollouts across all envs ──────────────────────────
//         //
//         //   Each env runs episodes until the combined buffer reaches
//         //   rollout_steps total steps.  Workers step sequentially here;
//         //   the expensive part (compile + benchmark) could be parallelised
//         //   with rayon later.
//         //
//         //   Per step we need: state features, action, log_prob, reward, value, done.
//         //   hidden state is threaded through within each episode and reset on done.
//         //
//         let mut rollout = Rollout::new();
//         let mut hiddens: Vec<Option<Tensor<NdArray, 2>>> = vec![None; envs.len()];
//         let mut states: Vec<State> = envs.iter_mut().map(|e| e.reset()).collect::<Result<_>>()?;
//
//         while rollout.len() < config.rollout_steps {
//             for (i, env) in envs.iter_mut().enumerate() {
//                 let features = Tensor::from_data(..., &device); // state.features → tensor
//                 let prev_action = ...; // last action taken in this env (0 at start)
//
//                 let (logits, value, new_hidden) =
//                     model.forward(features, prev_action, hiddens[i].take());
//
//                 let action = sample_action(&logits);   // categorical sample
//                 let log_prob = log_prob_of(&logits, action);
//
//                 let step = env.step(action)?;
//
//                 rollout.push(
//                     states[i].features.clone(),
//                     action,
//                     log_prob,
//                     step.reward,
//                     value.into_scalar(),
//                     step.done,
//                 );
//
//                 if step.done {
//                     hiddens[i] = None;          // reset hidden on episode end
//                     states[i] = env.reset()?;
//                 } else {
//                     hiddens[i] = Some(new_hidden);
//                     states[i] = step.state;
//                 }
//             }
//         }
//
//         // ── 3b. Compute advantages (GAE) ──────────────────────────────────
//         let (advantages, returns) = rollout.compute_advantages(
//             config.ppo.gamma,
//             config.ppo.gae_lambda,
//         );
//
//         // ── 3c. PPO update — K epochs over minibatches ────────────────────
//         //
//         //   NOTE: recurrent PPO needs to re-roll the GRU from the start of
//         //   each episode to get valid hidden states for the loss computation.
//         //   For simplicity in the first version, treat each step independently
//         //   (feed zeros as hidden) — this loses some sequence information but
//         //   is much simpler to implement and still works reasonably well.
//         //
//         let stats = ppo_update(&mut model, &mut optim, &rollout, &advantages, &returns, &config.ppo, &device);
//
//         // ── 3d. Log ───────────────────────────────────────────────────────
//         if iteration % config.log_interval == 0 {
//             eprintln!(
//                 "[{iteration}] policy_loss={:.4} value_loss={:.4} entropy={:.4} kl={:.4}",
//                 stats.policy_loss, stats.value_loss, stats.entropy, stats.approx_kl,
//             );
//         }
//
//         // ── 3e. Checkpoint ────────────────────────────────────────────────
//         if iteration % config.eval_interval == 0 {
//             model.save_file(format!("{}/model_{iteration}.bin", config.checkpoint_dir), &recorder)?;
//         }
//     }
//
//     Ok(())
// }
