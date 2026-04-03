use crate::config::Cfg;
use crate::ppo::model::{Actor, Input, Output};
use burn::module::AutodiffModule;
use burn::nn::Linear;
use burn::prelude::{Backend, Config, Module};
use burn::tensor::backend::AutodiffBackend;

#[derive(Config, Debug)]
pub(crate) struct GruActorConfig {}
#[derive(Module, Debug)]
pub(crate) struct GruActor<B: Backend> {
    linear: Linear<B>,
}
impl<AD: Backend + AutodiffBackend<InnerBackend = AD>> Actor for GruActor<AD> {
    type Config = GruActorConfig;
    fn init<B: Backend>(cfg: Self::Config, device: &B::Device) -> Self {
        todo!()
    }
    fn forward<B: Backend>(&self, cfg: &Cfg, input: Input<B>) -> Output<B> {
        todo!()
    }

    fn cfg(cfg: &Cfg) -> Self::Config {
        todo!()
    }

    fn no_grads(&self) -> Self {
        self.valid()
    }
}
