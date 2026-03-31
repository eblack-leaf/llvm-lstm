use burn::config::Config;
use burn::module::{AutodiffModule, Module};
use burn::nn::conv::{Conv1d, Conv1dConfig};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig, PaddingConfig1d};
use burn::optim::optim::adaptor::OptimizerAdaptor;
use burn::optim::{Adam, AdamConfig, GradientsParams, Optimizer};
use burn::tensor::activation::relu;
use burn::tensor::backend::AutodiffBackend;
use burn::tensor::{Int, Tensor, TensorData};

use crate::episode_store::BestEpisodeStore;

// ── Critic trait ──────────────────────────────────────────────────────────────

/// Common interface for all baseline critic architectures.
///
/// A `Critic` scores an action sequence and returns a scalar baseline value.
/// The actor's gradient path never touches this module.
pub trait Critic: Send {
    fn score(&self, func: &str, actions: &[usize]) -> f32;
    fn update(&mut self, store: &BestEpisodeStore);
    fn name(&self) -> &str;
}

// ── NullCritic ────────────────────────────────────────────────────────────────

pub struct NullCritic;

impl Critic for NullCritic {
    fn score(&self, _func: &str, _actions: &[usize]) -> f32 { 0.0 }
    fn update(&mut self, _store: &BestEpisodeStore) {}
    fn name(&self) -> &str { "null" }
}

// ── PatternCNN burn Module ────────────────────────────────────────────────────

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

/// 1D CNN over action sequences.
///
/// Architecture:
///   action_ids → embedding [b, seq, embed] → permute [b, embed, seq]
///   → conv3/conv5/conv7 (same padding) → global mean-pool each
///   → concat [b, conv_channels*3] → FC (ReLU) → scalar per sequence
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
    /// `actions`: `[batch, seq_len]` int tensor  →  `[batch]` scalar estimates.
    pub fn forward(&self, actions: Tensor<B, 2, Int>) -> Tensor<B, 1> {
        let [batch, _] = actions.dims();

        // Embed → [b, seq, embed] → permute → [b, embed, seq] for Conv1d
        let emb = self.action_embed.forward(actions);
        let x   = emb.permute([0, 2, 1]);

        let c3 = relu(self.conv3.forward(x.clone())); // [b, ch, seq]
        let c5 = relu(self.conv5.forward(x.clone()));
        let c7 = relu(self.conv7.forward(x));

        // Global mean-pool over sequence; extract channel count from tensor shape.
        let [_, ch3, _] = c3.dims();
        let [_, ch5, _] = c5.dims();
        let [_, ch7, _] = c7.dims();
        let p3 = c3.mean_dim(2).reshape([batch, ch3]);
        let p5 = c5.mean_dim(2).reshape([batch, ch5]);
        let p7 = c7.mean_dim(2).reshape([batch, ch7]);

        let pooled = Tensor::cat(vec![p3, p5, p7], 1);
        let h      = relu(self.fc.forward(pooled));
        self.value_out.forward(h).reshape([batch])
    }
}

// ── PatternCnnCritic ──────────────────────────────────────────────────────────

pub struct PatternCnnCritic<B: AutodiffBackend> {
    model:  Option<PatternCNN<B>>,
    optim:  OptimizerAdaptor<Adam, PatternCNN<B>, B>,
    device: B::Device,
    lr:     f64,
}

impl<B: AutodiffBackend> PatternCnnCritic<B> {
    pub fn new(config: PatternCnnConfig, lr: f64, device: B::Device) -> Self {
        let model = config.init::<B>(&device);
        let optim = AdamConfig::new().init::<B, PatternCNN<B>>();
        Self { model: Some(model), optim, device, lr }
    }
}

impl<B: AutodiffBackend + 'static> Critic for PatternCnnCritic<B>
where
    B::Device: Clone,
{
    fn score(&self, _func: &str, actions: &[usize]) -> f32 {
        if actions.is_empty() { return 0.0; }

        let model_inf  = self.model.as_ref().unwrap().valid();
        let action_ids: Vec<i64> = actions.iter().map(|&a| a as i64).collect();
        let seq        = action_ids.len();
        let actions_t  = Tensor::<B::InnerBackend, 2, Int>::from_data(
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
        let all_episodes: Vec<(&Vec<usize>, f32)> = store
            .iter_funcs()
            .flat_map(|(_, eps)| eps.iter().map(|e| (&e.actions, e.g0)))
            .collect();

        if all_episodes.is_empty() { return; }

        let max_len = all_episodes.iter().map(|(a, _)| a.len()).max().unwrap_or(1);
        let batch   = all_episodes.len();

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

        let model       = self.model.take().unwrap();
        let predicted   = model.forward(actions_t);
        let loss        = (predicted - targets_t).powf_scalar(2.0f32).mean();

        let grads       = loss.backward();
        let grad_params = GradientsParams::from_grads(grads, &model);
        let model       = self.optim.step(self.lr, model, grad_params);
        self.model      = Some(model);
    }

    fn name(&self) -> &str { "pattern-cnn" }
}
