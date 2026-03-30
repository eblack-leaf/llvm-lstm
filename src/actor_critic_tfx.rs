use burn::config::Config;
use burn::module::Module;
use burn::nn::transformer::{TransformerEncoder, TransformerEncoderConfig, TransformerEncoderInput};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::tensor::backend::Backend;
use burn::tensor::{Bool, Int, Tensor, TensorData};

#[derive(Config, Debug)]
pub struct TransformerActorCriticConfig {
    #[config(default = 34)]
    pub input_dim: usize,
    #[config(default = 29)]
    pub num_actions: usize,
    /// Token embedding dimension — must be divisible by n_heads.
    #[config(default = 256)]
    pub d_model: usize,
    #[config(default = 8)]
    pub n_heads: usize,
    #[config(default = 3)]
    pub n_layers: usize,
    /// FFN hidden dim inside each transformer layer.
    #[config(default = 512)]
    pub d_ff: usize,
    #[config(default = 0.1)]
    pub dropout: f64,
    /// Dimensionality of the learned previous-action embedding.
    #[config(default = 32)]
    pub action_embed_dim: usize,
    /// Learned positional embedding table size — must exceed max episode length.
    #[config(default = 64)]
    pub max_seq_len: usize,
}

impl TransformerActorCriticConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> TransformerActorCritic<B> {
        assert_eq!(
            self.d_model % self.n_heads,
            0,
            "d_model ({}) must be divisible by n_heads ({})",
            self.d_model,
            self.n_heads
        );
        TransformerActorCritic {
            token_proj: LinearConfig::new(
                self.input_dim + self.action_embed_dim,
                self.d_model,
            )
            .init(device),
            action_embed: EmbeddingConfig::new(self.num_actions, self.action_embed_dim)
                .init(device),
            pos_embed: EmbeddingConfig::new(self.max_seq_len, self.d_model).init(device),
            transformer: TransformerEncoderConfig::new(
                self.d_model,
                self.d_ff,
                self.n_heads,
                self.n_layers,
            )
            .with_dropout(self.dropout)
            .init(device),
            policy_head: LinearConfig::new(self.d_model, self.num_actions).init(device),
            value_head: LinearConfig::new(self.d_model, 1).init(device),
        }
    }
}

/// Causal Transformer actor-critic.
///
/// Replaces the GRU's rolling hidden state with full causal self-attention over
/// the sequence of (IR_features, prev_action) tokens accumulated during the
/// episode. At step t the model attends directly to every earlier step — no
/// hidden-state bottleneck compressing prior decisions into a fixed-size vector.
///
/// Two calling modes (identical interface to the GRU actor-critic):
/// - `forward`       — growing sequence, single step during rollout.
/// - `forward_batch` — padded episode batch with causal mask, PPO updates.
#[derive(Module, Debug)]
pub struct TransformerActorCritic<B: Backend> {
    token_proj:   Linear<B>,
    action_embed: Embedding<B>,
    pos_embed:    Embedding<B>,
    transformer:  TransformerEncoder<B>,
    policy_head:  Linear<B>,
    value_head:   Linear<B>,
}

impl<B: Backend> TransformerActorCritic<B> {
    /// Project (IR_features ++ prev_action_embed) into d_model tokens, add
    /// learned positional embeddings.
    fn tokenize(
        &self,
        features:     Tensor<B, 3>,      // [batch, seq, feat_dim]
        prev_actions: Tensor<B, 2, Int>, // [batch, seq]
    ) -> Tensor<B, 3> {                  // [batch, seq, d_model]
        let [_, seq, _] = features.dims();
        let device = features.device();

        let act_emb = self.action_embed.forward(prev_actions); // [batch, seq, act_emb_dim]
        let tokens  = Tensor::cat(vec![features, act_emb], 2); // [batch, seq, feat+act_emb]
        let tokens  = self.token_proj.forward(tokens);          // [batch, seq, d_model]

        let pos_ids = Tensor::<B, 1, Int>::arange(0..seq as i64, &device)
            .unsqueeze::<2>();                                   // [1, seq]
        let pos_emb = self.pos_embed.forward(pos_ids);          // [1, seq, d_model]

        tokens + pos_emb // broadcast over batch
    }

/// Single-episode forward used during rollout collection.
    ///
    /// Takes the full sequence accumulated so far this episode and returns
    /// the policy logits and value for the LAST position (the current step).
    /// No hidden state is threaded — the full context is always available.
    pub fn forward(
        &self,
        features_seq: Tensor<B, 3>,      // [1, t, feat_dim]
        actions_seq:  Tensor<B, 2, Int>, // [1, t]  prev_action at each position
    ) -> (Tensor<B, 2>, Tensor<B, 2>) {  // (logits [1, num_actions], value [1, 1])
        let [_, seq, _] = features_seq.dims();
        let tokens = self.tokenize(features_seq, actions_seq); // [1, seq, d_model]
        let out = self.transformer.forward(TransformerEncoderInput::new(tokens));
        // [1, seq, d_model] → last position → [1, d_model]
        let d = out.dims()[2];
        let last = out.slice([0..1, (seq - 1)..seq, 0..d]).reshape([1, d]);
        let logits = self.policy_head.forward(last.clone()); // [1, num_actions]
        let value  = self.value_head.forward(last);           // [1, 1]
        (logits, value)
    }

    /// Batched multi-episode forward used during PPO updates.
    ///
    /// Identical interface to the GRU's `forward_batch` so ppo_update is unchanged.
    /// A causal attention mask ensures position t attends only to 0..=t-1, matching
    /// the strictly causal rollout.
    pub fn forward_batch(
        &self,
        features_pad:  Tensor<B, 3>,      // [n_ep, max_t, feat_dim]
        prev_actions:  Tensor<B, 2, Int>, // [n_ep, max_t]
    ) -> (Tensor<B, 3>, Tensor<B, 3>) {   // (logits [n_ep, max_t, n_act], values [n_ep, max_t, 1])
        let [n_ep, max_t, _] = features_pad.dims();
        let device = features_pad.device();

        let tokens = self.tokenize(features_pad, prev_actions); // [n_ep, max_t, d_model]

        // Build [n_ep, max_t, max_t] causal mask directly.
        let mut mask_data = vec![false; n_ep * max_t * max_t];
        for ep in 0..n_ep {
            for i in 0..max_t {
                for j in (i + 1)..max_t {
                    mask_data[ep * max_t * max_t + i * max_t + j] = true;
                }
            }
        }
        let causal = Tensor::<B, 3, Bool>::from_data(
            TensorData::new(mask_data, [n_ep, max_t, max_t]),
            &device,
        );

        let out = self.transformer.forward(
            TransformerEncoderInput::new(tokens).mask_attn(causal),
        ); // [n_ep, max_t, d_model]

        let d = out.dims()[2];
        let out_flat = out.reshape([n_ep * max_t, d]);

        let logits_flat = self.policy_head.forward(out_flat.clone());
        let n_act = logits_flat.dims()[1];
        let logits = logits_flat.reshape([n_ep, max_t, n_act]);

        let values = self.value_head.forward(out_flat).reshape([n_ep, max_t, 1]);

        (logits, values)
    }
}
