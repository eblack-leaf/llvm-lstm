pub(crate) mod baseline;
pub(crate) mod gae;
pub(crate) mod lookahead;
pub(crate) mod rank;

use crate::ppo::episode::Results;
use crate::ppo::BatchStep;

/// Produces a per-step advantage from pre-computed returns and the full Results batch.
///
/// Two entry points:
///
/// `compute` — called once after episode collection, over full episode data with
/// rollout values. Used for metrics/logging and for implementors (e.g. RankAdvantage)
/// that require cross-episode context not available inside a mini-batch.
///
/// `compute_live` — called inside the PPO update loop each mini-batch, with the
/// model's *current* value predictions for that chunk. This keeps the advantage
/// baseline fresh as V improves across PPO epochs, preventing policy gradients
/// from chasing a stale baseline. Implementors that only need `ret` and `pred_v`
/// (e.g. BaselineAdvantage) do all their real work here; implementors that require
/// cross-episode structure (e.g. RankAdvantage) fall back to `ret - pred_v`.
pub(crate) trait Advantages {
    /// Full-batch pre-compute for metrics/logging. Returns per-episode per-step advantages.
    fn compute(&self, returns: &[Vec<f32>], results: &[Results]) -> Vec<Vec<f32>>;

    /// Per-mini-batch live compute with fresh value predictions.
    /// `pred_v[i]` is the model's current V estimate for `steps[i]`.
    /// Default: `ret - pred_v` — override to apply normalisation or a different formula.
    fn compute_live(&self, steps: &[BatchStep], pred_v: &[f32]) -> Vec<f32> {
        steps.iter().zip(pred_v).map(|(s, &v)| s.ret - v).collect()
    }
}
