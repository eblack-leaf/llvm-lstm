use crate::llvm::pass::Pass;
use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Per-step instruction-count delta: return[t] = (instr[t] - instr[t+1]) / instr[0].
///
/// Positive when step t removed instructions. Zero when it had no effect.
/// Negative when step t *added* instructions.
///
/// `noop_penalty`: subtracted from steps where |delta| < threshold and the
/// action is not Stop.  This makes Stop strictly preferable to repeating
/// passes that no longer help, teaching the policy to terminate rather than
/// run out the sequence.  Set to 0.0 to disable.
///
/// `noop_threshold`: |delta| below this is considered a wasted step.
pub(crate) struct IrStepReturn {
    pub(crate) noop_penalty: f32,
    pub(crate) noop_threshold: f32,
}

impl Returns for IrStepReturn {
    fn compute(&self, results: &Results) -> Vec<f32> {
        let base = results.instr_counts.first().copied().unwrap_or(1).max(1) as f32;
        results
            .instr_counts
            .windows(2)
            .take(results.ep_len)
            .enumerate()
            .map(|(t, w)| {
                let delta = (w[0] as f32 - w[1] as f32) / base;
                let action = results.actions.get(t).copied().unwrap_or(Pass::Stop);
                if self.noop_penalty > 0.0
                    && action != Pass::Stop
                    && delta.abs() < self.noop_threshold
                {
                    delta - self.noop_penalty
                } else {
                    delta
                }
            })
            .collect()
    }
}
