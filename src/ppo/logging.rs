use crate::ppo::metrics::Metrics;
use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use owo_colors::OwoColorize;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

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
            let speedup = metrics.speedup_ema();
            let speedup_str = if speedup >= 0.0 {
                format!("{:+.4}", speedup).green().to_string()
            } else {
                format!("{:+.4}", speedup).red().to_string()
            };

            // Policy: always show sign for easy alignment with negative values.
            let policy_str = {
                let v = metrics.policy_loss();
                if v >= 0.0 {
                    format!("{:+.4}", v).yellow().to_string()
                } else {
                    format!("{:+.4}", v).yellow().to_string()
                }
            };

            // Total accumulated wall time.
            let total_s = metrics.total_elapsed_ms as f64 / 1000.0;
            let total_str = if total_s < 60.0 {
                format!("{:.0}s", total_s)
            } else {
                format!("{:.1}m", total_s / 60.0)
            };

            let line = format!(
                "epoch {:>5}  speedup_ema={}  policy={}  value={}  entropy={}  kl={:.4}  ev={:+.3}  ep_len={}  collect={}  ppo={}  total={}  lr={:.3e}",
                epoch,
                speedup_str,
                policy_str,
                format!("{:.4}", metrics.value_loss()).yellow(),
                format!("{:.1}%", metrics.entropy_pct()).yellow(),
                metrics.kl_div(),
                metrics.explained_variance(),
                format!("{:.1}", metrics.avg_episode_len()).bold(),
                format!("{}ms", metrics.episode_collection_ms).cyan(),
                format!("{}ms", metrics.ppo_update_ms).cyan(),
                total_str.cyan().to_string(),
                lr,
            );
            self.epoch_bar.println(line);
        }

        if let Some(f) = &mut self.file {
            let record = serde_json::json!({
                "epoch":                  epoch,
                "policy_loss":            metrics.policy_loss(),
                "value_loss":             metrics.value_loss(),
                "entropy":                metrics.entropy(),
                "entropy_pct":            metrics.entropy_pct(),
                "kl_div":                 metrics.kl_div(),
                "explained_variance":     metrics.explained_variance(),
                "speedup_ema":            metrics.speedup_ema(),
                "avg_final_speedup":      metrics.avg_final_speedup(),
                "avg_episode_len":        metrics.avg_episode_len(),
                "episode_collection_ms":  metrics.episode_collection_ms,
                "ppo_update_ms":          metrics.ppo_update_ms,
                "total_elapsed_ms":       metrics.total_elapsed_ms,
                "avg_func_ir_ms":         metrics.avg_func_ir_ms(),
                "lr":                     lr,
            });
            let _ = writeln!(f, "{}", record);
            let _ = f.flush();
        }

        self.epoch_bar.inc(1);
    }

    pub(crate) fn finish(&mut self) {
        self.epoch_bar.finish_with_message("done");
        if let Some(f) = &mut self.file {
            let _ = f.flush();
        }
    }
}
