use crate::config::Cfg;
use crate::llvm::Llvm;
use crate::llvm::benchmark::Baselines;
use crate::llvm::ir::{Features, Ir};
use crate::llvm::pass::Pass;
use crate::ppo::step::Step;

pub(crate) struct Episode {
    pub(crate) llvm: Llvm,
    /// Base (unoptimised) IR — constant for the episode lifetime.
    pub(crate) ir: Ir,
    /// Current IR state, updated by apply_one after every step.
    /// Starts as a clone of ir; diverges as passes are applied.
    pub(crate) current_ir: Ir,
    pub(crate) cfg: Cfg,
    pub(crate) steps: Vec<Step>,
    // Initialised with Start so the model always sees a non-empty sequence.
    pub(crate) actions: Vec<Pass>,
    pub(crate) log_probs: Vec<f32>,
    // V(s_t) estimate from the critic at each step; used to compute advantages
    pub(crate) values: Vec<f32>,
    /// Per-function baseline timings; carried so the train loop can compute
    /// speedup and so Returns implementors can compare against any opt level.
    pub(crate) baselines: Baselines,
    /// Feature vector of the base IR, parsed once at construction.
    /// Subtracted from the current IR features each step to form delta_features on Step.
    pub(crate) base_features: Vec<f32>,
}
impl Episode {
    pub(crate) async fn new(
        idx: usize,
        llvm: Llvm,
        ir: Ir,
        cfg: Cfg,
        baselines: Baselines,
    ) -> Self {
        tokio::fs::create_dir_all(&llvm.work_dir)
            .await
            .expect("failed to create worker dir");
        let content = tokio::fs::read_to_string(&ir.file)
            .await
            .expect("failed to read base IR");
        let base_features = Features::from_ll_str(&content)
            .expect("failed to parse base IR features")
            .to_vec();
        let current_ir = ir.clone();
        Self {
            llvm,
            current_ir,
            ir,
            cfg,
            steps: vec![],
            actions: vec![Pass::Start],
            log_probs: vec![],
            values: vec![],
            baselines,
            base_features,
        }
    }
    pub(crate) fn results(self) -> Results {
        Results {
            actions: self.actions,
            log_probs: self.log_probs,
            values: self.values,
            steps: self.steps,
            baselines: self.baselines,
            base_features: self.base_features,
        }
    }
}

pub(crate) struct Results {
    pub(crate) actions: Vec<Pass>,
    pub(crate) log_probs: Vec<f32>,
    pub(crate) values: Vec<f32>,
    /// Full step record. Each Step carries pass, step_idx, and an optional
    /// benchmark (Some when per_step_benchmark or at episode end, else None).
    pub(crate) steps: Vec<Step>,
    /// Per-function baselines; available to Returns implementors to compare
    /// the episode reward against any standard opt level.
    pub(crate) baselines: Baselines,
    /// Feature vector of the base IR; combined with Step::delta_features to
    /// reconstruct the full 68-dim model input for each step during PPO update.
    pub(crate) base_features: Vec<f32>,
}
