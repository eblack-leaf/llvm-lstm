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
    #[config(default = 18)]
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
        let x = self.input_proj.forward(features);              // [batch, hidden_size]
        let e = self.action_embed
            .forward(prev_action.unsqueeze_dim(1))              // [batch, 1, embed_dim]
            .squeeze::<2>();                                    // [batch, embed_dim]
        let inp = Tensor::cat(vec![x, e], 1).unsqueeze_dim(1); // [batch, 1, hidden+embed]
        let out = self.gru.forward(inp, hidden);                // [batch, 1, hidden_size]
        let h   = out.squeeze::<2>();                           // [batch, hidden_size]
        let logits = self.policy_head.forward(h.clone());       // [batch, num_actions]
        (logits, h)
    }

    /// Full-episode sequence forward. Used during PPO updates.
    ///
    /// Runs the GRU over the whole episode from a clean hidden state (None),
    /// matching the episode-start condition during collection — no approximation.
    ///
    /// `prev_actions[t]` = action taken at step t-1; 0 for t=0 (episode start).
    ///
    /// Returns `logits [seq_len, num_actions]`.
    pub fn forward_sequence(
        &self,
        features:     Tensor<B, 2>,        // [seq_len, input_dim]
        prev_actions: Tensor<B, 1, Int>,   // [seq_len]
    ) -> Tensor<B, 2> {                    // [seq_len, num_actions]
        let x = self.input_proj.forward(features);               // [seq_len, hidden_size]
        let e = self.action_embed
            .forward(prev_actions.unsqueeze_dim(1))              // [seq_len, 1, embed_dim]
            .squeeze::<2>();                                     // [seq_len, embed_dim]
        // Add batch dim=1 so the GRU sees [1, seq_len, hidden+embed].
        let inp = Tensor::cat(vec![x, e], 1).unsqueeze_dim(0);  // [1, seq_len, hidden+embed]
        let out = self.gru.forward(inp, None);                   // [1, seq_len, hidden_size]
        let h   = out.squeeze::<2>();                            // [seq_len, hidden_size]
        self.policy_head.forward(h)                              // [seq_len, num_actions]
    }
}

// ── Critic ───────────────────────────────────────────────────────────────────

#[derive(Config, Debug)]
pub struct CriticConfig {
    #[config(default = 18)]
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
