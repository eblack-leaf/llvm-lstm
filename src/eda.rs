use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::dataset::{BaselineRecord, DataRecord};
use crate::ir_features::IrFeatures;
use crate::pipeline::CompilationPipeline;
use crate::plots;

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct BaselineEntry {
    function: String,
    o0_ns: u64,
    o2_ns: u64,
    o3_ns: u64,
    o3_speedup: f64,
    o2_speedup: f64,
}

#[derive(Debug, Serialize)]
struct CeilingEntry {
    function: String,
    o0_ns: u64,
    o2_ns: u64,
    o3_ns: u64,
    best_ns: u64,
    gap_vs_o3_pct: f64,
    gap_vs_o2_pct: f64,
    speedup_vs_o0: f64,
    best_passes: Vec<String>,
    best_seq_len: usize,
    top10_median_ns: u64,
    top10_gap_vs_o3_pct: f64,
    p25_ns: u64,
    median_ns: u64,
    p75_ns: u64,
}

#[derive(Debug, Serialize)]
struct PassEnrichment {
    pass_name: String,
    presence_in_top10pct: f64,
    presence_overall: f64,
    enrichment: f64,
}

/// Median speedup attributable to using a pass (with vs. without, across functions).
#[derive(Debug, Serialize)]
struct PassImpact {
    pass_name: String,
    /// Number of functions where we had enough data for both groups.
    n_functions: usize,
    /// Geometric mean of (median_without / median_with) across functions.
    /// > 1.0 means the pass helps on average.
    geo_mean_speedup: f64,
    /// Same metric but restricted to top-10% sequences within each function.
    geo_mean_speedup_top10: f64,
    enrichment: f64,
}

/// Which pairs of passes co-occur in top-10% sequences and how strongly.
#[derive(Debug, Serialize)]
struct PassCoOccurrence {
    pass_a: String,
    pass_b: String,
    /// Number of top-10% sequences that contain both.
    count: usize,
    /// P(A∩B | top10) / (P(A|top10) * P(B|top10)).  >1 = synergistic.
    lift: f64,
}

#[derive(Debug, Serialize)]
struct BenchmarkFeatureEntry {
    function: String,
    difficulty: String,
    gap_vs_o3_pct: f64,
    #[serde(flatten)]
    features: IrFeatures,
}

#[derive(Debug, Clone, Serialize)]
struct FeatureCorrelation {
    feature: String,
    /// Pearson r with gap_vs_o3_pct.  Positive = harder benchmarks have more of this feature.
    pearson_r: f64,
    abs_r: f64,
    mean_beats_o3: f64,
    mean_hard: f64,
}

#[derive(Debug, Serialize)]
struct DistributionStats {
    function: String,
    n: usize,
    min_ns: u64,
    p10_ns: u64,
    p25_ns: u64,
    median_ns: u64,
    p75_ns: u64,
    p90_ns: u64,
    max_ns: u64,
    mean_ns: f64,
    std_ns: f64,
    cv_pct: f64,
}

#[derive(Debug, Serialize)]
struct SeqLengthTier {
    difficulty: String,
    n_functions: usize,
    mean_best_len: f64,
    median_best_len: usize,
    p25_best_len: usize,
    p75_best_len: usize,
}

// ---------------------------------------------------------------------------
// Analyzer
// ---------------------------------------------------------------------------

pub struct EdaAnalyzer {
    records: Vec<DataRecord>,
    baselines: Vec<BaselineRecord>,
}

impl EdaAnalyzer {
    pub fn load(input_dir: &Path) -> Result<Self> {
        let baselines_path = input_dir.join("baselines.jsonl");
        let mut baselines = Vec::new();
        if baselines_path.exists() {
            let file = BufReader::new(File::open(&baselines_path)?);
            for line in file.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let record: BaselineRecord = serde_json::from_str(&line)
                    .with_context(|| format!("parsing baseline: {line}"))?;
                baselines.push(record);
            }
        }

