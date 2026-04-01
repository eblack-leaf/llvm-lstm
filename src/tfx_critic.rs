// File: src/tfx_critic.rs (or inside critic.rs)
use burn::module::Module;
use burn::nn::transformer::{
    TransformerEncoder, TransformerEncoderConfig, TransformerEncoderInput,
};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::optim::optim::adaptor::OptimizerAdaptor;
use burn::optim::{Adam, AdamConfig, GradientsParams, Optimizer};
use burn::prelude::ElementConversion;
use burn::tensor::backend::AutodiffBackend;
use burn::tensor::{Int, Tensor, TensorData};

use crate::actor_critic_tfx::TransformerActorCriticConfig;
use crate::critic::Critic;
use crate::episode_store::BestEpisodeStore;

/// Transformer-based critic: uses the same encoder as the actor but without a policy head.
pub struct TransformerCritic<B: AutodiffBackend> {
    model: Option<TransformerCriticModel<B>>,
    optim: OptimizerAdaptor<Adam, TransformerCriticModel<B>, B>,
    lr: f64,
    ir_dim: usize, // IR feature dimension (34 or 68)
}

impl<B: AutodiffBackend> TransformerCritic<B> {
    pub fn new(actor_config: TransformerActorCriticConfig, lr: f64, device: &B::Device) -> Self {
        let model = TransformerCriticModel::new(actor_config.clone(), device);
        let optim = AdamConfig::new().init::<B, TransformerCriticModel<B>>();
        Self {
            model: Some(model),
            optim,
            lr,
            ir_dim: actor_config.input_dim,
        }
    }
}

#[derive(Module, Debug)]
struct TransformerCriticModel<B: burn::tensor::backend::Backend> {
    ir_proj: Linear<B>,
    action_embed: Embedding<B>,
    action_proj: Linear<B>,
    pos_embed: Embedding<B>,
    encoder: TransformerEncoder<B>,
    value_head: Linear<B>,
}

impl<B: burn::tensor::backend::Backend> TransformerCriticModel<B> {
    fn new(config: TransformerActorCriticConfig, device: &B::Device) -> Self {
        let encoder = TransformerEncoderConfig::new(
            config.d_model,
            config.d_ff,
            config.n_heads,
            config.n_layers,
        )
        .with_dropout(config.dropout)
        .init(device);

        Self {
            ir_proj: LinearConfig::new(config.input_dim, config.d_model).init(device),
            action_embed: EmbeddingConfig::new(config.num_actions, config.action_embed_dim)
                .init(device),
            action_proj: LinearConfig::new(config.action_embed_dim, config.d_model).init(device),
            pos_embed: EmbeddingConfig::new(config.max_seq_len, config.d_model).init(device),
            encoder,
            value_head: LinearConfig::new(config.d_model, 1).init(device),
        }
    }

    fn forward(&self, actions: Tensor<B, 2, Int>, ir_features: Tensor<B, 2>) -> Tensor<B, 1> {
        let [batch, seq_len] = actions.dims();
        let device = actions.device();

        // IR token
        let ir_token = self.ir_proj.forward(ir_features).unsqueeze_dim::<3>(1); // [b,1,d]

        // Action embeddings
        let act_emb = self.action_embed.forward(actions); // [b,seq,ae]
        let act_tok = self.action_proj.forward(act_emb); // [b,seq,d]

        let tokens = Tensor::cat(vec![ir_token, act_tok], 1); // [b,1+seq,d]

        // Positional embeddings for length 1+seq
        let pos_ids =
            Tensor::<B, 1, Int>::arange(0..(seq_len + 1) as i64, &device).unsqueeze::<2>(); // [1,1+seq]
        let pos_emb = self.pos_embed.forward(pos_ids); // [1,1+seq,d]
        let tokens = tokens + pos_emb;

        // Bidirectional attention (no mask)
        let out = self.encoder.forward(TransformerEncoderInput::new(tokens)); // [b,1+seq,d]

        // Take the last token (the final action) as the representation
        let d = out.dims()[2];
        let last = out.slice([0..batch, seq_len..seq_len + 1, 0..d]); // [b,1,d]
        let value = self.value_head.forward(last).reshape([batch]); // [b]
        value
    }
}

