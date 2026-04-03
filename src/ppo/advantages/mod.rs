pub(crate) mod rank;

/// Produces a per-step advantage from pre-computed returns and value estimates.
///
/// Takes the full batch so implementors that need cross-episode context (rank
/// normalisation, population statistics) have it available.
///
/// Input:  parallel slices of (returns[t], values[t]) for each episode.
/// Output: per-step advantages for each episode, same shape as input.
pub(crate) trait Advantages {
    fn compute(&self, batch: &[(Vec<f32>, Vec<f32>)]) -> Vec<Vec<f32>>;
}
