use crate::ppo::metrics::Metrics;
use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use owo_colors::OwoColorize;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub(crate) enum LogMode {
    FileOnly,
    FileAndStdout,
    StdoutOnly,
}

pub(crate) struct Logger {
    mode: LogMode,
    multi: MultiProgress,
    epoch_bar: ProgressBar,
    baseline_bar: Option<ProgressBar>,
    file: Option<BufWriter<File>>,
}

impl Logger {
    pub(crate) fn init(
        mode: LogMode,
        log_path: Option<&Path>,
        total_epochs: u64,
        total_funcs: u64,
    ) -> Result<Self> {
        let multi = MultiProgress::new();

        let epoch_style = ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} epochs  {msg}",
        )?
        .progress_chars("=>-");
        let epoch_bar = multi.add(ProgressBar::new(total_epochs));
        epoch_bar.set_style(epoch_style);

        let baseline_style = ProgressStyle::with_template(
            "  baseline {bar:30.yellow/black} {pos}/{len} funcs  {msg}",
        )?
        .progress_chars("=>-");
        let baseline_bar = multi.insert_before(&epoch_bar, ProgressBar::new(total_funcs));
        baseline_bar.set_style(baseline_style);

        if matches!(mode, LogMode::FileOnly) {
            epoch_bar.set_draw_target(ProgressDrawTarget::hidden());
            baseline_bar.set_draw_target(ProgressDrawTarget::hidden());
        } else {
            epoch_bar.enable_steady_tick(Duration::from_millis(100));
        }

        let file = match mode {
            LogMode::StdoutOnly => None,
            _ => {
                let path = log_path.expect("log_path required for FileOnly / FileAndStdout");
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let f = OpenOptions::new().create(true).append(true).open(path)?;
                Some(BufWriter::new(f))
            }
        };

        Ok(Self { mode, multi, epoch_bar, baseline_bar: Some(baseline_bar), file })
    }

    pub(crate) fn log_baseline_progress(&mut self, func_name: &str, elapsed_ms: u64) {
        if let Some(bar) = &self.baseline_bar {
            bar.set_message(func_name.to_string());
            bar.inc(1);
            if !matches!(self.mode, LogMode::FileOnly) {
                let line = format!("  {}  {}", func_name, format!("{}ms", elapsed_ms).cyan());
                self.epoch_bar.println(line);
            }
        }
    }

    pub(crate) fn finish_baseline_phase(&mut self) {
        if let Some(bar) = self.baseline_bar.take() {
            bar.finish_and_clear();
        }
    }

    /// Transient progress bar for episode collection. Caller must call `.finish_and_clear()`.
    pub(crate) fn collection_bar(&self, total_episodes: u64) -> ProgressBar {
        let style = ProgressStyle::with_template(
            "  collect {bar:30.green/black} {pos}/{len} eps",
        )
        .unwrap()
        .progress_chars("=>-");
        let bar = self.multi.insert_before(&self.epoch_bar, ProgressBar::new(total_episodes));
        bar.set_style(style);
        if matches!(self.mode, LogMode::FileOnly) {
            bar.set_draw_target(ProgressDrawTarget::hidden());
        }
        bar
    }

    /// Transient progress bar for the PPO update. Caller must call `.finish_and_clear()`.
    pub(crate) fn ppo_bar(&self, total_steps: u64) -> ProgressBar {
        let style = ProgressStyle::with_template(
            "  ppo    {bar:30.magenta/black} {pos}/{len}  {msg}",
        )
        .unwrap()
        .progress_chars("=>-");
        let bar = self.multi.insert_before(&self.epoch_bar, ProgressBar::new(total_steps));
        bar.set_style(style);
        if matches!(self.mode, LogMode::FileOnly) {
            bar.set_draw_target(ProgressDrawTarget::hidden());
        }
        bar
    }

    /// Log a colored epoch summary to stdout and/or a JSON line to file.
    pub(crate) fn log_epoch(&mut self, epoch: usize, metrics: &Metrics, lr: f64) {
        if !matches!(self.mode, LogMode::FileOnly) {
            let speedup = metrics.avg_final_speedup();
            let speedup_str = if speedup >= 0.0 {
                format!("{:+.4}", speedup).green().to_string()
            } else {
                format!("{:+.4}", speedup).red().to_string()
            };
            let ema = metrics.ema();
            let ema_str = if ema >= 0.0 {
                format!("{:+.4}", ema).green().to_string()
            } else {
                format!("{:+.4}", ema).red().to_string()
            };
            let total_s = (metrics.total_elapsed_ms + metrics.episode_collection_ms + metrics.ppo_update_ms) as f64 / 1000.0;
            let total_str = if total_s < 60.0 {
                format!("{:.0}s", total_s)
            } else {
                format!("{:.1}m", total_s / 60.0)
            };

            // Grayscale helpers — varied per line so rows are visually distinct.
            macro_rules! g1 { ($s:expr) => { $s.truecolor(210, 210, 210).to_string() } }  // bright
            macro_rules! g2 { ($s:expr) => { $s.truecolor(160, 160, 160).to_string() } }  // mid
            macro_rules! g3 { ($s:expr) => { $s.truecolor(115, 115, 115).to_string() } }  // dim
            macro_rules! g4 { ($s:expr) => { $s.truecolor( 85,  85,  85).to_string() } }  // dark

            // Line 1: performance + timing — bright gray labels
            let cache_str = metrics.la_cache_hit_pct()
                .map(|p| format!("  {}={}", g1!("la_cache"), format!("{:.1}%", p).cyan()))
                .unwrap_or_default();
            let bench_cache_str = metrics.bench_cache_hit_pct()
                .map(|p| format!("  {}={}", g1!("bench_cache"), format!("{:.1}%", p).cyan()))
                .unwrap_or_default();
            self.epoch_bar.println(format!(
                "{} {:>5}  {}{}  {}{}  {}{}  {}{}  {}{}  {}{}",
                g1!("epoch"), epoch,
                g1!("speedup="), speedup_str,
                g1!("ema="), ema_str,
                g1!("ep_len="), format!("{:.1}", metrics.avg_episode_len()).bold(),
                g1!("collect="), format!("{}ms", metrics.episode_collection_ms).cyan(),
                g1!("ppo="), format!("{}ms", metrics.ppo_update_ms).cyan(),
                g1!("total="), total_str.cyan().to_string(),
            ) + &cache_str + &bench_cache_str);

            // Line 2: training losses — mid gray labels
            self.epoch_bar.println(format!(
                "         {}  {}{}  {}{}  {}{}  {}{}  {}{}",
                g2!("losses"),
                g2!("policy="), format!("{:+.4}", metrics.policy_loss()).yellow(),
                g2!("value="),  format!("{:.4}",  metrics.value_loss()).yellow(),
                g2!("entropy="),format!("{:.1}%", metrics.entropy_pct()).yellow(),
                g2!("kl="),     format!("{:.4}",  metrics.kl_div()).truecolor(200, 160, 80).to_string(),
                g2!("ev="),     format!("{:+.3}", metrics.explained_variance()).truecolor(200, 160, 80).to_string(),
            ));

            // Line 3: return distribution — dim gray labels
            if let Some(ra) = &metrics.ret_adv {
                let raw_std_str = ra.raw_ret_std
                    .map(|s| format!("  {}{}",  g3!("raw_std="), format!("{:.3}", s).cyan()))
                    .unwrap_or_default();
                self.epoch_bar.println(format!(
                    "         {}  {}{}  {}[{}, {}]  {}{}  {}{}",
                    g3!("ret"),
                    g3!("mean="),    format!("{:+.3}", ra.ret_mean).cyan(),
                    g3!("range="),   format!("{:+.3}", ra.ret_min).cyan(),
                                     format!("{:+.3}", ra.ret_max).cyan(),
                    g3!("noop="),    format!("{:.0}%", ra.noop_frac * 100.0).cyan(),
                    g3!("adv_std="), format!("{:.3}",  ra.adv_std).cyan(),
                ).to_string() + &raw_std_str);
            }

            // Lines 4+: one line per func, alternating dark/dim gray labels
            if let Some(ss) = &metrics.store_stats {
                self.epoch_bar.println(format!(
                    "         {}  {}",
                    g3!("store"),
                    format!("{}", ss.total_entries).truecolor(180, 180, 180).to_string(),
                ));
                for (i, f) in ss.per_func.iter().enumerate() {
                    let lbl = |s: &str| -> String {
                        if i % 2 == 0 { s.truecolor(100, 100, 100).to_string() }
                        else           { s.truecolor( 70,  70,  70).to_string() }
                    };
                    let best_str = if f.best >= 0.0 {
                        format!("{:+.3}", f.best).green().to_string()
                    } else {
                        format!("{:+.3}", f.best).red().to_string()
                    };
                    self.epoch_bar.println(format!(
                        "           {}  {}{}  {}{}  {}{}  {}{}",
                        f.func_name.truecolor(200, 200, 200).to_string(),
                        lbl("n="),       format!("{}", f.entries).bold(),
                        lbl(" best="),   best_str,
                        lbl(" spread="), format!("{:.3}", f.spread).cyan(),
                        lbl(" div="),    format!("{:.0}%", f.diversity * 100.0).yellow(),
                    ));
                }
            }
        }

        if let Some(f) = &mut self.file {
            let mut record = serde_json::json!({
                "epoch":                  epoch,
                "policy_loss":            metrics.policy_loss(),
                "value_loss":             metrics.value_loss(),
                "entropy":                metrics.entropy(),
                "entropy_pct":            metrics.entropy_pct(),
                "kl_div":                 metrics.kl_div(),
                "explained_variance":     metrics.explained_variance(),
                "ema":                    metrics.ema(),
                "avg_final_speedup":      metrics.avg_final_speedup(),
                "avg_episode_len":        metrics.avg_episode_len(),
                "episode_collection_ms":  metrics.episode_collection_ms,
                "ppo_update_ms":          metrics.ppo_update_ms,
                "total_elapsed_ms":       metrics.total_elapsed_ms,
                "avg_func_ir_ms":         metrics.avg_func_ir_ms(),
                "lr":                     lr,
                "func_speedups":          metrics.func_speedups(),
                "la_cache_hit_pct": metrics.la_cache_hit_pct(),
                "bench_cache_hit_pct":    metrics.bench_cache_hit_pct(),
            });
            if let Some(ra) = &metrics.ret_adv {
                record["ret_mean"]    = serde_json::json!(ra.ret_mean);
                record["raw_ret_std"] = serde_json::json!(ra.raw_ret_std);
                record["ret_min"]   = serde_json::json!(ra.ret_min);
                record["ret_max"]   = serde_json::json!(ra.ret_max);
                record["noop_frac"] = serde_json::json!(ra.noop_frac);
                record["adv_std"]   = serde_json::json!(ra.adv_std);
            }
            if let Some(ss) = &metrics.store_stats {
                let per_func: serde_json::Value = ss.per_func.iter().map(|f| {
                    serde_json::json!({
                        "func":      f.func_name,
                        "entries":   f.entries,
                        "best":      f.best,
                        "worst":     f.worst,
                        "spread":    f.spread,
                        "diversity": f.diversity,
                    })
                }).collect::<Vec<_>>().into();
                record["store_total"]   = serde_json::json!(ss.total_entries);
                record["store_per_func"] = per_func;
            }
            let _ = writeln!(f, "{}", record);
            let _ = f.flush();
        }

        self.epoch_bar.inc(1);
    }

    /// Print a one-line marker when a new best checkpoint is saved.
    pub(crate) fn log_best(&self, epoch: usize, mean: f32) {
        if !matches!(self.mode, LogMode::FileOnly) {
            self.epoch_bar.println(
                format!("  * epoch {} new best  speedup={:+.4}", epoch, mean)
                    .green()
                    .to_string(),
            );
        }
    }

    pub(crate) fn finish(&mut self) {
        self.epoch_bar.finish_with_message("done");
        if let Some(f) = &mut self.file {
            let _ = f.flush();
        }
    }
}
