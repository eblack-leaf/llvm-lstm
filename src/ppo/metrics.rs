use crate::ppo::episode::Results;
use crate::ppo::model::ACTIONS;

/// Scalar losses averaged over all ppo_epochs × mini-batches for one outer epoch.
pub(crate) struct PpoLosses {
    pub(crate) policy_loss: f32,
    pub(crate) value_loss: f32,
    pub(crate) entropy: f32,
    pub(crate) kl_div: f32,
}

struct RunningAvg {
    sum: f64,
    count: u64,
}

impl RunningAvg {
    fn new() -> Self { Self { sum: 0.0, count: 0 } }
    fn push(&mut self, v: f32) { self.sum += v as f64; self.count += 1; }
    fn mean(&self) -> f32 {
        if self.count == 0 { 0.0 } else { (self.sum / self.count as f64) as f32 }
    }
    fn reset(&mut self) { self.sum = 0.0; self.count = 0; }
}

struct Ema {
    value: f32,
    alpha: f32,
    initialized: bool,
}

impl Ema {
    fn new(alpha: f32) -> Self { Self { value: 0.0, alpha, initialized: false } }
    fn update(&mut self, x: f32) {
        if !self.initialized { self.value = x; self.initialized = true; }
        else { self.value = self.alpha * x + (1.0 - self.alpha) * self.value; }
    }
    fn get(&self) -> f32 { self.value }
}

pub(crate) struct Metrics {
    pub(crate) epoch: usize,

    // Per-epoch loss averages (reset in next_epoch)
    policy_loss_avg:   RunningAvg,
    value_loss_avg:    RunningAvg,
    entropy_avg:       RunningAvg,
    kl_div_avg:        RunningAvg,

    // Per-epoch explained variance (reset in next_epoch)
    explained_var_avg: RunningAvg,

    // Cross-epoch EMA on final episode speedup
    speedup_ema: Ema,

    // Per-epoch episode stats (reset in next_epoch)
    episode_len_avg:   RunningAvg,
    final_speedup_avg: RunningAvg,

    // Per-epoch timing (ms), reset in next_epoch
    pub(crate) per_func_ir_ms_total: u64,
    pub(crate) per_func_ir_ms_count: u32,
    pub(crate) episode_collection_ms: u64,
    pub(crate) ppo_update_ms: u64,

    // Cumulative wall time across all epochs (never reset)
    pub(crate) total_elapsed_ms: u64,
}

impl Metrics {
    pub(crate) fn new(ema_alpha: f32) -> Self {
        Self {
            epoch: 0,
            policy_loss_avg:   RunningAvg::new(),
            value_loss_avg:    RunningAvg::new(),
            entropy_avg:       RunningAvg::new(),
            kl_div_avg:        RunningAvg::new(),
            explained_var_avg: RunningAvg::new(),
            speedup_ema:       Ema::new(ema_alpha),
            episode_len_avg:   RunningAvg::new(),
            final_speedup_avg: RunningAvg::new(),
            per_func_ir_ms_total: 0,
            per_func_ir_ms_count: 0,
            episode_collection_ms: 0,
            ppo_update_ms: 0,
            total_elapsed_ms: 0,
        }
    }

    pub(crate) fn update_episode(&mut self, results: &[Results]) {
        for r in results {
            self.episode_len_avg.push(r.steps.len() as f32);
            if let Some(speedup) = r
                .steps.last()
                .and_then(|s| s.benchmark.as_ref())
                .map(|b| b.speedup)
            {
                self.final_speedup_avg.push(speedup);
                self.speedup_ema.update(speedup);
            }
        }
    }

    /// Record explained variance computed from rollout values vs computed returns.
    pub(crate) fn update_explained_variance(&mut self, ev: f32) {
        self.explained_var_avg.push(ev);
    }

    pub(crate) fn update_ppo(&mut self, losses: PpoLosses) {
        self.policy_loss_avg.push(losses.policy_loss);
        self.value_loss_avg.push(losses.value_loss);
        self.entropy_avg.push(losses.entropy);
        self.kl_div_avg.push(losses.kl_div);
    }

    pub(crate) fn record_func_ir_ms(&mut self, ms: u64) {
        self.per_func_ir_ms_total += ms;
        self.per_func_ir_ms_count += 1;
    }

    pub(crate) fn record_collection_ms(&mut self, ms: u64) {
        self.episode_collection_ms = ms;
    }

    pub(crate) fn record_ppo_ms(&mut self, ms: u64) {
        self.ppo_update_ms = ms;
    }

    /// Advance epoch, accumulate timing into total, reset per-epoch accumulators.
    /// The speedup EMA and total_elapsed_ms are NOT reset.
    pub(crate) fn next_epoch(&mut self) {
        self.total_elapsed_ms += self.episode_collection_ms + self.ppo_update_ms;
        self.epoch += 1;
        self.policy_loss_avg.reset();
        self.value_loss_avg.reset();
        self.entropy_avg.reset();
        self.kl_div_avg.reset();
        self.explained_var_avg.reset();
        self.episode_len_avg.reset();
        self.final_speedup_avg.reset();
        self.episode_collection_ms = 0;
        self.ppo_update_ms = 0;
    }

    pub(crate) fn policy_loss(&self) -> f32 { self.policy_loss_avg.mean() }
    pub(crate) fn value_loss(&self) -> f32 { self.value_loss_avg.mean() }
    pub(crate) fn entropy(&self) -> f32 { self.entropy_avg.mean() }
    /// Entropy as a percentage of the uniform-policy maximum (ln(|A|)).
    pub(crate) fn entropy_pct(&self) -> f32 {
        let max_entropy = (ACTIONS.len() as f32).ln();
        self.entropy_avg.mean() / max_entropy * 100.0
    }
    pub(crate) fn kl_div(&self) -> f32 { self.kl_div_avg.mean() }
    pub(crate) fn explained_variance(&self) -> f32 { self.explained_var_avg.mean() }
    pub(crate) fn speedup_ema(&self) -> f32 { self.speedup_ema.get() }
    pub(crate) fn avg_episode_len(&self) -> f32 { self.episode_len_avg.mean() }
    pub(crate) fn avg_final_speedup(&self) -> f32 { self.final_speedup_avg.mean() }
    pub(crate) fn avg_func_ir_ms(&self) -> f32 {
        if self.per_func_ir_ms_count == 0 { 0.0 }
        else { self.per_func_ir_ms_total as f32 / self.per_func_ir_ms_count as f32 }
    }
}

/// Explained variance of predicted values relative to actual returns.
/// EV = 1 − Var(returns − values) / Var(returns).
/// Range: (−∞, 1]. 1 = perfect, 0 = no better than constant, negative = harmful.
pub(crate) fn explained_variance(returns: &[f32], values: &[f32]) -> f32 {
    let n = returns.len().min(values.len());
    if n == 0 { return 0.0; }
    let var_ret = variance(&returns[..n]);
    if var_ret == 0.0 { return 0.0; }
    let residuals: Vec<f32> = returns[..n].iter().zip(&values[..n]).map(|(r, v)| r - v).collect();
    1.0 - variance(&residuals) / var_ret
}

fn variance(xs: &[f32]) -> f32 {
    if xs.is_empty() { return 0.0; }
    let mean = xs.iter().sum::<f32>() / xs.len() as f32;
    xs.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / xs.len() as f32
}
