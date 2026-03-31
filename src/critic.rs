use burn::config::Config;
use burn::module::{AutodiffModule, Module};
use burn::nn::conv::{Conv1d, Conv1dConfig};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig, PaddingConfig1d};
use burn::optim::optim::adaptor::OptimizerAdaptor;
use burn::optim::{Adam, AdamConfig, GradientsParams, Optimizer};
use burn::prelude::ElementConversion;
use burn::tensor::activation::relu;
use burn::tensor::backend::AutodiffBackend;
use burn::tensor::{Int, Tensor, TensorData};

use crate::episode_store::BestEpisodeStore;

// ── Critic trait ──────────────────────────────────────────────────────────────

/// Common interface for all baseline critic architectures.
///
/// A `Critic` scores an action sequence + base IR features and returns a scalar
/// baseline value used to centre advantages.
pub trait Critic: Send {
    fn score(&self, func: &str, actions: &[usize], ir_features: &[f32]) -> f32;
    fn update(&mut self, store: &BestEpisodeStore) -> Option<f32>;
    fn name(&self) -> &str;
}

// ── NullCritic ────────────────────────────────────────────────────────────────

pub struct NullCritic;

impl Critic for NullCritic {
    fn score(&self, _func: &str, _actions: &[usize], _ir: &[f32]) -> f32 {
        0.0
    }
    fn update(&mut self, _store: &BestEpisodeStore) -> Option<f32> {
        None
    }
    fn name(&self) -> &str {
        "null"
    }
}

// ── IrFilmCNN burn Module ─────────────────────────────────────────────────────

#[derive(Config, Debug)]
pub struct IrFilmCnnConfig {
    /// Pass vocabulary size — must match the actor's `num_actions`.
    pub num_actions: usize,
    /// Dimension of the IR feature vector (34 for "base", 68 for "base+current").
    #[config(default = 34)]
    pub ir_dim: usize,
    /// Dimension of the learned action embedding.
    #[config(default = 16)]
    pub embed_dim: usize,
    /// Output channels per convolution kernel.
    /// Pooled concat width before FiLM = `conv_channels * 3`.
    #[config(default = 24)]
    pub conv_channels: usize,
}

/// IR-conditioned 1D CNN over action sequences via FiLM.
///
/// Architecture:
///   action_ids → embedding [b, seq, embed] → permute → [b, embed, seq]
///   → conv3/conv5/conv7 (same padding) → global mean-pool each
///   → concat [b, ch*3]
///   FiLM: ir_features → film_scale(ir_dim → ch*3)
///                      → film_bias(ir_dim  → ch*3)
///   → h = relu(pooled * (film_scale + 1) + film_bias)
///   → FC(ReLU) → scalar per sequence
#[derive(Module, Debug)]
pub struct IrFilmCNN<B: burn::tensor::backend::Backend> {
    action_embed: Embedding<B>,
    conv3: Conv1d<B>,
    conv5: Conv1d<B>,
    conv7: Conv1d<B>,
    film_scale: Linear<B>, // ir_dim → conv_channels*3
    film_bias: Linear<B>,  // ir_dim → conv_channels*3
    fc: Linear<B>,
    value_out: Linear<B>,
}

impl IrFilmCnnConfig {
    pub fn init<B: burn::tensor::backend::Backend>(&self, device: &B::Device) -> IrFilmCNN<B> {
        let concat_width = self.conv_channels * 3;
        let hidden = concat_width * 2;
        IrFilmCNN {
            action_embed: EmbeddingConfig::new(self.num_actions, self.embed_dim).init(device),
            // Valid + manual pre-pad: same semantics as Same but stable backward
            conv3: Conv1dConfig::new(self.embed_dim, self.conv_channels, 3)
                .with_padding(PaddingConfig1d::Valid)
                .init(device),
            conv5: Conv1dConfig::new(self.embed_dim, self.conv_channels, 5)
                .with_padding(PaddingConfig1d::Valid)
                .init(device),
            conv7: Conv1dConfig::new(self.embed_dim, self.conv_channels, 7)
                .with_padding(PaddingConfig1d::Valid)
                .init(device),
            film_scale: LinearConfig::new(self.ir_dim, concat_width).init(device),
            film_bias: LinearConfig::new(self.ir_dim, concat_width).init(device),
            fc: LinearConfig::new(concat_width, hidden).init(device),
            value_out: LinearConfig::new(hidden, 1).init(device),
        }
    }
}

