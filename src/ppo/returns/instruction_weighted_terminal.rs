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

        // Compute per-slot deltas up front.
        let deltas: Vec<f32> = (0..results.ep_len).map(|t| {
            let before = results.instr_counts.get(t).copied().unwrap_or(0) as f32;
            let after  = results.instr_counts.get(t + 1).copied().unwrap_or(0) as f32;
            before - after  // positive = instructions removed = good
        }).collect();

        // Only instruction-removing steps share the terminal signal.
        // weight = d.max(0) / total_pos, so:
        //   - no-ops and instruction-adders always get 0
        //   - return sign always matches terminal sign (no sign flips)
        //   - return bounded to [terminal, 0] or [0, terminal] ⊆ [-1, 1]
        let total_pos: f32 = deltas.iter().map(|&d| d.max(0.0)).sum();
        if total_pos == 0.0 {
            return vec![0.0; results.ep_len];
        }

        deltas.iter().map(|&d| (d.max(0.0) / total_pos) * terminal).collect()
    }
}
