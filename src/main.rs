#![recursion_limit = "256"]
#![allow(unused)]
use crate::config::{BurnAutoDiff, BurnDevice, Cfg};
use crate::llvm::Llvm;
use crate::llvm::functions::Functions;
use crate::llvm::pass::Pass;
use crate::llvm::top_sequences::TopSequences;
use crate::ppo::advantages::baseline::BaselineAdvantage;
use crate::ppo::advantages::group_relative::GroupRelativeAdvantage;
use crate::ppo::checkpoint::Checkpoint;
use crate::ppo::logging::LogMode;
use crate::ppo::returns::episode_return::EpisodeReturn;
use crate::ppo::returns::instruction_proxy::InstructionProxyReturn;
use crate::ppo::returns::instruction_weighted_terminal::InstructionWeightedTerminal;
use crate::ppo::returns::ir_count_return::IrCountReturn;
use crate::ppo::returns::ir_step_return::IrStepReturn;
use crate::train::Trainer;
use burn::module::AutodiffModule;
use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;

mod config;
mod llvm;
mod ppo;
mod predictor;
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
        /// PPO inner-loop KL early-stop threshold.
        /// If per-minibatch KL exceeds this after the first inner epoch, the
        /// remaining inner epochs are skipped.  Prevents entropy collapse from
        /// a single over-large update.  Set to 0 to disable.
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
        /// Blend weight for terminal speedup in instruction proxy returns.
        /// 1.0 = pure speedup (default), 0.0 = pure instruction-count delta.
        /// Values in (0, 1) blend both signals for denser credit assignment.
        /// Only used when --returns=proxy.
        #[arg(long, default_value = "0.5")]
        proxy_alpha: f32,
        /// Return signal:
        ///   episode  — uniform terminal speedup across all slots
        ///   proxy    — blended instr+terminal (see --proxy-alpha)
        ///   weighted — terminal weighted by per-slot instr reduction; no-ops get 0
        ///   predictor — per-step marginal from pretrained SpeedupPredictor
        ///   ir       — terminal IR-count reduction, uniform across slots
        ///   ir-step  — per-step IR-count delta (dense; ideal for --features auto-tfx/gru)
        #[arg(long, default_value = "weighted")]
        returns: String,
        /// Path to predictor checkpoint directory. Required when --returns=predictor.
        #[arg(long)]
        predictor_checkpoint: Option<PathBuf>,
        /// Scale factor applied to all predictor returns.
        #[arg(long, default_value = "1.0")]
        predictor_scale: f32,
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
        /// Advantage estimator:
        ///   baseline — return minus learned value (standard PPO)
        ///   grpo     — group-relative: normalise by per-function mean/std, no value head
        #[arg(long, default_value = "baseline")]
        advantages: String,
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
        /// Beam width for beam-search decoding (0 = disabled).
        #[arg(long, default_value = "0")]
        beam_width: usize,
        #[arg(long, default_value = "4")]
        ir_chunks: usize,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    PlotTrain {
        #[arg(long, default_value = "checkpoints")]
        dir: PathBuf,
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
    PlotPredictor {
        #[arg(long, default_value = "predictor_checkpoints/train.jsonl")]
        log: PathBuf,
    },
    // Inside Command enum, add:
    Collect {
        #[arg(long, default_value = "checkpoints/data.cache")]
        cache_file: PathBuf,
        #[arg(long, default_value = "benchmarks")]
        functions_dir: PathBuf,
        #[arg(long, default_value = "clang-20")]
        clang: String,
        #[arg(long, default_value = "opt-20")]
        opt: String,
        #[arg(long, default_value = "work/collect")]
        work_dir: PathBuf,
        #[arg(long, default_value = "dataset.jsonl")]
        output: PathBuf,
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
    TrainPredictor {
        #[arg(long, default_value = "dataset.jsonl")]
        data: PathBuf,
        #[arg(long, default_value = "predictor_checkpoints")]
        checkpoint_dir: PathBuf,
        /// Directory containing the benchmark .c source files (needed to emit IR at load time).
        #[arg(long, default_value = "benchmarks")]
        functions_dir: PathBuf,
        #[arg(long, default_value = "clang-20")]
        clang: String,
        #[arg(long, default_value = "opt-20")]
        opt: String,
        #[arg(long, default_value = "work/predictor_ir")]
        work_dir: PathBuf,
        #[arg(long, default_value = "100")]
        epochs: usize,
        #[arg(long, default_value = "3072")]
        batch_size: usize,
        #[arg(long, default_value = "1e-3")]
        learning_rate: f64,
        #[arg(long, default_value = "0.2")]
        val_split: f32,
        #[arg(long, default_value = "20")]
        max_seq_len: usize,
        /// Number of positional chunks for the IR histogram (match the value used during Train).
        #[arg(long, default_value = "4")]
        ir_chunks: usize,
        #[arg(long, default_value = "256")]
        d_model: usize,
        #[arg(long, default_value = "8")]
        n_heads: usize,
        #[arg(long, default_value = "4")]
        n_layers: usize,
        #[arg(long, default_value = "512")]
        d_ff: usize,
        #[arg(long, default_value = "0.3")]
        dropout: f64,
        #[arg(long, default_value = "-3.0")]
        clip_min: f32,
        #[arg(long, default_value = "2.0")]
        huber_delta: f32,
        #[arg(long)]
        max_samples: Option<usize>,
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
            proxy_alpha,
            returns,
            noop_threshold,
            noop_feature_threshold,
            noop_penalty,
            weighted_direction_bonus,
            advantages,
            predictor_checkpoint,
            predictor_scale,
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
                skip_benchmark: returns == "ir" || returns == "ir-step",
                kl_target,
            };
            let log_path = checkpoint_dir.join("train.jsonl");
            let seq_path =
                sequences_file.or_else(|| Some(checkpoint_dir.join("top_sequences.bin")));
            let returns_impl: Box<dyn crate::ppo::returns::Returns> = match returns.as_str() {
                "proxy" => Box::new(InstructionProxyReturn {
                    alpha: proxy_alpha,
                    noop,
                }),
                "weighted" => Box::new(InstructionWeightedTerminal {
                    noop,
                    direction_bonus: weighted_direction_bonus,
                }),
                "predictor" => {
                    let ckpt = predictor_checkpoint
                        .expect("--predictor-checkpoint required when --returns=predictor");
                    Box::new(
                        crate::ppo::returns::predictor_return::PredictorReturn::load(
                            &ckpt,
                            noop,
                            predictor_scale,
                        )
                        .expect("failed to load predictor checkpoint"),
                    )
                }
                "ir" => Box::new(IrCountReturn),
                "ir-step" => Box::new(IrStepReturn { noop }),
                _ => Box::new(EpisodeReturn),
            };
            let trainer = Trainer::new(
                cfg,
                returns_impl,
                match advantages.as_str() {
                    "grpo" => Box::new(GroupRelativeAdvantage) as Box<dyn crate::ppo::advantages::Advantages>,
                    _ => Box::new(BaselineAdvantage),
                },
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
            beam_width,
            ir_chunks,
            output,
        } => {
            #[cfg(any(feature = "auto-tfx", feature = "auto-gru"))]
            {
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

                let col_beam = beam_width > 0;
                let col_samples = policy_samples > 0;

                // All numeric columns are 9 wide; header labels match exactly.
                let mut header = format!(
                    "\n{:<22} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}",
                    "function", "O0", "O1", "O2", "rand_mean", "greedy", "rand_best"
                );
                if col_samples { header.push_str(&format!(" {:>9}", "samp_best")); }
                if col_beam    { header.push_str(&format!(" {:>9}", "beam")); }
                let sep_len = 22 + (9 + 1) * (6 + if col_samples { 1 } else { 0 } + if col_beam { 1 } else { 0 });
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
                    for rand_i in 0..random_sequences {
                        let rand_llvm = llvm.with_env(
                            work_dir.join(format!("eval_{}_r{rand_i}", func.name)),
                        );
                        std::fs::create_dir_all(&rand_llvm.work_dir).expect("create rand dir");
                        let mut cur = func.ir.clone();
                        for step in 0..max_seq_len {
                            let idx = (rand::random::<f32>() * crate::ppo::model::ACTIONS.len() as f32) as usize;
                            let action = crate::ppo::model::ACTIONS[idx];
                            if action == Pass::Stop { break; }
                            cur = rand_llvm.apply_one(&cur, action, step).expect("apply rand");
                        }
                        let bin = rand_llvm.compile(&cur).expect("compile rand");
                        let bm = rand_llvm.benchmark(&bin, runs, iters).expect("bench rand");
                        random_speedups.push(baselines.speedup_vs_o3(bm.mean_ns));
                    }
                    let rand_mean = random_speedups.iter().sum::<f32>() / random_speedups.len().max(1) as f32;
                    let rand_best = random_speedups.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

                    // ── Policy samples (stochastic rollouts) ──────────────────────────
                    let mut sample_speedups: Vec<f32> = Vec::new();
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
                            for step in 0..max_seq_len {
                                ir_feats.push(cur.model_features(ir_chunks));
                                let (logits, _, new_hid) = inference_model
                                    .infer_step_stateful(&ir_feats, &acts, hid, &device);
                                hid = new_hid;
                                let (idx, _lp) = sample_logits(&logits);
                                let action = crate::ppo::model::ACTIONS[idx];
                                acts.push(idx);
                                if action == Pass::Stop { break; }
                                cur = samp_llvm.apply_one(&cur, action, step).expect("apply samp");
                            }
                            let bin = samp_llvm.compile(&cur).expect("compile samp");
                            let bm = samp_llvm.benchmark(&bin, runs, iters).expect("bench samp");
                            sample_speedups.push(baselines.speedup_vs_o3(bm.mean_ns));
                        }
                    }
                    let samp_best = sample_speedups.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

                    // ── Beam search ───────────────────────────────────────────────────
                    // Each beam: (current_ir, ir_features, action_history, cumulative_log_prob, passes)
                    let beam_speedup = if col_beam {
                        struct Beam {
                            ir: crate::llvm::ir::Ir,
                            ir_feats: Vec<Vec<f32>>,
                            acts: Vec<usize>,
                            log_prob: f32,
                            passes: Vec<String>,
                        }
                        let mut beams: Vec<Beam> = vec![Beam {
                            ir: func.ir.clone(),
                            ir_feats: Vec::new(),
                            acts: Vec::new(),
                            log_prob: 0.0,
                            passes: Vec::new(),
                        }];
                        let beam_llvm = llvm.with_env(work_dir.join(format!("eval_{}_beam", func.name)));
                        std::fs::create_dir_all(&beam_llvm.work_dir).expect("create beam dir");

                        for step in 0..max_seq_len {
                            let mut candidates: Vec<Beam> = Vec::new();
                            for beam in &beams {
                                let mut feats = beam.ir_feats.clone();
                                feats.push(beam.ir.model_features(ir_chunks));
                                let (logits, _, _) = inference_model
                                    .infer_step_stateful(&feats, &beam.acts, None, &device);

                                // Compute log-probs for all actions.
                                let max_l = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                                let exp: Vec<f32> = logits.iter().map(|&x| (x - max_l).exp()).collect();
                                let sum: f32 = exp.iter().sum::<f32>().max(f32::EPSILON);

                                for (idx, &e) in exp.iter().enumerate() {
                                    let lp = (e / sum).max(f32::EPSILON).ln();
                                    let action = crate::ppo::model::ACTIONS[idx];
                                    let mut new_acts = beam.acts.clone();
                                    new_acts.push(idx);
                                    let mut new_passes = beam.passes.clone();
                                    let new_ir = if action == Pass::Stop {
                                        beam.ir.clone()
                                    } else {
                                        new_passes.push(format!("{action:?}"));
                                        beam_llvm.apply_one(&beam.ir, action, step).expect("beam apply")
                                    };
                                    candidates.push(Beam {
                                        ir: new_ir,
                                        ir_feats: feats.clone(),
                                        acts: new_acts,
                                        log_prob: beam.log_prob + lp,
                                        passes: new_passes,
                                    });
                                }
                            }
                            // Keep top beam_width by log_prob. Stop beams are included.
                            candidates.sort_by(|a, b| b.log_prob.partial_cmp(&a.log_prob).unwrap_or(std::cmp::Ordering::Equal));
                            candidates.truncate(beam_width);
                            // Remove beams that chose Stop (they're done; keep IR as final).
                            beams = candidates.into_iter().filter(|b| {
                                b.acts.last().map(|&i| crate::ppo::model::ACTIONS[i] != Pass::Stop).unwrap_or(true)
                            }).collect();
                            if beams.is_empty() { break; }
                        }

                        // Benchmark all surviving beams + stopped beams; just use surviving.
                        // (beams that stopped were dropped; in practice stop is usually last)
                        // Benchmark the best log_prob beam.
                        if let Some(best_beam) = beams.into_iter().max_by(|a, b| a.log_prob.partial_cmp(&b.log_prob).unwrap_or(std::cmp::Ordering::Equal)) {
                            let bin = beam_llvm.compile(&best_beam.ir).expect("compile beam");
                            let bm = beam_llvm.benchmark(&bin, runs, iters).expect("bench beam");
                            Some(baselines.speedup_vs_o3(bm.mean_ns))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // ── Print row ─────────────────────────────────────────────────────
                    let mut row = format!(
                        "{:<22} {:>+9.3} {:>+9.3} {:>+9.3} {:>+9.3} {:>+9.3} {:>+9.3}",
                        func.name, o0_speedup, o1_speedup, o2_speedup,
                        rand_mean, greedy_speedup, rand_best,
                    );
                    if col_samples {
                        row.push_str(&format!(" {:>+9.3}", samp_best));
                    }
                    if col_beam {
                        match beam_speedup {
                            Some(s) => row.push_str(&format!(" {:>+9.3}", s)),
                            None    => row.push_str(&format!(" {:>9}", "—")),
                        }
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
                        "random_speedups": random_speedups,
                    });
                    if col_samples {
                        record["sample_best_speedup"] = serde_json::json!(samp_best);
                        record["sample_speedups"] = serde_json::json!(sample_speedups);
                    }
                    if let Some(s) = beam_speedup {
                        record["beam_speedup"] = serde_json::json!(s);
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
            #[cfg(not(any(feature = "auto-tfx", feature = "auto-gru")))]
            {
                eprintln!("evaluate requires --features auto-tfx or auto-gru");
                std::process::exit(1);
            }
        }
        Command::PlotTrain { dir } => {
            // Resolve paths relative to the current working directory.
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

            // Collect serial baselines for each function.
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

                // Apply passes (skip Stop).
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

                // Benchmark `runs` times individually for std.
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
        Command::Collect {
            cache_file,
            functions_dir,
            clang,
            opt,
            work_dir,
            output,
        } => {
            use crate::llvm::Llvm;
            use crate::llvm::functions::Functions;
            use crate::llvm::pass::Pass;
            use std::collections::HashMap;

            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent).expect("create output dir");
            }

            // Load cache
            let cache_data: Vec<((String, Vec<Pass>), (f32, Vec<f32>))> = {
                let bytes = std::fs::read(&cache_file).expect("read cache file");
                bincode::deserialize(&bytes).expect("deserialize cache")
            };

            println!("loaded cache w/ {} samples", cache_data.len());
            // Group by func_name
            let mut func_cache: HashMap<String, Vec<(Vec<Pass>, f32, Vec<f32>)>> = HashMap::new();
            for ((func_name, passes), (speedup, step_deltas)) in cache_data {
                func_cache
                    .entry(func_name)
                    .or_default()
                    .push((passes, speedup, step_deltas));
            }

            // Setup LLVM to extract IR opcode sequences for each function.
            let llvm = Llvm::new(&clang, &opt, work_dir.clone());
            let mut functions = Functions::new(&functions_dir);
            std::fs::create_dir_all(&work_dir).expect("create work dir");

            let mut out_file = std::fs::File::create(&output).expect("create output file");
            for func in &mut functions.functions {
                println!("Processing {}...", func.name);

                if let Some(entries) = func_cache.get(&func.name) {
                    for (passes, speedup, step_deltas) in entries {
                        for len in 1..=passes.len() {
                            let sample = crate::predictor::data::Sample {
                                func_name: func.name.clone(),
                                passes: passes[0..len].to_vec(),
                                step_deltas: step_deltas[0..len].to_vec(),
                                speedup: *speedup,
                            };
                            serde_json::to_writer(&mut out_file, &sample).expect("write sample");
                            writeln!(&mut out_file).expect("newline");
                        }
                    }
                }
            }
            println!("Dataset written to {:?}", output);
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
                // Histogram for human-readable display only.
                let hist = chunked_opcode_histogram(&opcodes, ir_chunks);
                // Deltas — the actual model input.
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

                // Split feats into opcode and meta sections for JSON.
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
        Command::TrainPredictor {
            data,
            checkpoint_dir,
            functions_dir,
            clang,
            opt,
            work_dir,
            epochs,
            batch_size,
            learning_rate,
            val_split,
            max_seq_len,
            ir_chunks,
            d_model,
            n_heads,
            n_layers,
            d_ff,
            dropout,
            clip_min,
            huber_delta,
            max_samples,
        } => {
            let config = crate::predictor::model::SpeedupPredictorConfig {
                num_passes: 29,
                ir_chunks,
                output_dim: 1,
                d_model,
                n_heads,
                n_layers,
                d_ff,
                dropout,
                max_seq_len,
            };
            crate::predictor::train::train_predictor(
                &data,
                &checkpoint_dir,
                &functions_dir,
                &clang,
                &opt,
                &work_dir,
                epochs,
                batch_size,
                learning_rate,
                val_split,
                clip_min,
                huber_delta,
                max_samples,
                config,
            )
            .expect("training failed");
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
        Command::PlotPredictor { log } => {
            let cwd = std::env::current_dir().expect("cwd");
            let python = cwd.join(".venv/bin/python");
            let script = cwd.join("scripts/plot_predictor.py");
            let status = std::process::Command::new(&python)
                .arg(&script)
                .arg("--log")
                .arg(&log)
                .status()
                .expect("failed to spawn python");
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
    }
}
