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
    #[config(default = 256)]
    pub d_model: usize,
    #[config(default = 8)]
    pub n_heads: usize,
    #[config(default = 3)]
    pub n_layers: usize,
    #[config(default = 512)]
    pub d_ff: usize,
    #[config(default = 0.1)]
    pub dropout: f64,
    /// Dimensionality of the learned action embedding.
    #[config(default = 32)]
    pub action_embed_dim: usize,
    /// Positional embedding table size — must exceed max episode length + 1.
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
            ir_proj: LinearConfig::new(self.input_dim, self.d_model).init(device),
            action_embed: EmbeddingConfig::new(self.num_actions, self.action_embed_dim)
                .init(device),
            action_proj: LinearConfig::new(self.action_embed_dim, self.d_model).init(device),
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
/// Input structure: one fixed IR-features token (base IR at episode start) followed
/// by the ordered sequence of action tokens (passes applied so far).  The transformer
/// attends causally over [IR | a_0 | a_1 | … | a_{t-1}] and the output at position t
/// becomes the policy logits and value for step t.
///
/// Compared to the previous approach (IR features repeated at every position), this
/// separates what the code looks like (IR prefix, one token) from what has been tried
/// (action sequence).  The attention layers learn pass-combination patterns directly
/// without having to filter noisy per-step IR snapshots.
///
/// Convention: position 0 in the sequence is always the IR token.  The action
/// sequence starts at position 1, with a zero-padding token at position 1 (representing
/// "no previous action") so the sequence is never empty.  Position t+1 holds the action
/// taken at step t-1 (for t≥1); the output at position t is used for the decision at
/// step t.
#[derive(Module, Debug)]
pub struct TransformerActorCritic<B: Backend> {
    ir_proj:      Linear<B>,
    action_embed: Embedding<B>,
    action_proj:  Linear<B>,
    pos_embed:    Embedding<B>,
    transformer:  TransformerEncoder<B>,
    policy_head:  Linear<B>,
    value_head:   Linear<B>,
}

impl<B: Backend> TransformerActorCritic<B> {
    /// Build token sequence: [IR_token, action_tokens…].
    ///
    /// `base_features`: [batch, feat_dim]
    /// `actions`:       [batch, seq]  (seq ≥ 1; position 0 = zero-pad "no prior action")
    /// Returns:         [batch, 1+seq, d_model]
    fn tokenize(
        &self,
        base_features: Tensor<B, 2>,
        actions:       Tensor<B, 2, Int>,
    ) -> Tensor<B, 3> {
        let [_, seq] = actions.dims();
        let device   = base_features.device();

        let ir_token  = self.ir_proj.forward(base_features).unsqueeze_dim::<3>(1); // [b,1,d]
        let act_emb   = self.action_embed.forward(actions);                         // [b,seq,ae]
        let act_tok   = self.action_proj.forward(act_emb);                          // [b,seq,d]
        let tokens    = Tensor::cat(vec![ir_token, act_tok], 1);                    // [b,1+seq,d]

        let total = seq + 1;
        let pos_ids = Tensor::<B, 1, Int>::arange(0..total as i64, &device)
            .unsqueeze::<2>();                                                       // [1,1+seq]
        let pos_emb = self.pos_embed.forward(pos_ids);                              // [1,1+seq,d]

        tokens + pos_emb
    }

    /// Single-episode forward used during rollout collection.
    ///
    /// `base_features`: [1, feat_dim] — IR at episode start (never changes)
    /// `actions`:       [1, t+1]      — zero-pad at index 0, then actions taken so far
    ///
    /// Returns logits and value for the current step (output at last position).
    pub fn forward(
        &self,
        base_features: Tensor<B, 2>,
        actions:       Tensor<B, 2, Int>,
    ) -> (Tensor<B, 2>, Tensor<B, 2>) {
        let tokens = self.tokenize(base_features, actions); // [1, 1+t+1, d]
        let out    = self.transformer.forward(TransformerEncoderInput::new(tokens));
        let [_, seq, d] = out.dims();
        let last   = out.slice([0..1, (seq - 1)..seq, 0..d]).reshape([1, d]);
        let logits = self.policy_head.forward(last.clone());
        let value  = self.value_head.forward(last);
        (logits, value)
    }

    /// Batched multi-episode forward used during PPO updates.
    ///
    /// `base_features`: [n_ep, feat_dim]  — base IR per episode
    /// `prev_actions`:  [n_ep, max_t]     — prev_actions[ep][t] = action at step t-1
    ///                                      (0 at t=0, same format as before)
    ///
    /// Returns (logits [n_ep, max_t, n_act], values [n_ep, max_t, 1]).
    /// Output at position t (1-indexed in the token sequence, 0-indexed in the slice)
    /// corresponds to step t decisions — identical output contract to the old interface.
    pub fn forward_batch(
        &self,
        base_features: Tensor<B, 2>,
        prev_actions:  Tensor<B, 2, Int>,
    ) -> (Tensor<B, 3>, Tensor<B, 3>) {
        let [n_ep, max_t] = prev_actions.dims();
        let device        = base_features.device();

        // Sequence: [IR_token, prev_action_0=0, prev_action_1=a_0, ..., prev_action_{max_t-1}]
        // Length = max_t + 1
        let tokens  = self.tokenize(base_features, prev_actions); // [n_ep, max_t+1, d]
        let seq_len = max_t + 1;

        // Causal mask [n_ep, seq_len, seq_len]: position i cannot attend to j > i
        let mut mask_data = vec![false; n_ep * seq_len * seq_len];
        for ep in 0..n_ep {
            for i in 0..seq_len {
                for j in (i + 1)..seq_len {
                    mask_data[ep * seq_len * seq_len + i * seq_len + j] = true;
                }
            }
        }
        let causal = Tensor::<B, 3, Bool>::from_data(
            TensorData::new(mask_data, [n_ep, seq_len, seq_len]),
            &device,
        );

        let out = self.transformer.forward(
            TransformerEncoderInput::new(tokens).mask_attn(causal),
        ); // [n_ep, max_t+1, d]

        let d = out.dims()[2];

        // Positions 1..max_t (0-indexed slice 1..max_t+1) correspond to step decisions 0..max_t-1.
        // This matches the output contract: logits[ep][t] = logits for step t.
        let out_steps = out.slice([0..n_ep, 1..(max_t + 1), 0..d]); // [n_ep, max_t, d]
        let out_flat  = out_steps.reshape([n_ep * max_t, d]);

        let logits_flat = self.policy_head.forward(out_flat.clone());
        let n_act       = logits_flat.dims()[1];
        let logits      = logits_flat.reshape([n_ep, max_t, n_act]);
        let values      = self.value_head.forward(out_flat).reshape([n_ep, max_t, 1]);

        (logits, values)
    }

