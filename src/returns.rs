use crate::rollout::Rollout;

/// How to compute returns from a rollout's reward sequence.
#[derive(Debug, Clone, PartialEq)]
pub enum ReturnMode {
    /// Episode-level: G0 = Σ γ^t · r_t, broadcast to every step.
    /// Treats the full episode as the unit of credit.
    Episode,
    /// Per-step discounted: G_t = r_t + γ · G_{t+1}.
    /// More fine-grained credit assignment — useful when intermediate
    /// rewards are informative (e.g. instruction-proxy reward mode).
    PerStep,
}

impl ReturnMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "per-step" => ReturnMode::PerStep,
            _ => ReturnMode::Episode,
        }
    }
}

/// Computed return values aligned to the flat (episode × step) ordering.
pub struct Returns {
    /// One value per (episode, step) pair in rollout order.
    /// Episode mode: all steps in an episode share the same G0.
    /// PerStep mode: each step has its own G_t.
    pub values: Vec<f32>,
    /// Episode-level G0 per episode (index = episode index).
    /// Used for BestEpisodeStore updates and per-function EMA tracking.
    pub g0_per_ep: Vec<f32>,
}

impl Returns {
    pub fn compute(rollouts: &[Rollout], mode: &ReturnMode, gamma: f32) -> Self {
        let mut values = Vec::new();
        let mut g0_per_ep = Vec::new();

        for rollout in rollouts {
            let t_len = rollout.len();
            match mode {
                ReturnMode::Episode => {
                    let g0: f32 = rollout
                        .rewards
                        .iter()
                        .enumerate()
                        .map(|(t, &r)| r * gamma.powi(t as i32))
                        .sum();
                    g0_per_ep.push(g0);
                    for _ in 0..t_len {
                        values.push(g0);
                    }
                }
                ReturnMode::PerStep => {
                    let mut per_step = vec![0.0f32; t_len];
                    let mut running = 0.0f32;
                    for t in (0..t_len).rev() {
                        running = rollout.rewards[t] + gamma * running;
                        per_step[t] = running;
                    }
                    let g0 = per_step[0];
                    g0_per_ep.push(g0);
                    values.extend_from_slice(&per_step);
                }
            }
        }

        Self { values, g0_per_ep }
    }
}
