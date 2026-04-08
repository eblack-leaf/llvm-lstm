use crate::config::Cfg;
use crate::llvm::ir::IR_CATEGORY_COUNT;
use crate::ppo::model::{Actor, Input, MlpHead, MlpHeadConfig, Output};
use burn::nn::transformer::{
    TransformerEncoder, TransformerEncoderConfig, TransformerEncoderInput,
};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::prelude::{Backend, Bool, Config, Int, Module};
use burn::tensor::Tensor;

#[derive(Config, Debug)]
pub(crate) struct SeqActorConfig {
    /// Number of IR histogram chunks — feature dim = ir_chunks * IR_VOCAB_SIZE.
    #[config(default = 4)]
    pub(crate) ir_chunks: usize,
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
    #[config(default = 40)]
    pub(crate) max_seq_len: usize,
    #[config(default = 128)]
    pub(crate) head_hidden: usize,
}

/// Sequence actor with causal attention.
///
/// IR input: chunked opcode histogram [N, ir_chunks * 64] → Linear → [N, 1, d_model]
/// → prepended as a single IR token to the slot sequence.
///
/// Tokens: [IR_token, slot_0, slot_1, ..., slot_{K-1}]
/// Causal mask: position i can only attend to positions 0..=i.
#[derive(Module, Debug)]
pub(crate) struct SeqActor<B: Backend> {
    ir_proj: Linear<B>,
    slot_embed: Embedding<B>,
    transformer: TransformerEncoder<B>,
    policy_head: MlpHead<B>,
    value_head: MlpHead<B>,
}

impl<B: Backend> Actor<B> for SeqActor<B> {
    type Config = SeqActorConfig;

    fn init(cfg: Self::Config, device: &B::Device) -> Self {
        let ir_feature_dim = cfg.ir_chunks * IR_CATEGORY_COUNT;
        Self {
            ir_proj: LinearConfig::new(ir_feature_dim, cfg.d_model).init(device),
            slot_embed: EmbeddingConfig::new(cfg.max_seq_len, cfg.d_model).init(device),
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
            value_head: MlpHeadConfig::new(cfg.d_model, cfg.head_hidden, 1).init(device),
        }
    }

    fn forward(&self, cfg: &Cfg, input: Input<B>) -> Output<B> {
        let n = input.ir_features.dims()[0];
        let k = cfg.max_seq_len;
        let device = input.ir_features.device();
        let seq = k + 1; // IR token + K slot tokens

        // IR histogram → single prepended IR token.
        let ir_emb = self.ir_proj.forward(input.ir_features).unsqueeze_dim(1); // [N, 1, d_model]

        // Slot tokens.
        let slot_ids = Tensor::<B, 1, Int>::arange(0..k as i64, &device).unsqueeze_dim(0);
        let slot_emb = self.slot_embed.forward(slot_ids).repeat(&[n, 1, 1]); // [N, K, d_model]

        // Sequence: [IR_token | slot_0 .. slot_{K-1}]
        let tokens = Tensor::cat(vec![ir_emb, slot_emb], 1); // [N, K+1, d_model]

        // Causal mask.
        let mask_data: Vec<bool> = (0..seq)
            .flat_map(|i| (0..seq).map(move |j| j > i))
            .collect();
        let causal_mask = Tensor::<B, 3, Bool>::from_data(
            burn::tensor::TensorData::new(mask_data, [1, seq, seq]),
            &device,
        );

        let enc_input = TransformerEncoderInput::new(tokens).mask_attn(causal_mask);
        let out = self.transformer.forward(enc_input); // [N, K+1, d_model]

        let d_model = out.dims()[2];
        let slot_out = out.narrow(1, 1, k).reshape([n * k, d_model]);
        let policy_flat = self.policy_head.forward(slot_out.clone()); // [N*K, num_actions]
        let value_flat = self.value_head.forward(slot_out); // [N*K, 1]
        let num_actions = policy_flat.dims()[1];
        let policy = policy_flat.reshape([n, k, num_actions]).unsqueeze_dim(2); // [N, K, 1, num_actions]
        let value = value_flat.reshape([n, k, 1]); // [N, K, 1]

        Output { policy, value }
    }

    fn cfg(cfg: &Cfg) -> Self::Config {
        SeqActorConfig::new()
            .with_max_seq_len(cfg.max_seq_len)
            .with_ir_chunks(cfg.ir_chunks)
    }
}
