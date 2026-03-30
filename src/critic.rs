use burn::config::Config;
use burn::module::Module;
use burn::nn::conv::{Conv1d, Conv1dConfig, PaddingConfig1d};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::optim::{Adam, AdamConfig, GradientsParams, Optimizer as _};
use burn::tensor::activation::relu;
use burn::tensor::backend::AutodiffBackend;
use burn::tensor::{Int, Tensor, TensorData};

use crate::episode_store::BestEpisodeStore;

// ── Critic trait ──────────────────────────────────────────────────────────────

/// Common interface for all baseline critic architectures.
///
/// A `Critic` scores an action sequence and returns a scalar — this becomes the
/// per-episode baseline that advantages are subtracted from.  The actor's
/// gradient path never touches this module.
pub trait Critic: Send {
    /// Estimate the episode return for a given (function, action-sequence) pair.
    fn score(&self, func: &str, actions: &[usize]) -> f32;

    /// Update the critic from the current `BestEpisodeStore` (one gradient step
    /// or rule-based update).  Called once per training iteration.
    fn update(&mut self, store: &BestEpisodeStore);

    /// Human-readable name used in logs.
    fn name(&self) -> &str;
}

// ── NullCritic ────────────────────────────────────────────────────────────────

/// No-op critic — always returns 0.0.  Used as the default when no critic
/// architecture is configured, or before the store has any data.
pub struct NullCritic;

impl Critic for NullCritic {
    fn score(&self, _func: &str, _actions: &[usize]) -> f32 { 0.0 }
    fn update(&mut self, _store: &BestEpisodeStore) {}
    fn name(&self) -> &str { "null" }
}

// ── PatternCNN burn Module ────────────────────────────────────────────────────

/// Configuration for the inner `PatternCNN` burn module.
#[derive(Config, Debug)]
pub struct PatternCnnConfig {
    /// Pass vocabulary size — must match the actor's `num_actions`.
    pub num_actions: usize,
    /// Dimension of the learned action embedding.
    #[config(default = 16)]
    pub embed_dim: usize,
    /// Output channels per convolution kernel.
    /// Total concat width before the FC layer = `conv_channels * 3`.
    #[config(default = 24)]
    pub conv_channels: usize,
}

/// 1D convolutional network over action sequences.
///
/// Architecture:
///   action_ids → embedding [batch, seq, embed] → permute [batch, embed, seq]
///   → conv3/conv5/conv7 (each → conv_channels channels, same padding)
///   → global mean-pool each → concat [batch, conv_channels*3]
///   → FC (ReLU) → value_out (scalar per sequence)
#[derive(Module, Debug)]
pub struct PatternCNN<B: burn::tensor::backend::Backend> {
    action_embed: Embedding<B>,
    conv3:        Conv1d<B>,
    conv5:        Conv1d<B>,
    conv7:        Conv1d<B>,
    fc:           Linear<B>,
    value_out:    Linear<B>,
}

impl PatternCnnConfig {
    pub fn init<B: burn::tensor::backend::Backend>(
        &self,
        device: &B::Device,
    ) -> PatternCNN<B> {
        let hidden = self.conv_channels * 2;
        PatternCNN {
            action_embed: EmbeddingConfig::new(self.num_actions, self.embed_dim).init(device),
            conv3: Conv1dConfig::new(self.embed_dim, self.conv_channels, 3)
                .with_padding(PaddingConfig1d::Same)
                .init(device),
            conv5: Conv1dConfig::new(self.embed_dim, self.conv_channels, 5)
                .with_padding(PaddingConfig1d::Same)
                .init(device),
            conv7: Conv1dConfig::new(self.embed_dim, self.conv_channels, 7)
                .with_padding(PaddingConfig1d::Same)
                .init(device),
            fc:        LinearConfig::new(self.conv_channels * 3, hidden).init(device),
            value_out: LinearConfig::new(hidden, 1).init(device),
        }
    }
}

