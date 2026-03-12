use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write as _};
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

#[derive(Debug, Serialize)]
struct BenchmarkFeatureEntry {
    function: String,
    difficulty: String,
    cluster: usize,
    basic_block_count: u32,
    total_instruction_count: u32,
    load_count: u32,
    store_count: u32,
    br_count: u32,
    call_count: u32,
    phi_count: u32,
    gep_count: u32,
    alloca_count: u32,
    loop_depth_approx: u32,
    function_count: u32,
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

        // 5. IR feature landscape with clustering (optional)
        let (features, zscore_matrix): (Vec<BenchmarkFeatureEntry>, Vec<Vec<f64>>) =
            if let Some(fdir) = functions_dir {
                let (f, zmat) = self.ir_feature_landscape(fdir, &ceiling)?;
                Self::write_features_section(&f, &mut report);
                let file = File::create(output_dir.join("ir_features.json"))?;
                serde_json::to_writer_pretty(file, &f)?;
                (f, zmat)
            } else {
                (Vec::new(), Vec::new())
            };

        // 6. Summary
        Self::write_summary(&ceiling, &mut report);

        fs::write(output_dir.join("report.txt"), &report)?;
        eprintln!(
            "Wrote report to {}",
            output_dir.join("report.txt").display()
        );

        // 7. Generate SVG plots
        let plot_data = self.build_plot_data(&ceiling, &enrichment, &dist, &features, &zscore_matrix);
        plots::generate_all(output_dir, &plot_data)?;

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
            let top10_idx = (n / 10).max(1);
            let top10_median = times[top10_idx / 2];
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
                speedup_vs_o0: speedup_vs_o0,
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

