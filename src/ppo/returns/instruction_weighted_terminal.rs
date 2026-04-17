use crate::ppo::episode::Results;
use crate::ppo::noop::NoOp;
use crate::ppo::returns::Returns;

/// Distributes the terminal speedup across slots weighted by each slot's
/// share of total instruction reduction, using normalised fractional deltas.
///
/// return[t] = (frac_delta[t] / total_frac_delta) * terminal_speedup
///
/// where frac_delta[t] = (instr[t] - instr[t+1]) / instr[0]
/// and   total_frac_delta = sum of positive frac_deltas
///
/// Slots that are structural no-ops (both count and feature unchanged)
/// receive 0 credit. Slots that increase instructions get negative credit.
pub(crate) struct InstructionWeightedTerminal {
    pub(crate) noop: NoOp,
}

impl Returns for InstructionWeightedTerminal {
    fn compute(&self, results: &Results) -> Vec<f32> {
        let terminal = results.episode_return;
        let base = results.instr_counts.first().copied().unwrap_or(1).max(1) as f32;

        let frac_deltas: Vec<f32> = (0..results.ep_len)
            .map(|t| {
                let before = results.instr_counts.get(t).copied().unwrap_or(0) as f32;
                let after  = results.instr_counts.get(t + 1).copied().unwrap_or(0) as f32;
                (before - after) / base
            })
            .collect();

        let total_positive: f32 = frac_deltas.iter().filter(|&&d| d > 0.0).sum();
        if total_positive == 0.0 {
            return vec![0.0; results.ep_len];
        }

        (0..results.ep_len)
            .map(|t| {
                let d = frac_deltas[t];
                let is_noop = self.noop.is_noop(
                    d,
                    results.ir_features_per_step.get(t).map(Vec::as_slice),
                    results.ir_features_per_step.get(t + 1).map(Vec::as_slice),
                );
                if is_noop {
                    if self.noop.penalty > 0.0
                        && results.actions.get(t).copied()
                            .unwrap_or(crate::llvm::pass::Pass::Stop)
                            != crate::llvm::pass::Pass::Stop
                    {
                        -self.noop.penalty
                    } else {
                        0.0
                    }
                } else {
                    (d / total_positive) * terminal
                }
            })
            .collect()
    }
}