    /// Single-episode forward for **per-step** IR mode.
    ///
    /// Each sequence position t combines the IR snapshot at step t with the
    /// action taken at step t-1.  The transformer attends causally over this
    /// sequence; the output at the last position drives the current decision.
    ///
    /// `ir_seq`:      [1, seq, feat_dim] — IR features at each step so far
    /// `prev_actions`: [1, seq]          — prev_actions[t] = action at step t-1 (0 at t=0)
    pub fn forward_persteoir(
        &self,
        ir_seq:       Tensor<B, 3>,
        prev_actions: Tensor<B, 2, Int>,
    ) -> (Tensor<B, 2>, Tensor<B, 2>) {
        let [_, seq, feat_dim] = ir_seq.dims();
        let device = ir_seq.device();

        // Project each step's IR to d_model, add projected action embedding.
        let ir_flat  = self.ir_proj.forward(ir_seq.reshape([seq, feat_dim]));
        let d        = ir_flat.dims()[1];
        let ir_tok   = ir_flat.reshape([1, seq, d]);

        let act_emb  = self.action_embed.forward(prev_actions.reshape([seq]).unsqueeze_dim::<2>(1));
        let ae       = act_emb.dims()[2];
        let act_tok  = self.action_proj.forward(act_emb.reshape([seq, ae])).reshape([1, seq, d]);

        let pos_ids  = Tensor::<B, 1, Int>::arange(0..seq as i64, &device).unsqueeze::<2>();
        let pos_emb  = self.pos_embed.forward(pos_ids);
        let tokens   = ir_tok + act_tok + pos_emb;

        // No causal mask needed for single-step: only seq tokens exist so no future leakage.
        let out  = self.transformer.forward(TransformerEncoderInput::new(tokens));
        let d2   = out.dims()[2];
        let last = out.slice([0..1, (seq - 1)..seq, 0..d2]).reshape([1, d2]);
        let logits = self.policy_head.forward(last.clone());
        let value  = self.value_head.forward(last);
        (logits, value)
    }

    /// Batched multi-episode forward for **per-step** IR mode.
    ///
    /// `ir_features`:  [n_ep, max_t, feat_dim] — per-step IR snapshots
    /// `prev_actions`: [n_ep, max_t]            — prev_actions[ep][t] = action at step t-1
    ///
    /// Returns (logits [n_ep, max_t, n_act], values [n_ep, max_t, 1]).
    pub fn forward_batch_persteoir(
        &self,
        ir_features:  Tensor<B, 3>,
        prev_actions: Tensor<B, 2, Int>,
    ) -> (Tensor<B, 3>, Tensor<B, 3>) {
        let [n_ep, max_t, feat_dim] = ir_features.dims();
        let device = ir_features.device();

        let ir_flat  = self.ir_proj.forward(ir_features.reshape([n_ep * max_t, feat_dim]));
        let d        = ir_flat.dims()[1];
        let ir_tok   = ir_flat.reshape([n_ep, max_t, d]);

        let act_emb  = self.action_embed
            .forward(prev_actions.reshape([n_ep * max_t]).unsqueeze_dim::<2>(1));
        let ae       = act_emb.dims()[2];
        let act_tok  = self.action_proj
            .forward(act_emb.reshape([n_ep * max_t, ae]))
            .reshape([n_ep, max_t, d]);

        let pos_ids  = Tensor::<B, 1, Int>::arange(0..max_t as i64, &device).unsqueeze::<2>();
        let pos_emb  = self.pos_embed.forward(pos_ids);
        let tokens   = ir_tok + act_tok + pos_emb; // [n_ep, max_t, d]

        // Causal mask [n_ep, max_t, max_t]
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

        let out      = self.transformer.forward(
            TransformerEncoderInput::new(tokens).mask_attn(causal),
        ); // [n_ep, max_t, d]
        let h_dim    = out.dims()[2];
        let out_flat = out.reshape([n_ep * max_t, h_dim]);

        let logits_flat = self.policy_head.forward(out_flat.clone());
        let n_act  = logits_flat.dims()[1];
        let logits = logits_flat.reshape([n_ep, max_t, n_act]);
        let values = self.value_head.forward(out_flat).reshape([n_ep, max_t, 1]);

        (logits, values)
    }
}