impl<B: AutodiffBackend + 'static> Critic for TransformerCritic<B>
where
    B::Device: Clone,
{
    fn score(&self, _func: &str, actions: &[usize], ir_features: &[f32]) -> f32 {
        if actions.is_empty() {
            return 0.0;
        }
        let model = self.model.as_ref().unwrap();
        let device = model.ir_proj.weight.device();

        let seq_len = actions.len();
        let mut ir = ir_features.to_vec();
        ir.resize(self.ir_dim, 0.0);

        let actions_t = Tensor::<B, 2, Int>::from_data(
            TensorData::new(
                actions.iter().map(|&a| a as i64).collect::<Vec<_>>(),
                [1, seq_len],
            ),
            &device,
        );
        let ir_t = Tensor::<B, 2>::from_data(TensorData::new(ir, [1, self.ir_dim]), &device);
        model
            .forward(actions_t, ir_t)
            .into_data()
            .to_vec::<f32>()
            .unwrap_or_default()
            .first()
            .copied()
            .unwrap_or(0.0)
    }

    fn update(&mut self, store: &BestEpisodeStore) -> Option<f32> {
        const SAMPLE_SIZE: usize = 500;
        const BATCH_SIZE: usize = 64;
        const EPOCHS: usize = 4;

        let mut episodes: Vec<(Vec<usize>, Vec<f32>, f32)> = store
            .iter_funcs()
            .flat_map(|(_, eps)| eps.iter().map(|e| (e.actions.clone(), e.ir_features.clone(), e.g0)))
            .collect();

        if episodes.is_empty() {
            return None;
        }

        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        episodes.shuffle(&mut rng);
        episodes.truncate(SAMPLE_SIZE);

        let max_len = episodes.iter().map(|(a, _, _)| a.len()).max().unwrap_or(1);
        let total = episodes.len();
        let device = self.model.as_ref().unwrap().ir_proj.weight.device();

        let mut action_buf = vec![0i64; total * max_len];
        let mut ir_buf = vec![0.0f32; total * self.ir_dim];
        let mut target_buf = vec![0.0f32; total];

        for (i, (actions, ir, g0)) in episodes.iter().enumerate() {
            for (t, &a) in actions.iter().enumerate() {
                action_buf[i * max_len + t] = a as i64;
            }
            let src_len = ir.len().min(self.ir_dim);
            ir_buf[i * self.ir_dim..i * self.ir_dim + src_len].copy_from_slice(&ir[..src_len]);
            target_buf[i] = *g0;
        }

        let actions_full = Tensor::<B, 2, Int>::from_data(
            TensorData::new(action_buf, [total, max_len]),
            &device,
        );
        let ir_full = Tensor::<B, 2>::from_data(
            TensorData::new(ir_buf, [total, self.ir_dim]),
            &device,
        );
        let targets_full = Tensor::<B, 1>::from_data(
            TensorData::new(target_buf, [total]),
            &device,
        );

        let mut indices: Vec<usize> = (0..total).collect();
        indices.shuffle(&mut rng);

        let mut total_loss = 0.0f32;
        let mut n_batches = 0;

        for _ in 0..EPOCHS {
            for chunk in indices.chunks(BATCH_SIZE) {
                let batch = chunk.len();
                let idx = Tensor::<B, 1, Int>::from_data(
                    TensorData::new(chunk.iter().map(|&i| i as i64).collect::<Vec<_>>(), [batch]),
                    &device,
                );

                let actions = actions_full.clone().select(0, idx.clone());
                let ir = ir_full.clone().select(0, idx.clone());
                let targets = targets_full.clone().select(0, idx);

                let model = self.model.take().unwrap();
                let predicted = model.forward(actions, ir);
                let loss = (predicted - targets).powf_scalar(2.0).mean();

                let l: f32 = loss.clone().into_scalar().elem();
                total_loss += l;
                n_batches += 1;

                let grads = loss.backward();
                let grad_params = GradientsParams::from_grads(grads, &model);
                let model = self.optim.step(self.lr, model, grad_params);
                self.model = Some(model);
            }
        }

        Some(total_loss / (n_batches as f32))
    }

    fn name(&self) -> &str {
        "transformer"
    }
}
