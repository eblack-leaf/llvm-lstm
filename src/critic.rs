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
/// A `Critic` scores an action sequence + base IR features and returns a scalar
/// baseline value used to centre advantages.
pub trait Critic: Send {
    fn score(&self, func: &str, actions: &[usize], ir_features: &[f32]) -> f32;
    fn update(&mut self, store: &BestEpisodeStore);
    fn name(&self) -> &str;
}

// ── NullCritic ────────────────────────────────────────────────────────────────

pub struct NullCritic;

impl Critic for NullCritic {
    fn score(&self, _func: &str, _actions: &[usize], _ir: &[f32]) -> f32 { 0.0 }
    fn update(&mut self, _store: &BestEpisodeStore) {}
    fn name(&self) -> &str { "null" }
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
///   FiLM: ir_features → Linear(ir_dim, ch*3*2) → split into (scale, bias)
///   → h = relu(pooled * scale + bias)
///   → FC(ReLU) → scalar per sequence
#[derive(Module, Debug)]
pub struct IrFilmCNN<B: burn::tensor::backend::Backend> {
    action_embed: Embedding<B>,
    conv3:        Conv1d<B>,
    conv5:        Conv1d<B>,
    conv7:        Conv1d<B>,
    film:         Linear<B>,   // ir_dim → conv_channels*3*2 (scale + bias)
    fc:           Linear<B>,
    value_out:    Linear<B>,
}

impl IrFilmCnnConfig {
    pub fn init<B: burn::tensor::backend::Backend>(
        &self,
        device: &B::Device,
    ) -> IrFilmCNN<B> {
        let concat_width = self.conv_channels * 3;
        let hidden       = concat_width * 2;
        IrFilmCNN {
            action_embed: EmbeddingConfig::new(self.num_actions, self.embed_dim).init(device),
            conv3: Conv1dConfig::new(self.embed_dim, self.conv_channels, 3)
                .with_padding(PaddingConfig1d::Same).init(device),
            conv5: Conv1dConfig::new(self.embed_dim, self.conv_channels, 5)
                .with_padding(PaddingConfig1d::Same).init(device),
            conv7: Conv1dConfig::new(self.embed_dim, self.conv_channels, 7)
                .with_padding(PaddingConfig1d::Same).init(device),
            film:      LinearConfig::new(self.ir_dim, concat_width * 2).init(device),
            fc:        LinearConfig::new(concat_width, hidden).init(device),
            value_out: LinearConfig::new(hidden, 1).init(device),
        }
    }
}

impl<B: burn::tensor::backend::Backend> IrFilmCNN<B> {
    /// `actions`:     `[batch, seq_len]` int tensor
    /// `ir_features`: `[batch, ir_dim]`  float tensor
    /// Returns:       `[batch]` scalar estimates.
    pub fn forward(
        &self,
        actions:     Tensor<B, 2, Int>,
        ir_features: Tensor<B, 2>,
    ) -> Tensor<B, 1> {
        let [batch, _] = actions.dims();

        // Convolve over action sequence
        let emb = self.action_embed.forward(actions);
        let x   = emb.permute([0, 2, 1]);             // [b, embed, seq]

        let c3 = relu(self.conv3.forward(x.clone())); // [b, ch, seq]
        let c5 = relu(self.conv5.forward(x.clone()));
        let c7 = relu(self.conv7.forward(x));

        let [_, ch3, _] = c3.dims();
        let [_, ch5, _] = c5.dims();
        let [_, ch7, _] = c7.dims();
        let p3 = c3.mean_dim(2).reshape([batch, ch3]);
        let p5 = c5.mean_dim(2).reshape([batch, ch5]);
        let p7 = c7.mean_dim(2).reshape([batch, ch7]);

        let pooled = Tensor::cat(vec![p3, p5, p7], 1); // [b, ch*3]

        // FiLM conditioning: scale + bias from IR features
        let film_out = self.film.forward(ir_features);             // [b, ch*3*2]
        let cw       = pooled.shape().dims[1];
        let scale    = film_out.clone().slice([0..batch, 0..cw]);  // [b, ch*3]
        let bias     = film_out.slice([0..batch, cw..cw * 2]);     // [b, ch*3]

        let modulated = relu(pooled * (scale + 1.0) + bias);       // [b, ch*3]
        let h         = relu(self.fc.forward(modulated));
        self.value_out.forward(h).reshape([batch])
    }
}

// ── IrFilmCritic ─────────────────────────────────────────────────────────────

pub struct IrFilmCritic<B: AutodiffBackend> {
    model:  Option<IrFilmCNN<B>>,
    optim:  OptimizerAdaptor<Adam, IrFilmCNN<B>, B>,
    device: B::Device,
    lr:     f64,
    ir_dim: usize,
}

impl<B: AutodiffBackend> IrFilmCritic<B> {
    pub fn new(config: IrFilmCnnConfig, lr: f64, device: B::Device) -> Self {
        let ir_dim = config.ir_dim;
        let model  = config.init::<B>(&device);
        let optim  = AdamConfig::new().init::<B, IrFilmCNN<B>>();
        Self { model: Some(model), optim, device, lr, ir_dim }
    }
}

impl<B: AutodiffBackend + 'static> Critic for IrFilmCritic<B>
where
    B::Device: Clone,
{
    fn score(&self, _func: &str, actions: &[usize], ir_features: &[f32]) -> f32 {
        if actions.is_empty() { return 0.0; }
        let model_inf  = self.model.as_ref().unwrap().valid();
        let action_ids: Vec<i64> = actions.iter().map(|&a| a as i64).collect();
        let seq        = action_ids.len();

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
        model_inf.forward(actions_t, ir_t)
            .into_data()
            .to_vec::<f32>()
            .unwrap_or_default()
            .first()
            .copied()
            .unwrap_or(0.0)
    }

    fn update(&mut self, store: &BestEpisodeStore) {
        let all_episodes: Vec<(&Vec<usize>, &Vec<f32>, f32)> = store
            .iter_funcs()
            .flat_map(|(_, eps)| eps.iter().map(|e| (&e.actions, &e.ir_features, e.g0)))
            .collect();

        if all_episodes.is_empty() { return; }

        let max_len = all_episodes.iter().map(|(a, _, _)| a.len()).max().unwrap_or(1);
        let batch   = all_episodes.len();

        let mut action_buf = vec![0i64;  batch * max_len];
        let mut ir_buf     = vec![0.0f32; batch * self.ir_dim];
        let mut target_buf = vec![0.0f32; batch];

        for (i, (actions, ir, g0)) in all_episodes.iter().enumerate() {
            for (t, &a) in actions.iter().enumerate() {
                action_buf[i * max_len + t] = a as i64;
            }
            let ir_slice = &mut ir_buf[i * self.ir_dim..(i + 1) * self.ir_dim];
            let src_len = ir.len().min(self.ir_dim);
            ir_slice[..src_len].copy_from_slice(&ir[..src_len]);
            target_buf[i] = *g0;
        }

        let actions_t = Tensor::<B, 2, Int>::from_data(
            TensorData::new(action_buf, [batch, max_len]),
            &self.device,
        );
        let ir_t = Tensor::<B, 2>::from_data(
            TensorData::new(ir_buf, [batch, self.ir_dim]),
            &self.device,
        );
        let targets_t = Tensor::<B, 1>::from_data(
            TensorData::new(target_buf, [batch]),
            &self.device,
        );

        let model     = self.model.take().unwrap();
        let predicted = model.forward(actions_t, ir_t);
        let loss      = (predicted - targets_t).powf_scalar(2.0f32).mean();

        let grads       = loss.backward();
        let grad_params = GradientsParams::from_grads(grads, &model);
        let model       = self.optim.step(self.lr, model, grad_params);
        self.model      = Some(model);
    }

    fn name(&self) -> &str { "ir-film" }
}

// ── RetrievalCritic ───────────────────────────────────────────────────────────
//
// Non-parametric k-NN baseline: Jaccard similarity on action-id sets,
// weighted average of top-k G0 values from BestEpisodeStore.
// No gradient, no parameters — always up to date.

const KNN_K: usize = 5;

fn jaccard(a: &[usize], b: &[usize]) -> f32 {
    if a.is_empty() && b.is_empty() { return 1.0; }
    // Use sorted deduplication for set ops without heap allocation
    let mut sa: Vec<usize> = a.to_vec(); sa.sort_unstable(); sa.dedup();
    let mut sb: Vec<usize> = b.to_vec(); sb.sort_unstable(); sb.dedup();
    let mut inter = 0usize;
    let (mut i, mut j) = (0, 0);
    while i < sa.len() && j < sb.len() {
        match sa[i].cmp(&sb[j]) {
            std::cmp::Ordering::Equal => { inter += 1; i += 1; j += 1; }
            std::cmp::Ordering::Less  => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }
    let union = sa.len() + sb.len() - inter;
    if union == 0 { 1.0 } else { inter as f32 / union as f32 }
}

pub struct RetrievalCritic;

impl Critic for RetrievalCritic {
    fn score(&self, func: &str, actions: &[usize], _ir: &[f32]) -> f32 {
        // score() needs the store — but the Critic trait doesn't pass it.
        // RetrievalCritic is only useful via BaselineMode::Retrieval which
        // calls store directly; score() is a no-op here.
        let _ = func;
        let _ = actions;
        0.0
    }

    fn update(&mut self, _store: &BestEpisodeStore) {}
    fn name(&self) -> &str { "retrieval" }
}

/// Standalone retrieval score used by BaselineMode::Retrieval.
/// Looks up `func` in `store`, computes Jaccard similarity against `actions`,
/// and returns a weighted-average G0 of the top-k matches.
pub fn retrieval_score(store: &BestEpisodeStore, func: &str, actions: &[usize]) -> f32 {
    let episodes = store.get(func);
    if episodes.is_empty() { return 0.0; }

    let mut sims: Vec<(f32, f32)> = episodes.iter()
        .map(|e| (jaccard(actions, &e.actions), e.g0))
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

// ── HybridCritic ─────────────────────────────────────────────────────────────
//
// Uses retrieval when the store is sparse (< nn_threshold episodes),
// then switches to the IR-FiLM CNN once enough data has accumulated.
// Both run in parallel after the switch point; CNN takes precedence.

pub struct HybridCritic<B: AutodiffBackend> {
    film:         IrFilmCritic<B>,
    nn_threshold: usize,
}

impl<B: AutodiffBackend> HybridCritic<B> {
    pub fn new(config: IrFilmCnnConfig, lr: f64, device: B::Device, nn_threshold: usize) -> Self {
        Self { film: IrFilmCritic::new(config, lr, device), nn_threshold }
    }
}

impl<B: AutodiffBackend + 'static> Critic for HybridCritic<B>
where
    B::Device: Clone,
{
    fn score(&self, func: &str, actions: &[usize], ir_features: &[f32]) -> f32 {
        // score() without store: fall back to film score.
        // Retrieval is only meaningful via BaselineMode::Retrieval.
        self.film.score(func, actions, ir_features)
    }

    fn update(&mut self, store: &BestEpisodeStore) {
        if store.total_count() >= self.nn_threshold {
            self.film.update(store);
        }
    }

    fn name(&self) -> &str {
        "hybrid"
    }
}

/// Score function that respects the hybrid threshold:
/// retrieval when store is sparse, film CNN when rich.
pub fn hybrid_score<B: AutodiffBackend + 'static>(
    hybrid:  &HybridCritic<B>,
    store:   &BestEpisodeStore,
    func:    &str,
    actions: &[usize],
    ir:      &[f32],
) -> f32
where
    B::Device: Clone,
{
    if store.total_count() < hybrid.nn_threshold {
        retrieval_score(store, func, actions)
    } else {
        hybrid.film.score(func, actions, ir)
    }
}
