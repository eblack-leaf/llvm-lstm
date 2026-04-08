use crate::llvm::ir::IR_VOCAB_SIZE;
use burn::config::Config;
use burn::module::Module;
use burn::nn::conv::{Conv1d, Conv1dConfig};
use burn::nn::transformer::{
    TransformerEncoder, TransformerEncoderConfig, TransformerEncoderInput,
};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::prelude::{Backend, Tensor};
use burn::tensor::Bool;
use burn::tensor::Int;
use burn::tensor::TensorData;

#[derive(Module, Debug)]
pub struct SpeedupPredictor<B: Backend> {
    ir_opcode_embed: Embedding<B>,
    ir_conv: Conv1d<B>,
    ir_encoder: TransformerEncoder<B>,
    ir_proj: Linear<B>,
    pass_embed: Embedding<B>,
    delta_proj: Linear<B>,
    pos_embed: Embedding<B>,
    transformer: TransformerEncoder<B>,
    output_head: Linear<B>,
}

#[derive(Config, Debug)]
pub struct SpeedupPredictorConfig {
    pub num_passes: usize,
    /// Opcode vocabulary size — must match IR_VOCAB_SIZE (64).
    #[config(default = 64)]
    pub ir_vocab_size: usize,
    /// Hidden dim of the small IR encoder.
    #[config(default = 64)]
    pub d_ir: usize,
    /// Layers in the IR encoder.
    #[config(default = 2)]
    pub ir_n_layers: usize,
    /// Attention heads in the IR encoder (d_ir must be divisible by this).
    #[config(default = 4)]
    pub ir_n_heads: usize,
    /// Feed-forward dim in the IR encoder.
    #[config(default = 128)]
    pub ir_d_ff: usize,
    /// Max IR opcode sequence length (shorter sequences are padded).
    #[config(default = 512)]
    pub max_ir_len: usize,
    /// Conv1D stride that compresses the opcode sequence before the IR encoder.
    /// The IR encoder sees max_ir_len / ir_conv_stride tokens.
    #[config(default = 4)]
    pub ir_conv_stride: usize,
    pub output_dim: usize,
    pub d_model: usize,
    pub n_heads: usize,
    pub n_layers: usize,
    pub d_ff: usize,
    pub dropout: f64,
    pub max_seq_len: usize,
}

impl SpeedupPredictorConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> SpeedupPredictor<B> {
        let max_positions = self.max_seq_len + 1;
        SpeedupPredictor {
            ir_opcode_embed: EmbeddingConfig::new(self.ir_vocab_size, self.d_ir).init(device),
            ir_conv: Conv1dConfig::new(self.d_ir, self.d_ir, self.ir_conv_stride)
                .with_stride(self.ir_conv_stride)
                .init(device),
            ir_encoder: TransformerEncoderConfig::new(
                self.d_ir,
                self.ir_d_ff,
                self.ir_n_heads,
                self.ir_n_layers,
            )
            .with_dropout(self.dropout)
            .init(device),
            ir_proj: LinearConfig::new(self.d_ir, self.d_model).init(device),
            pass_embed: EmbeddingConfig::new(self.num_passes, self.d_model).init(device),
            delta_proj: LinearConfig::new(1, self.d_model)
                .with_bias(false)
                .init(device),
            pos_embed: EmbeddingConfig::new(max_positions, self.d_model).init(device),
            transformer: TransformerEncoderConfig::new(
                self.d_model,
                self.d_ff,
                self.n_heads,
                self.n_layers,
            )
            .with_dropout(self.dropout)
            .init(device),
            output_head: LinearConfig::new(self.d_model, self.output_dim).init(device),
        }
    }
}

