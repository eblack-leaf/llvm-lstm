use crate::config::{BurnAutoDiff, BurnBackend, BurnDevice, Cfg};
use crate::ppo::model::{Actor, Input, Output};
use burn::module::AutodiffModule;
use burn::nn::Linear;
use burn::prelude::{Config, Module};

#[derive(Config, Debug)]
pub(crate) struct GruActorConfig {}
#[derive(Module, Debug, Clone)]
pub(crate) struct GruActor {
    linear: Linear<BurnBackend>,
}
impl Actor for GruActor
where
    Self: AutodiffModule<BurnAutoDiff>,
{
    type Config = GruActorConfig;
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
        <GruActor as AutodiffModule<BurnAutoDiff>>::valid(&self)
    }
}
