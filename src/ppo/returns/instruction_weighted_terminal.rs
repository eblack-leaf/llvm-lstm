use crate::ppo::episode::Results;
use crate::ppo::noop::NoOp;
use crate::ppo::returns::Returns;

/// Active (non-noop) steps receive the full terminal return, gated by whether
/// the step had any influence (threshold-based, not magnitude-proportional).
/// A fixed direction bonus is added for instruction reductions and subtracted
/// for instruction increases, giving the policy a local signal independent of
/// the terminal outcome.
///
///   is_noop         → 0  (or -noop.penalty if penalty > 0 and action != Stop)
///   d > 0 (reduce)  → terminal + direction_bonus
///   d < 0 (increase)→ terminal - direction_bonus
///   d == 0, active  → terminal  (feature-only change, no directional signal)
pub(crate) struct InstructionWeightedTerminal {
    pub(crate) noop: NoOp,
    pub(crate) direction_bonus: f32,
}

impl Returns for InstructionWeightedTerminal {
    fn compute(&self, results: &Results) -> Vec<f32> {
        let terminal = results.episode_return;
        let base = results.instr_counts.first().copied().unwrap_or(1).max(1) as f32;

        (0..results.ep_len)
            .map(|t| {
                let before = results.instr_counts.get(t).copied().unwrap_or(0) as f32;
                let after = results.instr_counts.get(t + 1).copied().unwrap_or(0) as f32;
                let d = (before - after) / base;

                let is_noop = self.noop.is_noop(
                    d,
                    results.ir_features_per_step.get(t).map(Vec::as_slice),
                    results.ir_features_per_step.get(t + 1).map(Vec::as_slice),
                );

                if is_noop {
                    if self.noop.penalty > 0.0
                        && results
                            .actions
                            .get(t)
                            .copied()
                            .unwrap_or(crate::llvm::pass::Pass::Stop)
                            != crate::llvm::pass::Pass::Stop
                    {
                        -self.noop.penalty
                    } else {
                        0.0
                    }
                } else if d > 0.0 {
                    terminal + self.direction_bonus
                } else if d < 0.0 {
                    terminal - self.direction_bonus
                } else {
                    terminal
                }
            })
            .collect()
    }
}
