use crate::ppo::episode::Results;
use crate::ppo::returns::Returns;

/// Blends a per-step instruction-count reduction signal with the terminal speedup.
///
/// For slot t: return[t] = alpha * terminal_speedup + (1 - alpha) * instr_delta[t]
/// where instr_delta[t] = (instr[t] - instr[t+1]) / instr[0]
///   (positive = instructions removed = good)
///
/// alpha = 1.0 → pure terminal speedup (same as EpisodeReturn)
/// alpha = 0.0 → pure instruction-count proxy (dense, but no timing signal)
pub(crate) struct InstructionProxyReturn {
    pub(crate) alpha: f32,
}

impl Returns for InstructionProxyReturn {
    fn compute(&self, results: &Results) -> Vec<f32> {
        let base = results.instr_counts.first().copied().unwrap_or(1).max(1) as f32;
        let terminal = results.episode_return;

        (0..results.ep_len)
            .map(|t| {
                let before = results.instr_counts.get(t).copied().unwrap_or(0) as f32;
                let after = results.instr_counts.get(t + 1).copied().unwrap_or(0) as f32;
                // Normalize by whichever is larger: the original base or the current bloated
                // intermediate. Prevents inline-then-cleanup steps from getting delta >> 1.
                let norm = before.max(base).max(1.0);
                let delta = (before - after) / norm;
                self.alpha * terminal + (1.0 - self.alpha) * delta
            })
            .collect()
    }
}
