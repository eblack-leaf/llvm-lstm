use crate::ppo::episode::Results;
use crate::ppo::returns::StoreStats;
use std::collections::HashMap;
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

/// Snapshot of return/advantage distribution for one epoch.
pub(crate) struct RetAdvStats {
    pub(crate) ret_mean:      f32,
    /// Pre-normalisation return std (before compute_batch divides). None when not available.
    pub(crate) raw_ret_std:   Option<f32>,
    pub(crate) ret_min:       f32,
    pub(crate) ret_max:       f32,
    /// Fraction of steps whose |return| < 0.01 — effectively no attribution.
    pub(crate) noop_frac:     f32,
    pub(crate) adv_std:       f32,
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
    // Per-function speedup averages for the current epoch (reset in next_epoch)
    func_speedup_avgs: HashMap<String, RunningAvg>,

    // Return/advantage distribution snapshot for the most recent epoch.
    pub(crate) ret_adv: Option<RetAdvStats>,

    // Per-epoch lookahead cache stats (reset in next_epoch)
    lookahead_hits: u64,
    lookahead_misses: u64,
    // Per-epoch episode-end benchmark cache stats (reset in next_epoch)
    bench_cache_hits: u64,
    bench_cache_misses: u64,

    // Survivorship store snapshot — Some when using EpisodicPatternReturn (reset each epoch)
    pub(crate) store_stats: Option<StoreStats>,

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
            func_speedup_avgs: HashMap::new(),
            ret_adv:           None,
            store_stats:       None,
            lookahead_hits: 0,
            lookahead_misses: 0,
            bench_cache_hits: 0,
            bench_cache_misses: 0,
            per_func_ir_ms_total: 0,
            per_func_ir_ms_count: 0,
            episode_collection_ms: 0,
            ppo_update_ms: 0,
            total_elapsed_ms: 0,
        }
    }

    pub(crate) fn update_episode(&mut self, results: &[Results]) {
        let mut any_speedup = false;
        for r in results {
            self.episode_len_avg.push(r.steps.len() as f32);
            if let Some(speedup) = r
                .steps.last()
                .and_then(|s| s.benchmark.as_ref())
                .map(|b| b.speedup)
            {
                self.final_speedup_avg.push(speedup);
                self.func_speedup_avgs
                    .entry(r.func_name.clone())
                    .or_insert_with(RunningAvg::new)
                    .push(speedup);
                any_speedup = true;
            }
        }
        // Update EMA once per epoch with the epoch's mean speedup, not once per episode.
        // Updating per-episode with alpha=0.05 and 16 episodes would give an effective
        // per-epoch alpha of 1-(1-0.05)^16 ≈ 0.56, far more aggressive than intended.
        if any_speedup {
            self.speedup_ema.update(self.final_speedup_avg.mean());
        }
    }

    /// Record explained variance computed from rollout values vs computed returns.
    pub(crate) fn update_explained_variance(&mut self, ev: f32) {
        self.explained_var_avg.push(ev);
    }

    /// Snapshot the return and advantage distributions for this epoch.
    /// Call after computing returns/advantages, before the PPO update.
    /// raw_ret_std: pre-normalisation std from the Returns implementor, if available.
    pub(crate) fn update_returns_advs(
        &mut self,
        returns: &[Vec<f32>],
        advantages: &[Vec<f32>],
        raw_ret_std: Option<f32>,
    ) {
        let rets: Vec<f32> = returns.iter().flatten().copied().collect();
        let advs: Vec<f32> = advantages.iter().flatten().copied().collect();
        if rets.is_empty() {
            return;
        }

        let n = rets.len() as f32;
        let ret_mean = rets.iter().sum::<f32>() / n;
        let ret_min = rets.iter().cloned().fold(f32::INFINITY, f32::min);
        let ret_max = rets.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let noop_frac = rets.iter().filter(|r| r.abs() < 0.01).count() as f32 / n;

        let adv_std = if advs.is_empty() {
            0.0
        } else {
            let m = advs.len() as f32;
            let mean = advs.iter().sum::<f32>() / m;
            let var = advs.iter().map(|a| (a - mean).powi(2)).sum::<f32>() / m;
            var.sqrt()
        };

        self.ret_adv = Some(RetAdvStats { ret_mean, raw_ret_std, ret_min, ret_max, noop_frac, adv_std });
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
        self.func_speedup_avgs.clear();
        self.ret_adv = None;
        self.store_stats = None;
        self.lookahead_hits = 0;
        self.lookahead_misses = 0;
        self.bench_cache_hits = 0;
        self.bench_cache_misses = 0;
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
    pub(crate) fn record_la_cache(&mut self, hits: u64, misses: u64) {
        self.lookahead_hits += hits;
        self.lookahead_misses += misses;
    }
    pub(crate) fn record_bench_cache(&mut self, hits: u64, misses: u64) {
        self.bench_cache_hits += hits;
        self.bench_cache_misses += misses;
    }
    pub(crate) fn bench_cache_hit_pct(&self) -> Option<f32> {
        let total = self.bench_cache_hits + self.bench_cache_misses;
        if total == 0 { None } else { Some(self.bench_cache_hits as f32 / total as f32 * 100.0) }
    }
    /// Cache hit rate as a percentage, or None if no lookahead was done this epoch.
    pub(crate) fn la_cache_hit_pct(&self) -> Option<f32> {
        let total = self.lookahead_hits + self.lookahead_misses;
        if total == 0 { None } else { Some(self.lookahead_hits as f32 / total as f32 * 100.0) }
    }
    pub(crate) fn ema(&self) -> f32 { self.speedup_ema.get() }
    pub(crate) fn avg_episode_len(&self) -> f32 { self.episode_len_avg.mean() }
    pub(crate) fn avg_final_speedup(&self) -> f32 { self.final_speedup_avg.mean() }
    pub(crate) fn func_speedups(&self) -> HashMap<String, f32> {
        self.func_speedup_avgs.iter().map(|(k, v)| (k.clone(), v.mean())).collect()
    }
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
