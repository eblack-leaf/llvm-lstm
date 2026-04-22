#![recursion_limit = "256"]
#![allow(unused)]
use crate::config::{BurnAutoDiff, BurnDevice, Cfg};
use crate::llvm::Llvm;
use crate::llvm::functions::Functions;
use crate::llvm::pass::Pass;
use crate::llvm::top_sequences::TopSequences;
use crate::ppo::advantages::baseline::BaselineAdvantage;
use crate::ppo::checkpoint::Checkpoint;
use crate::ppo::logging::LogMode;
use crate::ppo::returns::episode_return::EpisodeReturn;
use crate::ppo::returns::weighted::Weighted;
use crate::ppo::returns::ir_step_return::IrStepReturn;
use crate::train::Trainer;
use burn::module::AutodiffModule;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod config;
mod llvm;
mod ppo;
mod train;

#[derive(Parser)]
struct LlvmLstm {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Train {
        #[arg(long, default_value = "benchmarks")]
        directory: PathBuf,
        #[arg(long, default_value = "work")]
        work_dir: PathBuf,
        #[arg(long, default_value = "checkpoints")]
        checkpoint_dir: PathBuf,
        #[arg(long, default_value = "clang-20")]
        clang: String,
        #[arg(long, default_value = "opt-20")]
        opt: String,
        #[arg(long, default_value = "100")]
        epochs: usize,
        #[arg(long, default_value = "4")]
        ppo_epochs: usize,
        #[arg(long, default_value = "256")]
        episodes: usize,
        #[arg(long, default_value = "2")]
        benchmark_runs: usize,
        #[arg(long, default_value = "100")]
        benchmark_iters: usize,
        #[arg(long, default_value = "3")]
        baseline_runs: usize,
        #[arg(long, default_value = "200")]
        baseline_iters: usize,
        #[arg(long, default_value = "20")]
        max_seq_len: usize,
        #[arg(long, default_value = "1e-3")]
        learning_rate: f64,
        #[arg(long, default_value = "0.2")]
        clip_epsilon: f32,
        #[arg(long, default_value = "0.5")]
        value_coef: f32,
        #[arg(long, default_value = "0.03")]
        entropy_coef: f32,
        /// PPO inner-loop KL early-stop threshold (0 = disabled).
        #[arg(long, default_value = "0.05")]
        kl_target: f32,
        /// Number of episodes per PPO mini-batch.
        #[arg(long, default_value = "128")]
        mini_batch_size: usize,
        #[arg(long)]
        cache_file: Option<PathBuf>,
        /// Path to save/load the top-sequences file for the Diagnose command.
        #[arg(long)]
        sequences_file: Option<PathBuf>,
        /// Return signal:
        ///   episode  — uniform terminal speedup across all slots
        ///   weighted — terminal weighted by per-slot instr reduction; no-ops get 0
        ///   ir-step  — per-step IR-count delta (dense; skips benchmarking)
        #[arg(long, default_value = "weighted")]
        returns: String,
        /// |instr_delta| below this is a candidate no-op.
        #[arg(long, default_value = "0.01")]
        noop_threshold: f32,
        /// L1 feature-vector distance below which a step is also structurally a no-op.
        #[arg(long, default_value = "0.05")]
        noop_feature_threshold: f32,
        /// Penalty subtracted when both noop conditions are met and action != Stop.
        #[arg(long, default_value = "0.025")]
        noop_penalty: f32,
        /// Fixed bonus added to active steps that reduced instructions, subtracted for increases.
        #[arg(long, default_value = "0.05")]
        weighted_direction_bonus: f32,
        /// Number of positional chunks for the IR opcode histogram.
        /// Feature dim = (ir_chunks - 1) * 16.  Default 4 → 48-dim vector.
        #[arg(long, default_value = "4")]
        ir_chunks: usize,
    },
    Evaluate {
        #[arg(long, default_value = "checkpoints/best")]
        model: PathBuf,
        #[arg(long, default_value = "benchmarks")]
        directory: PathBuf,
        #[arg(long, default_value = "work/eval")]
        work_dir: PathBuf,
        #[arg(long, default_value = "clang-20")]
        clang: String,
        #[arg(long, default_value = "opt-20")]
        opt: String,
        #[arg(long, default_value = "5")]
        runs: usize,
        #[arg(long, default_value = "200")]
        iters: usize,
        #[arg(long, default_value = "3")]
        baseline_runs: usize,
        #[arg(long, default_value = "200")]
        baseline_iters: usize,
        /// Number of random sequences to sample per function for the random baseline.
        #[arg(long, default_value = "20")]
        random_sequences: usize,
        /// Number of policy samples (stochastic rollouts) per function.
        #[arg(long, default_value = "20")]
        policy_samples: usize,
        #[arg(long, default_value = "4")]
        ir_chunks: usize,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    PlotTrain {
        #[arg(long, default_value = "checkpoints")]
        dir: PathBuf,
    },
    PlotEval {
        /// Path to the eval JSON produced by the evaluate command.
        #[arg(long)]
        input: PathBuf,
        /// Output PNG path (defaults to <input>.png).
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Re-benchmark the top sequences from training to check if speedups are reproducible.
    Diagnose {
        #[arg(long, default_value = "checkpoints/bench-top.bin")]
        sequences: PathBuf,
        #[arg(long, default_value = "benchmarks")]
        directory: PathBuf,
        #[arg(long, default_value = "work/diagnose")]
        work_dir: PathBuf,
        #[arg(long, default_value = "clang-20")]
        clang: String,
        #[arg(long, default_value = "opt-20")]
        opt: String,
        /// How many top sequences to re-test.
        #[arg(long, default_value = "10")]
        top: usize,
        /// Benchmark runs per sequence per trial (for mean/std).
        #[arg(long, default_value = "20")]
        runs: usize,
        #[arg(long, default_value = "200")]
        iters: usize,
        #[arg(long, default_value = "3")]
        baseline_runs: usize,
        #[arg(long, default_value = "200")]
        baseline_iters: usize,
        /// Save results as JSON for plotting (e.g. checkpoints/diagnose.json).
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Measure parallel-worker benchmark timing contention.
    BenchNoise {
        #[arg(long)]
        source: PathBuf,
        #[arg(long, default_value = "clang-20")]
        clang: String,
        #[arg(long, default_value = "work/bench_noise")]
        work_dir: PathBuf,
        #[arg(long, default_value = "5")]
        runs: usize,
        #[arg(long, default_value = "200")]
        iters: usize,
        #[arg(long, default_value = "16")]
        workers: usize,
        /// Save results as JSON for plotting (e.g. checkpoints/bench_noise.json).
        #[arg(long)]
        output: Option<PathBuf>,
    },
    PlotDiagnose {
        #[arg(long, default_value = "checkpoints/diagnose.json")]
        results: PathBuf,
    },
    /// Dump the bench-cache to a JSONL dataset file for plot-dataset.
    CollectDataset {
        #[arg(long, default_value = "checkpoints/data.cache")]
        cache_file: PathBuf,
        #[arg(long, default_value = "dataset.jsonl")]
        output: PathBuf,
    },
    PlotDataset {
        #[arg(long, default_value = "dataset.jsonl")]
        data: PathBuf,
    },
    PlotBenchNoise {
        #[arg(long, default_value = "checkpoints/bench_noise.json")]
        results: PathBuf,
    },
    PlotFeatures {
        #[arg(long, default_value = "features.json")]
        features: PathBuf,
    },
    /// Export IR features for all functions in a benchmark directory to JSON.
    ExportFeatures {
        #[arg(long, default_value = "benchmarks")]
        directory: PathBuf,
        #[arg(long, default_value = "features.json")]
        output: PathBuf,
        #[arg(long, default_value = "clang-20")]
        clang: String,
        #[arg(long, default_value = "opt-20")]
        opt: String,
        #[arg(long, default_value = "work/export_features")]
        work_dir: PathBuf,
        /// Number of positional chunks — must match the value used in Train.
        #[arg(long, default_value = "4")]
        ir_chunks: usize,
    },
}

fn print_stats(label: &str, workers: usize, solo_ns: u64, results: &mut Vec<u64>) {
    results.sort_unstable();
    let mean = results.iter().sum::<u64>() / results.len() as u64;
    let median = results[results.len() / 2];
    let min = results[0];
    let max = results[results.len() - 1];
    let solo = solo_ns as f64;
    let ratio = mean as f64 / solo;
    let pct = (ratio - 1.0) * 100.0;
    let spread = (max - min) as f64 / solo * 100.0;
    println!("\n=== {} ({} workers) ===", label, workers);
    println!("  mean:   {} ns  ({:+.1}% vs solo)", mean, pct);
    println!("  median: {} ns", median);
    println!("  min:    {} ns", min);
    println!("  max:    {} ns", max);
    println!("  spread: {:.1}% of solo  (max-min / solo)", spread);
}

fn main() {
    let args = LlvmLstm::parse();
    match args.command {
        Command::Train {
            directory,
            work_dir,
            checkpoint_dir,
            clang,
            opt,
            epochs,
            ppo_epochs,
            episodes,
            benchmark_runs,
            benchmark_iters,
            baseline_runs,
            baseline_iters,
            max_seq_len,
            learning_rate,
            clip_epsilon,
            value_coef,
            entropy_coef,
            mini_batch_size,
            cache_file,
            sequences_file,
            returns,
            noop_threshold,
            noop_feature_threshold,
            noop_penalty,
            weighted_direction_bonus,
            ir_chunks,
            kl_target,
        } => {
            let noop = crate::ppo::noop::NoOp {
                count_threshold: noop_threshold,
                feature_threshold: noop_feature_threshold,
                penalty: noop_penalty,
            };
            let cfg = Cfg {
                functions: directory,
                clang,
                opt,
                epochs,
                ppo_epochs,
                episodes,
                benchmark_runs,
                benchmark_iters,
                baseline_runs,
                baseline_iters,
                max_seq_len,
                work_dir,
                checkpoint_dir: checkpoint_dir.clone(),
                learning_rate,
                clip_epsilon,
                value_coef,
                entropy_coef,
                mini_batch_size,
                cache_file,
                noop,
                ir_chunks,
                skip_benchmark: returns == "ir-step",
                kl_target,
            };
            let log_path = checkpoint_dir.join("train.jsonl");
            let seq_path =
                sequences_file.or_else(|| Some(checkpoint_dir.join("top_sequences.bin")));
            let returns_impl: Box<dyn crate::ppo::returns::Returns> = match returns.as_str() {
                "weighted" => Box::new(Weighted {
                    noop,
                    direction_bonus: weighted_direction_bonus,
                }),
                "ir-step" => Box::new(IrStepReturn { noop }),
                _ => Box::new(EpisodeReturn),
            };
            let trainer = Trainer::new(
                cfg,
                returns_impl,
                Box::new(BaselineAdvantage),
                LogMode::FileAndStdout,
                Some(log_path),
                seq_path,
            );
            trainer.train();
        }
        Command::Evaluate {
            model,
            directory,
            work_dir,
            clang,
            opt,
            runs,
            iters,
            baseline_runs,
            baseline_iters,
            random_sequences,
            policy_samples,
            ir_chunks,
            output,
        } => {
            use crate::ppo::model::AutoActor;

            let device = BurnDevice::default();
            let (loaded_model, meta) =
                Checkpoint::load(&model, &device).expect("failed to load checkpoint");
            let inference_model = loaded_model.valid();
            let max_seq_len = meta.max_seq_len;

            std::fs::create_dir_all(&work_dir).expect("create work dir");
            let llvm = Llvm::new(&clang, &opt, work_dir.clone());

            let mut functions = Functions::new(&directory);
            println!("Collecting baselines...");
            for func in &mut functions.functions {
                let func_llvm = llvm.with_env(work_dir.join(&func.name));
                std::fs::create_dir_all(&func_llvm.work_dir).expect("create func dir");
                func.ir = func_llvm.ir(&func.source).expect("emit ir");
                func.baselines = Some(
                    func_llvm
                        .collect_baselines(&func.source, baseline_runs, baseline_iters)
                        .expect("baselines"),
                );
                println!(
                    "  {} O3={} ns",
                    func.name,
                    func.baselines.as_ref().unwrap().o3.mean_ns
                );
            }

            // Helper: softmax sample from raw logits → (action_idx, log_prob).
            let sample_logits = |logits: &[f32]| -> (usize, f32) {
                let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let exp: Vec<f32> = logits.iter().map(|&x| (x - max).exp()).collect();
                let sum: f32 = exp.iter().sum::<f32>().max(f32::EPSILON);
                let probs: Vec<f32> = exp.iter().map(|&e| e / sum).collect();
                let u: f32 = rand::random();
                let mut cum = 0.0f32;
                let mut idx = probs.len() - 1;
                for (i, &p) in probs.iter().enumerate() {
                    cum += p;
                    if cum > u { idx = i; break; }
                }
                (idx, probs[idx].max(f32::EPSILON).ln())
            };

            let col_samples = policy_samples > 0;

            // All numeric columns are 9 wide; header labels match exactly.
            let mut header = format!(
                "\n{:<22} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}",
                "function", "O0", "O1", "O2", "rand_mean", "greedy", "rand_best"
            );
            if col_samples { header.push_str(&format!(" {:>9}", "samp_best")); }
            let sep_len = 22 + (9 + 1) * (6 + if col_samples { 1 } else { 0 });
            println!("{header}");
            println!("{}", "-".repeat(sep_len));

            let mut json_records: Vec<serde_json::Value> = Vec::new();

            for func in &functions.functions {
                let baselines = func.baselines.as_ref().unwrap();
                let func_llvm =
                    llvm.with_env(work_dir.join(format!("eval_{}", func.name)));
                std::fs::create_dir_all(&func_llvm.work_dir).expect("create eval dir");

                let o0_speedup = baselines.speedup_vs_o3(baselines.o0.mean_ns);
                let o1_speedup = baselines.speedup_vs_o3(baselines.o1.mean_ns);
                let o2_speedup = baselines.speedup_vs_o3(baselines.o2.mean_ns);

                // ── Greedy rollout (argmax) ────────────────────────────────────────
                let mut ir_features_so_far: Vec<Vec<f32>> = Vec::new();
                let mut action_history: Vec<usize> = Vec::new();
                let mut current_ir = func.ir.clone();
                let mut hidden: Option<Vec<f32>> = None;
                let mut greedy_passes: Vec<String> = Vec::new();

                for step in 0..max_seq_len {
                    let ir_feat = current_ir.model_features(ir_chunks);
                    ir_features_so_far.push(ir_feat);
                    let (logits, _value, new_hidden) = inference_model
                        .infer_step_stateful(
                            &ir_features_so_far,
                            &action_history,
                            hidden,
                            &device,
                        );
                    hidden = new_hidden;
                    let action_idx = logits
                        .iter()
                        .enumerate()
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let action = crate::ppo::model::ACTIONS[action_idx];
                    action_history.push(action_idx);
                    if action == Pass::Stop { break; }
                    greedy_passes.push(format!("{action:?}"));
                    current_ir = func_llvm.apply_one(&current_ir, action, step).expect("apply greedy");
                }
                let greedy_bin = func_llvm.compile(&current_ir).expect("compile greedy");
                let greedy_bm = func_llvm.benchmark(&greedy_bin, runs, iters).expect("bench greedy");
                let greedy_speedup = baselines.speedup_vs_o3(greedy_bm.mean_ns);

                // ── Random sequences ──────────────────────────────────────────────
                let mut random_speedups: Vec<f32> = Vec::new();
                let mut random_passes_all: Vec<Vec<String>> = Vec::new();
                for rand_i in 0..random_sequences {
                    let rand_llvm = llvm.with_env(
                        work_dir.join(format!("eval_{}_r{rand_i}", func.name)),
                    );
                    std::fs::create_dir_all(&rand_llvm.work_dir).expect("create rand dir");
                    let mut cur = func.ir.clone();
                    let mut rpasses: Vec<String> = Vec::new();
                    for step in 0..max_seq_len {
                        let idx = (rand::random::<f32>() * crate::ppo::model::ACTIONS.len() as f32) as usize;
                        let action = crate::ppo::model::ACTIONS[idx];
                        if action == Pass::Stop { break; }
                        rpasses.push(format!("{action:?}"));
                        cur = rand_llvm.apply_one(&cur, action, step).expect("apply rand");
                    }
                    let bin = rand_llvm.compile(&cur).expect("compile rand");
                    let bm = rand_llvm.benchmark(&bin, runs, iters).expect("bench rand");
                    random_speedups.push(baselines.speedup_vs_o3(bm.mean_ns));
                    random_passes_all.push(rpasses);
                }
                let rand_mean = random_speedups.iter().sum::<f32>() / random_speedups.len().max(1) as f32;
                let (rand_best, rand_best_passes) = random_speedups
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, &s)| (s, random_passes_all[i].clone()))
                    .unwrap_or((f32::NEG_INFINITY, Vec::new()));

                // ── Policy samples (stochastic rollouts) ──────────────────────────
                let mut sample_speedups: Vec<f32> = Vec::new();
                let mut sample_passes_all: Vec<Vec<String>> = Vec::new();
                if col_samples {
                    for samp_i in 0..policy_samples {
                        let samp_llvm = llvm.with_env(
                            work_dir.join(format!("eval_{}_s{samp_i}", func.name)),
                        );
                        std::fs::create_dir_all(&samp_llvm.work_dir).expect("create samp dir");
                        let mut ir_feats: Vec<Vec<f32>> = Vec::new();
                        let mut acts: Vec<usize> = Vec::new();
                        let mut cur = func.ir.clone();
                        let mut hid: Option<Vec<f32>> = None;
                        let mut spasses: Vec<String> = Vec::new();
                        for step in 0..max_seq_len {
                            ir_feats.push(cur.model_features(ir_chunks));
                            let (logits, _, new_hid) = inference_model
                                .infer_step_stateful(&ir_feats, &acts, hid, &device);
                            hid = new_hid;
                            let (idx, _lp) = sample_logits(&logits);
                            let action = crate::ppo::model::ACTIONS[idx];
                            acts.push(idx);
                            if action == Pass::Stop { break; }
                            spasses.push(format!("{action:?}"));
                            cur = samp_llvm.apply_one(&cur, action, step).expect("apply samp");
                        }
                        let bin = samp_llvm.compile(&cur).expect("compile samp");
                        let bm = samp_llvm.benchmark(&bin, runs, iters).expect("bench samp");
                        sample_speedups.push(baselines.speedup_vs_o3(bm.mean_ns));
                        sample_passes_all.push(spasses);
                    }
                }
                let (samp_best, samp_best_passes) = sample_speedups
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, &s)| (s, sample_passes_all[i].clone()))
                    .unwrap_or((f32::NEG_INFINITY, Vec::new()));

                // ── Print row ─────────────────────────────────────────────────────
                let mut row = format!(
                    "{:<22} {:>+9.3} {:>+9.3} {:>+9.3} {:>+9.3} {:>+9.3} {:>+9.3}",
                    func.name, o0_speedup, o1_speedup, o2_speedup,
                    rand_mean, greedy_speedup, rand_best,
                );
                if col_samples {
                    row.push_str(&format!(" {:>+9.3}", samp_best));
                }
                println!("{row}");

                let mut record = serde_json::json!({
                    "name": func.name,
                    "o3_ns": baselines.o3.mean_ns,
                    "o0_speedup": o0_speedup,
                    "o1_speedup": o1_speedup,
                    "o2_speedup": o2_speedup,
                    "greedy_ns": greedy_bm.mean_ns,
                    "greedy_speedup": greedy_speedup,
                    "greedy_passes": greedy_passes,
                    "random_mean_speedup": rand_mean,
                    "random_best_speedup": rand_best,
                    "random_best_passes": rand_best_passes,
                    "random_speedups": random_speedups,
                });
                if col_samples {
                    record["sample_best_speedup"] = serde_json::json!(samp_best);
                    record["sample_best_passes"] = serde_json::json!(samp_best_passes);
                    record["sample_speedups"] = serde_json::json!(sample_speedups);
                }
                json_records.push(record);
            }

            if let Some(out) = output {
                std::fs::write(
                    &out,
                    serde_json::to_string_pretty(&json_records).unwrap(),
                )
                .expect("write output");
                println!("\nSaved → {}", out.display());
            }
        }
        Command::PlotEval { input, output } => {
            let cwd = std::env::current_dir().expect("cwd");
            let python = cwd.join(".venv/bin/python");
            let script = cwd.join("scripts/plot_eval.py");
            if !python.exists() {
                eprintln!("error: .venv not found");
                std::process::exit(1);
            }
            if !script.exists() {
                eprintln!("error: scripts/plot_eval.py not found");
                std::process::exit(1);
            }
            let out = output.unwrap_or_else(|| input.with_extension("png"));
            let status = std::process::Command::new(&python)
                .arg(&script)
                .arg("--input").arg(&input)
                .arg("--output").arg(&out)
                .status()
                .expect("failed to spawn python");
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Command::PlotTrain { dir } => {
            let cwd = std::env::current_dir().expect("cwd");
            let python = cwd.join(".venv/bin/python");
            let script = cwd.join("scripts/plot_training.py");

            if !python.exists() {
                eprintln!(
                    "error: .venv not found — run: python3 -m venv .venv && .venv/bin/pip install seaborn matplotlib pandas"
                );
                std::process::exit(1);
            }
            if !script.exists() {
                eprintln!("error: scripts/plot_training.py not found");
                std::process::exit(1);
            }

            let status = std::process::Command::new(&python)
                .arg(&script)
                .arg("--dir")
                .arg(&dir)
                .status()
                .expect("failed to spawn python");

            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Command::Diagnose {
            sequences,
            directory,
            work_dir,
            clang,
            opt,
            top,
            runs,
            iters,
            baseline_runs,
            baseline_iters,
            output,
        } => {
            use crate::llvm::ir::Source;

            let top_seqs = TopSequences::load(&sequences).expect("load top sequences");
            if top_seqs.entries.is_empty() {
                println!("No sequences recorded yet.");
                return;
            }

            std::fs::create_dir_all(&work_dir).expect("create work dir");
            let mut functions = Functions::new(&directory);

            let llvm = Llvm::new(&clang, &opt, work_dir.clone());
            println!("Collecting baselines...");
            for func in &mut functions.functions {
                let func_llvm = llvm.with_env(work_dir.join(&func.name));
                std::fs::create_dir_all(&func_llvm.work_dir).expect("create func work dir");
                func.ir = func_llvm.ir(&func.source).expect("emit ir");
                func.baselines = Some(
                    func_llvm
                        .collect_baselines(&func.source, baseline_runs, baseline_iters)
                        .expect("baselines"),
                );
                println!(
                    "  {} O3={} ns",
                    func.name,
                    func.baselines.as_ref().unwrap().o3.mean_ns
                );
            }

            let candidates: Vec<_> = top_seqs.entries.iter().take(top).collect();
            println!(
                "\nRe-benchmarking top {} sequences ({} runs each):\n",
                candidates.len(),
                runs
            );

            let mut json_records: Vec<serde_json::Value> = Vec::new();

            for (rank, entry) in candidates.iter().enumerate() {
                let func = match functions
                    .functions
                    .iter()
                    .find(|f| f.name == entry.func_name)
                {
                    Some(f) => f,
                    None => {
                        println!(
                            "  #{} [{}] func '{}' not found — skipping",
                            rank + 1,
                            entry.speedup,
                            entry.func_name
                        );
                        continue;
                    }
                };
                let baselines = func.baselines.as_ref().unwrap();
                let func_llvm = llvm.with_env(work_dir.join(format!("diag_{}", rank)));
                std::fs::create_dir_all(&func_llvm.work_dir).expect("create diag work dir");

                let mut current_ir = func.ir.clone();
                let pass_strs: Vec<&str> = entry
                    .passes
                    .iter()
                    .filter(|&&p| p != Pass::Stop)
                    .map(|p| p.to_opt())
                    .collect();
                for (step, &pass) in entry.passes.iter().enumerate() {
                    if pass != Pass::Stop {
                        current_ir = func_llvm
                            .apply_one(&current_ir, pass, step)
                            .expect("apply pass");
                    }
                }
                let bin = func_llvm.compile(&current_ir).expect("compile");

                let mut speedups: Vec<f32> = Vec::with_capacity(runs);
                for _ in 0..runs {
                    let bm = func_llvm.benchmark(&bin, 1, iters).expect("benchmark");
                    speedups.push(baselines.speedup_vs_o3(bm.mean_ns));
                }
                speedups.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let mean = speedups.iter().sum::<f32>() / speedups.len() as f32;
                let var = speedups.iter().map(|&x| (x - mean).powi(2)).sum::<f32>()
                    / speedups.len() as f32;
                let std = var.sqrt();
                let med = speedups[speedups.len() / 2];

                println!(
                    "  #{:2}  func={}  cached={:+.4}  mean={:+.4}  std={:.4}  med={:+.4}  [{:+.4}, {:+.4}]",
                    rank + 1,
                    entry.func_name,
                    entry.speedup,
                    mean,
                    std,
                    med,
                    speedups[0],
                    speedups[speedups.len() - 1]
                );
                println!("       passes: [{}]", pass_strs.join(", "));

                json_records.push(serde_json::json!({
                    "rank": rank + 1,
                    "func_name": entry.func_name,
                    "cached_speedup": entry.speedup,
                    "passes": pass_strs,
                    "mean": mean,
                    "std": std,
                    "median": med,
                    "min": speedups[0],
                    "max": speedups[speedups.len() - 1],
                    "all_speedups": speedups,
                }));
            }

            if let Some(out) = output {
                let json = serde_json::to_string_pretty(&json_records).expect("serialize");
                std::fs::write(&out, json).expect("write diagnose json");
                println!("\nsaved → {:?}", out);
            }
        }
        Command::BenchNoise {
            source,
            clang,
            work_dir,
            runs,
            iters,
            workers,
            output,
        } => {
            use crate::llvm::Llvm;
            use crate::llvm::ir::Source;
            use rayon::prelude::*;

            std::fs::create_dir_all(&work_dir).expect("create work dir");
            let llvm = Llvm::new(&clang, "opt-20", work_dir.clone());
            let src = Source {
                file: source.clone(),
            };

            println!("Emitting IR...");
            let ir = llvm.ir(&src).expect("emit IR");
            println!("Compiling IR...");
            let bin = llvm.compile(&ir).expect("compile IR");

            let solo = llvm.benchmark(&bin, runs, iters).expect("solo benchmark");
            println!("\n=== Serial (solo) ===");
            println!("  mean: {} ns", solo.mean_ns);

            let mut rayon_ns: Vec<u64> = (0..workers)
                .into_par_iter()
                .map(|_| {
                    let llvm2 = llvm.clone();
                    let bin2 = crate::llvm::ir::Bin {
                        file: bin.file.clone(),
                    };
                    llvm2
                        .benchmark(&bin2, runs, iters)
                        .expect("rayon worker bench")
                        .mean_ns
                })
                .collect();
            print_stats("Rayon parallel", workers, solo.mean_ns, &mut rayon_ns);

            if let Some(out) = output {
                let json = serde_json::json!({
                    "source": source.display().to_string(),
                    "runs": runs,
                    "iters": iters,
                    "workers": workers,
                    "solo_ns": solo.mean_ns,
                    "parallel_ns": rayon_ns,
                });
                std::fs::write(&out, serde_json::to_string_pretty(&json).unwrap())
                    .expect("write bench_noise json");
                println!("\nsaved → {:?}", out);
            }
        }
        Command::ExportFeatures {
            directory,
            output,
            clang,
            opt,
            work_dir,
            ir_chunks,
        } => {
            use crate::llvm::ir::{
                IR_CATEGORY_COUNT, IR_CATEGORY_NAMES, META_CATEGORY_COUNT, META_CATEGORY_NAMES,
                chunked_opcode_histogram, ir_feature_dim,
            };
            let stride = IR_CATEGORY_COUNT + META_CATEGORY_COUNT;
            std::fs::create_dir_all(&work_dir).expect("create work dir");
            let llvm = Llvm::new(&clang, &opt, work_dir.clone());
            let mut functions = Functions::new(&directory);

            let mut records: Vec<serde_json::Value> = Vec::new();
            for func in &mut functions.functions {
                let func_llvm = llvm.with_env(work_dir.join(&func.name));
                std::fs::create_dir_all(&func_llvm.work_dir).expect("create func work dir");
                let ir = func_llvm.ir(&func.source).expect("emit IR");
                let opcodes = ir.opcode_sequence();
                let hist = chunked_opcode_histogram(&opcodes, ir_chunks);
                let feats = ir.model_features(ir_chunks);

                println!("  {}  raw_opcodes={}", func.name, opcodes.len());
                for c in 0..ir_chunks {
                    let base = c * IR_CATEGORY_COUNT;
                    let chunk = &hist[base..base + IR_CATEGORY_COUNT];
                    print!("    chunk[{}] ", c);
                    for (cat, &v) in chunk.iter().enumerate() {
                        if v > 0.01 {
                            print!("  {}={:.2}", IR_CATEGORY_NAMES[cat], v);
                        }
                    }
                    println!();
                }
                for d in 0..ir_chunks.saturating_sub(1) {
                    let base = d * stride;
                    let op_delta = &feats[base..base + IR_CATEGORY_COUNT];
                    let meta_delta = &feats[base + IR_CATEGORY_COUNT..base + stride];
                    print!("    delta[{}→{}]", d, d + 1);
                    for (cat, &v) in op_delta.iter().enumerate() {
                        if v.abs() > 0.02 {
                            print!("  {}={:+.2}", IR_CATEGORY_NAMES[cat], v);
                        }
                    }
                    for (cat, &v) in meta_delta.iter().enumerate() {
                        if v.abs() > 0.005 {
                            print!("  !{}={:+.3}", META_CATEGORY_NAMES[cat], v);
                        }
                    }
                    println!();
                }

                let n_deltas = ir_chunks.saturating_sub(1);
                let op_deltas: Vec<f32> = (0..n_deltas)
                    .flat_map(|d| {
                        feats[d * stride..d * stride + IR_CATEGORY_COUNT]
                            .iter()
                            .copied()
                    })
                    .collect();
                let meta_deltas: Vec<f32> = (0..n_deltas)
                    .flat_map(|d| {
                        feats[d * stride + IR_CATEGORY_COUNT..d * stride + stride]
                            .iter()
                            .copied()
                    })
                    .collect();

                records.push(serde_json::json!({
                    "name":        func.name,
                    "raw_opcodes": opcodes.len(),
                    "ir_chunks":   ir_chunks,
                    "histogram":   hist,
                    "op_deltas":   op_deltas,
                    "meta_deltas": meta_deltas,
                    "deltas":      feats,
                }));
            }

            let json = serde_json::to_string_pretty(&records).expect("serialize");
            std::fs::write(&output, json).expect("write output");
            println!("Wrote {} entries to {:?}", records.len(), output);
        }
        Command::PlotDiagnose { results } => {
            let cwd = std::env::current_dir().expect("cwd");
            let python = cwd.join(".venv/bin/python");
            let script = cwd.join("scripts/plot_diagnose.py");
            let status = std::process::Command::new(&python)
                .arg(&script)
                .arg("--results")
                .arg(&results)
                .status()
                .expect("failed to spawn python");
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Command::CollectDataset { cache_file, output } => {
            use crate::llvm::{load_cache, BenchCache};
            use std::io::Write;

            let cache = load_cache(&cache_file).expect("load bench cache");
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let mut out = std::fs::File::create(&output).expect("create output file");
            let mut count = 0usize;
            for entry in cache.iter() {
                let (func_name, passes) = entry.key();
                let (speedup, step_deltas) = entry.value();
                let pass_names: Vec<String> = passes
                    .iter()
                    .filter(|&&p| p != crate::llvm::pass::Pass::Stop)
                    .map(|p| format!("{p:?}"))
                    .collect();
                // expand prefixes: length 1..=pass_names.len()
                for len in 1..=pass_names.len().max(1) {
                    let record = serde_json::json!({
                        "func_name": func_name,
                        "passes": &pass_names[..len.min(pass_names.len())],
                        "step_deltas": &step_deltas[..len.min(step_deltas.len())],
                        "speedup": speedup,
                    });
                    serde_json::to_writer(&mut out, &record).expect("write record");
                    writeln!(&mut out).expect("newline");
                    count += 1;
                }
            }
            println!("Wrote {count} samples to {}", output.display());
        }
        Command::PlotDataset { data } => {
            let cwd = std::env::current_dir().expect("cwd");
            let python = cwd.join(".venv/bin/python");
            let script = cwd.join("scripts/plot_dataset.py");
            let status = std::process::Command::new(&python)
                .arg(&script)
                .arg("--data")
                .arg(&data)
                .status()
                .expect("failed to spawn python");
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Command::PlotBenchNoise { results } => {
            let cwd = std::env::current_dir().expect("cwd");
            let python = cwd.join(".venv/bin/python");
            let script = cwd.join("scripts/plot_bench_noise.py");
            let status = std::process::Command::new(&python)
                .arg(&script)
                .arg("--results")
                .arg(&results)
                .status()
                .expect("failed to spawn python");
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Command::PlotFeatures { features } => {
            let cwd = std::env::current_dir().expect("cwd");
            let python = cwd.join(".venv/bin/python");
            let script = cwd.join("scripts/plot_features.py");
            let status = std::process::Command::new(&python)
                .arg(&script)
                .arg("--features")
                .arg(&features)
                .status()
                .expect("failed to spawn python");
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
    }
}
