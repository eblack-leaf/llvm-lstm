pub(crate) mod rank;

use crate::ppo::episode::Results;

/// Produces a per-step advantage from pre-computed returns and the full Results batch.
///
/// Returns are passed in pre-computed (from the Returns implementor) so the
/// implementor does not have to re-derive them.  Results gives access to values,
/// actions, log_probs, and Step metadata — so implementors that want to correlate
/// advantages with IR-state changes, pass no-op flags, or historical orderings can
/// do so without a signature change.
///
/// Takes the full batch so cross-episode context (rank normalisation, population
/// statistics) is available to any implementor that needs it.
///
/// Output: per-step advantages for each episode, same shape as returns.
pub(crate) trait Advantages {
    fn compute(&self, returns: &[Vec<f32>], results: &[Results]) -> Vec<Vec<f32>>;
}
