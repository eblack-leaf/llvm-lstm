use crate::config::{BurnAutoDiff, Cfg};
use crate::ppo::model::{Actor, Input, MlpHead, MlpHeadConfig, Output};
use burn::module::AutodiffModule;
use burn::nn::transformer::{
    TransformerEncoder, TransformerEncoderConfig, TransformerEncoderInput,
};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::prelude::{Backend, Bool, Config, Int, Module};
use burn::tensor::Tensor;

#[derive(Config, Debug)]
pub(crate) struct TransformerActorConfig {
    #[config(default = 68)]
    pub(crate) input_dim: usize,
    #[config(default = 29)]
    pub(crate) num_actions: usize,
    #[config(default = 256)]
    pub(crate) d_model: usize,
    #[config(default = 8)]
    pub(crate) n_heads: usize,
    #[config(default = 3)]
    pub(crate) n_layers: usize,
    #[config(default = 512)]
    pub(crate) d_ff: usize,
    #[config(default = 0.3)]
    pub(crate) dropout: f64,
    #[config(default = 32)]
    pub(crate) action_embed_dim: usize,
    /// Positional embedding table size — must exceed max episode length + 1.
    #[config(default = 64)]
    pub(crate) max_seq_len: usize,
    #[config(default = 128)]
    pub(crate) head_hidden: usize,
}

// Shared trunk: IR feature vector is projected to a context token prepended to the action
// sequence; a transformer encoder processes the full sequence; the IR token at position 0
// is pooled as the context for both heads.
// Policy and value heads are each a 2-layer MLP.
#[derive(Module, Debug)]
pub(crate) struct TransformerActor<B: Backend> {
    // Projects IR feature vector [batch, input_dim] → [batch, d_model]
    ir_proj: Linear<B>,
    // Embeds action indices [batch, seq] → [batch, seq, action_embed_dim]
    action_embed: Embedding<B>,
    // Projects embedded actions → [batch, seq, d_model]
    action_proj: Linear<B>,
    // Learned positional embeddings; table size = max_seq_len (covers seq_len + 1 IR token)
    pos_embed: Embedding<B>,
    transformer: TransformerEncoder<B>,
    // Policy head: [batch, d_model] → [batch, num_actions]
    policy_head: MlpHead<B>,
    // Value head: [batch, d_model] → [batch, 1]
    value_head: MlpHead<B>,
}

impl<B: Backend> Actor<B> for TransformerActor<B> {
    type Config = TransformerActorConfig;

    fn init(cfg: Self::Config, device: &B::Device) -> Self {
        Self {
            ir_proj: LinearConfig::new(cfg.input_dim, cfg.d_model).init(device),
            action_embed: EmbeddingConfig::new(cfg.num_actions, cfg.action_embed_dim).init(device),
            action_proj: LinearConfig::new(cfg.action_embed_dim, cfg.d_model).init(device),
            pos_embed: EmbeddingConfig::new(cfg.max_seq_len, cfg.d_model).init(device),
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

    fn forward(&self, _cfg: &Cfg, input: Input<B>) -> Output<B> {
        let [_batch, seq_len] = input.actions.dims();
        let device = input.features.device();

        // IR features → context token [batch, 1, d_model]
        let ir_tok = self.ir_proj.forward(input.features).unsqueeze_dim(1);

        // Action sequence → [batch, seq_len, d_model]
        let act = self.action_embed.forward(input.actions); // [batch, seq, action_embed_dim]
        let act = self.action_proj.forward(act); // [batch, seq, d_model]

        // Prepend IR token → [batch, seq_len+1, d_model]
        let x = Tensor::cat(vec![ir_tok, act], 1);

        // Positional encoding → [batch, seq_len+1, d_model]
        let positions =
            Tensor::<B, 1, Int>::arange(0..(seq_len + 1) as i64, &device).unsqueeze_dim(0); // [1, seq_len+1]
        let pos = self.pos_embed.forward(positions);
        let x = x + pos;

        // Transformer encoder → [batch, seq_len+1, d_model]
        let enc_input = TransformerEncoderInput::new(x);
        let enc_input = match input.mask_pad {
            Some(mask) => enc_input.mask_pad(mask),
            None => enc_input,
        };
        let out = self.transformer.forward(enc_input);

        // Pool the IR token (position 0) as the shared context [batch, d_model]
        let ctx = out.narrow(1, 0, 1).flatten::<2>(1, 2);

        // 2-layer MLP heads
        let policy = self.policy_head.forward(ctx.clone()).unsqueeze_dim(1); // [batch, 1, num_actions]
        let value = self.value_head.forward(ctx); // [batch, 1]

        Output { policy, value }
    }

    fn cfg(_cfg: &Cfg) -> Self::Config {
        TransformerActorConfig::new()
    }
}
