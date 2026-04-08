use crate::config::Cfg;
use crate::llvm::ir::IR_VOCAB_SIZE;
use crate::ppo::model::{Actor, Input, MlpHead, MlpHeadConfig, Output};
use burn::nn::transformer::{
    TransformerEncoder, TransformerEncoderConfig, TransformerEncoderInput,
};
use burn::nn::conv::{Conv1d, Conv1dConfig};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::prelude::{Backend, Bool, Config, Int, Module};
use burn::tensor::Tensor;

#[derive(Config, Debug)]
pub(crate) struct SeqActorConfig {
    /// Opcode vocabulary size — must match IR_VOCAB_SIZE.
    #[config(default = 64)]
    pub(crate) ir_vocab_size: usize,
    /// Hidden dim of the small IR encoder.
    #[config(default = 64)]
    pub(crate) d_ir: usize,
    /// Number of transformer layers in the IR encoder.
    #[config(default = 2)]
    pub(crate) ir_n_layers: usize,
    /// Number of attention heads in the IR encoder.
    #[config(default = 4)]
    pub(crate) ir_n_heads: usize,
    /// Feed-forward dim inside the IR encoder.
    #[config(default = 128)]
    pub(crate) ir_d_ff: usize,
    /// Max IR opcode sequence length (must match Cfg::max_ir_len).
    #[config(default = 512)]
    pub(crate) max_ir_len: usize,
    /// Conv1D stride used to compress the opcode sequence before the IR encoder.
    #[config(default = 4)]
    pub(crate) ir_conv_stride: usize,
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
/// IR input: opcode-ID sequence [N, max_ir_len] → small IR encoder → mean pool → [N, d_model]
/// → prepended as a single IR token to the slot sequence.
///
/// Tokens: [IR_token, slot_0, slot_1, ..., slot_{K-1}]
/// Causal mask: position i can only attend to positions 0..=i.
#[derive(Module, Debug)]
pub(crate) struct SeqActor<B: Backend> {
    ir_opcode_embed: Embedding<B>,
    ir_conv: Conv1d<B>,
    ir_encoder: TransformerEncoder<B>,
    ir_proj: Linear<B>,
    slot_embed: Embedding<B>,
    transformer: TransformerEncoder<B>,
    policy_head: MlpHead<B>,
    value_head: MlpHead<B>,
}

impl<B: Backend> Actor<B> for SeqActor<B> {
    type Config = SeqActorConfig;

    fn init(cfg: Self::Config, device: &B::Device) -> Self {
        Self {
            ir_opcode_embed: EmbeddingConfig::new(cfg.ir_vocab_size, cfg.d_ir).init(device),
            ir_conv: Conv1dConfig::new(cfg.d_ir, cfg.d_ir, cfg.ir_conv_stride)
                .with_stride(cfg.ir_conv_stride)
                .init(device),
            ir_encoder: TransformerEncoderConfig::new(cfg.d_ir, cfg.ir_d_ff, cfg.ir_n_heads, cfg.ir_n_layers)
                .with_dropout(cfg.dropout)
                .init(device),
            ir_proj: LinearConfig::new(cfg.d_ir, cfg.d_model).init(device),
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
        let n = input.ir_opcodes.dims()[0];
        let k = cfg.max_seq_len;
        let device = input.ir_opcodes.device();
        let seq = k + 1; // IR token + K slot tokens

        // --- IR encoder with Conv1D downsampling ---
        let ir_embed = self.ir_opcode_embed.forward(input.ir_opcodes); // [N, L, d_ir]
        let l = ir_embed.dims()[1];

        // Conv1D: [N, L, d_ir] → [N, d_ir, L] → conv → [N, d_ir, L/s] → [N, L/s, d_ir]
        let ir_conv_in = ir_embed.swap_dims(1, 2);
        let ir_conv_out = self.ir_conv.forward(ir_conv_in);
        let l_pooled = ir_conv_out.dims()[2];
        let ir_down = ir_conv_out.swap_dims(1, 2); // [N, L/s, d_ir]

        // Pool padding mask.
        let stride = l / l_pooled;
        let not_pad_f = input.ir_padding_mask.float().neg() + 1.0f32; // [N, L], 1=real
        let pooled_mask: Tensor<B, 2, Bool> = not_pad_f
            .reshape([n, l_pooled, stride])
            .sum_dim(2)
            .squeeze_dim(2)
            .lower_elem(0.5f32); // true=PAD

        let ir_enc_input = TransformerEncoderInput::new(ir_down).mask_pad(pooled_mask.clone());
        let ir_enc = self.ir_encoder.forward(ir_enc_input); // [N, L/s, d_ir]

        let not_pad = pooled_mask.float().neg() + 1.0f32; // [N, L/s], 1=real
        let counts = not_pad.clone().sum_dim(1).unsqueeze_dim(2).clamp_min(1.0);
        let weighted_sum = (ir_enc * not_pad.unsqueeze_dim(2)).sum_dim(1);
        let d_ir = weighted_sum.dims()[2];
        let ir_mean = (weighted_sum / counts).reshape([n, d_ir]); // [N, d_ir]

        // Project to d_model, use as the single prepended IR token.
        let ir_emb = self.ir_proj.forward(ir_mean).unsqueeze_dim(1); // [N, 1, d_model]

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
        let value_flat = self.value_head.forward(slot_out);           // [N*K, 1]
        let num_actions = policy_flat.dims()[1];
        let policy = policy_flat.reshape([n, k, num_actions]).unsqueeze_dim(2); // [N, K, 1, num_actions]
        let value = value_flat.reshape([n, k, 1]);                               // [N, K, 1]

        Output { policy, value }
    }

    fn cfg(cfg: &Cfg) -> Self::Config {
        SeqActorConfig::new()
            .with_max_seq_len(cfg.max_seq_len)
            .with_max_ir_len(cfg.max_ir_len)
            .with_ir_conv_stride(cfg.ir_conv_stride)
            .with_ir_vocab_size(IR_VOCAB_SIZE)
    }
}
