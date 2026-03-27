use burn::config::Config;
use burn::module::Module;
use burn::nn::gru::{Gru, GruConfig};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::tensor::backend::Backend;
use burn::tensor::{Int, Tensor};

#[derive(Config, Debug)]
pub struct ActorCriticConfig {
    /// Dimensionality of the IR feature vector input.
    #[config(default = 18)]
    pub input_dim: usize,
    /// Number of actions (passes + STOP).
    #[config(default = 29)]
    pub num_actions: usize,
    /// GRU hidden state size — also the input projection size.
    #[config(default = 128)]
    pub hidden_size: usize,
    /// Dimensionality of the learned previous-action embedding.
    #[config(default = 32)]
    pub action_embed_dim: usize,
}

impl ActorCriticConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> ActorCritic<B> {
        let gru_input_size = self.hidden_size + self.action_embed_dim;
        ActorCritic {
            input_proj:  LinearConfig::new(self.input_dim, self.hidden_size).init(device),
            action_embed: EmbeddingConfig::new(self.num_actions, self.action_embed_dim).init(device),
            gru:         GruConfig::new(gru_input_size, self.hidden_size, true).init(device),
            policy_head: LinearConfig::new(self.hidden_size, self.num_actions).init(device),
            value_head:  LinearConfig::new(self.hidden_size, 1).init(device),
        }
    }
}

/// Shared-backbone actor-critic for PPO pass-sequence selection.
///
/// The GRU runs once per step and produces a hidden state `h`.
/// Both the policy head and value head read from the same `h` —
/// no duplication, no separate forward passes.
///
///   features ──► input_proj ──► cat ──► GRU ──► h ──► policy_head ──► logits
///   prev_action ──► action_embed ──┘          └──► value_head  ──► value
#[derive(Module, Debug)]
pub struct ActorCritic<B: Backend> {
    /// Projects 18 IR features → hidden_size.
    input_proj:   Linear<B>,
    /// Embeds the previous action index → action_embed_dim.
    action_embed: Embedding<B>,
    /// GRU: (hidden_size + action_embed_dim) → hidden_size.
    gru:          Gru<B>,
    /// Maps GRU hidden state → num_actions logits (unnormalized).
    policy_head:  Linear<B>,
    /// Maps GRU hidden state → scalar value estimate V(s).
    value_head:   Linear<B>,
}

impl<B: Backend> ActorCritic<B> {
    /// Single-step forward pass.
    ///
    /// # Arguments
    /// - `features`    — `[batch, input_dim]`   IR feature vector for the current state.
    /// - `prev_action` — `[batch]`              Index of the action taken at the previous step.
    ///                                          Pass zeros at the start of each episode.
    /// - `hidden`      — `[batch, hidden_size]` GRU carry from the previous step,
    ///                                          or `None` at episode start (zeros).
    ///
    /// # Returns
    /// `(logits, value, new_hidden)` where:
    /// - `logits`      — `[batch, num_actions]` Unnormalized action scores for softmax/sampling.
    /// - `value`       — `[batch, 1]`           State-value estimate V(s) for PPO advantage.
    /// - `new_hidden`  — `[batch, hidden_size]` GRU state to pass into the next step.
    pub fn forward(
        &self,
        features:    Tensor<B, 2>,         // [batch, input_dim]
        prev_action: Tensor<B, 1, Int>,    // [batch]
        hidden:      Option<Tensor<B, 2>>, // [batch, hidden_size] or None
    ) -> (Tensor<B, 2>, Tensor<B, 2>, Tensor<B, 2>) {
        // Project IR features to hidden_size
        let x = self.input_proj.forward(features);              // [batch, hidden_size]

        // Embed previous action.
        // Embedding expects [batch, seq_len] Int; unsqueeze adds seq_len=1.
        let e = self.action_embed
            .forward(prev_action.unsqueeze_dim(1))              // [batch, 1, embed_dim]
            .squeeze::<2>();                                   // [batch, embed_dim]

        // Concatenate projections and add seq_len=1 dim for GRU input.
        let inp = Tensor::cat(vec![x, e], 1).unsqueeze_dim(1); // [batch, 1, hidden+embed]

        // Burn's Gru::forward returns Tensor<B,3> [batch, seq_len, hidden].
        // With seq_len=1, the output's single timestep is also the new hidden state.
        let out = self.gru.forward(inp, hidden);                // [batch, 1, hidden_size]
        let h = out.squeeze::<2>();                            // [batch, hidden_size]

        let logits = self.policy_head.forward(h.clone());       // [batch, num_actions]
        let value  = self.value_head.forward(h.clone());        // [batch, 1]

        (logits, value, h)
    }
}
