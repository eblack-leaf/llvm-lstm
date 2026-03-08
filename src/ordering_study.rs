use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use rayon::prelude::*;
use serde::Serialize;
use statrs::distribution::{ChiSquared, ContinuousCDF};

use crate::pass_menu::Pass;
use crate::pipeline::CompilationPipeline;

// ---------------------------------------------------------------------------
// Target functions and passes for the study
// ---------------------------------------------------------------------------

const TARGET_FUNCTIONS: &[&str] = &[
    "struct_pack",
    "select_chain",
    "tail_recursive",
    "kmp_search",
    "convolution",
    "stencil2d",
];

const TOP5_PASSES: &[Pass] = &[
    Pass::Sroa,
    Pass::Mem2reg,
    Pass::Gvn,
    Pass::Licm,
    Pass::Simplifycfg,
];

// ---------------------------------------------------------------------------
// Serializable result types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct Exp1Results {
    per_function: Vec<Exp1FunctionResult>,
}

#[derive(Debug, Serialize)]
struct Exp1FunctionResult {
    function: String,
    o0_ns: u64,
    o3_ns: u64,
    permutations: Vec<PermutationResult>,
    best_ordering: Vec<String>,
    worst_ordering: Vec<String>,
    best_ns: u64,
    worst_ns: u64,
    spread_pct: f64,
    kruskal_wallis_h: f64,
    kruskal_wallis_p: f64,
    significant: bool,
}

#[derive(Debug, Clone, Serialize)]
struct PermutationResult {
    ordering: Vec<String>,
    pipeline_string: String,
    median_ns: u64,
    all_times_ns: Vec<u64>,
}

#[derive(Debug, Serialize)]
struct Exp2Results {
    per_function: Vec<Exp2FunctionResult>,
}

#[derive(Debug, Serialize)]
struct Exp2FunctionResult {
    function: String,
    per_step: Vec<Exp2StepResult>,
}

#[derive(Debug, Serialize)]
struct Exp2StepResult {
    prefix_length: usize,
    num_prefixes: usize,
    best_ns: u64,
    worst_ns: u64,
    spread_pct: f64,
    prefixes: Vec<PrefixResult>,
}

#[derive(Debug, Clone, Serialize)]
struct PrefixResult {
    passes: Vec<String>,
    median_ns: u64,
}

#[derive(Debug, Serialize)]
struct Exp3Results {
    o3_pipeline_raw: String,
    top_level_pass_count: usize,
    per_function: Vec<Exp3FunctionResult>,
}

#[derive(Debug, Serialize)]
struct Exp3FunctionResult {
    function: String,
    native_o3_ns: u64,
    full_o3_via_opt_ns: u64,
    our_passes_o3_order_ns: u64,
    our_passes_o3_rep_ns: u64,
    shuffled_results: Vec<ShuffledResult>,
    subsequence_results: Vec<SubsequenceResult>,
    our_passes_in_o3: Vec<String>,
    our_passes_in_o3_with_rep: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ShuffledResult {
    ordering: Vec<String>,
    median_ns: u64,
}

#[derive(Debug, Clone, Serialize)]
struct SubsequenceResult {
    label: String,
    pipeline: String,
    median_ns: u64,
}

// ---------------------------------------------------------------------------
// Permutation generation (Heap's algorithm)
// ---------------------------------------------------------------------------

fn all_permutations<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
    let n = items.len();
    let mut result = Vec::new();
    let mut arr = items.to_vec();
    let mut c = vec![0usize; n];

    result.push(arr.clone());

    let mut i = 0;
    while i < n {
        if c[i] < i {
            if i % 2 == 0 {
                arr.swap(0, i);
            } else {
                arr.swap(c[i], i);
            }
            result.push(arr.clone());
            c[i] += 1;
            i = 0;
        } else {
            c[i] = 0;
            i += 1;
        }
    }

    result
}

