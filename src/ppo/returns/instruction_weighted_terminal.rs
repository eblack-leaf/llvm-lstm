use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Distributes the terminal speedup across slots weighted by each slot's share of total
/// instruction reduction. No-op slots (delta = 0) get 0 terminal credit; slots that
/// removed more instructions get proportionally more.
///
/// return[t] = (delta[t] / total_net_delta) * terminal_speedup
///
/// where delta[t] = instr[t] - instr[t+1]  (positive = reduced = good)
/// and   total_net_delta = instr[0] - instr[ep_len]  (net reduction over whole episode)
///
/// If total_net_delta == 0 (no net change), all returns are 0.
/// If a slot increases instructions (negative delta), it receives negative credit
/// proportional to how much it undid prior work.
pub(crate) struct InstructionWeightedTerminal;

impl Returns for InstructionWeightedTerminal {
    fn compute(&self, results: &Results) -> Vec<f32> {
        let terminal = results.episode_return;

        // Compute per-slot deltas
        let deltas: Vec<f32> = (0..results.ep_len)
            .map(|t| {
                let before = results.instr_counts.get(t).copied().unwrap_or(0) as f32;
                let after = results.instr_counts.get(t + 1).copied().unwrap_or(0) as f32;
                before - after
            })
            .collect();

        let num_positive = deltas.iter().filter(|&&d| d > 0.0).count() as f32;
        if num_positive == 0.0 {
            return vec![0.0; results.ep_len];
        }

        let reward_per_positive = terminal / num_positive;
        deltas
            .iter()
            .map(|&d| if d > 0.0 { reward_per_positive } else { 0.0 })
            .collect()
    }
}
