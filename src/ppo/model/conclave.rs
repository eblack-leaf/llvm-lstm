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
pub(crate) struct ConclaveActorConfig {
    /// Opcode vocabulary size — must match IR_VOCAB_SIZE.
    #[config(default = 64)]
    pub(crate) ir_vocab_size: usize,
    /// Hidden dim of the small IR encoder (projects up to d_model via ir_proj).
    #[config(default = 64)]
    pub(crate) d_ir: usize,
    /// Number of transformer layers in the IR encoder.
    #[config(default = 2)]
    pub(crate) ir_n_layers: usize,
    /// Number of attention heads in the IR encoder (d_ir must be divisible by this).
    #[config(default = 4)]
    pub(crate) ir_n_heads: usize,
    /// Feed-forward dim inside the IR encoder.
    #[config(default = 128)]
    pub(crate) ir_d_ff: usize,
    /// Max IR opcode sequence length (must match Cfg::max_ir_len).
    #[config(default = 512)]
    pub(crate) max_ir_len: usize,
    /// Conv1D stride used to compress the opcode sequence before the IR encoder.
    /// The transformer sees max_ir_len / ir_conv_stride tokens.
    /// Must divide max_ir_len evenly.
    #[config(default = 4)]
    pub(crate) ir_conv_stride: usize,
    #[config(default = 29)]
    pub(crate) num_passes: usize,
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

/// ConclaveActor — passes convene with full knowledge of the slot ordering context,
/// slots listen to the resolved pass deliberation without coordinating with each other.
///
/// IR input: opcode-ID sequence [N, max_ir_len] → small IR encoder → mean pool → [N, d_model].
///
/// Joint sequence: [pass_0 .. pass_28 | slot_0 .. slot_{K-1}]
///
/// Attention mask:
///   pass → pass : open   — passes deliberate bidirectionally, all seeing all
///   pass → slot : open   — passes know which positions are asking
///   slot → pass : open   — slots absorb the full pass deliberation
///   slot → slot : closed — slots don't coordinate; each reads the conclave independently
#[derive(Module, Debug)]
pub(crate) struct ConclaveActor<B: Backend> {
    ir_opcode_embed: Embedding<B>,
    ir_conv: Conv1d<B>,
    ir_encoder: TransformerEncoder<B>,
    ir_proj: Linear<B>,
    pass_embed: Embedding<B>,
    slot_embed: Embedding<B>,
    transformer: TransformerEncoder<B>,
    policy_head: MlpHead<B>,
    value_head: MlpHead<B>,
}

impl<B: Backend> Actor<B> for ConclaveActor<B> {
    type Config = ConclaveActorConfig;

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
            pass_embed: EmbeddingConfig::new(cfg.num_passes, cfg.d_model).init(device),
            slot_embed: EmbeddingConfig::new(cfg.max_seq_len, cfg.d_model).init(device),
            transformer: TransformerEncoderConfig::new(
                cfg.d_model,
                cfg.d_ff,
                cfg.n_heads,
                cfg.n_layers,
            )
            .with_dropout(cfg.dropout)
            .init(device),
            policy_head: MlpHeadConfig::new(cfg.d_model, cfg.head_hidden, cfg.num_passes)
                .init(device),
            value_head: MlpHeadConfig::new(cfg.d_model, cfg.head_hidden, 1).init(device),
        }
    }

    fn forward(&self, cfg: &Cfg, input: Input<B>) -> Output<B> {
        let n = input.ir_opcodes.dims()[0];
        let max_ir_len = input.ir_opcodes.dims()[1];
        let k = cfg.max_seq_len;
        let device = input.ir_opcodes.device();
        let np = 29usize;
        let seq = np + k;

        // --- IR encoder with Conv1D downsampling ---
        // Embed: [N, L] → [N, L, d_ir]
        let ir_embed = self.ir_opcode_embed.forward(input.ir_opcodes);
        let l = ir_embed.dims()[1];

        // Conv1D: [N, L, d_ir] → swap → [N, d_ir, L] → conv → [N, d_ir, L/s] → swap → [N, L/s, d_ir]
        let ir_conv_in = ir_embed.swap_dims(1, 2);
        let ir_conv_out = self.ir_conv.forward(ir_conv_in);
        let l_pooled = ir_conv_out.dims()[2];
        let ir_down = ir_conv_out.swap_dims(1, 2); // [N, L/s, d_ir]

        // Pool padding mask: a window is PAD only if all its tokens are PAD.
        let stride = l / l_pooled;
        let not_pad_f = input.ir_padding_mask.float().neg() + 1.0f32; // [N, L], 1=real
        let pooled_mask: Tensor<B, 2, Bool> = not_pad_f
            .reshape([n, l_pooled, stride])
            .sum_dim(2)
            .squeeze_dim(2)
            .lower_elem(0.5f32); // true=PAD (all tokens in window are PAD)

        // IR encoder on downsampled sequence.
        let ir_enc_input = TransformerEncoderInput::new(ir_down).mask_pad(pooled_mask.clone());
        let ir_enc = self.ir_encoder.forward(ir_enc_input); // [N, L/s, d_ir]

        // Masked mean pool over real tokens.
        let not_pad = pooled_mask.float().neg() + 1.0f32; // [N, L/s], 1=real
        let counts = not_pad.clone().sum_dim(1).unsqueeze_dim(2).clamp_min(1.0); // [N, 1, 1]
        let weighted_sum = (ir_enc * not_pad.unsqueeze_dim(2)).sum_dim(1); // [N, 1, d_ir]
        let d_ir = weighted_sum.dims()[2];
        let ir_mean = (weighted_sum / counts).reshape([n, d_ir]); // [N, d_ir]
        let ir_emb = self.ir_proj.forward(ir_mean); // [N, d_model]

        // Pass nodes: learned pass identity + IR conditioning, broadcast over batch.
        let pass_ids = Tensor::<B, 1, Int>::arange(0..np as i64, &device).unsqueeze_dim(0); // [1, np]
        let pass_emb = self.pass_embed.forward(pass_ids)     // [1, np, d_model]
            .repeat(&[n, 1, 1])                               // [N, np, d_model]
            + ir_emb.clone().unsqueeze_dim(1).repeat(&[1, np, 1]);

        // Slot nodes: positional identity.
        let slot_ids = Tensor::<B, 1, Int>::arange(0..k as i64, &device).unsqueeze_dim(0);
        let slot_emb = self.slot_embed.forward(slot_ids).repeat(&[n, 1, 1]); // [N, K, d_model]

        // Joint sequence: [pass_0..pass_28 | slot_0..slot_{K-1}]
        let tokens = Tensor::cat(vec![pass_emb, slot_emb], 1); // [N, np+K, d_model]

        // Attention mask: slot→slot blocked (except self).
        let mask_data: Vec<bool> = (0..seq)
            .flat_map(|i| {
                (0..seq).map(move |j| {
                    let i_is_slot = i >= np;
                    let j_is_slot = j >= np;
                    i_is_slot && j_is_slot && i != j
                })
            })
            .collect();
        let mask = Tensor::<B, 3, Bool>::from_data(
            burn::tensor::TensorData::new(mask_data, [1, seq, seq]),
            &device,
        );

        let enc_input = TransformerEncoderInput::new(tokens).mask_attn(mask);
        let out = self.transformer.forward(enc_input); // [N, np+K, d_model]

        let d_model = out.dims()[2];
        let slot_out = out.narrow(1, np, k).reshape([n * k, d_model]);
        let policy_flat = self.policy_head.forward(slot_out.clone()); // [N*K, num_passes]
        let value_flat = self.value_head.forward(slot_out);           // [N*K, 1]
        let num_actions = policy_flat.dims()[1];
        let policy = policy_flat.reshape([n, k, num_actions]).unsqueeze_dim(2); // [N, K, 1, num_passes]
        let value = value_flat.reshape([n, k, 1]);                               // [N, K, 1]

        Output { policy, value }
    }

    fn cfg(cfg: &Cfg) -> Self::Config {
        ConclaveActorConfig::new()
            .with_max_seq_len(cfg.max_seq_len)
            .with_max_ir_len(cfg.max_ir_len)
            .with_ir_conv_stride(cfg.ir_conv_stride)
            .with_ir_vocab_size(IR_VOCAB_SIZE)
    }
}
