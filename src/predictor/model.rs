use burn::config::Config;
use burn::module::Module;
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::prelude::{Backend, Tensor};
use burn::tensor::activation::relu;
use burn::tensor::Bool;
use burn::tensor::Int;

#[derive(Module, Debug)]
pub struct SpeedupPredictor<B: Backend> {
    pass_embed: Embedding<B>,
    pass_proj: Linear<B>,
    ir_proj: Linear<B>,
    combined_proj: Linear<B>,
    output: Linear<B>,
}

#[derive(Config, Debug)]
pub struct SpeedupPredictorConfig {
    pub num_passes: usize,
    pub pass_embed_dim: usize,
    pub ir_feature_dim: usize,
    pub hidden_dim: usize,
    pub output_dim: usize,
}

impl SpeedupPredictorConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> SpeedupPredictor<B> {
        SpeedupPredictor {
            pass_embed: EmbeddingConfig::new(self.num_passes, self.pass_embed_dim).init(device),
            pass_proj: LinearConfig::new(self.pass_embed_dim, self.hidden_dim).init(device),
            ir_proj: LinearConfig::new(self.ir_feature_dim, self.hidden_dim).init(device),
            combined_proj: LinearConfig::new(self.hidden_dim * 2, self.hidden_dim).init(device),
            output: LinearConfig::new(self.hidden_dim, self.output_dim).init(device),
        }
    }
}

impl<B: Backend> SpeedupPredictor<B> {
    pub fn forward(
        &self,
        ir_features: Tensor<B, 2>,             // [batch, ir_dim]
        passes: Tensor<B, 2, burn::tensor::Int>, // [batch, seq_len]
        mask: Tensor<B, 2, burn::tensor::Bool>,  // [batch, seq_len]
    ) -> Tensor<B, 2> {                        // [batch, output_dim]
        let batch_size = passes.dims()[0];

        // 1️⃣ Embed passes: [B, seq_len, E]
        let embedded = self.pass_embed.forward(passes);

        // 2️⃣ Apply mask
        let mask_float = mask.float().unsqueeze_dim(2); // [B, seq_len, 1]
        let masked = embedded * mask_float;

        // 3️⃣ Sum over sequence → [B, 1, E]
        let pass_sum_3d = masked.sum_dim(1);

        // 4️⃣ Flatten to 2D → [B, E]
        let pass_sum: Tensor<B, 2> = {
            let seq_dim = pass_sum_3d.dims()[2];
            pass_sum_3d.reshape([batch_size, seq_dim])
        };

        // 5️⃣ Project pass and IR features → [B, hidden_dim]
        let pass_hidden = self.pass_proj.forward(pass_sum);
        let ir_hidden = self.ir_proj.forward(ir_features);

        // 6️⃣ Concatenate along feature dimension → [B, hidden_dim*2]
        let combined = Tensor::cat(vec![pass_hidden, ir_hidden], 1);

        // 7️⃣ Hidden layer
        let hidden = relu(self.combined_proj.forward(combined));

        // 8️⃣ Output layer → [B, output_dim]
        self.output.forward(hidden)
    }
}