impl<B: burn::tensor::backend::Backend> IrFilmCNN<B> {
    /// `actions`:     `[batch, seq_len]` int tensor
    /// `ir_features`: `[batch, ir_dim]`  float tensor
    /// Returns:       `[batch]` scalar estimates.
    pub fn forward(&self, actions: Tensor<B, 2, Int>, ir_features: Tensor<B, 2>) -> Tensor<B, 1> {
        let [batch, _] = actions.dims();

        let emb = self.action_embed.forward(actions);
        let x = emb.permute([0, 2, 1]); // [b, embed, seq]
        let device = x.device();
        let [xb, xc, _] = x.dims();

        // Manual symmetric padding so Valid conv produces same-length output.
        let pad = |t: Tensor<B, 3>, p: usize| -> Tensor<B, 3> {
            let z = Tensor::zeros([xb, xc, p], &device);
            Tensor::cat(vec![z.clone(), t, z], 2)
        };

        let c3 = relu(self.conv3.forward(pad(x.clone(), 1))); // [b, ch, seq]
        let c5 = relu(self.conv5.forward(pad(x.clone(), 2)));
        let c7 = relu(self.conv7.forward(pad(x, 3)));

        let [_, ch3, _] = c3.dims();
        let [_, ch5, _] = c5.dims();
        let [_, ch7, _] = c7.dims();
        let p3 = c3.mean_dim(2).reshape([batch, ch3]);
        let p5 = c5.mean_dim(2).reshape([batch, ch5]);
        let p7 = c7.mean_dim(2).reshape([batch, ch7]);

        let pooled = Tensor::cat(vec![p3, p5, p7], 1); // [b, ch*3]

        // FiLM conditioning: multiplicative scale + additive bias, both from IR
        let scale = self.film_scale.forward(ir_features.clone()); // [b, ch*3]
        let bias = self.film_bias.forward(ir_features); // [b, ch*3]
        let modulated = relu(pooled * (scale + 1.0) + bias); // [b, ch*3]
        let h = relu(self.fc.forward(modulated));
        self.value_out.forward(h).reshape([batch])
    }
}

// ── IrFilmCritic ─────────────────────────────────────────────────────────────

pub struct IrFilmCritic<B: AutodiffBackend> {
    model: Option<IrFilmCNN<B>>,
    optim: OptimizerAdaptor<Adam, IrFilmCNN<B>, B>,
    device: B::Device,
    lr: f64,
    ir_dim: usize,
}

impl<B: AutodiffBackend> IrFilmCritic<B> {
    pub fn new(config: IrFilmCnnConfig, lr: f64, device: B::Device) -> Self {
        let ir_dim = config.ir_dim;
        let model = config.init::<B>(&device);
        let optim = AdamConfig::new().init::<B, IrFilmCNN<B>>();
        Self {
            model: Some(model),
            optim,
            device,
            lr,
            ir_dim,
        }
    }
}

