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

        // Truncate the log file on every new run — runs accumulate otherwise.
        let file = match mode {
            LogMode::StdoutOnly => None,
            _ => {
                let path = log_path.expect("log_path required for FileOnly / FileAndStdout");
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let f = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(path)?;
                Some(BufWriter::new(f))
            }
        };

        Ok(Self {
            mode,
            multi,
            epoch_bar,
            baseline_bar: Some(baseline_bar),
            file,
        })
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

    pub(crate) fn collection_bar(&self, total_episodes: u64) -> ProgressBar {
        let style = ProgressStyle::with_template("  collect {bar:30.green/black} {pos}/{len} eps")
            .unwrap()
            .progress_chars("=>-");
        let bar = self
            .multi
            .insert_before(&self.epoch_bar, ProgressBar::new(total_episodes));
        bar.set_style(style);
        if matches!(self.mode, LogMode::FileOnly) {
            bar.set_draw_target(ProgressDrawTarget::hidden());
        }
        bar
    }

    pub(crate) fn ppo_bar(&self, total_steps: u64) -> ProgressBar {
        let style =
            ProgressStyle::with_template("  ppo    {bar:30.magenta/black} {pos}/{len}  {msg}")
                .unwrap()
                .progress_chars("=>-");
        let bar = self
            .multi
            .insert_before(&self.epoch_bar, ProgressBar::new(total_steps));
        bar.set_style(style);
        if matches!(self.mode, LogMode::FileOnly) {
            bar.set_draw_target(ProgressDrawTarget::hidden());
        }
        bar
    }

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
            let total_s = (metrics.total_elapsed_ms
                + metrics.episode_collection_ms
                + metrics.ppo_update_ms) as f64
                / 1000.0;
            let total_str = if total_s < 60.0 {
                format!("{:.0}s", total_s)
            } else {
                format!("{:.1}m", total_s / 60.0)
            };

            macro_rules! g1 {
                ($s:expr) => {
                    $s.truecolor(210, 210, 210).to_string()
                };
            }
            macro_rules! g2 {
                ($s:expr) => {
                    $s.truecolor(160, 160, 160).to_string()
                };
            }
            macro_rules! g3 {
                ($s:expr) => {
                    $s.truecolor(115, 115, 115).to_string()
                };
            }

            let bench_cache_str = metrics
                .bench_cache_hit_pct()
                .map(|p| format!("  {}={}", g1!("bench_cache"), format!("{:.1}%", p).cyan()))
                .unwrap_or_default();

            let noop_str = metrics
                .noop_pct()
                .map(|p| {
                    let exact = metrics
                        .exact_noop_pct()
                        .map(|e| format!(" exact={:.1}%", e))
                        .unwrap_or_default();
                    format!(
                        "  {}={}{}",
                        g2!("noop%"),
                        format!("{:.1}%", p).truecolor(180, 120, 60).to_string(),
                        exact.truecolor(140, 100, 60).to_string(),
                    )
                })
                .unwrap_or_default();

            self.epoch_bar.println(
                format!(
                    "{} {:>5}  {}{}  {}{}  {}{}  {}{}  {}{}  {}{}",
                    g1!("epoch"),
                    epoch,
                    g1!("speedup="),
                    speedup_str,
                    g1!("ema="),
                    ema_str,
                    g1!("ep_len="),
                    format!("{:.1}", metrics.avg_episode_len()).bold(),
                    g1!("collect="),
                    format!("{}ms", metrics.episode_collection_ms).cyan(),
                    g1!("ppo="),
                    format!("{}ms", metrics.ppo_update_ms).cyan(),
                    g1!("total="),
                    total_str.cyan().to_string(),
                ) + &bench_cache_str
                    + &noop_str,
            );

            self.epoch_bar.println(format!(
                "         {}  {}{}  {}{}  {}{}  {}{}  {}{}  {}{}",
                g2!("losses"),
                g2!("policy="),
                format!("{:+.4}", metrics.policy_loss()).yellow(),
                g2!("value="),
                format!("{:.4}", metrics.value_loss()).yellow(),
                g2!("entropy="),
                format!("{:.1}%", metrics.entropy_pct()).yellow(),
                g2!("kl="),
                format!("{:.4}", metrics.kl_div())
                    .truecolor(200, 160, 80)
                    .to_string(),
                g2!("clip="),
                format!("{:.1}%", metrics.clip_frac() * 100.0)
                    .truecolor(200, 160, 80)
                    .to_string(),
                g2!("ev="),
                format!("{:+.3}", metrics.explained_variance())
                    .truecolor(200, 160, 80)
                    .to_string(),
            ));

            if let Some(ra) = &metrics.ret_adv {
                self.epoch_bar.println(format!(
                    "         {}  {}{}  {}[{}, {}]  {}{}  {}[{}, {}]",
                    g3!("ret"),
                    g3!("mean="),
                    format!("{:+.3}", ra.ret_mean).cyan(),
                    g3!("range="),
                    format!("{:+.3}", ra.ret_min).cyan(),
                    format!("{:+.3}", ra.ret_max).cyan(),
                    g3!("adv_std="),
                    format!("{:.3}", ra.adv_std).cyan(),
                    g3!("adv_range="),
                    format!("{:+.3}", ra.adv_min).cyan(),
                    format!("{:+.3}", ra.adv_max).cyan(),
                ));
            }

            let func_entries = metrics.func_speedups_current_and_avg();
            if !func_entries.is_empty() {
                let parts: Vec<String> = func_entries
                    .iter()
                    .map(|(name, cur, avg)| {
                        let cur_s = if *cur >= 0.0 {
                            format!("{:+.2}%", cur).green().to_string()
                        } else {
                            format!("{:+.2}%", cur).red().to_string()
                        };
                        let avg_s = if *avg >= 0.0 {
                            format!("{:+.2}%", avg).truecolor(100, 200, 100).to_string()
                        } else {
                            format!("{:+.2}%", avg).truecolor(200, 100, 100).to_string()
                        };
                        format!("{}: {}|{}", g3!(name.as_str()), cur_s, avg_s)
                    })
                    .collect();
                self.epoch_bar.println(format!(
                    "         {}  [{}]",
                    g3!("funcs"),
                    parts.join("  ")
                ));
            }
        }

        if let Some(f) = &mut self.file {
            let record = serde_json::json!({
                "epoch":                  epoch,
                "policy_loss":            metrics.policy_loss(),
                "value_loss":             metrics.value_loss(),
                "entropy":                metrics.entropy(),
                "entropy_pct":            metrics.entropy_pct(),
                "kl_div":                 metrics.kl_div(),
                "clip_frac":              metrics.clip_frac(),
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
                "bench_cache_hit_pct":    metrics.bench_cache_hit_pct(),
                "noop_pct":               metrics.noop_pct(),
                "exact_noop_pct":         metrics.exact_noop_pct(),
            });
            let _ = writeln!(f, "{}", record);
            let _ = f.flush();
        }

        self.epoch_bar.inc(1);
    }

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
