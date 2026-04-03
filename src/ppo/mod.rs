use crate::config::Cfg;

pub(crate) mod advantages;
pub(crate) mod checkpoint;
pub(crate) mod episode;
pub(crate) mod logging;
pub(crate) mod metrics;
pub(crate) mod model;
pub(crate) mod returns;
pub(crate) mod step;
pub(crate) mod tokens;

pub(crate) struct Ppo {
    // data to run ppo process
}

impl Ppo {
    pub(crate) fn new(cfg: &Cfg) -> Self {
        // read all needed vars from cfg (no hidden defaults here)
        Self {}
    }
}