impl<B: AutodiffBackend + 'static> Critic for IrFilmCritic<B>
where
    B::Device: Clone,
{
    fn score(&self, _func: &str, actions: &[usize], ir_features: &[f32]) -> f32 {
        if actions.is_empty() {
            return 0.0;
        }
        let model_inf = self.model.as_ref().unwrap().valid();
        let action_ids: Vec<i64> = actions.iter().map(|&a| a as i64).collect();
        let seq = action_ids.len();

        // Pad or truncate ir_features to ir_dim
        let mut ir = ir_features.to_vec();
        ir.resize(self.ir_dim, 0.0);

        let actions_t = Tensor::<B::InnerBackend, 2, Int>::from_data(
            TensorData::new(action_ids, [1, seq]),
            &self.device,
        );
        let ir_t = Tensor::<B::InnerBackend, 2>::from_data(
            TensorData::new(ir, [1, self.ir_dim]),
            &self.device,
        );
        model_inf
            .forward(actions_t, ir_t)
            .into_data()
            .to_vec::<f32>()
            .unwrap_or_default()
            .first()
            .copied()
            .unwrap_or(0.0)
    }

    fn update(&mut self, store: &BestEpisodeStore) -> Option<f32> {
        let all_episodes: Vec<(&Vec<usize>, &Vec<f32>, f32)> = store
            .iter_funcs()
            .flat_map(|(_, eps)| eps.iter().map(|e| (&e.actions, &e.ir_features, e.g0)))
            .collect();

        if all_episodes.is_empty() {
            return None;
        }

        let max_len = all_episodes.iter().map(|(a, _, _)| a.len()).max().unwrap_or(1);
        let batch = all_episodes.len();

        let mut action_buf = vec![0i64; batch * max_len];
        let mut ir_buf = vec![0.0f32; batch * self.ir_dim];
        let mut target_buf = vec![0.0f32; batch];

        for (i, (actions, ir, g0)) in all_episodes.iter().enumerate() {
            for (t, &a) in actions.iter().enumerate() {
                action_buf[i * max_len + t] = a as i64;
            }
            let src_len = ir.len().min(self.ir_dim);
            ir_buf[i * self.ir_dim..i * self.ir_dim + src_len].copy_from_slice(&ir[..src_len]);
            target_buf[i] = *g0;
        }

        let device = self.model.as_ref().unwrap().action_embed.weight.device();
        let actions_t = Tensor::<B, 2, Int>::from_data(
            TensorData::new(action_buf, [batch, max_len]),
            &device,
        );
        let ir_t = Tensor::<B, 2>::from_data(
            TensorData::new(ir_buf, [batch, self.ir_dim]),
            &device,
        );
        let targets_t = Tensor::<B, 1>::from_data(
            TensorData::new(target_buf, [batch]),
            &device,
        );

        let model = self.model.take().unwrap();
        let predicted = model.forward(actions_t, ir_t);
        let loss = (predicted - targets_t).powf_scalar(2.0).mean();
        let loss_val = loss.clone().into_scalar().elem();

        let grads = loss.backward();
        let grad_params = GradientsParams::from_grads(grads, &model);
        let model = self.optim.step(self.lr, model, grad_params);
        self.model = Some(model);

        Some(loss_val)
    }

    fn name(&self) -> &str {
        "ir-film"
    }
}

// ── k-NN retrieval ────────────────────────────────────────────────────────────

const KNN_K: usize = 5;

/// LCS-based sequence similarity: 2*lcs / (|a| + |b|).
/// Order-aware — treats the action sequence as a sequence, not a set.
fn seq_sim(a: &[usize], b: &[usize]) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let denom = (a.len() + b.len()) as f32;
    if denom == 0.0 {
        return 1.0;
    }
    // DP LCS — O(n*m), sequences are short (≤ max_seq_length)
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![0u32; (n + 1) * (m + 1)];
    for i in 1..=n {
        for j in 1..=m {
            dp[i * (m + 1) + j] = if a[i - 1] == b[j - 1] {
                dp[(i - 1) * (m + 1) + (j - 1)] + 1
            } else {
                dp[(i - 1) * (m + 1) + j].max(dp[i * (m + 1) + (j - 1)])
            };
        }
    }
    2.0 * dp[n * (m + 1) + m] as f32 / denom
}

/// Standalone retrieval score used by BaselineMode::Retrieval.
/// Looks up `func` in `store`, computes Jaccard similarity against `actions`,
/// and returns a weighted-average G0 of the top-k matches.
pub fn retrieval_score(store: &BestEpisodeStore, func: &str, actions: &[usize]) -> f32 {
    let episodes = store.get(func);
    if episodes.is_empty() {
        return 0.0;
    }

    let mut sims: Vec<(f32, f32)> = episodes
        .iter()
        .map(|e| (seq_sim(actions, &e.actions), e.g0))
        .collect();

    // Take top-k by similarity
    sims.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    sims.truncate(KNN_K);

    let weight_sum: f32 = sims.iter().map(|&(s, _)| s).sum();
    if weight_sum < 1e-9 {
        // All similarities zero → fall back to mean of top-k
        sims.iter().map(|&(_, g)| g).sum::<f32>() / sims.len() as f32
    } else {
        sims.iter().map(|&(s, g)| s * g).sum::<f32>() / weight_sum
    }
}

// ── PerFuncCritic ─────────────────────────────────────────────────────────────

