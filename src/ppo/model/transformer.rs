use crate::config::{BurnAutoDiff, BurnBackend, BurnDevice, Cfg};
use crate::ppo::model::{Actor, Input, Output};
use burn::config::Config;
use burn::module::AutodiffModule;
use burn::nn::Linear;
use burn::prelude::{Backend, Module};

#[derive(Config, Debug)]
pub(crate) struct TransformerActorConfig {
    #[config(default = 34)]
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
    /// Dimensionality of the learned action embedding.
    #[config(default = 32)]
    pub(crate) action_embed_dim: usize,
    /// Positional embedding table size — must exceed max episode length + 1.
    #[config(default = 64)]
    pub(crate) max_seq_len: usize,
}
#[derive(Module, Debug, Clone)]
pub(crate) struct TransformerActor {
    value: Linear<BurnBackend>,
}

impl Actor for TransformerActor {
    type Config = TransformerActorConfig;
    fn init(cfg: Self::Config, device: &BurnDevice) -> Self {
        todo!()
    }
    fn forward(&self, cfg: &Cfg, input: Input) -> Output {
        todo!()
    }

    fn cfg(cfg: &Cfg) -> Self::Config {
        todo!()
    }

    fn no_grads(&self) -> Self {
        <TransformerActor as AutodiffModule<BurnAutoDiff>>::valid(&self)
    }
}
