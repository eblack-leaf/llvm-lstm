use crate::ppo::episode::Results;
use crate::ppo::noop::NoOp;
use crate::ppo::returns::Returns;

pub(crate) struct InstructionProxyReturn {
    pub(crate) alpha: f32,
    pub(crate) noop: NoOp,
}

impl Returns for InstructionProxyReturn {
    fn compute(&self, results: &Results) -> Vec<f32> {
        let base = results.instr_counts.first().copied().unwrap_or(1).max(1) as f32;
        let terminal = results.episode_return;

        (0..results.ep_len)
            .map(|t| {
                let before = results.instr_counts.get(t).copied().unwrap_or(0) as f32;
                let after = results.instr_counts.get(t + 1).copied().unwrap_or(0) as f32;
                let norm = before.max(base).max(1.0);
                let delta = (before - after) / norm;
                let r = self.alpha * terminal + (1.0 - self.alpha) * delta;
                if self.noop.penalty > 0.0
                    && results
                        .actions
                        .get(t)
                        .copied()
                        .unwrap_or(crate::llvm::pass::Pass::Stop)
                        != crate::llvm::pass::Pass::Stop
                    && self.noop.is_noop(
                        delta,
                        results.ir_features_per_step.get(t).map(Vec::as_slice),
                        results.ir_features_per_step.get(t + 1).map(Vec::as_slice),
                    )
                {
                    r - self.noop.penalty
                } else {
                    r
                }
            })
            .collect()
    }
}