impl<B: burn::tensor::backend::Backend> PatternCNN<B> {
    /// Forward pass.
    ///
    /// `actions`: `[batch, seq_len]` integer action ids (0-padded for short seqs)
    /// Returns:   `[batch]` scalar estimates.
    pub fn forward(&self, actions: Tensor<B, 2, Int>) -> Tensor<B, 1> {
        let [batch, seq] = actions.dims();

        // Embed → [batch, seq, embed_dim] → [batch, embed_dim, seq] for Conv1d
        let emb = self.action_embed.forward(actions);                    // [b, s, e]
        let x   = emb.permute([0, 2, 1]);                                // [b, e, s]

        let c3 = relu(self.conv3.forward(x.clone()));                    // [b, ch, s]
        let c5 = relu(self.conv5.forward(x.clone()));
        let c7 = relu(self.conv7.forward(x));

        // Global mean-pool over sequence dimension
        let p3 = c3.mean_dim(2).reshape([batch, self.conv3.weight.dims()[0]]);
        let p5 = c5.mean_dim(2).reshape([batch, self.conv5.weight.dims()[0]]);
        let p7 = c7.mean_dim(2).reshape([batch, self.conv7.weight.dims()[0]]);

        let pooled = Tensor::cat(vec![p3, p5, p7], 1);                  // [b, ch*3]
        let h      = relu(self.fc.forward(pooled));                      // [b, hidden]
        self.value_out.forward(h).reshape([batch])                       // [b]
    }
}

// ── PatternCnnCritic ──────────────────────────────────────────────────────────

/// Critic backed by a `PatternCNN` trained via MSE on `BestEpisodeStore` episodes.
///
/// Internally holds both the autodiff model and its Adam optimizer so that
/// `update()` can do a self-contained gradient step without exposing generics
/// through the `Critic` trait boundary.
pub struct PatternCnnCritic<B: AutodiffBackend> {
    model:  Option<PatternCNN<B>>,
    optim:  Adam<B>,
    device: B::Device,
    lr:     f64,
    config: PatternCnnConfig,
}

impl<B: AutodiffBackend> PatternCnnCritic<B> {
    pub fn new(config: PatternCnnConfig, lr: f64, device: B::Device) -> Self {
        let model = config.init::<B>(&device);
        let optim = AdamConfig::new().init();
        Self { model: Some(model), optim, device, lr, config }
    }
}

impl<B: AutodiffBackend + 'static> Critic for PatternCnnCritic<B>
where
    B::Device: Clone,
{
    fn score(&self, _func: &str, actions: &[usize]) -> f32 {
        use burn::module::AutodiffModule;

        if actions.is_empty() {
            return 0.0;
        }

        let model_inf = self.model.as_ref().unwrap().valid();

        let action_ids: Vec<i64> = actions.iter().map(|&a| a as i64).collect();
        let seq = action_ids.len();
        let actions_t = Tensor::<B::InnerBackend, 2, Int>::from_data(
            TensorData::new(action_ids, [1, seq]),
            &self.device,
        );

        model_inf.forward(actions_t)
            .into_data()
            .to_vec::<f32>()
            .unwrap_or_default()
            .first()
            .copied()
            .unwrap_or(0.0)
    }

    fn update(&mut self, store: &BestEpisodeStore) {
        // Collect (actions, g0) from all functions in the store.
        let all_episodes: Vec<(&Vec<usize>, f32)> = store
            .store
            .values()
            .flat_map(|eps| eps.iter().map(|e| (&e.actions, e.g0)))
            .collect();

        if all_episodes.is_empty() {
            return;
        }

        let max_len = all_episodes.iter().map(|(a, _)| a.len()).max().unwrap_or(1);
        let batch   = all_episodes.len();

        // Pad action sequences with 0 (stop action) to max_len.
        let mut action_buf = vec![0i64; batch * max_len];
        let mut target_buf = vec![0.0f32; batch];

        for (i, (actions, g0)) in all_episodes.iter().enumerate() {
            for (t, &a) in actions.iter().enumerate() {
                action_buf[i * max_len + t] = a as i64;
            }
            target_buf[i] = *g0;
        }

        let actions_t = Tensor::<B, 2, Int>::from_data(
            TensorData::new(action_buf, [batch, max_len]),
            &self.device,
        );
        let targets_t = Tensor::<B, 1>::from_data(
            TensorData::new(target_buf, [batch]),
            &self.device,
        );

        let model = self.model.take().unwrap();
        let predicted = model.forward(actions_t);
        let loss      = (predicted - targets_t).powf_scalar(2.0f32).mean();

        let grads      = loss.backward();
        let grad_params = GradientsParams::from_grads(grads, &model);
        let model       = self.optim.step(self.lr, model, grad_params);
        self.model      = Some(model);
    }

    fn name(&self) -> &str { "pattern-cnn" }
}
