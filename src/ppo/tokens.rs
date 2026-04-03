use crate::llvm::ir::Ir;
use crate::llvm::pass::Pass;

pub(crate) struct Tokens {
    // base ir + action sequence of passes
}
impl Tokens {
    pub(crate) fn new(ir: &Ir, actions: &[Pass]) -> Self {
        Self {}
    }
}
