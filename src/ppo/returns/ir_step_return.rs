use crate::llvm::pass::Pass;
use crate::ppo::episode::Results;
use crate::ppo::noop::NoOp;
use crate::ppo::returns::Returns;

pub(crate) struct IrStepReturn {
    pub(crate) noop: NoOp,
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
                if self.noop.penalty > 0.0
                    && action != Pass::Stop
                    && self.noop.is_noop(
                        delta,
                        results.ir_features_per_step.get(t).map(Vec::as_slice),
                        results.ir_features_per_step.get(t + 1).map(Vec::as_slice),
                    )
                {
                    delta - self.noop.penalty
                } else {
                    delta
                }
            })
            .collect()
    }
}
