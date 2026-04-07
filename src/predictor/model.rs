use burn::config::Config;
use burn::module::Module;
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
    pass_embed: Embedding<B>,
    ir_proj: Linear<B>,
    delta_proj: Linear<B>,
    pos_embed: Embedding<B>,
    transformer: TransformerEncoder<B>,
    output_head: Linear<B>,
}

#[derive(Config, Debug)]
pub struct SpeedupPredictorConfig {
    pub num_passes: usize,
    pub ir_feature_dim: usize,
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
            pass_embed: EmbeddingConfig::new(self.num_passes, self.d_model).init(device),
            ir_proj: LinearConfig::new(self.ir_feature_dim, self.d_model).init(device),
            delta_proj: LinearConfig::new(1, self.d_model).with_bias(false).init(device),
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
        ir_features: Tensor<B, 2>,             // [batch, ir_dim]
        passes: Tensor<B, 2, Int>,             // [batch, seq_len]
        mask: Tensor<B, 2, Bool>,              // [batch, seq_len] true = valid
        step_deltas: Tensor<B, 2>,             // [batch, seq_len] normalised instr-count delta per step
    ) -> Tensor<B, 2> {
        let batch_size = passes.dims()[0];
        let seq_len = passes.dims()[1];
        let device = passes.device();

        // 1. Embed passes and add per-step instruction-delta signal: [batch, seq_len, d_model]
        let pass_embeds = self.pass_embed.forward(passes);
        let delta_embeds = self.delta_proj.forward(step_deltas.unsqueeze_dim(2)); // [batch, seq_len, d_model]
        let pass_embeds = pass_embeds + delta_embeds;

        // 2. Project IR features: [batch, d_model] -> [batch, 1, d_model]
        let ir_token = self.ir_proj.forward(ir_features).unsqueeze_dim(1);

        // 3. Positional embeddings for all positions (IR token + passes)
        let positions = Tensor::<B, 1, Int>::arange(0..(1 + seq_len) as i64, &device)
            .unsqueeze_dim(0)
            .repeat(&[batch_size, 1]);
        let pos_embeds = self.pos_embed.forward(positions); // [batch, 1+seq_len, d_model]

        // 4. Build full sequence and add position embeddings
        let tokens = Tensor::cat(vec![ir_token, pass_embeds], 1);
        let tokens = tokens + pos_embeds;

        // 5. Build padding mask for transformer: true = ignore (mask out)
        //    mask is [batch, seq_len] where true = valid token.
        //    For IR token (position 0) we always keep it (valid).
        //    So we prepend a column of false (keep) to the inverted mask.
        let pad_mask = mask.equal_elem(false); // [batch, seq_len], true where padding
        let ir_pad = Tensor::<B, 2, Bool>::from_data(
            TensorData::new(vec![false; batch_size], [batch_size, 1]),
            &device,
        ); // [batch, 1], false (IR token is never masked)
        let full_pad_mask = Tensor::cat(vec![ir_pad, pad_mask], 1); // [batch, 1+seq_len]

        // 6. Run transformer with padding mask (2D padding mask)
        let encoder_input = TransformerEncoderInput::new(tokens).mask_pad(full_pad_mask);
        let encoded = self.transformer.forward(encoder_input); // [batch, 1+seq_len, d_model]

        // 7. Take [CLS] token (first position) and remove the singleton dimension
        let cls_output = encoded.narrow(1, 0, 1).squeeze_dim(1); // [batch, d_model]

        // 8. Regression head
        self.output_head.forward(cls_output)
    }
}