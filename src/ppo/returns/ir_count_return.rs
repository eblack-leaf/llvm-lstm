use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Uses the terminal instruction-count reduction as the episode return,
/// distributed uniformly across all executed slots.
///
/// return[t] = (instr[0] - instr[ep_len]) / instr[0]
///
/// Positive = instructions removed = good.
/// No benchmark is run; the signal is purely structural.
pub(crate) struct IrCountReturn;

impl Returns for IrCountReturn {
    fn compute(&self, results: &Results) -> Vec<f32> {
        let base = results.instr_counts.first().copied().unwrap_or(1).max(1) as f32;
        let final_count = results.instr_counts.last().copied().unwrap_or(0) as f32;
        let r = (base - final_count) / base;
        vec![r; results.ep_len]
    }
}
