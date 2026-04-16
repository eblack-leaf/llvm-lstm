use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Per-step instruction-count delta: return[t] = (instr[t] - instr[t+1]) / instr[0].
///
/// Positive when step t removed instructions. Zero when it had no effect.
/// Negative when step t *added* instructions.
///
/// This is the densest possible IR-based signal — every step gets its own
/// reward reflecting its marginal contribution to code size reduction.
/// Best paired with the autoregressive collection path, but also valid for
/// the parallel path (same values, different learning dynamics).
pub(crate) struct IrStepReturn;

impl Returns for IrStepReturn {
    fn compute(&self, results: &Results) -> Vec<f32> {
        let base = results.instr_counts.first().copied().unwrap_or(1).max(1) as f32;
        results
            .instr_counts
            .windows(2)
            .take(results.ep_len)
            .map(|w| (w[0] as f32 - w[1] as f32) / base)
            .collect()
    }
}
