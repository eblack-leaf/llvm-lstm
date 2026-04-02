use crate::config::Cfg;
use crate::ppo::model::{Actor, Input, Output};
use burn::prelude::Backend;

pub(crate) struct GruActor {}
impl Actor for GruActor {
    type Config = ();
    fn init<B: Backend>(cfg: Self::Config, device: &B::Device) -> Self {
        todo!()
    }
    fn forward<B: Backend>(&self, cfg: &Cfg, input: Input<B>) -> Output<B> {
        todo!()
    }
}
