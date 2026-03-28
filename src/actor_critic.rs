use burn::config::Config;
use burn::module::Module;
use burn::nn::gru::{Gru, GruConfig};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::tensor::activation;
use burn::tensor::backend::Backend;
use burn::tensor::{Int, Tensor};

// ── Actor ────────────────────────────────────────────────────────────────────

#[derive(Config, Debug)]
pub struct ActorConfig {
    #[config(default = 24)]
    pub input_dim: usize,
    #[config(default = 29)]
    pub num_actions: usize,
    /// GRU hidden state size and input projection size.
    #[config(default = 128)]
    pub hidden_size: usize,
    /// Dimensionality of the learned previous-action embedding.
    #[config(default = 32)]
    pub action_embed_dim: usize,
}

impl ActorConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> Actor<B> {
        let gru_input_size = self.hidden_size + self.action_embed_dim;
        Actor {
            input_proj:   LinearConfig::new(self.input_dim, self.hidden_size).init(device),
            action_embed: EmbeddingConfig::new(self.num_actions, self.action_embed_dim).init(device),
            gru:          GruConfig::new(gru_input_size, self.hidden_size, true).init(device),
            policy_head:  LinearConfig::new(self.hidden_size, self.num_actions).init(device),
        }
    }
}

/// GRU-based actor: maps (IR features, previous action, hidden state) → action logits.
///
/// Two calling modes:
/// - `forward`          — single step, threads hidden state during episode collection.
/// - `forward_sequence` — full episode, used during PPO updates for exact log-probs.
#[derive(Module, Debug)]
pub struct Actor<B: Backend> {
    input_proj:   Linear<B>,
    action_embed: Embedding<B>,
    gru:          Gru<B>,
    policy_head:  Linear<B>,
}

impl<B: Backend> Actor<B> {
    /// Single-step forward. Used during rollout collection.
    ///
    /// Returns `(logits [batch, num_actions], new_hidden [batch, hidden_size])`.
    pub fn forward(
        &self,
        features:    Tensor<B, 2>,         // [batch, input_dim]
        prev_action: Tensor<B, 1, Int>,    // [batch]
        hidden:      Option<Tensor<B, 2>>, // [batch, hidden_size] or None
    ) -> (Tensor<B, 2>, Tensor<B, 2>) {
        let x = self.input_proj.forward(features);             // [batch, hidden_size]

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
        (logits, h)
    }

    /// Batched multi-episode forward. Used during PPO updates instead of
    /// calling `forward_sequence` once per episode.
    ///
    /// Episodes are zero-padded to `max_T` so the GRU processes all of them
    /// in one call. Only real (non-padding) logits are used in the loss.
    ///
    /// Returns `logits [n_ep, max_T, num_actions]`. Caller selects real steps.
    pub fn forward_batch(
        &self,
        features:     Tensor<B, 3>,         // [n_ep, max_T, feat_dim]
        prev_actions: Tensor<B, 2, Int>,     // [n_ep, max_T]
    ) -> Tensor<B, 3> {                      // [n_ep, max_T, num_actions]
        let dims = features.shape().dims;
        let (n_ep, max_t, feat_dim) = (dims[0], dims[1], dims[2]);

        // Linear on last dim: flatten to [n_ep*max_T, feat_dim], project, unflatten.
        let x_flat = self.input_proj.forward(features.reshape([n_ep * max_t, feat_dim]));
        let hidden_size = x_flat.shape().dims[1];
        let x = x_flat.reshape([n_ep, max_t, hidden_size]);  // [n_ep, max_T, hidden]

        // Embedding: reshape [n_ep, max_T] → [n_ep*max_T, 1] → embed → [n_ep, max_T, embed_dim]
        let e_raw = self.action_embed
            .forward(prev_actions.reshape([n_ep * max_t]).unsqueeze_dim::<2>(1));
        let embed_dim = e_raw.shape().dims[2];
        let e = e_raw.reshape([n_ep, max_t, embed_dim]);     // [n_ep, max_T, embed_dim]

        // GRU takes [batch, seq, input] — already in that shape, no unsqueeze needed.
        let gru_inp = Tensor::cat(vec![x, e], 2);            // [n_ep, max_T, hidden+embed]
        let out = self.gru.forward(gru_inp, None);           // [n_ep, max_T, hidden_size]
        let h = out.shape().dims[2];

        // Policy head: flatten, project, unflatten.
        let logits_flat = self.policy_head.forward(out.reshape([n_ep * max_t, h]));
        let n_act = logits_flat.shape().dims[1];
        logits_flat.reshape([n_ep, max_t, n_act])
    }

}

// ── Critic ───────────────────────────────────────────────────────────────────

#[derive(Config, Debug)]
pub struct CriticConfig {
    #[config(default = 24)]
    pub input_dim: usize,
    /// Hidden layer width for both MLP layers.
    #[config(default = 64)]
    pub hidden_size: usize,
}

impl CriticConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> Critic<B> {
        Critic {
            fc1:        LinearConfig::new(self.input_dim, self.hidden_size).init(device),
            fc2:        LinearConfig::new(self.hidden_size, self.hidden_size).init(device),
            value_head: LinearConfig::new(self.hidden_size, 1).init(device),
        }
    }
}

/// Feedforward critic: maps current IR features → scalar state-value estimate V(s).
///
/// No hidden state — the IR features already encode the result of all passes applied.
/// This lets the value loss be computed exactly during PPO updates.
#[derive(Module, Debug)]
pub struct Critic<B: Backend> {
    fc1:        Linear<B>,
    fc2:        Linear<B>,
    value_head: Linear<B>,
}

impl<B: Backend> Critic<B> {
    /// Returns `value [batch, 1]`.
    pub fn forward(&self, features: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = activation::relu(self.fc1.forward(features));
        let x = activation::relu(self.fc2.forward(x));
        self.value_head.forward(x)
    }
}
