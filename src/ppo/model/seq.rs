use crate::config::{BurnAutoDiff, Cfg};
use crate::ppo::model::{Actor, Input, MlpHead, MlpHeadConfig, Output};
use burn::nn::transformer::{TransformerEncoder, TransformerEncoderConfig, TransformerEncoderInput};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::prelude::{Backend, Bool, Config, Int, Module};
use burn::tensor::Tensor;

#[derive(Config, Debug)]
pub(crate) struct SeqActorConfig {
    #[config(default = 34)]
    pub(crate) input_dim: usize,
    #[config(default = 29)]
    pub(crate) num_actions: usize,
    #[config(default = 256)]
    pub(crate) d_model: usize,
    #[config(default = 4)]
    pub(crate) n_heads: usize,
    #[config(default = 3)]
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

/// Sequence actor with causal attention.
///
/// Tokens: [IR_token, slot_0, slot_1, ..., slot_{K-1}]
///
/// IR token (position 0): ir_proj(ir_features[0])
/// Slot token t (position t+1): slot_embed(t)
///
/// Causal mask: position i can only attend to positions 0..=i.
/// This means:
///   - IR token sees only itself  → value head reads pure IR representation
///   - slot_t sees IR token + slots 0..t → policy conditioned on IR + prior slots
///   - slot_{t+1..K-1} CANNOT reach slot_t → no gradient corruption from untrained late slots
///
/// Prefix-independence: slot_t's output is identical whether the sequence has
/// K or ep_len tokens (as long as ep_len > t). PPO can therefore run with
/// K=ep_len and get exactly the same distributions as collection used for those slots.
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
        let k      = input.ir_features.dims()[0];
        let device = input.ir_features.device();
        let seq    = k + 1; // IR token + K slot tokens

        // IR token: project first row of ir_features (all rows identical).
        let ir_feat = input.ir_features.narrow(0, 0, 1);            // [1, input_dim]
        let ir_emb  = self.ir_proj.forward(ir_feat);                 // [1, d_model]

        // Slot tokens: embed positions 0..K.
        let slot_ids  = Tensor::<B, 1, Int>::arange(0..k as i64, &device)
            .unsqueeze_dim(0);                                        // [1, K]
        let slot_emb  = self.slot_embed.forward(slot_ids);            // [1, K, d_model]

        // Sequence: [IR_token | slot_0 .. slot_{K-1}], shape [1, K+1, d_model]
        let ir_tok = ir_emb.unsqueeze_dim(0);                         // [1, 1, d_model]
        let tokens = Tensor::cat(vec![ir_tok, slot_emb], 1);          // [1, K+1, d_model]

        // Causal attention mask: position i cannot attend to position j > i.
        // Shape [1, seq, seq]. true = masked (blocked).
        let mask_data: Vec<bool> = (0..seq)
            .flat_map(|i| (0..seq).map(move |j| j > i))
            .collect();
        let causal_mask = Tensor::<B, 3, Bool>::from_data(
            burn::tensor::TensorData::new(mask_data, [1, seq, seq]),
            &device,
        );

        let enc_input = TransformerEncoderInput::new(tokens).mask_attn(causal_mask);
        let out = self.transformer.forward(enc_input);                // [1, K+1, d_model]

        // Value: from IR token (position 0) — pure IR representation, no slot leakage.
        let ir_out    = out.clone().narrow(1, 0, 1).flatten::<2>(1, 2); // [1, d_model]
        let value_one = self.value_head.forward(ir_out);                 // [1, 1]
        let value     = Tensor::<B, 2>::ones([k, 1], &device) * value_one; // [K, 1]

        // Policy: from slot token outputs (positions 1..K+1).
        let slot_out = out.narrow(1, 1, k).flatten::<2>(0, 1);           // [K, d_model]
        let policy   = self.policy_head.forward(slot_out).unsqueeze_dim(1); // [K, 1, num_actions]

        Output { policy, value }
    }

    fn cfg(cfg: &Cfg) -> Self::Config {
        SeqActorConfig::new().with_max_seq_len(cfg.max_seq_len)
    }
}
