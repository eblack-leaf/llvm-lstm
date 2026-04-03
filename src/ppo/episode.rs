use crate::config::Cfg;
use crate::llvm::Llvm;
use crate::llvm::ir::Ir;
use crate::llvm::pass::Pass;
use crate::ppo::step::Step;

pub(crate) struct Episode {
    pub(crate) llvm: Llvm,
    pub(crate) ir: Ir,
    pub(crate) cfg: Cfg,
    pub(crate) steps: Vec<Step>,
    // Initialised with Start so the model always sees a non-empty sequence.
    pub(crate) actions: Vec<Pass>,
    pub(crate) log_probs: Vec<f32>,
    // V(s_t) estimate from the critic at each step; used to compute advantages
    pub(crate) values: Vec<f32>,
}
impl Episode {
    pub(crate) fn new(idx: usize, llvm: Llvm, ir: Ir, cfg: Cfg) -> Self {
        Self {
            llvm,
            ir,
            cfg,
            steps: vec![],
            actions: vec![Pass::Start],
            log_probs: vec![],
            values: vec![],
        }
    }
    pub(crate) fn results(self) -> Results {
        Results {
            actions: self.actions,
            log_probs: self.log_probs,
            values: self.values,
            steps: self.steps,
        }
    }
}

pub(crate) struct Results {
    pub(crate) actions: Vec<Pass>,
    pub(crate) log_probs: Vec<f32>,
    pub(crate) values: Vec<f32>,
    /// Full step record.
    /// len == 1  → episode-level benchmark only (the trusted signal).
    /// len == T  → per-step benchmarks; cumulative state after passes 1..=t,
    ///             not the marginal contribution of pass t.
    /// Step carries metadata beyond just the benchmark score — future Returns
    /// implementors can use it for attribution (IR deltas, pass no-op flags, etc.).
    pub(crate) steps: Vec<Step>,
}
