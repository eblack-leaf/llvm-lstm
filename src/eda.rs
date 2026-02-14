use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use statrs::statistics::Statistics;

use crate::dataset::{BaselineRecord, DataRecord};

#[derive(Debug, Serialize)]
pub struct FunctionStats {
    pub function: String,
    pub count: usize,
    pub mean_ns: f64,
    pub median_ns: f64,
    pub std_ns: f64,
    pub min_ns: f64,
    pub max_ns: f64,
    pub skewness: f64,
    pub kurtosis: f64,
    pub baseline_o0_ns: Option<f64>,
    pub baseline_o2_ns: Option<f64>,
    pub baseline_o3_ns: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PassImpact {
    pub pass_name: String,
    pub avg_time_with: f64,
    pub avg_time_without: f64,
    pub count_with: usize,
    pub count_without: usize,
    pub delta_ns: f64,
    pub delta_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct PassOrderResult {
    pub pass_a: String,
    pub pass_b: String,
    pub avg_time_ab: f64,
    pub avg_time_ba: f64,
    pub count_ab: usize,
    pub count_ba: usize,
    pub delta_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct TripleOrderResult {
    pub passes: [String; 3],
    /// Average times for each of the 6 permutations (abc, acb, bac, bca, cab, cba).
    /// None if fewer than 3 samples for that permutation.
    pub permutations: Vec<TriplePermutation>,
    /// Best permutation ordering
    pub best_order: String,
    /// Worst permutation ordering
    pub worst_order: String,
    /// (worst - best) / best * 100
    pub spread_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct TriplePermutation {
    pub order: String,
    pub avg_time: f64,
    pub count: usize,
}

pub struct EdaAnalyzer {
    records: Vec<DataRecord>,
    baselines: HashMap<String, HashMap<String, f64>>,
}

impl EdaAnalyzer {
    pub fn load(input_dir: &Path) -> Result<Self> {
        let mut records = Vec::new();
        let data_path = input_dir.join("exploratory.jsonl");
        if data_path.exists() {
            let file = File::open(&data_path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if !line.trim().is_empty() {
                    let record: DataRecord = serde_json::from_str(&line)
                        .with_context(|| format!("failed to parse: {line}"))?;
                    records.push(record);
                }
            }
        }

        let mut baselines: HashMap<String, HashMap<String, f64>> = HashMap::new();
        let baseline_path = input_dir.join("baselines.jsonl");
        if baseline_path.exists() {
            let file = File::open(&baseline_path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if !line.trim().is_empty() {
                    let record: BaselineRecord = serde_json::from_str(&line)?;
                    baselines
                        .entry(record.function)
                        .or_default()
                        .insert(record.opt_level, record.execution_time_ns as f64);
                }
            }
        }

        eprintln!(
            "Loaded {} records, {} functions with baselines",
            records.len(),
            baselines.len()
        );

        Ok(Self { records, baselines })
    }

    /// Per-function descriptive statistics.
    pub fn function_stats(&self) -> Vec<FunctionStats> {
        let mut by_func: HashMap<&str, Vec<f64>> = HashMap::new();
        for r in &self.records {
            by_func
                .entry(&r.function)
                .or_default()
                .push(r.execution_time_ns as f64);
        }

        let mut stats: Vec<FunctionStats> = by_func
            .into_iter()
            .map(|(func, times)| {
                let data: Vec<f64> = times.clone();
                let mut sorted = times.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let n = sorted.len();
                let median = if n % 2 == 0 {
                    (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
                } else {
                    sorted[n / 2]
                };

                let baselines = self.baselines.get(func);

                FunctionStats {
                    function: func.to_string(),
                    count: n,
                    mean_ns: (&data).mean(),
                    median_ns: median,
                    std_ns: (&data).std_dev(),
                    min_ns: (&data).min(),
                    max_ns: (&data).max(),
                    skewness: compute_skewness(&data),
                    kurtosis: compute_kurtosis(&data),
                    baseline_o0_ns: baselines.and_then(|b| b.get("-O0").copied()),
                    baseline_o2_ns: baselines.and_then(|b| b.get("-O2").copied()),
                    baseline_o3_ns: baselines.and_then(|b| b.get("-O3").copied()),
                }
            })
            .collect();

        stats.sort_by(|a, b| a.function.cmp(&b.function));
        stats
    }

    /// Per-pass impact analysis: average time with vs without each pass.
    pub fn pass_impact(&self) -> Vec<PassImpact> {
        let all_passes: Vec<&str> = crate::pass_menu::Pass::all_transforms()
            .iter()
            .map(|p| p.opt_name())
            .collect();

        let mut results = Vec::new();

        for pass_name in all_passes {
            let mut times_with: Vec<f64> = Vec::new();
            let mut times_without: Vec<f64> = Vec::new();

            for r in &self.records {
                if r.pass_sequence.iter().any(|p| p == pass_name) {
                    times_with.push(r.execution_time_ns as f64);
                } else {
                    times_without.push(r.execution_time_ns as f64);
                }
            }

            if !times_with.is_empty() && !times_without.is_empty() {
                let avg_with = (&times_with).mean();
                let avg_without = (&times_without).mean();
                results.push(PassImpact {
                    pass_name: pass_name.to_string(),
                    avg_time_with: avg_with,
                    avg_time_without: avg_without,
                    count_with: times_with.len(),
                    count_without: times_without.len(),
                    delta_ns: avg_with - avg_without,
                    delta_pct: (avg_with - avg_without) / avg_without * 100.0,
                });
            }
        }

        results.sort_by(|a, b| a.delta_pct.partial_cmp(&b.delta_pct).unwrap());
        results
    }

    /// Pass ordering analysis: for top pass pairs, compare A→B vs B→A.
    pub fn pass_ordering(&self) -> Vec<PassOrderResult> {
        let all_passes: Vec<&str> = crate::pass_menu::Pass::all_transforms()
            .iter()
            .map(|p| p.opt_name())
            .collect();

        let mut results = Vec::new();

        for (i, &pa) in all_passes.iter().enumerate() {
            for &pb in &all_passes[i + 1..] {
                let mut times_ab: Vec<f64> = Vec::new();
                let mut times_ba: Vec<f64> = Vec::new();

                for r in &self.records {
                    let pos_a = r.pass_sequence.iter().position(|p| p == pa);
                    let pos_b = r.pass_sequence.iter().position(|p| p == pb);

                    if let (Some(ia), Some(ib)) = (pos_a, pos_b) {
                        if ia < ib {
                            times_ab.push(r.execution_time_ns as f64);
                        } else {
                            times_ba.push(r.execution_time_ns as f64);
                        }
                    }
                }

                if times_ab.len() >= 10 && times_ba.len() >= 10 {
                    let avg_ab = (&times_ab).mean();
                    let avg_ba = (&times_ba).mean();
                    results.push(PassOrderResult {
                        pass_a: pa.to_string(),
                        pass_b: pb.to_string(),
                        avg_time_ab: avg_ab,
                        avg_time_ba: avg_ba,
                        count_ab: times_ab.len(),
                        count_ba: times_ba.len(),
                        delta_pct: (avg_ab - avg_ba) / avg_ba * 100.0,
                    });
                }
            }
        }

        results.sort_by(|a, b| {
            b.delta_pct
                .abs()
                .partial_cmp(&a.delta_pct.abs())
                .unwrap()
        });
        results
    }

    /// Pass ordering analysis depth-3: for pass triples, compare all 6 permutations.
    pub fn pass_ordering_triples(&self) -> Vec<TripleOrderResult> {
        let all_passes: Vec<&str> = crate::pass_menu::Pass::all_transforms()
            .iter()
            .map(|p| p.opt_name())
            .collect();

        let mut results = Vec::new();

        for i in 0..all_passes.len() {
            for j in (i + 1)..all_passes.len() {
                for k in (j + 1)..all_passes.len() {
                    let triple = [all_passes[i], all_passes[j], all_passes[k]];

                    // All 6 permutations of 3 elements
                    let perms: [(usize, usize, usize); 6] = [
                        (0, 1, 2), (0, 2, 1), (1, 0, 2),
                        (1, 2, 0), (2, 0, 1), (2, 1, 0),
                    ];

                    let mut perm_times: Vec<(String, Vec<f64>)> = perms
                        .iter()
                        .map(|&(a, b, c)| {
                            let label = format!(
                                "{}->{}->{}",
                                triple[a], triple[b], triple[c]
                            );
                            (label, Vec::new())
                        })
                        .collect();

                    for r in &self.records {
                        // Find positions of all three passes
                        let positions: Vec<Option<usize>> = triple
                            .iter()
                            .map(|&p| r.pass_sequence.iter().position(|s| s == p))
                            .collect();

                        if let [Some(pa), Some(pb), Some(pc)] = positions[..] {
                            // Determine which permutation this matches
                            let order = {
                                let mut indexed = [(pa, 0usize), (pb, 1), (pc, 2)];
                                indexed.sort_by_key(|x| x.0);
                                (indexed[0].1, indexed[1].1, indexed[2].1)
                            };

                            let perm_idx = perms.iter().position(|p| *p == order);
                            if let Some(idx) = perm_idx {
                                perm_times[idx].1.push(r.execution_time_ns as f64);
                            }
                        }
                    }

                    // Keep only permutations with enough samples
                    let permutations: Vec<TriplePermutation> = perm_times
                        .into_iter()
                        .filter(|(_, times)| times.len() >= 10)
                        .map(|(order, times)| {
                            let avg_time = (&times).mean();
                            TriplePermutation {
                                order,
                                avg_time,
                                count: times.len(),
                            }
                        })
                        .collect();

                    if permutations.len() >= 2 {
                        let best = permutations
                            .iter()
                            .min_by(|a, b| a.avg_time.partial_cmp(&b.avg_time).unwrap())
                            .unwrap();
                        let worst = permutations
                            .iter()
                            .max_by(|a, b| a.avg_time.partial_cmp(&b.avg_time).unwrap())
                            .unwrap();
                        let spread_pct =
                            (worst.avg_time - best.avg_time) / best.avg_time * 100.0;

                        results.push(TripleOrderResult {
                            passes: [
                                triple[0].to_string(),
                                triple[1].to_string(),
                                triple[2].to_string(),
                            ],
                            best_order: best.order.clone(),
                            worst_order: worst.order.clone(),
                            spread_pct,
                            permutations,
                        });
                    }
                }
            }
        }

        results.sort_by(|a, b| {
            b.spread_pct
                .partial_cmp(&a.spread_pct)
                .unwrap()
        });
        results
    }

    /// Write all analysis results to output directory.
    pub fn write_all(&self, output_dir: &Path) -> Result<()> {
        fs::create_dir_all(output_dir)?;

        // Function stats
        let stats = self.function_stats();
        let file = File::create(output_dir.join("function_stats.json"))?;
        serde_json::to_writer_pretty(file, &stats)?;
        eprintln!("Wrote function_stats.json ({} functions)", stats.len());

        // Pass impact
        let impact = self.pass_impact();
        let file = File::create(output_dir.join("pass_impact.json"))?;
        serde_json::to_writer_pretty(file, &impact)?;
        eprintln!("Wrote pass_impact.json ({} passes)", impact.len());

        // Pass ordering (pairs)
        let ordering = self.pass_ordering();
        let file = File::create(output_dir.join("pass_ordering.json"))?;
        serde_json::to_writer_pretty(file, &ordering)?;
        eprintln!("Wrote pass_ordering.json ({} pairs)", ordering.len());

        // Pass ordering (triples)
        let triples = self.pass_ordering_triples();
        let file = File::create(output_dir.join("pass_ordering_triples.json"))?;
        serde_json::to_writer_pretty(file, &triples)?;
        eprintln!("Wrote pass_ordering_triples.json ({} triples)", triples.len());

        // IR features summary
        let mut ir_summary: Vec<serde_json::Value> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for r in &self.records {
            if seen.insert(r.function.clone()) {
                ir_summary.push(serde_json::json!({
                    "function": r.function,
                    "ir_features": r.ir_features,
                }));
            }
        }
        let file = File::create(output_dir.join("ir_features_summary.json"))?;
        serde_json::to_writer_pretty(file, &ir_summary)?;
        eprintln!("Wrote ir_features_summary.json");

        // Human-readable report
        let report = self.generate_report(&stats, &impact, &ordering, &triples);
        let mut file = File::create(output_dir.join("report.txt"))?;
        use std::io::Write;
        file.write_all(report.as_bytes())?;
        eprintln!("Wrote report.txt");

        Ok(())
    }

    fn generate_report(
        &self,
        stats: &[FunctionStats],
        impact: &[PassImpact],
        ordering: &[PassOrderResult],
        triples: &[TripleOrderResult],
    ) -> String {
        let mut r = String::new();

        r.push_str("================================================================\n");
        r.push_str("  LLVM Pass Ordering — Exploratory Data Analysis Report\n");
        r.push_str(&format!("  {} records across {} functions\n", self.records.len(), stats.len()));
        r.push_str("================================================================\n\n");

        // --- Function stats table ---
        r.push_str("1. FUNCTION PERFORMANCE OVERVIEW\n");
        r.push_str("--------------------------------\n");
        r.push_str(&format!(
            "{:<25} {:>8} {:>12} {:>12} {:>12} {:>10}\n",
            "Function", "Samples", "Median(ms)", "-O0(ms)", "-O3(ms)", "vs -O3"
        ));
        r.push_str(&format!("{}\n", "-".repeat(85)));

        for s in stats {
            let median_ms = s.median_ns / 1_000_000.0;
            let o0_ms = s.baseline_o0_ns.map(|v| v / 1_000_000.0);
            let o3_ms = s.baseline_o3_ns.map(|v| v / 1_000_000.0);
            let vs_o3 = s.baseline_o3_ns.map(|o3| {
                if s.median_ns < o3 {
                    format!("{:.1}x faster", o3 / s.median_ns)
                } else if s.median_ns > o3 * 1.05 {
                    format!("{:.1}x slower", s.median_ns / o3)
                } else {
                    "~same".to_string()
                }
            });

            r.push_str(&format!(
                "{:<25} {:>8} {:>12.2} {:>12.2} {:>12.2} {:>10}\n",
                s.function,
                s.count,
                median_ms,
                o0_ms.unwrap_or(0.0),
                o3_ms.unwrap_or(0.0),
                vs_o3.as_deref().unwrap_or("N/A"),
            ));
        }

        // Variance analysis
        r.push_str(&format!("\n  Timing variance across functions:\n"));
        let mut cv_pairs: Vec<(&str, f64)> = stats
            .iter()
            .filter(|s| s.mean_ns > 0.0)
            .map(|s| (s.function.as_str(), s.std_ns / s.mean_ns * 100.0))
            .collect();
        cv_pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (func, cv) in &cv_pairs {
            let flag = if *cv > 30.0 { " <-- high variance" } else { "" };
            r.push_str(&format!("    {:<25} CV={:>5.1}%{}\n", func, cv, flag));
        }

        // --- Pass impact ---
        r.push_str("\n\n2. PASS IMPACT ANALYSIS\n");
        r.push_str("----------------------\n");
        r.push_str("  Avg execution time when pass is present vs absent.\n");
        r.push_str("  Negative delta = pass helps, positive = pass hurts (on average).\n\n");
        r.push_str(&format!(
            "  {:<20} {:>10} {:>10} {:>9} {:>8}\n",
            "Pass", "With(ms)", "W/o(ms)", "Delta%", "Effect"
        ));
        r.push_str(&format!("  {}\n", "-".repeat(62)));

        for p in impact {
            let effect = if p.delta_pct < -15.0 {
                "HELPS"
            } else if p.delta_pct < -5.0 {
                "helps"
            } else if p.delta_pct > 15.0 {
                "HURTS"
            } else if p.delta_pct > 5.0 {
                "hurts"
            } else {
                "~neutral"
            };

            r.push_str(&format!(
                "  {:<20} {:>10.2} {:>10.2} {:>+8.1}% {:>8}\n",
                p.pass_name,
                p.avg_time_with / 1_000_000.0,
                p.avg_time_without / 1_000_000.0,
                p.delta_pct,
                effect,
            ));
        }

        r.push_str("\n  NOTE: Impact measured in isolation. A pass that 'hurts' alone may\n");
        r.push_str("  help in combination (e.g., gvn after mem2reg+sroa). This is why\n");
        r.push_str("  ordering matters and why we need a sequential model.\n");

        // --- Pass ordering ---
        r.push_str("\n\n3. PASS ORDERING EFFECTS (Top 20)\n");
        r.push_str("---------------------------------\n");
        r.push_str("  Comparing A->B vs B->A ordering. Large delta = ordering matters.\n\n");
        r.push_str(&format!(
            "  {:<16} {:<16} {:>10} {:>10} {:>9}\n",
            "First", "Second", "A->B(ms)", "B->A(ms)", "Delta%"
        ));
        r.push_str(&format!("  {}\n", "-".repeat(65)));

        for o in ordering.iter().take(20) {
            let (better_order, _) = if o.avg_time_ab < o.avg_time_ba {
                ("A->B", "B->A")
            } else {
                ("B->A", "A->B")
            };

            r.push_str(&format!(
                "  {:<16} {:<16} {:>10.2} {:>10.2} {:>+8.1}%  ({})\n",
                o.pass_a,
                o.pass_b,
                o.avg_time_ab / 1_000_000.0,
                o.avg_time_ba / 1_000_000.0,
                o.delta_pct,
                better_order,
            ));
        }

        // --- Triple ordering ---
        r.push_str("\n\n4. TRIPLE ORDERING EFFECTS (Top 20)\n");
        r.push_str("------------------------------------\n");
        r.push_str("  Comparing all permutations of 3-pass combinations.\n");
        r.push_str("  Spread = (worst - best) / best. Large spread = combined ordering matters.\n\n");

        for (idx, t) in triples.iter().take(20).enumerate() {
            r.push_str(&format!(
                "  #{:<2} {{{}, {}, {}}}  spread={:>+.1}%\n",
                idx + 1,
                t.passes[0],
                t.passes[1],
                t.passes[2],
                t.spread_pct,
            ));
            r.push_str(&format!(
                "      best:  {:<45} worst: {}\n",
                t.best_order, t.worst_order,
            ));
            for p in &t.permutations {
                r.push_str(&format!(
                    "        {:<45} {:>10.2}ms  (n={})\n",
                    p.order,
                    p.avg_time / 1_000_000.0,
                    p.count,
                ));
            }
            r.push_str("\n");
        }

        // --- Key findings ---
        r.push_str("\n\n5. KEY FINDINGS\n");
        r.push_str("--------------\n");

        // Best passes
        let helpful: Vec<&PassImpact> = impact.iter().filter(|p| p.delta_pct < -5.0).collect();
        if !helpful.is_empty() {
            r.push_str("  Generally helpful passes (agent should favor these):\n");
            for p in &helpful {
                r.push_str(&format!("    - {:<20} {:>+.1}%\n", p.pass_name, p.delta_pct));
            }
        }

        let harmful: Vec<&PassImpact> = impact.iter().filter(|p| p.delta_pct > 15.0).rev().collect();
        if !harmful.is_empty() {
            r.push_str("\n  Context-dependent passes (hurt in isolation, may help in combination):\n");
            for p in &harmful {
                r.push_str(&format!("    - {:<20} {:>+.1}%\n", p.pass_name, p.delta_pct));
            }
        }

        // Ordering significance
        let significant_orders = ordering.iter().filter(|o| o.delta_pct.abs() > 50.0).count();
        r.push_str(&format!(
            "\n  Pass ordering significance:\n    {}/{} pairs show >50% difference based on ordering.\n",
            significant_orders,
            ordering.len()
        ));
        r.push_str("    This strongly supports using a sequential model (LSTM) over\n");
        r.push_str("    a set-based approach for pass selection.\n");

        // Functions that beat O3
        let beats_o3: Vec<&FunctionStats> = stats
            .iter()
            .filter(|s| {
                s.baseline_o3_ns
                    .is_some_and(|o3| s.min_ns < o3)
            })
            .collect();
        if !beats_o3.is_empty() {
            r.push_str(&format!(
                "\n  Functions where random search found sequences beating -O3:\n"
            ));
            for s in &beats_o3 {
                let o3 = s.baseline_o3_ns.unwrap();
                let speedup = o3 / s.min_ns;
                r.push_str(&format!(
                    "    - {:<25} best={:.2}ms vs O3={:.2}ms ({:.2}x faster)\n",
                    s.function,
                    s.min_ns / 1_000_000.0,
                    o3 / 1_000_000.0,
                    speedup,
                ));
            }
        } else {
            r.push_str("\n  No random sequences beat -O3 in this dataset.\n");
            r.push_str("  (Try collecting more sequences to find better combinations.)\n");
        }

        // Reward scaling recommendation
        let max_median = stats.iter().map(|s| s.median_ns).fold(0.0f64, f64::max);
        let min_median = stats
            .iter()
            .map(|s| s.median_ns)
            .fold(f64::MAX, f64::min);
        let range_ratio = max_median / min_median.max(1.0);
        r.push_str(&format!(
            "\n  Timing range: {:.2}ms to {:.2}ms ({:.0}x ratio)\n",
            min_median / 1_000_000.0,
            max_median / 1_000_000.0,
            range_ratio,
        ));
        if range_ratio > 100.0 {
            r.push_str("  --> Large range. Consider log-transformed rewards to prevent\n");
            r.push_str("      slow functions from dominating the gradient signal.\n");
        } else if range_ratio > 20.0 {
            r.push_str("  --> Moderate range. Z-score normalization per function should suffice.\n");
        } else {
            r.push_str("  --> Reasonable range. Raw speedup ratios should work as rewards.\n");
        }

        r.push_str("\n================================================================\n");
        r
    }
}

fn compute_skewness(data: &[f64]) -> f64 {
    let n = data.len() as f64;
    if n < 3.0 {
        return 0.0;
    }
    let mean = data.mean();
    let std_dev = data.std_dev();
    if std_dev == 0.0 {
        return 0.0;
    }
    let m3: f64 = data.iter().map(|x| ((x - mean) / std_dev).powi(3)).sum();
    m3 / n
}

fn compute_kurtosis(data: &[f64]) -> f64 {
    let n = data.len() as f64;
    if n < 4.0 {
        return 0.0;
    }
    let mean = data.mean();
    let std_dev = data.std_dev();
    if std_dev == 0.0 {
        return 0.0;
    }
    let m4: f64 = data.iter().map(|x| ((x - mean) / std_dev).powi(4)).sum();
    m4 / n - 3.0 // excess kurtosis
}