        let data_path = input_dir.join("exploratory.jsonl");
        let mut records = Vec::new();
        if data_path.exists() {
            let file = BufReader::new(File::open(&data_path)?);
            for line in file.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let record: DataRecord = serde_json::from_str(&line)
                    .with_context(|| format!("parsing record: {line}"))?;
                records.push(record);
            }
        }

        eprintln!(
            "Loaded {} baselines, {} exploratory records",
            baselines.len(),
            records.len()
        );
        Ok(Self { records, baselines })
    }

    pub fn write_all(&self, output_dir: &Path, functions_dir: Option<&Path>) -> Result<()> {
        fs::create_dir_all(output_dir)?;

        let mut report = String::new();
        let baseline_map = self.build_baseline_map();

        let functions: Vec<String> = {
            let mut s: Vec<String> = baseline_map.keys().cloned().collect();
            s.sort();
            s
        };

        report.push_str("================================================================\n");
        report.push_str("  LLVM-LSTM: Analysis Report\n");
        report.push_str(&format!(
            "  {} records, {} functions\n",
            self.records.len(),
            functions.len()
        ));
        report.push_str("================================================================\n\n");

        // 1. Baseline landscape
        let baselines = self.baseline_landscape(&baseline_map);
        Self::write_baseline_section(&baselines, &mut report);
        let file = File::create(output_dir.join("baselines.json"))?;
        serde_json::to_writer_pretty(file, &baselines)?;

        // 2. Ceiling analysis (vs O0, O2, O3)
        let ceiling = self.ceiling_analysis(&baseline_map);
        Self::write_ceiling_section(&ceiling, &mut report);
        let file = File::create(output_dir.join("ceiling.json"))?;
        serde_json::to_writer_pretty(file, &ceiling)?;

        // 3. Distribution stats
        let dist = self.distribution_stats();
        Self::write_distribution_section(&dist, &mut report);
        let file = File::create(output_dir.join("distributions.json"))?;
        serde_json::to_writer_pretty(file, &dist)?;

        // 4. Pass enrichment
        let enrichment = self.pass_enrichment();
        Self::write_enrichment_section(&enrichment, &mut report);
        let file = File::create(output_dir.join("pass_enrichment.json"))?;
        serde_json::to_writer_pretty(file, &enrichment)?;

        // 5. Pass impact (median speedup with vs without)
        let impact = self.pass_impact(&enrichment);
        Self::write_impact_section(&impact, &mut report);
        let file = File::create(output_dir.join("pass_impact.json"))?;
        serde_json::to_writer_pretty(file, &impact)?;

        // 6. Pass co-occurrence in top sequences
        let cooccur = self.pass_cooccurrence(20);
        Self::write_cooccurrence_section(&cooccur, &mut report);
        let file = File::create(output_dir.join("pass_cooccurrence.json"))?;
        serde_json::to_writer_pretty(file, &cooccur)?;

        // 7. Sequence length by difficulty tier
        let seq_tiers = Self::sequence_length_by_difficulty(&ceiling);
        Self::write_seq_length_section(&seq_tiers, &mut report);
        let file = File::create(output_dir.join("seq_length_tiers.json"))?;
        serde_json::to_writer_pretty(file, &seq_tiers)?;

        // 8. IR feature landscape (optional, needs source files)
        let features: Vec<BenchmarkFeatureEntry> = if let Some(fdir) = functions_dir {
            let f = self.ir_feature_landscape(fdir, &ceiling)?;
            Self::write_features_section(&f, &mut report);
            let file = File::create(output_dir.join("ir_features.json"))?;
            serde_json::to_writer_pretty(file, &f)?;
            f
        } else {
            Vec::new()
        };

        // 9. Feature-performance correlations (requires IR features)
        if !features.is_empty() {
            let corr = Self::feature_performance_correlation(&features);
            Self::write_correlation_section(&corr, &mut report);
            let file = File::create(output_dir.join("feature_correlations.json"))?;
            serde_json::to_writer_pretty(file, &corr)?;
        }

        // 10. Summary
        Self::write_summary(&ceiling, &mut report);

        fs::write(output_dir.join("report.txt"), &report)?;
        eprintln!(
            "Wrote report to {}",
            output_dir.join("report.txt").display()
        );

        // Generate plots via Python
        plots::generate_all(output_dir)?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Analysis methods
    // -----------------------------------------------------------------------

    fn build_baseline_map(&self) -> HashMap<String, (u64, u64, u64)> {
        let mut map: HashMap<String, (u64, u64, u64)> = HashMap::new();
        for b in &self.baselines {
            let entry = map.entry(b.function.clone()).or_insert((0, 0, 0));
            match b.opt_level.as_str() {
                "-O0" => entry.0 = b.execution_time_ns,
                "-O2" => entry.1 = b.execution_time_ns,
                "-O3" => entry.2 = b.execution_time_ns,
                _ => {}
            }
        }
        map
    }

    fn baseline_landscape(
        &self,
        map: &HashMap<String, (u64, u64, u64)>,
    ) -> Vec<BaselineEntry> {
        let mut entries: Vec<BaselineEntry> = map
            .iter()
            .map(|(func, &(o0, o2, o3))| BaselineEntry {
                function: func.clone(),
                o0_ns: o0,
                o2_ns: o2,
                o3_ns: o3,
                o3_speedup: o0 as f64 / o3.max(1) as f64,
                o2_speedup: o0 as f64 / o2.max(1) as f64,
            })
            .collect();
        entries.sort_by(|a, b| b.o3_speedup.partial_cmp(&a.o3_speedup).unwrap());
        entries
    }

    fn ceiling_analysis(
        &self,
        baseline_map: &HashMap<String, (u64, u64, u64)>,
    ) -> Vec<CeilingEntry> {
        let mut by_func: HashMap<String, Vec<&DataRecord>> = HashMap::new();
        for r in &self.records {
            by_func.entry(r.function.clone()).or_default().push(r);
        }

        let mut entries = Vec::new();
        for (func, records) in &by_func {
            let &(o0_ns, o2_ns, o3_ns) = match baseline_map.get(func) {
                Some(b) => b,
                None => continue,
            };
            if o3_ns == 0 {
                continue;
            }

            let best = records
                .iter()
                .min_by_key(|r| r.execution_time_ns)
                .unwrap();
            let gap_vs_o3 =
                (best.execution_time_ns as f64 - o3_ns as f64) / o3_ns as f64 * 100.0;
            let gap_vs_o2 =
                (best.execution_time_ns as f64 - o2_ns as f64) / o2_ns as f64 * 100.0;
            let speedup_vs_o0 = o0_ns as f64 / best.execution_time_ns.max(1) as f64;

            let mut times: Vec<u64> = records.iter().map(|r| r.execution_time_ns).collect();
            times.sort();
            let n = times.len();
            let top10_n = (n / 10).max(1);
            let top10_median = times[top10_n / 2];
            let top10_gap =
                (top10_median as f64 - o3_ns as f64) / o3_ns as f64 * 100.0;

            entries.push(CeilingEntry {
                function: func.clone(),
                o0_ns,
                o2_ns,
                o3_ns,
                best_ns: best.execution_time_ns,
                gap_vs_o3_pct: gap_vs_o3,
                gap_vs_o2_pct: gap_vs_o2,
                speedup_vs_o0,
                best_passes: best.pass_sequence.clone(),
                best_seq_len: best.pass_sequence.len(),
                top10_median_ns: top10_median,
                top10_gap_vs_o3_pct: top10_gap,
                p25_ns: times[n / 4],
                median_ns: times[n / 2],
                p75_ns: times[3 * n / 4],
            });
        }
        entries.sort_by(|a, b| a.gap_vs_o3_pct.partial_cmp(&b.gap_vs_o3_pct).unwrap());
        entries
    }

    fn distribution_stats(&self) -> Vec<DistributionStats> {
        let mut by_func: HashMap<String, Vec<u64>> = HashMap::new();
        for r in &self.records {
            by_func
                .entry(r.function.clone())
                .or_default()
                .push(r.execution_time_ns);
        }

        let mut stats = Vec::new();
        for (func, mut times) in by_func {
            times.sort();
            let n = times.len();
            if n == 0 {
                continue;
            }

            let mean = times.iter().sum::<u64>() as f64 / n as f64;
            let variance = times.iter().map(|&t| (t as f64 - mean).powi(2)).sum::<f64>() / n as f64;
            let std = variance.sqrt();
            let cv = if mean > 0.0 { std / mean * 100.0 } else { 0.0 };

            stats.push(DistributionStats {
                function: func,
                n,
                min_ns: times[0],
                p10_ns: times[n / 10],
                p25_ns: times[n / 4],
                median_ns: times[n / 2],
                p75_ns: times[3 * n / 4],
                p90_ns: times[9 * n / 10],
                max_ns: times[n - 1],
                mean_ns: mean,
                std_ns: std,
                cv_pct: cv,
            });
        }
        stats.sort_by(|a, b| a.function.cmp(&b.function));
        stats
    }

    fn pass_enrichment(&self) -> Vec<PassEnrichment> {
        let mut by_func: HashMap<String, Vec<&DataRecord>> = HashMap::new();
        for r in &self.records {
            by_func.entry(r.function.clone()).or_default().push(r);
        }

        let mut overall_presence: HashMap<String, usize> = HashMap::new();
        let mut top10_presence: HashMap<String, usize> = HashMap::new();
        let mut total_seqs = 0usize;
        let mut total_top10 = 0usize;

        for (_func, records) in &mut by_func {
            records.sort_by_key(|r| r.execution_time_ns);
            let n = records.len();
            let top10_n = (n / 10).max(1);

            for (i, r) in records.iter().enumerate() {
                total_seqs += 1;
                let passes_set: HashSet<&String> = r.pass_sequence.iter().collect();

                for pass in &passes_set {
                    *overall_presence.entry((*pass).clone()).or_insert(0) += 1;
                    if i < top10_n {
                        *top10_presence.entry((*pass).clone()).or_insert(0) += 1;
                    }
                }
                if i < top10_n {
                    total_top10 += 1;
                }
            }
        }

        let mut profiles: Vec<PassEnrichment> = overall_presence
            .iter()
            .map(|(pass, &overall)| {
                let top10 = *top10_presence.get(pass).unwrap_or(&0);
                let pres_overall = overall as f64 / total_seqs as f64;
                let pres_top10 = top10 as f64 / total_top10 as f64;
                let enrichment = if pres_overall > 0.0 {
                    pres_top10 / pres_overall
                } else {
                    0.0
                };
                PassEnrichment {
                    pass_name: pass.clone(),
                    presence_in_top10pct: pres_top10,
                    presence_overall: pres_overall,
                    enrichment,
                }
            })
            .collect();
        profiles.sort_by(|a, b| b.enrichment.partial_cmp(&a.enrichment).unwrap());
        profiles
    }

    /// For each pass: geometric mean of (median_without / median_with) per function.
    /// Restricted to passes appearing in at least `min_functions` functions.
    fn pass_impact(&self, enrichment: &[PassEnrichment]) -> Vec<PassImpact> {
        let enrich_map: HashMap<&str, f64> = enrichment
            .iter()
            .map(|e| (e.pass_name.as_str(), e.enrichment))
            .collect();

        let mut by_func: HashMap<String, Vec<&DataRecord>> = HashMap::new();
        for r in &self.records {
            by_func.entry(r.function.clone()).or_default().push(r);
        }

        // Collect all unique pass names
        let all_passes: HashSet<String> = self
            .records
            .iter()
            .flat_map(|r| r.pass_sequence.iter().cloned())
            .collect();

        let min_functions = 2;

        let mut pass_log_speedups: HashMap<String, Vec<f64>> = HashMap::new();
        let mut pass_log_speedups_top10: HashMap<String, Vec<f64>> = HashMap::new();

        for (_func, records) in &by_func {
            let mut sorted: Vec<&DataRecord> = records.to_vec();
            sorted.sort_by_key(|r| r.execution_time_ns);
            let n = sorted.len();
            let top10_n = (n / 10).max(1);
            let top10_set: Vec<&DataRecord> = sorted.iter().copied().take(top10_n).collect();

            for pass in &all_passes {
                // All sequences
                let with: Vec<u64> = sorted
                    .iter()
                    .filter(|r| r.pass_sequence.contains(pass))
                    .map(|r| r.execution_time_ns)
                    .collect();
                let without: Vec<u64> = sorted
                    .iter()
                    .filter(|r| !r.pass_sequence.contains(pass))
                    .map(|r| r.execution_time_ns)
                    .collect();

                if with.len() >= 3 && without.len() >= 3 {
                    let med_with = median_u64(&with);
                    let med_without = median_u64(&without);
                    if med_with > 0 {
                        let log_sp = (med_without as f64 / med_with as f64).ln();
                        pass_log_speedups.entry(pass.clone()).or_default().push(log_sp);
                    }
                }

                // Top-10% sequences
                let with_top10: Vec<u64> = top10_set
                    .iter()
                    .filter(|r| r.pass_sequence.contains(pass))
                    .map(|r| r.execution_time_ns)
                    .collect();
                let without_top10: Vec<u64> = top10_set
                    .iter()
                    .filter(|r| !r.pass_sequence.contains(pass))
                    .map(|r| r.execution_time_ns)
                    .collect();

                if with_top10.len() >= 2 && without_top10.len() >= 2 {
                    let med_with = median_u64(&with_top10);
                    let med_without = median_u64(&without_top10);
                    if med_with > 0 {
                        let log_sp = (med_without as f64 / med_with as f64).ln();
                        pass_log_speedups_top10
                            .entry(pass.clone())
                            .or_default()
                            .push(log_sp);
                    }
                }
            }
        }

        let mut impacts: Vec<PassImpact> = pass_log_speedups
            .iter()
            .filter(|(_, v)| v.len() >= min_functions)
            .map(|(pass, log_sps)| {
                let geo_mean = log_sps.iter().sum::<f64>() / log_sps.len() as f64;
                let top10_log = pass_log_speedups_top10
                    .get(pass)
                    .filter(|v| !v.is_empty())
                    .map(|v| v.iter().sum::<f64>() / v.len() as f64)
                    .unwrap_or(0.0);
                PassImpact {
                    pass_name: pass.clone(),
                    n_functions: log_sps.len(),
                    geo_mean_speedup: geo_mean.exp(),
                    geo_mean_speedup_top10: top10_log.exp(),
                    enrichment: *enrich_map.get(pass.as_str()).unwrap_or(&1.0),
                }
            })
            .collect();
        impacts.sort_by(|a, b| b.geo_mean_speedup.partial_cmp(&a.geo_mean_speedup).unwrap());
        impacts
    }

    /// Pairwise co-occurrence lift for the top-`top_n` enriched passes in top-10% sequences.
    fn pass_cooccurrence(&self, top_n: usize) -> Vec<PassCoOccurrence> {
        let mut by_func: HashMap<String, Vec<&DataRecord>> = HashMap::new();
        for r in &self.records {
            by_func.entry(r.function.clone()).or_default().push(r);
        }

        // Collect top-10% sequences globally (normalise by function first)
        let mut top_sequences: Vec<HashSet<String>> = Vec::new();
        for (_func, mut records) in by_func {
            records.sort_by_key(|r| r.execution_time_ns);
            let top_n_func = (records.len() / 10).max(1);
            for r in records.into_iter().take(top_n_func) {
                top_sequences.push(r.pass_sequence.iter().cloned().collect());
            }
        }

        let total = top_sequences.len();
        if total == 0 {
            return Vec::new();
        }

        // Count individual pass presence
        let mut single_count: HashMap<String, usize> = HashMap::new();
        for seq in &top_sequences {
            for p in seq {
                *single_count.entry(p.clone()).or_default() += 1;
            }
        }

        // Restrict to top-N enriched passes by presence
        let mut by_count: Vec<(&String, usize)> = single_count.iter().map(|(k, &v)| (k, v)).collect();
        by_count.sort_by(|a, b| b.1.cmp(&a.1));
        let candidate_passes: Vec<String> = by_count
            .into_iter()
            .take(top_n)
            .map(|(k, _)| k.clone())
            .collect();

        // Count pairwise co-occurrence
        let mut pair_count: HashMap<(String, String), usize> = HashMap::new();
        for seq in &top_sequences {
            let present: Vec<&String> = candidate_passes
                .iter()
                .filter(|p| seq.contains(*p))
                .collect();
            for i in 0..present.len() {
                for j in (i + 1)..present.len() {
                    let a = present[i].clone();
                    let b = present[j].clone();
                    let key = if a <= b { (a, b) } else { (b, a) };
                    *pair_count.entry(key).or_default() += 1;
                }
            }
        }

        let total_f = total as f64;
        let mut pairs: Vec<PassCoOccurrence> = pair_count
            .into_iter()
            .map(|((a, b), count)| {
                let pa = *single_count.get(&a).unwrap_or(&0) as f64 / total_f;
                let pb = *single_count.get(&b).unwrap_or(&0) / total;
                let pab = count as f64 / total_f;
                let lift = if pa > 0.0 && pb > 0 {
                    pab / (pa * pb as f64)
                } else {
                    1.0
                };
                PassCoOccurrence {
                    pass_a: a,
                    pass_b: b,
                    count,
                    lift,
                }
            })
            .collect();
        pairs.sort_by(|a, b| b.lift.partial_cmp(&a.lift).unwrap());
        // Return only pairs with lift > 1 (positive synergy) and at least a few occurrences
        pairs.retain(|p| p.lift > 1.1 && p.count >= 3);
        pairs.truncate(50);
        pairs
    }

    fn sequence_length_by_difficulty(ceiling: &[CeilingEntry]) -> Vec<SeqLengthTier> {
        let tiers = [
            ("beats-O3", f64::NEG_INFINITY, 0.0),
            ("reachable", 0.0, 20.0),
            ("gap", 20.0, 100.0),
            ("hard", 100.0, f64::INFINITY),
        ];

        tiers
            .iter()
            .filter_map(|(name, lo, hi)| {
                let mut lens: Vec<usize> = ceiling
                    .iter()
                    .filter(|c| c.gap_vs_o3_pct >= *lo && c.gap_vs_o3_pct < *hi)
                    .map(|c| c.best_seq_len)
                    .collect();
                if lens.is_empty() {
                    return None;
                }
                lens.sort();
                let n = lens.len();
                let mean = lens.iter().sum::<usize>() as f64 / n as f64;
                Some(SeqLengthTier {
                    difficulty: name.to_string(),
                    n_functions: n,
                    mean_best_len: mean,
                    median_best_len: lens[n / 2],
                    p25_best_len: lens[n / 4],
                    p75_best_len: lens[3 * n / 4],
                })
            })
            .collect()
    }

    /// Returns features sorted by gap_vs_o3 ascending (best first).
    fn ir_feature_landscape(
        &self,
        functions_dir: &Path,
        ceiling: &[CeilingEntry],
    ) -> Result<Vec<BenchmarkFeatureEntry>> {
        let work_dir = std::path::PathBuf::from("/tmp/llvm-lstm-eda-features");
        let pipeline = CompilationPipeline::new(work_dir);

        let difficulty_map: HashMap<String, (String, f64)> = ceiling
            .iter()
            .map(|c| {
                let diff = if c.gap_vs_o3_pct < 0.0 {
                    "beats-O3"
                } else if c.gap_vs_o3_pct < 20.0 {
                    "reachable"
                } else if c.gap_vs_o3_pct < 100.0 {
                    "gap"
                } else {
                    "hard"
                };
                (c.function.clone(), (diff.to_string(), c.gap_vs_o3_pct))
            })
            .collect();

        let mut entries: Vec<BenchmarkFeatureEntry> = Vec::new();

        for entry in fs::read_dir(functions_dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "c") {
                let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                let ir = pipeline.emit_ir(&path)?;
                let features = IrFeatures::from_ll_file(&ir)?;
                let (difficulty, gap) = difficulty_map
                    .get(&stem)
                    .cloned()
                    .unwrap_or_else(|| ("unknown".into(), 0.0));
                entries.push(BenchmarkFeatureEntry {
                    function: stem,
                    difficulty,
                    gap_vs_o3_pct: gap,
                    features,
                });
            }
        }

        // Sort by gap ascending: beats-O3 first, hard last
        entries.sort_by(|a, b| a.gap_vs_o3_pct.partial_cmp(&b.gap_vs_o3_pct).unwrap());
        Ok(entries)
    }

    fn feature_performance_correlation(
        features: &[BenchmarkFeatureEntry],
    ) -> Vec<FeatureCorrelation> {
        let feature_names = [
            "add", "mul", "load", "store", "br", "call", "phi",
            "alloca", "gep", "icmp", "fcmp", "ret", "other",
            "basic_blocks", "total_insts", "functions", "loops", "load_store_ratio",
        ];

        let gaps: Vec<f64> = features.iter().map(|f| f.gap_vs_o3_pct).collect();
        let n = gaps.len();
        if n < 3 {
            return Vec::new();
        }

        let beats: Vec<&BenchmarkFeatureEntry> =
            features.iter().filter(|f| f.difficulty == "beats-O3").collect();
        let hard: Vec<&BenchmarkFeatureEntry> =
            features.iter().filter(|f| f.difficulty == "hard").collect();

        let feat_vecs: Vec<Vec<f64>> = features
            .iter()
            .map(|f| f.features.to_vec().iter().map(|&v| v as f64).collect())
            .collect();

        let beats_vecs: Vec<Vec<f64>> = beats
            .iter()
            .map(|f| f.features.to_vec().iter().map(|&v| v as f64).collect())
            .collect();

        let hard_vecs: Vec<Vec<f64>> = hard
            .iter()
            .map(|f| f.features.to_vec().iter().map(|&v| v as f64).collect())
            .collect();

        feature_names
            .iter()
            .enumerate()
            .map(|(i, &name)| {
                let xs: Vec<f64> = feat_vecs.iter().map(|v| v[i]).collect();
                let r = pearson_r(&xs, &gaps);

                let mean_beats = if beats_vecs.is_empty() {
                    0.0
                } else {
                    beats_vecs.iter().map(|v| v[i]).sum::<f64>() / beats_vecs.len() as f64
                };
                let mean_hard = if hard_vecs.is_empty() {
                    0.0
                } else {
                    hard_vecs.iter().map(|v| v[i]).sum::<f64>() / hard_vecs.len() as f64
                };

                FeatureCorrelation {
                    feature: name.to_string(),
                    pearson_r: r,
                    abs_r: r.abs(),
                    mean_beats_o3: mean_beats,
                    mean_hard,
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .collect()
    }

    // -----------------------------------------------------------------------
    // Report formatting
    // -----------------------------------------------------------------------

    fn write_baseline_section(baselines: &[BaselineEntry], report: &mut String) {
        report.push_str("1. BASELINE LANDSCAPE\n");
        report.push_str("---------------------\n");
        report.push_str("  O3/O0 = optimization headroom (higher = O3 does more work on this benchmark).\n\n");
        report.push_str(&format!(
            "  {:<25} {:>10} {:>10} {:>10} {:>8} {:>8}\n",
            "Benchmark", "O0(ns)", "O2(ns)", "O3(ns)", "O3/O0", "O2/O0"
        ));
        report.push_str(&format!("  {}\n", "-".repeat(75)));

        for b in baselines {
            report.push_str(&format!(
                "  {:<25} {:>10} {:>10} {:>10} {:>7.1}x {:>7.1}x\n",
                b.function, b.o0_ns, b.o2_ns, b.o3_ns, b.o3_speedup, b.o2_speedup
            ));
        }
        report.push('\n');
    }

    fn write_ceiling_section(ceiling: &[CeilingEntry], report: &mut String) {
        report.push_str("2. CEILING ANALYSIS (Best of Random Search vs Baselines)\n");
        report.push_str("--------------------------------------------------------\n");
        report.push_str("  Best = fastest from random pass sequences.\n");
        report.push_str("  vs O3/O2 = % gap (negative = beats baseline). O0x = speedup over O0.\n\n");
        report.push_str(&format!(
            "  {:<22} {:>9} {:>9} {:>8} {:>8} {:>6} {:>9} {:>8} {:>4}\n",
            "Benchmark", "O3(ns)", "Best(ns)", "vs O3", "vs O2", "O0x", "Top10%(ns)", "T10vsO3", "Len"
        ));
        report.push_str(&format!("  {}\n", "-".repeat(95)));

        let mut n_beats_o3 = 0;
        let mut n_beats_o2 = 0;
        let mut n_reach = 0;
        let mut n_gap = 0;
        let mut n_unreach = 0;

        for c in ceiling {
            if c.gap_vs_o3_pct < 0.0 {
                n_beats_o3 += 1;
            }
            if c.gap_vs_o2_pct < 0.0 {
                n_beats_o2 += 1;
            }

            let marker = if c.gap_vs_o3_pct < 0.0 {
                "<<"
            } else if c.gap_vs_o3_pct < 20.0 {
                n_reach += 1;
                "ok"
            } else if c.gap_vs_o3_pct < 100.0 {
                n_gap += 1;
                "  "
            } else {
                n_unreach += 1;
                "!!"
            };
            report.push_str(&format!(
                "  {:<22} {:>9} {:>9} {:>+7.1}% {:>+7.1}% {:>5.1}x {:>9} {:>+7.1}% {:>4} {}\n",
                c.function,
                c.o3_ns,
                c.best_ns,
                c.gap_vs_o3_pct,
                c.gap_vs_o2_pct,
                c.speedup_vs_o0,
                c.top10_median_ns,
                c.top10_gap_vs_o3_pct,
                c.best_seq_len,
                marker
            ));
        }

        let total = ceiling.len();
        report.push_str(&format!(
            "\n  Beats O3:         {:>2}/{total}    Beats O2: {:>2}/{total}\n",
            n_beats_o3, n_beats_o2
        ));
        report.push_str(&format!(
            "  Reachable (<20%): {:>2}/{total}    Gap:      {:>2}/{total}    Unreachable: {:>2}/{total}\n\n",
            n_reach, n_gap, n_unreach
        ));
    }

    fn write_distribution_section(dist: &[DistributionStats], report: &mut String) {
        report.push_str("3. PERFORMANCE DISTRIBUTIONS\n");
        report.push_str("----------------------------\n");
        report.push_str("  Quantiles of execution times across random sequences.\n");
        report.push_str("  CV = coefficient of variation (higher = more sensitive to pass choice).\n\n");
        report.push_str(&format!(
            "  {:<22} {:>5} {:>9} {:>9} {:>9} {:>9} {:>9} {:>6}\n",
            "Benchmark", "N", "P10(ns)", "P25(ns)", "Med(ns)", "P75(ns)", "P90(ns)", "CV%"
        ));
        report.push_str(&format!("  {}\n", "-".repeat(80)));

        for d in dist {
            report.push_str(&format!(
                "  {:<22} {:>5} {:>9} {:>9} {:>9} {:>9} {:>9} {:>5.1}%\n",
                d.function, d.n, d.p10_ns, d.p25_ns, d.median_ns, d.p75_ns, d.p90_ns, d.cv_pct
            ));
        }
        report.push('\n');
    }

    fn write_enrichment_section(enrichment: &[PassEnrichment], report: &mut String) {
        report.push_str("4. PASS ENRICHMENT IN TOP SEQUENCES\n");
        report.push_str("------------------------------------\n");
        report.push_str(
            "  Presence rate (per-sequence, deduplicated) in top-10% vs overall.\n",
        );
        report.push_str("  Enrichment > 1.0 = overrepresented in good sequences.\n\n");
        report.push_str(&format!(
            "  {:<28} {:>10} {:>10} {:>10}\n",
            "Pass", "Top10%", "Overall", "Enrich"
        ));
        report.push_str(&format!("  {}\n", "-".repeat(62)));

        for p in enrichment {
            let marker = if p.enrichment > 1.2 {
                " +"
            } else if p.enrichment < 0.8 {
                " -"
            } else {
                "  "
            };
            report.push_str(&format!(
                "  {:<28} {:>9.1}% {:>9.1}% {:>9.2}x{}\n",
                p.pass_name,
                p.presence_in_top10pct * 100.0,
                p.presence_overall * 100.0,
                p.enrichment,
                marker
            ));
        }
        report.push('\n');
    }

    fn write_impact_section(impacts: &[PassImpact], report: &mut String) {
        report.push_str("5. PASS IMPACT (median speedup with vs. without)\n");
        report.push_str("------------------------------------------------\n");
        report.push_str(
            "  Geo-mean speedup = exp(mean ln(median_without / median_with)) across functions.\n",
        );
        report.push_str("  > 1.0 means using this pass tends to produce faster code.\n\n");
        report.push_str(&format!(
            "  {:<28} {:>4} {:>12} {:>14} {:>10}\n",
            "Pass", "Fns", "GeoSpeedup", "GeoSpeedup(T10)", "Enrich"
        ));
        report.push_str(&format!("  {}\n", "-".repeat(72)));

        for imp in impacts.iter().take(30) {
            let marker = if imp.geo_mean_speedup > 1.05 {
                " +"
            } else if imp.geo_mean_speedup < 0.95 {
                " -"
            } else {
                "  "
            };
            report.push_str(&format!(
                "  {:<28} {:>4} {:>11.3}x {:>13.3}x {:>9.2}x{}\n",
                imp.pass_name,
                imp.n_functions,
                imp.geo_mean_speedup,
                imp.geo_mean_speedup_top10,
                imp.enrichment,
                marker
            ));
        }
        report.push('\n');
    }

    fn write_cooccurrence_section(pairs: &[PassCoOccurrence], report: &mut String) {
        report.push_str("6. PASS CO-OCCURRENCE IN TOP-10% SEQUENCES\n");
        report.push_str("-------------------------------------------\n");
        report.push_str(
            "  Lift = P(A∩B) / (P(A)*P(B)) — lift > 1 means synergistic pairing.\n",
        );
        report.push_str(
            "  Only pairs with lift > 1.1 and ≥3 co-occurrences shown.\n\n",
        );
        report.push_str(&format!(
            "  {:<28} {:<28} {:>6} {:>8}\n",
            "Pass A", "Pass B", "Count", "Lift"
        ));
        report.push_str(&format!("  {}\n", "-".repeat(76)));

        for p in pairs.iter().take(20) {
            report.push_str(&format!(
                "  {:<28} {:<28} {:>6} {:>7.2}x\n",
                p.pass_a, p.pass_b, p.count, p.lift
            ));
        }
        report.push('\n');
    }

    fn write_seq_length_section(tiers: &[SeqLengthTier], report: &mut String) {
        report.push_str("7. BEST SEQUENCE LENGTH BY DIFFICULTY TIER\n");
        report.push_str("-------------------------------------------\n");
        report.push_str("  Length of best-found pass sequence per benchmark, grouped by difficulty.\n\n");
        report.push_str(&format!(
            "  {:<12} {:>4} {:>8} {:>8} {:>6} {:>6}\n",
            "Tier", "N", "Mean", "Median", "P25", "P75"
        ));
        report.push_str(&format!("  {}\n", "-".repeat(50)));

        for t in tiers {
            report.push_str(&format!(
                "  {:<12} {:>4} {:>7.1} {:>8} {:>6} {:>6}\n",
                t.difficulty, t.n_functions, t.mean_best_len,
                t.median_best_len, t.p25_best_len, t.p75_best_len
            ));
        }
        report.push('\n');
    }

    fn write_features_section(features: &[BenchmarkFeatureEntry], report: &mut String) {
        report.push_str("8. IR FEATURE LANDSCAPE (Pre-optimization, sorted by gap vs O3)\n");
        report.push_str("----------------------------------------------------------------\n");
        report.push_str(
            "  Features from clang -O3 -disable-llvm-optzns (frontend-annotated, no LLVM passes).\n",
        );
        report.push_str("  Sorted by gap_vs_o3 ascending (best-performing first).\n\n");
        report.push_str(&format!(
            "  {:<22} {:>10} {:>11} {:>5} {:>5} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>5}\n",
            "Benchmark", "Difficulty", "GapVsO3%",
            "Add", "Mul", "Ld", "St", "Br", "Cal", "Phi", "Alc", "GEP",
            "Icm", "Fcm", "Ret", "Oth", "BB", "Inst", "Fn", "Lp", "ld/st"
        ));
        report.push_str(&format!("  {}\n", "-".repeat(145)));

        for f in features {
            let ir = &f.features;
            report.push_str(&format!(
                "  {:<22} {:>10} {:>+10.1}% {:>5} {:>5} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>5.2}\n",
                f.function,
                f.difficulty,
                f.gap_vs_o3_pct,
                ir.add_count,
                ir.mul_count,
                ir.load_count,
                ir.store_count,
                ir.br_count,
                ir.call_count,
                ir.phi_count,
                ir.alloca_count,
                ir.gep_count,
                ir.icmp_count,
                ir.fcmp_count,
                ir.ret_count,
                ir.other_inst_count,
                ir.basic_block_count,
                ir.total_instruction_count,
                ir.function_count,
                ir.loop_depth_approx,
                ir.load_store_ratio,
            ));
        }
        report.push('\n');
    }

    fn write_correlation_section(corr: &[FeatureCorrelation], report: &mut String) {
        let mut sorted = corr.to_vec();
        sorted.sort_by(|a, b| b.abs_r.partial_cmp(&a.abs_r).unwrap());

        report.push_str("9. FEATURE-PERFORMANCE CORRELATIONS\n");
        report.push_str("------------------------------------\n");
        report.push_str(
            "  Pearson r between raw IR feature and gap_vs_o3_pct.\n",
        );
        report.push_str(
            "  Positive r = harder benchmarks have more of this feature.\n\n",
        );
        report.push_str(&format!(
            "  {:<20} {:>10} {:>16} {:>12}\n",
            "Feature", "Pearson r", "Mean(beats-O3)", "Mean(hard)"
        ));
        report.push_str(&format!("  {}\n", "-".repeat(62)));

        for c in &sorted {
            let bar = bar_str(c.pearson_r, 12);
            report.push_str(&format!(
                "  {:<20} {:>+9.3} {:>15.1} {:>12.1}  {}\n",
                c.feature, c.pearson_r, c.mean_beats_o3, c.mean_hard, bar
            ));
        }
        report.push('\n');
    }

    fn write_summary(ceiling: &[CeilingEntry], report: &mut String) {
        report.push_str("10. ACTIONABLE SUMMARY\n");
        report.push_str("----------------------\n");

        let beats: Vec<&CeilingEntry> = ceiling.iter().filter(|c| c.gap_vs_o3_pct < 0.0).collect();
        let reachable: Vec<&CeilingEntry> = ceiling
            .iter()
            .filter(|c| c.gap_vs_o3_pct >= 0.0 && c.gap_vs_o3_pct < 20.0)
            .collect();
        let gap: Vec<&CeilingEntry> = ceiling
            .iter()
            .filter(|c| c.gap_vs_o3_pct >= 20.0 && c.gap_vs_o3_pct < 100.0)
            .collect();
        let unreachable: Vec<&CeilingEntry> =
            ceiling.iter().filter(|c| c.gap_vs_o3_pct >= 100.0).collect();

        if !beats.is_empty() {
            report.push_str(&format!(
                "\n  BEATS O3 ({} benchmarks — pipeline already competitive):\n",
                beats.len()
            ));
            for c in &beats {
                report.push_str(&format!(
                    "    {:<25} {:+.1}% vs O3, {:+.1}% vs O2, {:.1}x vs O0  (len={})\n",
                    c.function, c.gap_vs_o3_pct, c.gap_vs_o2_pct, c.speedup_vs_o0, c.best_seq_len
                ));
            }
        }

        if !reachable.is_empty() {
            report.push_str(&format!(
                "\n  REACHABLE ({} benchmarks — RL agent can likely find good sequences):\n",
                reachable.len()
            ));
            for c in &reachable {
                report.push_str(&format!(
                    "    {:<25} {:+.1}% vs O3, {:+.1}% vs O2, {:.1}x vs O0\n",
                    c.function, c.gap_vs_o3_pct, c.gap_vs_o2_pct, c.speedup_vs_o0
                ));
            }
        }

        let trainable = beats.len() + reachable.len();
        let total = ceiling.len();

        if !gap.is_empty() || !unreachable.is_empty() {
            report.push_str(&format!(
                "\n  NEEDS PIPELINE WORK ({} benchmarks — RL won't close this gap):\n",
                gap.len() + unreachable.len()
            ));
            for c in gap.iter().chain(unreachable.iter()) {
                report.push_str(&format!(
                    "    {:<25} {:+.1}% vs O3, {:+.1}% vs O2, {:.1}x vs O0\n",
                    c.function, c.gap_vs_o3_pct, c.gap_vs_o2_pct, c.speedup_vs_o0
                ));
            }
            report.push_str("\n  Root causes for large gaps:\n");
            report.push_str("    - Missing pass parameters (simplifycfg flags, sroa<modify-cfg>, etc.)\n");
            report.push_str("    - Missing DevirtSCCRepeatedPass nesting (O3 runs inner loop 4x)\n");
            report.push_str("    - Missing passes not in our menu (loop-vectorize params, SLP, etc.)\n");
        }

        report.push_str(&format!(
            "\n  Training viability: {trainable}/{total} benchmarks are trainable targets.\n"
        ));
        if trainable < total / 2 {
            report.push_str("  ** Less than half reachable. Fix pipeline first. **\n");
        }
        report.push('\n');
    }
}

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

fn median_u64(sorted: &[u64]) -> u64 {
    let n = sorted.len();
    if n == 0 {
        return 0;
    }
    sorted[n / 2]
}

fn pearson_r(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len().min(ys.len()) as f64;
    if n < 2.0 {
        return 0.0;
    }
    let mx = xs.iter().sum::<f64>() / n;
    let my = ys.iter().sum::<f64>() / n;
    let num: f64 = xs.iter().zip(ys.iter()).map(|(x, y)| (x - mx) * (y - my)).sum();
    let dx: f64 = xs.iter().map(|x| (x - mx).powi(2)).sum::<f64>().sqrt();
    let dy: f64 = ys.iter().map(|y| (y - my).powi(2)).sum::<f64>().sqrt();
    if dx * dy < 1e-10 {
        0.0
    } else {
        num / (dx * dy)
    }
}

/// ASCII bar for report legibility (range -1..+1 → width chars).
fn bar_str(r: f64, width: usize) -> String {
    let half = width / 2;
    let filled = ((r.abs() * half as f64).round() as usize).min(half);
    if r >= 0.0 {
        format!("{}{}", " ".repeat(half), "+".repeat(filled))
    } else {
        let pad = half - filled;
        format!("{}{}{}", " ".repeat(pad), "-".repeat(filled), " ".repeat(half))
    }
}
