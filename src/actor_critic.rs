use burn::config::Config;
use burn::module::Module;
use burn::nn::gru::{Gru, GruConfig};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::tensor::backend::Backend;
use burn::tensor::{Int, Tensor};

#[derive(Config, Debug)]
pub struct ActorCriticConfig {
    #[config(default = 32)]
    pub input_dim: usize,
    #[config(default = 29)]
    pub num_actions: usize,
    /// GRU hidden state size and input projection size.
    #[config(default = 256)]
    pub hidden_size: usize,
    /// Dimensionality of the learned previous-action embedding.
    #[config(default = 32)]
    pub action_embed_dim: usize,
}

impl ActorCriticConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> ActorCritic<B> {
        let gru_input_size = self.hidden_size + self.action_embed_dim;
        ActorCritic {
            input_proj:   LinearConfig::new(self.input_dim, self.hidden_size).init(device),
            action_embed: EmbeddingConfig::new(self.num_actions, self.action_embed_dim).init(device),
            gru:          GruConfig::new(gru_input_size, self.hidden_size, true).init(device),
            policy_head:  LinearConfig::new(self.hidden_size, self.num_actions).init(device),
            value_head:   LinearConfig::new(self.hidden_size, 1).init(device),
        }
    }
}

/// GRU-based actor-critic with shared trunk.
///
/// The input projection, action embedding, and GRU weights are shared between
/// the policy head and value head. This halves recurrent parameters and improves
/// sample efficiency — particularly important with sparse reward signals where
/// each trajectory provides limited gradient signal.
///
/// Two calling modes:
/// - `forward`       — single step, threads hidden state during episode collection.
/// - `forward_batch` — full padded batch, used during PPO updates for exact log-probs.
#[derive(Module, Debug)]
pub struct ActorCritic<B: Backend> {
    input_proj:   Linear<B>,
    action_embed: Embedding<B>,
    gru:          Gru<B>,
    policy_head:  Linear<B>,
    value_head:   Linear<B>,
}

impl<B: Backend> ActorCritic<B> {
    /// Single-step forward. Used during rollout collection.
    ///
    /// Returns `(logits [batch, num_actions], value [batch, 1], new_hidden [batch, hidden_size])`.
    pub fn forward(
        &self,
        features:    Tensor<B, 2>,         // [batch, input_dim]
        prev_action: Tensor<B, 1, Int>,    // [batch]
        hidden:      Option<Tensor<B, 2>>, // [batch, hidden_size] or None
    ) -> (Tensor<B, 2>, Tensor<B, 2>, Tensor<B, 2>) {
        let x = self.input_proj.forward(features);

        // Embedding returns [batch, 1, embed_dim]; reshape to [batch, embed_dim].
        // Can't use squeeze — if batch=1 both dims are size-1 and squeeze panics.
        let e_raw = self.action_embed.forward(prev_action.unsqueeze_dim(1));
        let ed = e_raw.shape().dims;
        let e = e_raw.reshape([ed[0], ed[2]]);                 // [batch, embed_dim]

        let inp = Tensor::cat(vec![x, e], 1).unsqueeze_dim(1); // [batch, 1, hidden+embed]
        let out = self.gru.forward(inp, hidden);               // [batch, 1, hidden_size]
        let od = out.shape().dims;
        let h = out.reshape([od[0], od[2]]);                   // [batch, hidden_size]

        let logits = self.policy_head.forward(h.clone());      // [batch, num_actions]
        let value  = self.value_head.forward(h.clone());       // [batch, 1]
        (logits, value, h)
    }

    /// Batched multi-episode forward. Used during PPO updates.
    ///
    /// Episodes are zero-padded to `max_T` so the GRU processes all of them
    /// in one call. Only real (non-padding) steps are used in the loss.
    ///
    /// Returns `(logits [n_ep, max_T, num_actions], values [n_ep, max_T, 1])`.
    /// Caller selects real steps via index gather.
    pub fn forward_batch(
        &self,
        features:     Tensor<B, 3>,        // [n_ep, max_T, feat_dim]
        prev_actions: Tensor<B, 2, Int>,   // [n_ep, max_T]
    ) -> (Tensor<B, 3>, Tensor<B, 3>) {   // (logits, values)
        let dims = features.shape().dims;
        let (n_ep, max_t, feat_dim) = (dims[0], dims[1], dims[2]);

        // Linear on last dim: flatten → project → unflatten.
        let x_flat = self.input_proj.forward(features.reshape([n_ep * max_t, feat_dim]));
        let hidden_size = x_flat.shape().dims[1];
        let x = x_flat.reshape([n_ep, max_t, hidden_size]);    // [n_ep, max_T, hidden]

        // Embedding: [n_ep, max_T] → [n_ep*max_T, 1] → embed → [n_ep, max_T, embed_dim]
        let e_raw = self.action_embed
            .forward(prev_actions.reshape([n_ep * max_t]).unsqueeze_dim::<2>(1));
        let embed_dim = e_raw.shape().dims[2];
        let e = e_raw.reshape([n_ep, max_t, embed_dim]);

        let gru_inp = Tensor::cat(vec![x, e], 2);              // [n_ep, max_T, hidden+embed]
        let out = self.gru.forward(gru_inp, None);             // [n_ep, max_T, hidden_size]
        let h_dim = out.shape().dims[2];
        let out_flat = out.reshape([n_ep * max_t, h_dim]);     // [n_ep*max_T, hidden_size]

        // Both heads share the GRU output. Value loss trains the GRU to encode
        // return-predictive features, which also benefits the policy representation.
        let logits_flat = self.policy_head.forward(out_flat.clone());
        let n_act = logits_flat.shape().dims[1];
        let logits = logits_flat.reshape([n_ep, max_t, n_act]);

        let val_flat = self.value_head.forward(out_flat);
        let values = val_flat.reshape([n_ep, max_t, 1]);

        (logits, values)
    }
}
