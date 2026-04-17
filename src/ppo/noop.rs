/// Unified no-op detection used by returns, metrics, and the weighted return.
///
/// A step is a no-op when BOTH:
///   1. |instr_delta| < count_threshold  — instruction count barely moved
///   2. L1(feat_before, feat_after) < feature_threshold — IR structure barely moved
///
/// Using both avoids penalising passes like loop-rotate that leave count
/// unchanged but meaningfully restructure the IR for later passes.
/// If feature vectors are unavailable, the feature condition is skipped
/// (treated as infinity → never a feature no-op alone).
#[derive(Clone, Copy, Debug)]
pub(crate) struct NoOp {
    /// |instr_delta| below this is candidate for no-op.
    pub(crate) count_threshold: f32,
    /// L1 feature distance below this confirms structural no-op.
    /// Set to f32::INFINITY to rely on count only.
    pub(crate) feature_threshold: f32,
    /// Penalty applied to no-op steps (subtracted from return). 0 = disabled.
    pub(crate) penalty: f32,
}

impl NoOp {
    pub(crate) fn is_noop(
        &self,
        count_delta: f32,
        feat_before: Option<&[f32]>,
        feat_after: Option<&[f32]>,
    ) -> bool {
        if count_delta.abs() >= self.count_threshold {
            return false;
        }
        let feat_dist = match (feat_before, feat_after) {
            (Some(a), Some(b)) => a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f32>(),
            _ => f32::INFINITY,
        };
        feat_dist < self.feature_threshold
    }
}