/// Generate all unique prefixes of all permutations at a given length.
fn unique_prefixes(perms: &[Vec<Pass>], len: usize) -> Vec<Vec<Pass>> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for perm in perms {
        let prefix: Vec<Pass> = perm[..len].to_vec();
        let key: Vec<String> = prefix.iter().map(|p| p.opt_name().to_string()).collect();
        if seen.insert(key) {
            result.push(prefix);
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Kruskal-Wallis H test
// ---------------------------------------------------------------------------

fn kruskal_wallis(groups: &[&[u64]]) -> (f64, f64) {
    // Flatten and rank all observations
    let mut all: Vec<(u64, usize)> = Vec::new(); // (value, group_index)
    for (gi, group) in groups.iter().enumerate() {
        for &val in *group {
            all.push((val, gi));
        }
    }
    let n_total = all.len() as f64;
    if n_total < 3.0 {
        return (0.0, 1.0);
    }

    // Sort by value
    all.sort_by_key(|&(v, _)| v);

    // Assign ranks (handle ties by averaging)
    let mut ranks = vec![0.0f64; all.len()];
    let mut i = 0;
    while i < all.len() {
        let mut j = i;
        while j < all.len() && all[j].0 == all[i].0 {
            j += 1;
        }
        let avg_rank = (i + 1 + j) as f64 / 2.0; // 1-indexed
        for k in i..j {
            ranks[k] = avg_rank;
        }
        i = j;
    }

    // Sum of ranks per group
    let k = groups.len();
    let mut rank_sums = vec![0.0f64; k];
    let mut group_sizes = vec![0usize; k];
    for (idx, &(_, gi)) in all.iter().enumerate() {
        rank_sums[gi] += ranks[idx];
        group_sizes[gi] += 1;
    }

    // H statistic
    let mut h = 0.0;
    for i in 0..k {
        let ni = group_sizes[i] as f64;
        if ni > 0.0 {
            h += rank_sums[i].powi(2) / ni;
        }
    }
    h = 12.0 / (n_total * (n_total + 1.0)) * h - 3.0 * (n_total + 1.0);

    // p-value from chi-squared distribution with k-1 degrees of freedom
    let df = (k - 1) as f64;
    if df <= 0.0 {
        return (h, 1.0);
    }
    let chi2 = ChiSquared::new(df).unwrap();
    let p = 1.0 - chi2.cdf(h);

    (h, p)
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn run(
    benchmarks_dir: &Path,
    output_dir: &Path,
    experiments: &str,
    runs: usize,
    threads: usize,
) -> Result<()> {
    if threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .ok();
    }

    fs::create_dir_all(output_dir)?;

    let run_exp1 = experiments == "all" || experiments == "1";
    let run_exp2 = experiments == "all" || experiments == "2";
    let run_exp3 = experiments == "all" || experiments == "3";

    if run_exp1 {
        let t0 = std::time::Instant::now();
        eprintln!("\n========== Experiment 1: Full Permutation Test ==========");
        experiment1(benchmarks_dir, output_dir, runs)?;
        eprintln!("Experiment 1 done in {:.1}s", t0.elapsed().as_secs_f64());
    }

    if run_exp2 {
        let t0 = std::time::Instant::now();
        eprintln!("\n========== Experiment 2: Incremental Build-up ==========");
        experiment2(benchmarks_dir, output_dir, runs)?;
        eprintln!("Experiment 2 done in {:.1}s", t0.elapsed().as_secs_f64());
    }

    if run_exp3 {
        let t0 = std::time::Instant::now();
        eprintln!("\n========== Experiment 3: O3 Pipeline Analysis ==========");
        experiment3(benchmarks_dir, output_dir, runs)?;
        eprintln!("Experiment 3 done in {:.1}s", t0.elapsed().as_secs_f64());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Experiment 1: Full Permutation Test
// ---------------------------------------------------------------------------

fn experiment1(benchmarks_dir: &Path, output_dir: &Path, runs: usize) -> Result<()> {
    let permutations = all_permutations(TOP5_PASSES);
    assert_eq!(permutations.len(), 120, "5! should be 120");

    // Verify distinct pipeline strings
    {
        let mut pipelines: Vec<String> = permutations
            .iter()
            .map(|p| Pass::to_opt_pipeline(p))
            .collect();
        pipelines.sort();
        pipelines.dedup();
        eprintln!(
            "  {} permutations -> {} distinct pipeline strings",
            permutations.len(),
            pipelines.len()
        );
    }

    let func_paths: Vec<PathBuf> = TARGET_FUNCTIONS
        .iter()
        .map(|f| benchmarks_dir.join(format!("{f}.c")))
        .collect();

    // Verify all target functions exist
    for p in &func_paths {
        if !p.exists() {
            bail!("Target benchmark not found: {}", p.display());
        }
    }

    eprintln!(
        "  {} permutations x {} functions x {} runs = {} pipeline runs",
        permutations.len(),
        func_paths.len(),
        runs,
        permutations.len() * func_paths.len() * runs,
    );

    let results: Vec<Result<Exp1FunctionResult>> = func_paths
        .par_iter()
        .map(|func_path| {
            let stem = func_path.file_stem().unwrap().to_string_lossy().to_string();
            let work_dir = output_dir.join("_work").join(&stem);
            fs::create_dir_all(&work_dir)?;
            let pipeline = CompilationPipeline::new(work_dir);

            // Baselines
            let o0 = pipeline.baseline(func_path, "-O0", runs)?;
            let o3 = pipeline.baseline(func_path, "-O3", runs)?;
            eprintln!("  [{stem}] O0={} ns, O3={} ns", o0.median_ns, o3.median_ns);

            // Emit IR once
            let ir = pipeline.emit_ir(func_path)?;

            let mut perm_results: Vec<PermutationResult> = Vec::with_capacity(120);

            for (i, perm) in permutations.iter().enumerate() {
                let pipeline_str = Pass::to_opt_pipeline(perm);
                let opt_ir = pipeline
                    .work_dir()
                    .join(format!("{stem}_perm{i}.ll"));
                pipeline.apply_passes(&ir, perm, &opt_ir)?;
                let binary = pipeline.compile_ir(&opt_ir)?;
                let bench = pipeline.benchmark(&binary, runs)?;

                perm_results.push(PermutationResult {
                    ordering: perm.iter().map(|p| p.opt_name().to_string()).collect(),
                    pipeline_string: pipeline_str,
                    median_ns: bench.median_ns,
                    all_times_ns: bench.all_times_ns,
                });

                if (i + 1) % 30 == 0 {
                    eprintln!("  [{stem}] {}/{} permutations done", i + 1, 120);
                }
            }

            // Analysis
            let best = perm_results.iter().min_by_key(|r| r.median_ns).unwrap();
            let worst = perm_results.iter().max_by_key(|r| r.median_ns).unwrap();
            let spread = (worst.median_ns as f64 - best.median_ns as f64)
                / best.median_ns as f64
                * 100.0;

            // Kruskal-Wallis
            let groups: Vec<&[u64]> = perm_results
                .iter()
                .map(|r| r.all_times_ns.as_slice())
                .collect();
            let (h, p) = kruskal_wallis(&groups);

            eprintln!(
                "  [{stem}] best={} ns, worst={} ns, spread={:.1}%, H={:.2}, p={:.4}",
                best.median_ns, worst.median_ns, spread, h, p,
            );

            Ok(Exp1FunctionResult {
                function: stem,
                o0_ns: o0.median_ns,
                o3_ns: o3.median_ns,
                best_ordering: best.ordering.clone(),
                worst_ordering: worst.ordering.clone(),
                best_ns: best.median_ns,
                worst_ns: worst.median_ns,
                spread_pct: spread,
                kruskal_wallis_h: h,
                kruskal_wallis_p: p,
                significant: p < 0.05,
                permutations: perm_results,
            })
        })
        .collect();

    let per_function: Vec<Exp1FunctionResult> = results
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

    let exp1 = Exp1Results { per_function };

    // Write JSON
    let json_path = output_dir.join("exp1_permutations.json");
    let file = File::create(&json_path)?;
    serde_json::to_writer_pretty(file, &exp1)?;
    eprintln!("  Wrote {}", json_path.display());

    // Write report
    let report = exp1_report(&exp1);
    let report_path = output_dir.join("exp1_report.txt");
    fs::write(&report_path, &report)?;
    eprintln!("  Wrote {}", report_path.display());

    Ok(())
}

fn exp1_report(results: &Exp1Results) -> String {
    let mut r = String::new();
    r.push_str("================================================================\n");
    r.push_str("  Experiment 1: Full Permutation Test\n");
    r.push_str("  All 120 permutations of {sroa, mem2reg, gvn, licm, simplifycfg}\n");
    r.push_str("================================================================\n\n");

    r.push_str(&format!(
        "{:<20} {:>10} {:>10} {:>10} {:>10} {:>8} {:>10} {:>8}\n",
        "Function", "O0 (ns)", "O3 (ns)", "Best", "Worst", "Spread%", "K-W H", "p-value"
    ));
    r.push_str(&format!("{}\n", "-".repeat(96)));

    for f in &results.per_function {
        let sig = if f.significant { " ***" } else { "" };
        r.push_str(&format!(
            "{:<20} {:>10} {:>10} {:>10} {:>10} {:>7.1}% {:>10.2} {:>7.4}{}\n",
            f.function, f.o0_ns, f.o3_ns, f.best_ns, f.worst_ns,
            f.spread_pct, f.kruskal_wallis_h, f.kruskal_wallis_p, sig,
        ));
    }

    r.push_str("\n  *** = significant at p < 0.05 (Kruskal-Wallis H test)\n");

    // Best/worst orderings
    r.push_str("\nBest and Worst Orderings per Function:\n");
    r.push_str(&format!("{}\n", "-".repeat(80)));
    for f in &results.per_function {
        r.push_str(&format!(
            "  {:<20}\n    Best:  {} ({} ns)\n    Worst: {} ({} ns)\n",
            f.function,
            f.best_ordering.join(" -> "),
            f.best_ns,
            f.worst_ordering.join(" -> "),
            f.worst_ns,
        ));
    }

    // Top 10 orderings per function
    r.push_str("\nTop 10 Orderings per Function:\n");
    r.push_str(&format!("{}\n", "-".repeat(80)));
    for f in &results.per_function {
        let mut sorted: Vec<&PermutationResult> = f.permutations.iter().collect();
        sorted.sort_by_key(|p| p.median_ns);
        r.push_str(&format!("  {} (best={}, worst={}):\n", f.function, f.best_ns, f.worst_ns));
        for (i, p) in sorted.iter().take(10).enumerate() {
            r.push_str(&format!(
                "    #{:<3} {:>10} ns  {}\n",
                i + 1,
                p.median_ns,
                p.ordering.join(" -> "),
            ));
        }
        r.push_str("\n");
    }

    // Summary
    let sig_count = results.per_function.iter().filter(|f| f.significant).count();
    let avg_spread: f64 = results.per_function.iter().map(|f| f.spread_pct).sum::<f64>()
        / results.per_function.len() as f64;
    let max_spread = results
        .per_function
        .iter()
        .map(|f| f.spread_pct)
        .fold(0.0f64, f64::max);

    r.push_str("\nSummary:\n");
    r.push_str(&format!("  Functions tested: {}\n", results.per_function.len()));
    r.push_str(&format!(
        "  Significant ordering effects: {}/{}\n",
        sig_count,
        results.per_function.len()
    ));
    r.push_str(&format!("  Average spread: {:.1}%\n", avg_spread));
    r.push_str(&format!("  Max spread: {:.1}%\n", max_spread));

    if sig_count > results.per_function.len() / 2 {
        r.push_str("\n  CONCLUSION: Strong evidence that pass ordering matters.\n");
        r.push_str("  The LSTM should learn to order these passes carefully.\n");
    } else if sig_count > 0 {
        r.push_str("\n  CONCLUSION: Some ordering effects detected.\n");
        r.push_str("  Ordering matters for specific functions, not universally.\n");
    } else if max_spread > 5.0 {
        r.push_str("\n  CONCLUSION: Spread exists but not statistically significant.\n");
        r.push_str("  May need more runs to detect reliably. Ordering is secondary.\n");
    } else {
        r.push_str("\n  CONCLUSION: Minimal ordering effects for these 5 passes.\n");
        r.push_str("  Pass selection matters more than ordering.\n");
    }

    r.push_str("\n================================================================\n");
    r
}

// ---------------------------------------------------------------------------
// Experiment 2: Incremental Build-up
// ---------------------------------------------------------------------------

fn experiment2(benchmarks_dir: &Path, output_dir: &Path, runs: usize) -> Result<()> {
    let permutations = all_permutations(TOP5_PASSES);

    // Count total unique prefixes
    let mut total_prefixes = 0;
    for len in 1..=5 {
        total_prefixes += unique_prefixes(&permutations, len).len();
    }
    eprintln!("  {} unique prefixes across lengths 1-5", total_prefixes);

    let func_paths: Vec<PathBuf> = TARGET_FUNCTIONS
        .iter()
        .map(|f| benchmarks_dir.join(format!("{f}.c")))
        .collect();

    for p in &func_paths {
        if !p.exists() {
            bail!("Target benchmark not found: {}", p.display());
        }
    }

    eprintln!(
        "  {} prefixes x {} functions x {} runs = {} pipeline runs",
        total_prefixes,
        func_paths.len(),
        runs,
        total_prefixes * func_paths.len() * runs,
    );

    let results: Vec<Result<Exp2FunctionResult>> = func_paths
        .par_iter()
        .map(|func_path| {
            let stem = func_path.file_stem().unwrap().to_string_lossy().to_string();
            let work_dir = output_dir.join("_work").join(&stem);
            fs::create_dir_all(&work_dir)?;
            let pipeline = CompilationPipeline::new(work_dir);

            let ir = pipeline.emit_ir(func_path)?;

            let mut per_step: Vec<Exp2StepResult> = Vec::new();
            let mut done = 0usize;

            for prefix_len in 1..=5 {
                let prefixes = unique_prefixes(&permutations, prefix_len);
                let mut prefix_results: Vec<PrefixResult> = Vec::with_capacity(prefixes.len());

                for (i, prefix) in prefixes.iter().enumerate() {
                    let opt_ir = pipeline
                        .work_dir()
                        .join(format!("{stem}_step{prefix_len}_{i}.ll"));
                    pipeline.apply_passes(&ir, prefix, &opt_ir)?;
                    let binary = pipeline.compile_ir(&opt_ir)?;
                    let bench = pipeline.benchmark(&binary, runs)?;

                    prefix_results.push(PrefixResult {
                        passes: prefix.iter().map(|p| p.opt_name().to_string()).collect(),
                        median_ns: bench.median_ns,
                    });

                    done += 1;
                }

                let best = prefix_results.iter().min_by_key(|r| r.median_ns).unwrap();
                let worst = prefix_results.iter().max_by_key(|r| r.median_ns).unwrap();
                let spread = if best.median_ns > 0 {
                    (worst.median_ns as f64 - best.median_ns as f64)
                        / best.median_ns as f64
                        * 100.0
                } else {
                    0.0
                };

                per_step.push(Exp2StepResult {
                    prefix_length: prefix_len,
                    num_prefixes: prefix_results.len(),
                    best_ns: best.median_ns,
                    worst_ns: worst.median_ns,
                    spread_pct: spread,
                    prefixes: prefix_results,
                });

                eprintln!(
                    "  [{stem}] step {prefix_len}: {} prefixes done (total {done}), spread={spread:.1}%",
                    per_step.last().unwrap().num_prefixes,
                );
            }

            Ok(Exp2FunctionResult {
                function: stem,
                per_step,
            })
        })
        .collect();

    let per_function: Vec<Exp2FunctionResult> = results
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

    let exp2 = Exp2Results { per_function };

    // Write JSON
    let json_path = output_dir.join("exp2_buildup.json");
    let file = File::create(&json_path)?;
    serde_json::to_writer_pretty(file, &exp2)?;
    eprintln!("  Wrote {}", json_path.display());

    // Write report
    let report = exp2_report(&exp2);
    let report_path = output_dir.join("exp2_report.txt");
    fs::write(&report_path, &report)?;
    eprintln!("  Wrote {}", report_path.display());

    Ok(())
}

fn exp2_report(results: &Exp2Results) -> String {
    let mut r = String::new();
    r.push_str("================================================================\n");
    r.push_str("  Experiment 2: Incremental Build-up\n");
    r.push_str("  All unique prefixes of 120 permutations at each length 1-5\n");
    r.push_str("================================================================\n\n");

    // Spread by step
    r.push_str("Spread by Prefix Length:\n");
    r.push_str(&format!(
        "{:<20} {:>6} {:>6} {:>6} {:>6} {:>6}\n",
        "Function", "Len=1", "Len=2", "Len=3", "Len=4", "Len=5"
    ));
    r.push_str(&format!("{}\n", "-".repeat(56)));

    for f in &results.per_function {
        let spreads: Vec<String> = f
            .per_step
            .iter()
            .map(|s| format!("{:.1}%", s.spread_pct))
            .collect();
        r.push_str(&format!(
            "{:<20} {:>6} {:>6} {:>6} {:>6} {:>6}\n",
            f.function,
            spreads.get(0).map_or("N/A", |s| s),
            spreads.get(1).map_or("N/A", |s| s),
            spreads.get(2).map_or("N/A", |s| s),
            spreads.get(3).map_or("N/A", |s| s),
            spreads.get(4).map_or("N/A", |s| s),
        ));
    }

    // Detailed per-function
    r.push_str("\nDetailed Results per Function:\n");
    r.push_str(&format!("{}\n", "-".repeat(80)));

    for f in &results.per_function {
        r.push_str(&format!("\n  {}:\n", f.function));
        for step in &f.per_step {
            r.push_str(&format!(
                "    Length {}: {} prefixes, best={} ns, worst={} ns, spread={:.1}%\n",
                step.prefix_length,
                step.num_prefixes,
                step.best_ns,
                step.worst_ns,
                step.spread_pct,
            ));

            // Show top 5 and bottom 5
            let mut sorted: Vec<&PrefixResult> = step.prefixes.iter().collect();
            sorted.sort_by_key(|p| p.median_ns);

            let show_n = 3.min(sorted.len());
            r.push_str("      Best:\n");
            for p in sorted.iter().take(show_n) {
                r.push_str(&format!(
                    "        {:>10} ns  {}\n",
                    p.median_ns,
                    p.passes.join(" -> "),
                ));
            }
            if sorted.len() > show_n * 2 {
                r.push_str("        ...\n");
            }
            r.push_str("      Worst:\n");
            for p in sorted.iter().rev().take(show_n) {
                r.push_str(&format!(
                    "        {:>10} ns  {}\n",
                    p.median_ns,
                    p.passes.join(" -> "),
                ));
            }
        }
    }

    // Key insight: at which step does divergence appear?
    r.push_str("\nKey Insight — Divergence Onset:\n");
    r.push_str(&format!("{}\n", "-".repeat(80)));

    for f in &results.per_function {
        let first_significant = f
            .per_step
            .iter()
            .find(|s| s.spread_pct > 2.0);
        match first_significant {
            Some(step) => {
                r.push_str(&format!(
                    "  {:<20} divergence at length {} ({:.1}% spread)\n",
                    f.function, step.prefix_length, step.spread_pct,
                ));
            }
            None => {
                let max_spread = f
                    .per_step
                    .iter()
                    .map(|s| s.spread_pct)
                    .fold(0.0f64, f64::max);
                r.push_str(&format!(
                    "  {:<20} no significant divergence (max spread {:.1}%)\n",
                    f.function, max_spread,
                ));
            }
        }
    }

    r.push_str("\n================================================================\n");
    r
}

// ---------------------------------------------------------------------------
// Experiment 3: O3 Pipeline Analysis
// ---------------------------------------------------------------------------

fn experiment3(benchmarks_dir: &Path, output_dir: &Path, runs: usize) -> Result<()> {
    // Get O3 pipeline string
    let o3_output = Command::new("opt-20")
        .args(["-O3", "--print-pipeline-passes", "-disable-output", "/dev/null"])
        .output()
        .context("failed to run opt-20 for O3 pipeline")?;

    // The pipeline is printed to stdout
    let o3_pipeline = String::from_utf8_lossy(&o3_output.stdout).trim().to_string();
    if o3_pipeline.is_empty() {
        // Try stderr — some versions print there
        let o3_pipeline_stderr = String::from_utf8_lossy(&o3_output.stderr).trim().to_string();
        if o3_pipeline_stderr.is_empty() {
            bail!("Could not get O3 pipeline from opt-20");
        }
    }
    let o3_pipeline = if o3_pipeline.is_empty() {
        String::from_utf8_lossy(&o3_output.stderr).trim().to_string()
    } else {
        o3_pipeline
    };

    eprintln!("  O3 pipeline length: {} chars", o3_pipeline.len());

    // Parse top-level passes (split on ',' at depth 0)
    let top_level_passes = split_top_level(&o3_pipeline);
    eprintln!("  O3 top-level passes: {}", top_level_passes.len());

    // Extract passes from O3 that match our menu (first occurrence, true O3 order)
    let our_passes_in_o3 = extract_our_passes(&o3_pipeline);
    eprintln!(
        "  Our passes found in O3 (unique, O3 order): {}",
        our_passes_in_o3.len(),
    );

    // Extract ALL occurrences (with repetition) in pipeline order
    let our_passes_in_o3_with_rep = extract_our_passes_with_repetition(&o3_pipeline);
    eprintln!(
        "  Our passes with repetition: {} total invocations ({} unique)",
        our_passes_in_o3_with_rep.len(),
        our_passes_in_o3.len(),
    );

    // Build subsequences
    let subsequences = build_subsequences(&top_level_passes);

    let func_paths: Vec<PathBuf> = TARGET_FUNCTIONS
        .iter()
        .map(|f| benchmarks_dir.join(format!("{f}.c")))
        .collect();

    for p in &func_paths {
        if !p.exists() {
            bail!("Target benchmark not found: {}", p.display());
        }
    }

    let o3_pipeline_clone = o3_pipeline.clone();
    let our_passes_clone = our_passes_in_o3.clone();
    let our_passes_rep_clone = our_passes_in_o3_with_rep.clone();
    let subsequences_clone = subsequences.clone();

    let results: Vec<Result<Exp3FunctionResult>> = func_paths
        .par_iter()
        .map(|func_path| {
            let stem = func_path.file_stem().unwrap().to_string_lossy().to_string();
            let work_dir = output_dir.join("_work").join(&stem);
            fs::create_dir_all(&work_dir)?;
            let pipeline = CompilationPipeline::new(work_dir);

            // Native O3
            let native_o3 = pipeline.baseline(func_path, "-O3", runs)?;
            eprintln!("  [{stem}] native O3 = {} ns", native_o3.median_ns);

            // Full O3 via opt
            let ir = pipeline.emit_ir(func_path)?;
            let opt_ir_o3 = pipeline.work_dir().join(format!("{stem}_o3_full.ll"));
            pipeline.apply_passes_raw(&ir, &o3_pipeline_clone, &opt_ir_o3)?;
            let binary_o3 = pipeline.compile_ir(&opt_ir_o3)?;
            let full_o3_bench = pipeline.benchmark(&binary_o3, runs)?;
            eprintln!("  [{stem}] full O3 via opt = {} ns", full_o3_bench.median_ns);

            // Our passes in O3 order (unique, one application each)
            let our_o3_order_ir = pipeline.work_dir().join(format!("{stem}_our_o3.ll"));
            pipeline.apply_passes(&ir, &our_passes_clone, &our_o3_order_ir)?;
            let binary_our = pipeline.compile_ir(&our_o3_order_ir)?;
            let our_o3_bench = pipeline.benchmark(&binary_our, runs)?;
            eprintln!("  [{stem}] our passes (O3 order, unique) = {} ns", our_o3_bench.median_ns);

            // Our passes with O3 repetition (all occurrences, in pipeline order)
            let our_rep_ir = pipeline.work_dir().join(format!("{stem}_our_o3_rep.ll"));
            pipeline.apply_passes(&ir, &our_passes_rep_clone, &our_rep_ir)?;
            let binary_rep = pipeline.compile_ir(&our_rep_ir)?;
            let our_rep_bench = pipeline.benchmark(&binary_rep, runs)?;
            eprintln!("  [{stem}] our passes (O3 order, +rep) = {} ns", our_rep_bench.median_ns);

            // Shuffle our passes 20x
            let mut shuffled_results: Vec<ShuffledResult> = Vec::new();
            let mut rng = rand::thread_rng();
            use rand::seq::SliceRandom;

            for shuffle_i in 0..20 {
                let mut shuffled = our_passes_clone.clone();
                shuffled.shuffle(&mut rng);
                let shuffled_ir = pipeline
                    .work_dir()
                    .join(format!("{stem}_shuffle{shuffle_i}.ll"));
                pipeline.apply_passes(&ir, &shuffled, &shuffled_ir)?;
                let binary = pipeline.compile_ir(&shuffled_ir)?;
                let bench = pipeline.benchmark(&binary, runs)?;

                shuffled_results.push(ShuffledResult {
                    ordering: shuffled.iter().map(|p| p.opt_name().to_string()).collect(),
                    median_ns: bench.median_ns,
                });
            }

            // Subsequences
            let mut subsequence_results: Vec<SubsequenceResult> = Vec::new();
            for (label, subseq_pipeline) in &subsequences_clone {
                let sub_ir = pipeline.work_dir().join(format!(
                    "{stem}_sub_{}.ll",
                    label.replace(['/', ' ', '%'], "_")
                ));
                match pipeline.apply_passes_raw(&ir, subseq_pipeline, &sub_ir) {
                    Ok(()) => {
                        match pipeline.compile_ir(&sub_ir) {
                            Ok(binary) => {
                                match pipeline.benchmark(&binary, runs) {
                                    Ok(bench) => {
                                        subsequence_results.push(SubsequenceResult {
                                            label: label.clone(),
                                            pipeline: subseq_pipeline.clone(),
                                            median_ns: bench.median_ns,
                                        });
                                    }
                                    Err(e) => {
                                        eprintln!("  [{stem}] benchmark failed for {label}: {e}");
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("  [{stem}] compile failed for {label}: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("  [{stem}] opt failed for {label}: {e}");
                    }
                }
            }

            Ok(Exp3FunctionResult {
                function: stem,
                native_o3_ns: native_o3.median_ns,
                full_o3_via_opt_ns: full_o3_bench.median_ns,
                our_passes_o3_order_ns: our_o3_bench.median_ns,
                our_passes_o3_rep_ns: our_rep_bench.median_ns,
                shuffled_results,
                subsequence_results,
                our_passes_in_o3: our_passes_clone
                    .iter()
                    .map(|p| p.opt_name().to_string())
                    .collect(),
                our_passes_in_o3_with_rep: our_passes_rep_clone
                    .iter()
                    .map(|p| p.opt_name().to_string())
                    .collect(),
            })
        })
        .collect();

    let per_function: Vec<Exp3FunctionResult> = results
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

    let exp3 = Exp3Results {
        o3_pipeline_raw: o3_pipeline,
        top_level_pass_count: top_level_passes.len(),
        per_function,
    };

    // Write JSON
    let json_path = output_dir.join("exp3_o3_analysis.json");
    let file = File::create(&json_path)?;
    serde_json::to_writer_pretty(file, &exp3)?;
    eprintln!("  Wrote {}", json_path.display());

    // Write report
    let report = exp3_report(&exp3);
    let report_path = output_dir.join("exp3_report.txt");
    fs::write(&report_path, &report)?;
    eprintln!("  Wrote {}", report_path.display());

    Ok(())
}

/// Split a pipeline string at top-level commas (not inside parentheses).
fn split_top_level(pipeline: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut depth = 0;
    let mut current = String::new();

    for ch in pipeline.chars() {
        match ch {
            '(' | '<' => {
                depth += 1;
                current.push(ch);
            }
            ')' | '>' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    result.push(trimmed);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        result.push(trimmed);
    }
    result
}

/// Returns true if the byte at `pos` in `pipeline` is a valid pass-name boundary.
fn is_pass_boundary(pipeline: &[u8], pos: usize) -> bool {
    let b = pipeline[pos];
    !b.is_ascii_alphanumeric() && b != b'-'
}

/// Returns true if the match of `name` ending at `after` (exclusive) is a
/// valid right-side boundary in `pipeline`.
///
/// The key case: plain `"sroa"` must NOT match inside `"sroa<modify-cfg>"`.
/// A `<` immediately after the match indicates the pipeline has a parameterised
/// form of this pass.  It is only a valid boundary when the search pattern
/// itself ends with `>` (i.e. we already matched the full parameterised name).
fn is_valid_right_boundary(pipeline: &[u8], after: usize, name: &str) -> bool {
    if after >= pipeline.len() {
        return true;
    }
    let next = pipeline[after];
    if next == b'<' && !name.ends_with('>') {
        return false; // plain name matching into a parameterised form — reject
    }
    is_pass_boundary(pipeline, after)
}

/// Extract the first occurrence of each of our passes from the O3 pipeline,
/// returned in the order they first appear in the pipeline string (true O3 order).
fn extract_our_passes(pipeline: &str) -> Vec<Pass> {
    let bytes = pipeline.as_bytes();
    let all_transforms = Pass::all_transforms();
    let mut found: Vec<(usize, Pass)> = Vec::new();

    for &pass in all_transforms {
        let name = pass.opt_name();
        let mut search_from = 0;
        while let Some(rel) = pipeline[search_from..].find(name) {
            let abs = search_from + rel;
            let before_ok = abs == 0 || is_pass_boundary(bytes, abs - 1);
            let after = abs + name.len();
            let after_ok = is_valid_right_boundary(bytes, after, name);
            if before_ok && after_ok {
                found.push((abs, pass));
                break; // first occurrence only
            }
            search_from = abs + 1;
        }
    }

    found.sort_by_key(|(pos, _)| *pos);
    found.into_iter().map(|(_, p)| p).collect()
}

/// Extract ALL occurrences of our passes from the O3 pipeline in pipeline order,
/// preserving repetition exactly as -O3 applies them.
fn extract_our_passes_with_repetition(pipeline: &str) -> Vec<Pass> {
    let bytes = pipeline.as_bytes();
    let all_transforms = Pass::all_transforms();
    let mut occurrences: Vec<(usize, Pass)> = Vec::new();

    for &pass in all_transforms {
        let name = pass.opt_name();
        let mut search_from = 0;
        while let Some(rel) = pipeline[search_from..].find(name) {
            let abs = search_from + rel;
            let before_ok = abs == 0 || is_pass_boundary(bytes, abs - 1);
            let after = abs + name.len();
            let after_ok = is_valid_right_boundary(bytes, after, name);
            if before_ok && after_ok {
                occurrences.push((abs, pass));
            }
            search_from = abs + 1;
        }
    }

    occurrences.sort_by_key(|(pos, _)| *pos);
    occurrences.into_iter().map(|(_, p)| p).collect()
}

/// Build subsequence pipelines from the top-level O3 passes.
fn build_subsequences(top_level: &[String]) -> Vec<(String, String)> {
    let n = top_level.len();
    if n == 0 {
        return Vec::new();
    }

    let mut result = Vec::new();

    // First/last 25%, 50%, 75%
    for pct in [25, 50, 75] {
        let count = (n * pct + 99) / 100; // ceil

        let first_n: Vec<&str> = top_level[..count.min(n)]
            .iter()
            .map(|s| s.as_str())
            .collect();
        result.push((
            format!("first_{}%", pct),
            first_n.join(","),
        ));

        let last_start = n.saturating_sub(count);
        let last_n: Vec<&str> = top_level[last_start..]
            .iter()
            .map(|s| s.as_str())
            .collect();
        result.push((
            format!("last_{}%", pct),
            last_n.join(","),
        ));
    }

    // Find CGSCC block(s) — passes starting with "cgscc("
    let cgscc_passes: Vec<&str> = top_level
        .iter()
        .filter(|p| p.starts_with("cgscc("))
        .map(|s| s.as_str())
        .collect();
    if !cgscc_passes.is_empty() {
        result.push((
            "cgscc_blocks_only".to_string(),
            cgscc_passes.join(","),
        ));
    }

    // Find post-vectorization block — passes after any *vectorize* pass
    if let Some(vec_pos) = top_level.iter().rposition(|p| {
        p.contains("vectorize") || p.contains("slp-vectorizer")
    }) {
        if vec_pos + 1 < n {
            let post_vec: Vec<&str> = top_level[vec_pos + 1..]
                .iter()
                .map(|s| s.as_str())
                .collect();
            result.push((
                "post_vectorization".to_string(),
                post_vec.join(","),
            ));
        }
    }

    result
}

fn exp3_report(results: &Exp3Results) -> String {
    let mut r = String::new();
    r.push_str("================================================================\n");
    r.push_str("  Experiment 3: O3 Pipeline Analysis\n");
    r.push_str("================================================================\n\n");

    r.push_str(&format!(
        "O3 pipeline: {} top-level passes, {} chars\n\n",
        results.top_level_pass_count,
        results.o3_pipeline_raw.len(),
    ));

    // Main comparison table
    r.push_str("Main Comparison:\n");
    r.push_str(&format!(
        "{:<20} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12}\n",
        "Function", "native -O3", "O3 via opt", "Our(O3ord)", "Our(+rep)", "Shuf best", "Shuf worst"
    ));
    r.push_str(&format!("{}\n", "-".repeat(97)));

    for f in &results.per_function {
        let shuf_best = f
            .shuffled_results
            .iter()
            .map(|s| s.median_ns)
            .min()
            .unwrap_or(0);
        let shuf_worst = f
            .shuffled_results
            .iter()
            .map(|s| s.median_ns)
            .max()
            .unwrap_or(0);

        r.push_str(&format!(
            "{:<20} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12}\n",
            f.function,
            f.native_o3_ns,
            f.full_o3_via_opt_ns,
            f.our_passes_o3_order_ns,
            f.our_passes_o3_rep_ns,
            shuf_best,
            shuf_worst,
        ));
    }

    // Shuffle analysis
    r.push_str("\nShuffle Analysis (O3-order vs random reorderings of our passes):\n");
    r.push_str(&format!("{}\n", "-".repeat(80)));
    for f in &results.per_function {
        let times: Vec<u64> = f.shuffled_results.iter().map(|s| s.median_ns).collect();
        if times.is_empty() {
            continue;
        }
        let min_t = *times.iter().min().unwrap();
        let max_t = *times.iter().max().unwrap();
        let mean_t = times.iter().sum::<u64>() as f64 / times.len() as f64;
        let spread = if min_t > 0 {
            (max_t as f64 - min_t as f64) / min_t as f64 * 100.0
        } else {
            0.0
        };

        let o3_rank = times
            .iter()
            .filter(|&&t| t < f.our_passes_o3_order_ns)
            .count();

        r.push_str(&format!(
            "  {:<20} O3-order={} ns, shuffled: min={}, max={}, mean={:.0}, spread={:.1}%\n",
            f.function, f.our_passes_o3_order_ns, min_t, max_t, mean_t, spread,
        ));
        r.push_str(&format!(
            "    O3-order rank: {}/20 shuffles beat it\n",
            o3_rank,
        ));
    }

    // Subsequence results
    r.push_str("\nSubsequence Results:\n");
    r.push_str(&format!("{}\n", "-".repeat(80)));

    // Collect all labels
    let mut all_labels: Vec<String> = Vec::new();
    for f in &results.per_function {
        for sub in &f.subsequence_results {
            if !all_labels.contains(&sub.label) {
                all_labels.push(sub.label.clone());
            }
        }
    }

    r.push_str(&format!("{:<20}", "Function"));
    for label in &all_labels {
        r.push_str(&format!(" {:>14}", label));
    }
    r.push_str("\n");
    r.push_str(&format!("{}\n", "-".repeat(20 + all_labels.len() * 15)));

    for f in &results.per_function {
        r.push_str(&format!("{:<20}", f.function));
        for label in &all_labels {
            let ns = f
                .subsequence_results
                .iter()
                .find(|s| s.label == *label)
                .map(|s| format!("{}", s.median_ns))
                .unwrap_or_else(|| "FAIL".to_string());
            r.push_str(&format!(" {:>14}", ns));
        }
        r.push_str("\n");
    }

    // Our passes found in O3
    if let Some(first) = results.per_function.first() {
        r.push_str(&format!(
            "\nOur passes extracted from O3 (true O3 order, unique): {:?}\n",
            first.our_passes_in_o3,
        ));
        r.push_str(&format!(
            "\nOur passes with repetition ({} invocations): {:?}\n",
            first.our_passes_in_o3_with_rep.len(),
            first.our_passes_in_o3_with_rep,
        ));
    }

    // Summary
    r.push_str("\nSummary:\n");

    let avg_o3_gap: f64 = results
        .per_function
        .iter()
        .filter(|f| f.native_o3_ns > 0)
        .map(|f| {
            (f.full_o3_via_opt_ns as f64 - f.native_o3_ns as f64) / f.native_o3_ns as f64 * 100.0
        })
        .sum::<f64>()
        / results.per_function.len().max(1) as f64;
    r.push_str(&format!(
        "  Avg gap native O3 vs O3-via-opt: {:+.1}%\n",
        avg_o3_gap
    ));

    let avg_our_gap: f64 = results
        .per_function
        .iter()
        .filter(|f| f.native_o3_ns > 0)
        .map(|f| {
            (f.our_passes_o3_order_ns as f64 - f.native_o3_ns as f64)
                / f.native_o3_ns as f64
                * 100.0
        })
        .sum::<f64>()
        / results.per_function.len().max(1) as f64;
    r.push_str(&format!(
        "  Avg gap native O3 vs our-passes-in-O3-order: {:+.1}%\n",
        avg_our_gap
    ));

    let avg_rep_gap: f64 = results
        .per_function
        .iter()
        .filter(|f| f.native_o3_ns > 0)
        .map(|f| {
            (f.our_passes_o3_rep_ns as f64 - f.native_o3_ns as f64)
                / f.native_o3_ns as f64
                * 100.0
        })
        .sum::<f64>()
        / results.per_function.len().max(1) as f64;
    r.push_str(&format!(
        "  Avg gap native O3 vs our-passes-with-repetition: {:+.1}%\n",
        avg_rep_gap
    ));

    let avg_shuffle_spread: f64 = results
        .per_function
        .iter()
        .map(|f| {
            let times: Vec<u64> = f.shuffled_results.iter().map(|s| s.median_ns).collect();
            if times.is_empty() {
                return 0.0;
            }
            let min_t = *times.iter().min().unwrap() as f64;
            let max_t = *times.iter().max().unwrap() as f64;
            if min_t > 0.0 {
                (max_t - min_t) / min_t * 100.0
            } else {
                0.0
            }
        })
        .sum::<f64>()
        / results.per_function.len().max(1) as f64;
    r.push_str(&format!(
        "  Avg shuffle spread (our passes): {:.1}%\n",
        avg_shuffle_spread
    ));

    r.push_str("\n================================================================\n");
    r
}

