use crate::llvm::benchmark::Baselines;
use crate::llvm::pass::Pass;

pub(crate) struct Results {
    pub(crate) func_name: String,
    pub(crate) bench_cache_hits: u64,
    pub(crate) bench_cache_misses: u64,
    pub(crate) ir_features: Vec<f32>, // kept for PredictorReturn
    pub(crate) ir_opcodes: Vec<u8>,   // opcode-ID sequence for actor input
    /// ep_len actions actually executed (index of first Stop + 1, or K if no Stop).
    /// Parallel to log_probs. Slots past ep_len were never applied or trained.
    pub(crate) actions: Vec<Pass>,
    pub(crate) log_probs: Vec<f32>,
    /// Number of slots in this episode (= actions.len() = log_probs.len()).
    pub(crate) ep_len: usize,
    /// Per-slot value estimates V_t from the rollout, length = ep_len.
    pub(crate) values: Vec<f32>,
    pub(crate) episode_return: f32,
    pub(crate) baselines: Baselines,
    /// Instruction count at each step: instr_counts[0] = base IR, instr_counts[t+1] = after step t.
    /// Length = ep_len + 1 (base + one per executed action).
    pub(crate) instr_counts: Vec<usize>,
}