impl<B: Backend> SpeedupPredictor<B> {
    pub fn forward(
        &self,
        ir_opcodes: Tensor<B, 2, Int>,       // [batch, max_ir_len]
        ir_padding_mask: Tensor<B, 2, Bool>, // [batch, max_ir_len] true=PAD
        passes: Tensor<B, 2, Int>,           // [batch, seq_len]
        mask: Tensor<B, 2, Bool>,            // [batch, seq_len] true = valid
        step_deltas: Tensor<B, 2>,           // [batch, seq_len]
    ) -> Tensor<B, 2> {
        let batch_size = passes.dims()[0];
        let seq_len = passes.dims()[1];
        let device = passes.device();

        // --- IR encoder with Conv1D downsampling ---
        let ir_embed = self.ir_opcode_embed.forward(ir_opcodes); // [batch, L, d_ir]
        let l = ir_embed.dims()[1];

        // Conv1D: [batch, L, d_ir] → [batch, d_ir, L] → conv → [batch, d_ir, L/s] → [batch, L/s, d_ir]
        let ir_conv_in = ir_embed.swap_dims(1, 2);
        let ir_conv_out = self.ir_conv.forward(ir_conv_in);
        let l_pooled = ir_conv_out.dims()[2];
        let ir_down = ir_conv_out.swap_dims(1, 2); // [batch, L/s, d_ir]

        // Pool padding mask.
        let stride = l / l_pooled;
        let not_pad_f = ir_padding_mask.float().neg() + 1.0f32; // [batch, L], 1=real
        let pooled_mask: Tensor<B, 2, Bool> = not_pad_f
            .reshape([batch_size, l_pooled, stride])
            .sum_dim(2)
            .squeeze_dim(2)
            .lower_elem(0.5f32); // true=PAD

        let ir_enc_input = TransformerEncoderInput::new(ir_down).mask_pad(pooled_mask.clone());
        let ir_enc = self.ir_encoder.forward(ir_enc_input); // [batch, L/s, d_ir]

        // Masked mean pool.
        let not_pad = pooled_mask.float().neg() + 1.0f32; // [batch, L/s], 1=real
        let counts = not_pad.clone().sum_dim(1).unsqueeze_dim(2).clamp_min(1.0);
        let weighted_sum = (ir_enc * not_pad.unsqueeze_dim(2)).sum_dim(1);
        let d_ir = weighted_sum.dims()[2];
        let ir_mean = (weighted_sum / counts).reshape([batch_size, d_ir]); // [batch, d_ir]
        let ir_token = self.ir_proj.forward(ir_mean).unsqueeze_dim(1); // [batch, 1, d_model]

        // Pass embeddings + per-step delta signal.
        let pass_embeds = self.pass_embed.forward(passes);
        let delta_embeds = self.delta_proj.forward(step_deltas.unsqueeze_dim(2));
        let pass_embeds = pass_embeds + delta_embeds; // [batch, seq_len, d_model]

        // Positional embeddings.
        let positions = Tensor::<B, 1, Int>::arange(0..(1 + seq_len) as i64, &device)
            .unsqueeze_dim(0)
            .repeat(&[batch_size, 1]);
        let pos_embeds = self.pos_embed.forward(positions); // [batch, 1+seq_len, d_model]

        // Full sequence: [IR_token | pass_0 .. pass_{seq_len-1}].
        let tokens = Tensor::cat(vec![ir_token, pass_embeds], 1) + pos_embeds;

        // Padding mask: true = ignore. IR token is never masked; pass positions use `mask`.
        let pad_mask = mask.equal_elem(false); // [batch, seq_len], true where padding
        let ir_pad = Tensor::<B, 2, Bool>::from_data(
            TensorData::new(vec![false; batch_size], [batch_size, 1]),
            &device,
        );
        let full_pad_mask = Tensor::cat(vec![ir_pad, pad_mask], 1); // [batch, 1+seq_len]

        let encoder_input = TransformerEncoderInput::new(tokens).mask_pad(full_pad_mask);
        let encoded = self.transformer.forward(encoder_input); // [batch, 1+seq_len, d_model]

        // CLS token output → regression head.
        let cls_output = encoded.narrow(1, 0, 1).squeeze_dim(1); // [batch, d_model]
        self.output_head.forward(cls_output)
    }
}
