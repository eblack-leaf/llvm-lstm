use crate::llvm::ir::{Features, Ir};
use crate::llvm::pass::Pass;

pub(crate) struct Tokens {
    // IR feature vector (log-transformed counts + derived ratios, length = 34)
    pub(crate) features: Vec<f32>,
    // Action history as integer indices (Pass discriminant values)
    pub(crate) actions: Vec<i64>,
}
impl Tokens {
    pub(crate) async fn new(ir: &Ir, actions: &[Pass]) -> Self {
        let content = tokio::fs::read_to_string(&ir.file)
            .await
            .expect("failed to read IR file");
        let features = Features::from_ll_str(&content)
            .expect("failed to parse IR features")
            .to_vec();
        let actions = actions.iter().map(|p| *p as i64).collect();
        Self { features, actions }
    }
}