pub struct PerFuncCritic<B: AutodiffBackend> {
    models: std::collections::HashMap<String, IrFilmCNN<B>>,
    optims: std::collections::HashMap<String, OptimizerAdaptor<Adam, IrFilmCNN<B>, B>>,
    config: IrFilmCnnConfig,
    device: B::Device,
    lr: f64,
}

impl<B: AutodiffBackend> PerFuncCritic<B> {
    pub fn new(config: IrFilmCnnConfig, lr: f64, device: B::Device) -> Self {
        Self {
            models: Default::default(),
            optims: Default::default(),
            config,
            device,
            lr,
        }
    }
}

impl<B: AutodiffBackend + 'static> Critic for PerFuncCritic<B>
where
    B::Device: Clone,
{
    fn score(&self, func: &str, actions: &[usize], ir_features: &[f32]) -> f32 {
        let Some(model) = self.models.get(func) else {
            return 0.0;
        };
        if actions.is_empty() {
            return 0.0;
        }
        let model_inf = model.valid();
        let seq = actions.len();
        let mut ir = ir_features.to_vec();
        ir.resize(self.config.ir_dim, 0.0);
        let actions_t = Tensor::<B::InnerBackend, 2, Int>::from_data(
            TensorData::new(
                actions.iter().map(|&a| a as i64).collect::<Vec<_>>(),
                [1, seq],
            ),
            &self.device,
        );
        let ir_t = Tensor::<B::InnerBackend, 2>::from_data(
            TensorData::new(ir, [1, self.config.ir_dim]),
            &self.device,
        );
        model_inf
            .forward(actions_t, ir_t)
            .into_data()
            .to_vec::<f32>()
            .unwrap_or_default()
            .first()
            .copied()
            .unwrap_or(0.0)
    }

    fn update(&mut self, store: &BestEpisodeStore) -> Option<f32> {
        let mut total_loss = 0.0;
        let mut func_count = 0;
        for (func, episodes) in store.iter_funcs() {
            if episodes.is_empty() {
                continue;
            }
            let model = self
                .models
                .entry(func.to_string())
                .or_insert_with(|| self.config.init::<B>(&self.device));
            let optim = self
                .optims
                .entry(func.to_string())
                .or_insert_with(|| AdamConfig::new().init::<B, IrFilmCNN<B>>());

            let max_len = episodes.iter().map(|e| e.actions.len()).max().unwrap_or(1);
            let batch = episodes.len();
            let ir_dim = self.config.ir_dim;
            let mut action_buf = vec![0i64; batch * max_len];
            let mut ir_buf = vec![0.0f32; batch * ir_dim];
            let mut target_buf = vec![0.0f32; batch];
            for (i, ep) in episodes.iter().enumerate() {
                for (t, &a) in ep.actions.iter().enumerate() {
                    action_buf[i * max_len + t] = a as i64;
                }
                let src = ep.ir_features.len().min(ir_dim);
                ir_buf[i * ir_dim..i * ir_dim + src].copy_from_slice(&ep.ir_features[..src]);
                target_buf[i] = ep.g0;
            }
            let actions_t = Tensor::<B, 2, Int>::from_data(
                TensorData::new(action_buf, [batch, max_len]),
                &self.device,
            );
            let ir_t =
                Tensor::<B, 2>::from_data(TensorData::new(ir_buf, [batch, ir_dim]), &self.device);
            let targets_t =
                Tensor::<B, 1>::from_data(TensorData::new(target_buf, [batch]), &self.device);

            // Need to take ownership for burn autodiff
            // SAFETY: we re-insert immediately after
            let m = std::mem::replace(model, self.config.init::<B>(&self.device));
            let predicted = m.forward(actions_t, ir_t);
            let loss = (predicted - targets_t).powf_scalar(2.0f32).mean();
            let loss_val: f32 = loss.clone().into_scalar().elem();
            total_loss += loss_val;
            func_count += 1;
            let grads = loss.backward();
            let grad_params = GradientsParams::from_grads(grads, &m);
            let m = optim.step(self.lr, m, grad_params);
            *model = m;
        }
        Some(total_loss / func_count as f32)
    }

    fn name(&self) -> &str {
        "per-func"
    }
}
