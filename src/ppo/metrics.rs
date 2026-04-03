use crate::ppo::episode::Results;

/// Scalar losses averaged over all ppo_epochs × batch steps for one outer epoch.
pub(crate) struct PpoLosses {
    pub(crate) policy_loss: f32,
    pub(crate) value_loss: f32,
    pub(crate) entropy: f32,
}

struct RunningAvg {
    sum: f64,
    count: u64,
}

impl RunningAvg {
    fn new() -> Self {
        Self { sum: 0.0, count: 0 }
    }
    fn push(&mut self, v: f32) {
        self.sum += v as f64;
        self.count += 1;
    }
    fn mean(&self) -> f32 {
        if self.count == 0 { 0.0 } else { (self.sum / self.count as f64) as f32 }
    }
    fn reset(&mut self) {
        self.sum = 0.0;
        self.count = 0;
    }
}

struct Ema {
    value: f32,
    alpha: f32,
    initialized: bool,
}

impl Ema {
    fn new(alpha: f32) -> Self {
        Self { value: 0.0, alpha, initialized: false }
    }
    fn update(&mut self, x: f32) {
        if !self.initialized {
            self.value = x;
            self.initialized = true;
        } else {
            self.value = self.alpha * x + (1.0 - self.alpha) * self.value;
        }
    }
    fn get(&self) -> f32 {
        self.value
    }
}

pub(crate) struct Metrics {
    pub(crate) epoch: usize,

    // Per-epoch loss averages (reset in next_epoch)
    policy_loss_avg: RunningAvg,
    value_loss_avg: RunningAvg,
    entropy_avg: RunningAvg,

    // Cross-epoch EMA on final episode speedup
    speedup_ema: Ema,

    // Per-epoch episode stats (reset in next_epoch)
    episode_len_avg: RunningAvg,
    final_speedup_avg: RunningAvg,

    // Timing (ms), reset in next_epoch
    pub(crate) per_func_ir_ms_total: u64,
    pub(crate) per_func_ir_ms_count: u32,
    pub(crate) episode_collection_ms: u64,
    pub(crate) ppo_update_ms: u64,
}

impl Metrics {
    pub(crate) fn new(ema_alpha: f32) -> Self {
        Self {
            epoch: 0,
            policy_loss_avg: RunningAvg::new(),
            value_loss_avg: RunningAvg::new(),
            entropy_avg: RunningAvg::new(),
            speedup_ema: Ema::new(ema_alpha),
            episode_len_avg: RunningAvg::new(),
            final_speedup_avg: RunningAvg::new(),
            per_func_ir_ms_total: 0,
            per_func_ir_ms_count: 0,
            episode_collection_ms: 0,
            ppo_update_ms: 0,
        }
    }

    /// Accumulate per-epoch episode statistics from all episode results.
    /// Extracts episode length and final-step speedup (skips episodes without a
    /// terminal benchmark). Updates the cross-epoch speedup EMA.
    pub(crate) fn update_episode(&mut self, results: &[Results]) {
        for r in results {
            self.episode_len_avg.push(r.steps.len() as f32);
            if let Some(speedup) = r
                .steps
                .last()
                .and_then(|s| s.benchmark.as_ref())
                .map(|b| b.speedup)
            {
                self.final_speedup_avg.push(speedup);
                self.speedup_ema.update(speedup);
            }
        }
    }

    /// Accumulate PPO loss statistics for one update call.
    pub(crate) fn update_ppo(&mut self, losses: PpoLosses) {
        self.policy_loss_avg.push(losses.policy_loss);
        self.value_loss_avg.push(losses.value_loss);
        self.entropy_avg.push(losses.entropy);
    }

    /// Record per-function IR generation + baseline collection time.
    pub(crate) fn record_func_ir_ms(&mut self, ms: u64) {
        self.per_func_ir_ms_total += ms;
        self.per_func_ir_ms_count += 1;
    }

    /// Record the wall time for the full episode collection phase this epoch.
    pub(crate) fn record_collection_ms(&mut self, ms: u64) {
        self.episode_collection_ms = ms;
    }

    /// Record the wall time for the PPO update phase this epoch.
    pub(crate) fn record_ppo_ms(&mut self, ms: u64) {
        self.ppo_update_ms = ms;
    }

    /// Advance the epoch counter and reset all per-epoch accumulators.
    /// The speedup EMA is NOT reset — it persists across epochs.
    pub(crate) fn next_epoch(&mut self) {
        self.epoch += 1;
        self.policy_loss_avg.reset();
        self.value_loss_avg.reset();
        self.entropy_avg.reset();
        self.episode_len_avg.reset();
        self.final_speedup_avg.reset();
        self.episode_collection_ms = 0;
        self.ppo_update_ms = 0;
    }

    pub(crate) fn policy_loss(&self) -> f32 { self.policy_loss_avg.mean() }
    pub(crate) fn value_loss(&self) -> f32 { self.value_loss_avg.mean() }
    pub(crate) fn entropy(&self) -> f32 { self.entropy_avg.mean() }
    pub(crate) fn speedup_ema(&self) -> f32 { self.speedup_ema.get() }
    pub(crate) fn avg_episode_len(&self) -> f32 { self.episode_len_avg.mean() }
    pub(crate) fn avg_final_speedup(&self) -> f32 { self.final_speedup_avg.mean() }
    pub(crate) fn avg_func_ir_ms(&self) -> f32 {
        if self.per_func_ir_ms_count == 0 {
            0.0
        } else {
            self.per_func_ir_ms_total as f32 / self.per_func_ir_ms_count as f32
        }
    }
}
