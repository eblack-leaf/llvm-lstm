use crate::config::{BurnAutoDiff, BurnBackend, BurnDevice, Cfg};
use crate::ppo::model::{Actor, Input, MlpHead, MlpHeadConfig, Output};
use burn::module::AutodiffModule;
use burn::nn::gru::{Gru, GruConfig};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::prelude::{Config, Module};

#[derive(Config, Debug)]
pub(crate) struct GruActorConfig {
    #[config(default = 68)]
    pub(crate) input_dim: usize,
    #[config(default = 29)]
    pub(crate) num_actions: usize,
    #[config(default = 256)]
    pub(crate) hidden_size: usize,
    #[config(default = 32)]
    pub(crate) action_embed_dim: usize,
    #[config(default = 128)]
    pub(crate) head_hidden: usize,
}

// Shared trunk: IR features initialise the hidden state; action sequence drives the GRU.
// Policy and value heads are each a 2-layer MLP.
#[derive(Module, Debug, Clone)]
pub(crate) struct GruActor {
    // Projects IR feature vector [batch, input_dim] → [batch, hidden_size] used as h0
    ir_proj: Linear<BurnBackend>,
    // Embeds action indices [batch, seq] → [batch, seq, action_embed_dim]
    action_embed: Embedding<BurnBackend>,
    // Projects embedded actions → [batch, seq, hidden_size] (GRU input_size = hidden_size)
    action_proj: Linear<BurnBackend>,
    // Single-layer unidirectional GRU
    gru: Gru<BurnBackend>,
    // Policy head: [batch, hidden_size] → [batch, num_actions]
    policy_head: MlpHead,
    // Value head: [batch, hidden_size] → [batch, 1]
    value_head: MlpHead,
}

impl Actor for GruActor
where
    Self: AutodiffModule<BurnAutoDiff>,
{
    type Config = GruActorConfig;

    fn init(cfg: Self::Config, device: &BurnDevice) -> Self {
        Self {
            ir_proj: LinearConfig::new(cfg.input_dim, cfg.hidden_size).init(device),
            action_embed: EmbeddingConfig::new(cfg.num_actions, cfg.action_embed_dim)
                .init(device),
            action_proj: LinearConfig::new(cfg.action_embed_dim, cfg.hidden_size).init(device),
            gru: GruConfig::new(cfg.hidden_size, cfg.hidden_size, false).init(device),
            policy_head: MlpHeadConfig::new(cfg.hidden_size, cfg.head_hidden, cfg.num_actions)
                .init(device),
            value_head: MlpHeadConfig::new(cfg.hidden_size, cfg.head_hidden, 1).init(device),
        }
    }

    fn forward(&self, _cfg: &Cfg, input: Input) -> Output {
        // IR features → initial GRU hidden state [batch, hidden_size]
        let h0 = self.ir_proj.forward(input.features);

        // Action sequence → [batch, seq_len, hidden_size]
        let seq = self.action_embed.forward(input.actions); // [batch, seq, action_embed_dim]
        let seq = self.action_proj.forward(seq);            // [batch, seq, hidden_size]
        let seq_len = seq.dims()[1];

        // GRU output [batch, seq_len, hidden_size]; take last step as context [batch, hidden_size]
        let out = self.gru.forward(seq, Some(h0));
        let hn = out.narrow(1, seq_len - 1, 1).squeeze::<2>();

        // 2-layer MLP heads
        let policy = self.policy_head.forward(hn.clone()).unsqueeze_dim(1); // [batch, 1, num_actions]
        let value = self.value_head.forward(hn);                             // [batch, 1]

        Output { policy, value }
    }

    fn cfg(_cfg: &Cfg) -> Self::Config {
        GruActorConfig::new()
    }

    fn no_grads(&self) -> Self {
        <GruActor as AutodiffModule<BurnAutoDiff>>::valid(self)
    }
}
