use crate::llvm::ir::{Features, Ir};
use crate::llvm::pass::Pass;

pub(crate) struct Tokens {
    // Concatenation of [base_features (34), delta_features (34)] = 68-dim vector.
    // base_features: log-transformed IR counts for the unoptimised function.
    // delta_features: current - base, element-wise. Zero at step 0 (no passes applied yet);
    //   grows as passes change the IR. Encodes optimisation progress without redundancy.
    pub(crate) features: Vec<f32>,
    // Action history as integer indices (Pass discriminant values)
    pub(crate) actions: Vec<i64>,
}
impl Tokens {
    pub(crate) fn new(ir: &Ir, current_ir: &Ir, actions: &[Pass]) -> Self {
        let base_content = std::fs::read_to_string(&ir.file).expect("failed to read base IR");
        let current_content = std::fs::read_to_string(&current_ir.file).expect("failed to read current IR");
        let base = Features::from_ll_str(&base_content)
            .expect("failed to parse base IR features")
            .to_vec();
        let current = Features::from_ll_str(&current_content)
            .expect("failed to parse current IR features")
            .to_vec();
        let delta: Vec<f32> = base.iter().zip(&current).map(|(b, c)| c - b).collect();
        let features = base.into_iter().chain(delta).collect();
        let actions = actions.iter().map(|p| *p as i64).collect();
        Self { features, actions }
    }
}
