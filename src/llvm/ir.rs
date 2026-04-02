use std::path::PathBuf;
#[derive(Clone)]
pub(crate) struct Source {
    pub(crate) file: PathBuf,
}
pub(crate) struct Bin {
    pub(crate) file: PathBuf,
}
#[derive(Clone)]
pub(crate) struct Ir {
    pub(crate) file: PathBuf,
}
impl Ir {
    pub(crate) fn features(&self) -> Features {
        todo!()
    }
}
pub(crate) struct Features {
    pub(crate) vector: Vec<f32>,
}
