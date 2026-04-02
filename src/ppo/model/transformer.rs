use crate::config::Cfg;
use crate::ppo::model::{Actor, Input, Output};
use burn::config::Config;
use burn::nn::Linear;
use burn::prelude::{Backend, Module};

#[derive(Config, Debug)]
pub struct TransformerActorConfig {
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
    #[config(default = 0.3)]
    pub dropout: f64,
    /// Dimensionality of the learned action embedding.
    #[config(default = 32)]
    pub action_embed_dim: usize,
    /// Positional embedding table size — must exceed max episode length + 1.
    #[config(default = 64)]
    pub max_seq_len: usize,
}
#[derive(Module, Debug)]
pub(crate) struct TransformerActor<B: Backend> {
    value: Linear<B>,
}

impl<AD: Backend> Actor for TransformerActor<AD> {
    type Config = TransformerActorConfig;
    fn init<B: Backend>(cfg: Self::Config, device: &B::Device) -> Self {
        todo!()
    }
    fn forward<B: Backend>(&self, cfg: &Cfg, input: Input<B>) -> Output<B> {
        todo!()
    }
}
