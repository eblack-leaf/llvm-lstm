pub(crate) mod gru;
pub(crate) mod transformer;

use crate::config::Cfg;
use burn::Tensor;
use burn::prelude::{Backend, Int};
pub(crate) struct Input<B: Backend> {
    pub(crate) features: Tensor<B, 2>,
    pub(crate) actions: Tensor<B, 2, Int>,
}
impl<B: Backend> Input<B> {
    pub(crate) fn new(dev: &B::Device) -> Self {
        Self {
            features: Tensor::from_data([[1]], dev),
            actions: Tensor::from_data([[1]], dev),
        }
    }
}
pub(crate) struct Output<B: Backend> {
    pub(crate) policy: Tensor<B, 3>,
    pub(crate) value: Tensor<B, 2>,
}
pub(crate) trait Actor {
    type Config;
    fn init<B: Backend>(cfg: Self::Config, device: &B::Device) -> Self;
    fn forward<B: Backend>(&self, cfg: &Cfg, input: Input<B>) -> Output<B>;
}
