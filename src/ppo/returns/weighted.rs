use crate::ppo::episode::Results;
use crate::ppo::noop::NoOp;
use crate::ppo::returns::Returns;

/// All steps receive the terminal as a base return so the value function
/// learns episode quality uniformly. A fixed differential is then added on top
/// so advantages reflect step-level contribution regardless of terminal sign:
///
///   d > 0 (reduce)  → terminal + direction_bonus
///   d < 0 (increase)→ terminal - direction_bonus
///   d == 0, active  → terminal
///   is_noop         → terminal - noop.penalty  (always worse than active steps)
pub(crate) struct Weighted {
    pub(crate) noop: NoOp,
    pub(crate) direction_bonus: f32,
}

impl Returns for Weighted {
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
                    let penalty = if self.noop.penalty > 0.0
                        && results
                            .actions
                            .get(t)
                            .copied()
                            .unwrap_or(crate::llvm::pass::Pass::Stop)
                            != crate::llvm::pass::Pass::Stop
                    {
                        self.noop.penalty
                    } else {
                        0.0
                    };
                    terminal - penalty
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
