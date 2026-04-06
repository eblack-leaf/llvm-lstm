use crate::config::{BurnAutoDiff, Cfg};
use crate::ppo::model::{Actor, Input, MlpHead, MlpHeadConfig, Output};
use burn::nn::transformer::{TransformerEncoder, TransformerEncoderConfig, TransformerEncoderInput};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::prelude::{Backend, Config, Int, Module};
use burn::tensor::Tensor;

#[derive(Config, Debug)]
pub(crate) struct SeqActorConfig {
    /// Dimensionality of the base IR feature vector.
    #[config(default = 34)]
    pub(crate) input_dim: usize,
    #[config(default = 29)]
    pub(crate) num_actions: usize,
    #[config(default = 256)]
    pub(crate) d_model: usize,
    #[config(default = 4)]
    pub(crate) n_heads: usize,
    #[config(default = 2)]
    pub(crate) n_layers: usize,
    #[config(default = 512)]
    pub(crate) d_ff: usize,
    #[config(default = 0.1)]
    pub(crate) dropout: f64,
    /// Slot embedding table size — must be >= max_seq_len.
    #[config(default = 40)]
    pub(crate) max_seq_len: usize,
    #[config(default = 128)]
    pub(crate) head_hidden: usize,
}

/// Sequence actor: one forward pass over all K output positions from the base IR.
///
/// Architecture:
///   ir_tok   = ir_proj(ir_features[0])            [1, d_model]   ← IR context token
///   slot_tok = slot_embed([0..K])                 [1, K, d_model]
///   tokens   = cat([ir_tok, slot_tok], dim=1)     [1, K+1, d_model]
///   out      = transformer_encoder(tokens)         [1, K+1, d_model]
///
///   policy   = policy_head(out[:, 1:, :])         [K, 1, num_actions]
///   value    = value_head(out[:, 0, :])           [K, 1]   ← IR token, same for all slots
///
/// All K slots attend to each other and to the IR token, capturing pass-ordering
/// dependencies that the independent-slot approach cannot model.
#[derive(Module, Debug)]
pub(crate) struct SeqActor<B: Backend> {
    ir_proj:     Linear<B>,
    slot_embed:  Embedding<B>,
    transformer: TransformerEncoder<B>,
    policy_head: MlpHead<B>,
    value_head:  MlpHead<B>,
}

impl<B: Backend> Actor<B> for SeqActor<B> {
    type Config = SeqActorConfig;

    fn init(cfg: Self::Config, device: &B::Device) -> Self {
        Self {
            ir_proj:     LinearConfig::new(cfg.input_dim, cfg.d_model).init(device),
            slot_embed:  EmbeddingConfig::new(cfg.max_seq_len, cfg.d_model).init(device),
            transformer: TransformerEncoderConfig::new(
                cfg.d_model,
                cfg.d_ff,
                cfg.n_heads,
                cfg.n_layers,
            )
            .with_dropout(cfg.dropout)
            .init(device),
            policy_head: MlpHeadConfig::new(cfg.d_model, cfg.head_hidden, cfg.num_actions)
                .init(device),
            value_head:  MlpHeadConfig::new(cfg.d_model, cfg.head_hidden, 1)
                .init(device),
        }
    }

    fn forward(&self, _cfg: &Cfg, input: Input<B>) -> Output<B> {
        let k = input.slot_idx.dims()[0];

        // Project the base IR features to d_model. All K rows are identical
        // (same episode), so we take the first row as the IR context token.
        let ir_feat = input.ir_features.narrow(0, 0, 1);       // [1, 34]
        let ir_emb  = self.ir_proj.forward(ir_feat);            // [1, d_model]

        // Slot embeddings: [K] indices → [1, K, d_model]
        let slot_emb = self.slot_embed.forward(input.slot_idx.unsqueeze_dim(0)); // [1, K, d_model]

        // Prepend IR token: [1, 1, d_model] cat [1, K, d_model] = [1, K+1, d_model]
        let ir_tok = ir_emb.unsqueeze_dim(0);                   // [1, 1, d_model]
        let tokens = Tensor::cat(vec![ir_tok, slot_emb], 1);    // [1, K+1, d_model]

        // Transformer encoder: all slots attend to each other and to the IR token.
        let enc_input = TransformerEncoderInput::new(tokens);
        let out = self.transformer.forward(enc_input);           // [1, K+1, d_model]

        // Value: from the IR token (position 0), slot-independent.
        let ir_out    = out.clone().narrow(1, 0, 1).flatten::<2>(1, 2); // [1, d_model]
        let value_one = self.value_head.forward(ir_out);                // [1, 1]

        // Broadcast V(IR) to [K, 1] — same scalar for all K slots.
        // Gradient accumulates across K steps back to the single value estimate.
        let device = value_one.device();
        let ones  = Tensor::<B, 2>::ones([k, 1], &device);
        let value = ones * value_one;                                    // [K, 1]

        // Policy: from slot token outputs (positions 1..K+1).
        let slot_out = out.narrow(1, 1, k).flatten::<2>(0, 1);          // [K, d_model]
        let policy   = self.policy_head.forward(slot_out).unsqueeze_dim(1); // [K, 1, num_actions]

        Output { policy, value }
    }

    fn cfg(cfg: &Cfg) -> Self::Config {
        SeqActorConfig::new().with_max_seq_len(cfg.max_seq_len)
    }
}