    /// Returns (feature entries for report/JSON, z-score normalized matrix for heatmap)
    fn ir_feature_landscape(
        &self,
        functions_dir: &Path,
        ceiling: &[CeilingEntry],
    ) -> Result<(Vec<BenchmarkFeatureEntry>, Vec<Vec<f64>>)> {
        let work_dir = std::path::PathBuf::from("/tmp/llvm-lstm-eda-features");
        let pipeline = CompilationPipeline::new(work_dir);

        let difficulty_map: HashMap<String, String> = ceiling
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
                (c.function.clone(), diff.to_string())
            })
            .collect();

        // Collect raw features
        let mut names = Vec::new();
        let mut raw_features: Vec<IrFeatures> = Vec::new();

        for entry in fs::read_dir(functions_dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "c") {
                let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                let ir = pipeline.emit_ir(&path)?;
                let f = IrFeatures::from_ll_file(&ir)?;
                names.push(stem);
                raw_features.push(f);
            }
        }

        // Build feature matrix and z-score normalize
        let ndims = IrFeatures::feature_count();
        let n = raw_features.len();
        let mut feature_matrix: Vec<Vec<f64>> = raw_features
            .iter()
            .map(|f| f.to_vec().iter().map(|&v| v as f64).collect())
            .collect();

        for d in 0..ndims {
            let col: Vec<f64> = feature_matrix.iter().map(|row| row[d]).collect();
            let mean = col.iter().sum::<f64>() / n as f64;
            let var = col.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
            let std = var.sqrt().max(1e-8);
            for row in feature_matrix.iter_mut() {
                row[d] = (row[d] - mean) / std;
            }
        }

        // k-means clustering (k=4) on normalized features
        let clusters = simple_kmeans(&feature_matrix, 4, 50);

        // Build entries paired with z-scored rows
        let mut paired: Vec<(BenchmarkFeatureEntry, Vec<f64>)> = Vec::new();
        for (i, (name, f)) in names.iter().zip(raw_features.iter()).enumerate() {
            let difficulty = difficulty_map
                .get(name)
                .cloned()
                .unwrap_or_else(|| "unknown".into());
            let entry = BenchmarkFeatureEntry {
                function: name.clone(),
                difficulty,
                cluster: clusters[i],
                basic_block_count: f.basic_block_count,
                total_instruction_count: f.total_instruction_count,
                load_count: f.load_count,
                store_count: f.store_count,
                br_count: f.br_count,
                call_count: f.call_count,
                phi_count: f.phi_count,
                gep_count: f.gep_count,
                alloca_count: f.alloca_count,
                loop_depth_approx: f.loop_depth_approx,
                function_count: f.function_count,
            };
            paired.push((entry, feature_matrix[i].clone()));
        }
        paired.sort_by(|a, b| a.0.cluster.cmp(&b.0.cluster).then(a.0.function.cmp(&b.0.function)));
        let (features, zscore_matrix): (Vec<_>, Vec<_>) = paired.into_iter().unzip();
        Ok((features, zscore_matrix))
    }

    // -----------------------------------------------------------------------
    // Plot data construction
    // -----------------------------------------------------------------------

    fn build_plot_data(
        &self,
        ceiling: &[CeilingEntry],
        enrichment: &[PassEnrichment],
        dist: &[DistributionStats],
        features: &[BenchmarkFeatureEntry],
        zscore_matrix: &[Vec<f64>],
    ) -> plots::PlotData {
        let ceiling_pts = ceiling
            .iter()
            .map(|c| plots::CeilingPoint {
                name: c.function.clone(),
                gap_vs_o3: c.gap_vs_o3_pct,
                gap_vs_o2: c.gap_vs_o2_pct,
            })
            .collect();

        let enrich_pts = enrichment
            .iter()
            .map(|e| plots::EnrichPoint {
                name: e.pass_name.clone(),
                enrichment: e.enrichment,
                top10_pct: e.presence_in_top10pct,
                overall_pct: e.presence_overall,
            })
            .collect();

        // Build baseline lookup for O3 times
        let baseline_map = self.build_baseline_map();

        let dist_pts = dist
            .iter()
            .map(|d| {
                let o3 = baseline_map
                    .get(&d.function)
                    .map(|b| b.2 as f64 / 1_000_000.0)
                    .unwrap_or(0.0);
                plots::DistPoint {
                    name: d.function.clone(),
                    p10: d.p10_ns as f64 / 1_000_000.0,
                    p25: d.p25_ns as f64 / 1_000_000.0,
                    median: d.median_ns as f64 / 1_000_000.0,
                    p75: d.p75_ns as f64 / 1_000_000.0,
                    p90: d.p90_ns as f64 / 1_000_000.0,
                    o3,
                }
            })
            .collect();

        // IR feature heatmap data (already z-score normalized from ir_feature_landscape)
        let ir_features: Vec<plots::FeatureRow> = features
            .iter()
            .zip(zscore_matrix.iter())
            .map(|(f, zrow)| plots::FeatureRow {
                name: f.function.clone(),
                cluster: f.cluster,
                values: zrow.clone(),
            })
            .collect();

        plots::PlotData {
            ceiling: ceiling_pts,
            enrichment: enrich_pts,
            distributions: dist_pts,
            ir_features,
        }
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
        report.push_str("  Best = fastest from 2000 random pass sequences.\n");
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

    fn write_features_section(features: &[BenchmarkFeatureEntry], report: &mut String) {
        report.push_str("5. IR FEATURE LANDSCAPE (Pre-optimization)\n");
        report.push_str("-------------------------------------------\n");
        report.push_str(
            "  Features from clang -O3 -disable-llvm-optzns (frontend-annotated, no LLVM passes).\n",
        );
        report.push_str(
            "  Clustered by IR similarity (k-means, k=4 on z-scored features).\n\n",
        );
        report.push_str(&format!(
            "  {:<22} {:>3} {:>5} {:>5} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>9}\n",
            "Benchmark", "C", "BB", "Inst", "Ld", "St", "Br", "Cal", "Phi", "GEP", "Alc", "Lp", "Reachable"
        ));
        report.push_str(&format!("  {}\n", "-".repeat(92)));

        let mut current_cluster = None;
        for f in features {
            if current_cluster != Some(f.cluster) {
                if current_cluster.is_some() {
                    report.push_str(&format!("  {}\n", "-".repeat(92)));
                }
                current_cluster = Some(f.cluster);
            }
            report.push_str(&format!(
                "  {:<22} {:>3} {:>5} {:>5} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>9}\n",
                f.function,
                f.cluster,
                f.basic_block_count,
                f.total_instruction_count,
                f.load_count,
                f.store_count,
                f.br_count,
                f.call_count,
                f.phi_count,
                f.gep_count,
                f.alloca_count,
                f.loop_depth_approx,
                f.difficulty
            ));
        }
        report.push('\n');
    }

    fn write_summary(ceiling: &[CeilingEntry], report: &mut String) {
        report.push_str("6. ACTIONABLE SUMMARY\n");
        report.push_str("---------------------\n");

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
// Simple k-means (no external dep)
// ---------------------------------------------------------------------------

fn simple_kmeans(data: &[Vec<f64>], k: usize, max_iter: usize) -> Vec<usize> {
    let n = data.len();
    let dims = data[0].len();
    if n <= k {
        return (0..n).collect();
    }

    // Initialize centroids with evenly spaced indices
    let mut centroids: Vec<Vec<f64>> = (0..k)
        .map(|i| data[i * n / k].clone())
        .collect();

    let mut assignments = vec![0usize; n];

    for _ in 0..max_iter {
        // Assign each point to nearest centroid
        let mut changed = false;
        for (i, point) in data.iter().enumerate() {
            let mut best_c = 0;
            let mut best_dist = f64::MAX;
            for (c, centroid) in centroids.iter().enumerate() {
                let dist: f64 = point
                    .iter()
                    .zip(centroid.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum();
                if dist < best_dist {
                    best_dist = dist;
                    best_c = c;
                }
            }
            if assignments[i] != best_c {
                assignments[i] = best_c;
                changed = true;
            }
        }

        if !changed {
            break;
        }

        // Recompute centroids
        let mut sums = vec![vec![0.0f64; dims]; k];
        let mut counts = vec![0usize; k];
        for (i, point) in data.iter().enumerate() {
            let c = assignments[i];
            counts[c] += 1;
            for (d, &v) in point.iter().enumerate() {
                sums[c][d] += v;
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                for d in 0..dims {
                    centroids[c][d] = sums[c][d] / counts[c] as f64;
                }
            }
        }
    }

    assignments
}
