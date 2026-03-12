use std::path::Path;

use anyhow::Result;
use plotters::prelude::*;

// ---------------------------------------------------------------------------
// Public entry point — called from eda after analysis
// ---------------------------------------------------------------------------

pub fn generate_all(output_dir: &Path, report: &PlotData) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;

    ceiling_chart(output_dir, &report.ceiling)?;
    enrichment_chart(output_dir, &report.enrichment)?;
    distribution_chart(output_dir, &report.distributions)?;
    ir_heatmap(output_dir, &report.ir_features)?;

    eprintln!("Wrote 4 SVG plots to {}", output_dir.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Data transfer types (filled by eda.rs)
// ---------------------------------------------------------------------------

pub struct PlotData {
    pub ceiling: Vec<CeilingPoint>,
    pub enrichment: Vec<EnrichPoint>,
    pub distributions: Vec<DistPoint>,
    pub ir_features: Vec<FeatureRow>,
}

/// One benchmark's z-score normalized IR features for the heatmap
pub struct FeatureRow {
    pub name: String,
    pub cluster: usize,
    /// z-score normalized values, one per feature dimension
    pub values: Vec<f64>,
}

/// Feature dimension names (matches IrFeatures::to_vec() order — all 18 dimensions)
pub const FEATURE_NAMES: &[&str] = &[
    "add", "mul", "load", "store", "br", "call", "phi",
    "alloca", "gep", "icmp", "fcmp", "ret", "other",
    "bb", "inst", "func", "loops", "ld/st",
];

pub struct CeilingPoint {
    pub name: String,
    pub gap_vs_o3: f64,
    pub gap_vs_o2: f64,
}

pub struct EnrichPoint {
    pub name: String,
    pub enrichment: f64,
    pub top10_pct: f64,
    pub overall_pct: f64,
}

pub struct DistPoint {
    pub name: String,
    pub p10: f64,
    pub p25: f64,
    pub median: f64,
    pub p75: f64,
    pub p90: f64,
    pub o3: f64,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TITLE_FONT: u32 = 22;
const LABEL_FONT: u32 = 14;
const AXIS_FONT: u32 = 13;
const LEGEND_FONT: u32 = 14;
const TICK_FONT: u32 = 12;

// ---------------------------------------------------------------------------
// 1. Ceiling gap chart (horizontal bars)
//    Shows: how close the best random sequence got to O3 and O2
//    Negative = faster than baseline (good), Positive = slower
// ---------------------------------------------------------------------------

fn ceiling_chart(dir: &Path, points: &[CeilingPoint]) -> Result<()> {
    if points.is_empty() {
        return Ok(());
    }
    let path = dir.join("ceiling_gaps.svg");
    let n = points.len();
    let row_height = 28u32;
    let height = (n as u32 * row_height + 120).max(500).min(1400);
    let root = SVGBackend::new(&path, (1000, height)).into_drawing_area();
    root.fill(&WHITE)?;

    let x_min = points
        .iter()
        .map(|p| p.gap_vs_o3.min(p.gap_vs_o2))
        .fold(f64::MAX, f64::min)
        .min(-5.0)
        .max(-60.0)
        - 5.0;
    let x_max = points
        .iter()
        .map(|p| p.gap_vs_o3.max(p.gap_vs_o2))
        .fold(f64::MIN, f64::max)
        .max(5.0)
        .min(200.0)
        + 15.0;

    let y_range = -0.5..(n as f64 + 0.5);

    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Best Random Search vs Baselines (% gap)",
            ("sans-serif", TITLE_FONT),
        )
        .margin(15)
        .margin_right(30)
        .x_label_area_size(45)
        .y_label_area_size(200)
        .build_cartesian_2d(x_min..x_max, y_range)?;

    chart
        .configure_mesh()
        .disable_y_mesh()
        .x_label_style(("sans-serif", TICK_FONT))
        .y_label_style(("sans-serif", LABEL_FONT))
        .y_labels(n)
        .y_label_formatter(&|y| {
            let idx = y.round() as usize;
            let rev_idx = if idx < n { n - 1 - idx } else { return String::new() };
            points
                .get(rev_idx)
                .map(|p| p.name.clone())
                .unwrap_or_default()
        })
        .x_desc("% gap (negative = beats baseline)")
        .x_label_offset(0)
        .axis_desc_style(("sans-serif", AXIS_FONT))
        .draw()?;

    // Zero line (break-even)
    chart.draw_series(std::iter::once(PathElement::new(
        vec![(0.0, -0.5), (0.0, n as f64 + 0.5)],
        BLACK.stroke_width(1),
    )))?;

    let blue = RGBColor(31, 119, 180);
    let green = RGBColor(44, 160, 44);
    let orange = RGBColor(255, 140, 0);

    for (i, p) in points.iter().enumerate() {
        let y = (n - 1 - i) as f64;
        let gap_o3 = p.gap_vs_o3.clamp(x_min, x_max);
        let gap_o2 = p.gap_vs_o2.clamp(x_min, x_max);

        let bar_color = if p.gap_vs_o3 < 0.0 { green } else { blue };
        chart.draw_series(std::iter::once(Rectangle::new(
            [(0.0, y - 0.35), (gap_o3, y + 0.35)],
            bar_color.mix(0.8).filled(),
        )))?;

        // vs O2 diamond marker
        let dy = 0.15;
        chart.draw_series(std::iter::once(Polygon::new(
            vec![
                (gap_o2, y + dy),
                (gap_o2 + (x_max - x_min) * 0.008, y),
                (gap_o2, y - dy),
                (gap_o2 - (x_max - x_min) * 0.008, y),
            ],
            orange.filled(),
        )))?;
    }

    // Legend
    chart
        .draw_series(std::iter::once(Rectangle::new(
            [(x_max, 0.0), (x_max, 0.0)],
            blue.filled(),
        )))?
        .label("Gap vs O3 (green = beats)")
        .legend(move |(x, y)| {
            Rectangle::new([(x, y - 5), (x + 16, y + 5)], blue.mix(0.8).filled())
        });

    chart
        .draw_series(std::iter::once(Rectangle::new(
            [(x_max, 0.0), (x_max, 0.0)],
            orange.filled(),
        )))?
        .label("Gap vs O2")
        .legend(move |(x, y)| {
            Polygon::new(
                vec![
                    (x + 8, y - 5),
                    (x + 14, y),
                    (x + 8, y + 5),
                    (x + 2, y),
                ],
                orange.filled(),
            )
        });

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::LowerRight)
        .margin(15)
        .label_font(("sans-serif", LEGEND_FONT))
        .background_style(WHITE.mix(0.9))
        .border_style(BLACK.mix(0.4))
        .draw()?;

    root.present()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Pass enrichment chart
//    How often each pass appears in top-10% sequences vs overall.
//    Ratio > 1.0 means overrepresented in good sequences.
// ---------------------------------------------------------------------------

fn enrichment_chart(dir: &Path, points: &[EnrichPoint]) -> Result<()> {
    if points.is_empty() {
        return Ok(());
    }
    let path = dir.join("pass_enrichment.svg");
    let n = points.len();
    let row_height = 26u32;
    let height = (n as u32 * row_height + 120).max(500).min(1200);
    let root = SVGBackend::new(&path, (900, height)).into_drawing_area();
    root.fill(&WHITE)?;

    let x_max = points
        .iter()
        .map(|p| p.enrichment)
        .fold(0.0f64, f64::max)
        + 0.3;
    let y_range = -0.5..(n as f64 + 0.5);

    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Pass Enrichment in Top-10% Sequences",
            ("sans-serif", TITLE_FONT),
        )
        .margin(15)
        .margin_right(30)
        .x_label_area_size(45)
        .y_label_area_size(200)
        .build_cartesian_2d(0.0..x_max, y_range)?;

    chart
        .configure_mesh()
        .disable_y_mesh()
        .x_label_style(("sans-serif", TICK_FONT))
        .y_label_style(("sans-serif", LABEL_FONT))
        .y_labels(n)
        .y_label_formatter(&|y| {
            let idx = y.round() as usize;
            let rev_idx = if idx < n { n - 1 - idx } else { return String::new() };
            points
                .get(rev_idx)
                .map(|p| format!("{} ({:.2}x)", p.name, p.enrichment))
                .unwrap_or_default()
        })
        .x_desc("Enrichment ratio (>1.0 = overrepresented in good sequences)")
        .axis_desc_style(("sans-serif", AXIS_FONT))
        .draw()?;

    // 1.0x reference line
    chart.draw_series(std::iter::once(PathElement::new(
        vec![(1.0, -0.5), (1.0, n as f64 + 0.5)],
        BLACK.stroke_width(1),
    )))?;

    let green = RGBColor(44, 160, 44);
    let gray = RGBColor(160, 160, 160);

    for (i, p) in points.iter().enumerate() {
        let y = (n - 1 - i) as f64;
        let color = if p.enrichment > 1.2 { green } else { gray };

        chart.draw_series(std::iter::once(Rectangle::new(
            [(0.0, y - 0.38), (p.enrichment, y + 0.38)],
            color.mix(0.7).filled(),
        )))?;
    }

    root.present()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Distribution chart (box-plot style)
//    Each benchmark: box = P25-P75, whiskers = P10-P90, line = median
//    Red line = O3 baseline time
// ---------------------------------------------------------------------------

fn distribution_chart(dir: &Path, points: &[DistPoint]) -> Result<()> {
    if points.is_empty() {
        return Ok(());
    }
    let path = dir.join("distributions.svg");
    let n = points.len();
    let row_height = 28u32;
    let height = (n as u32 * row_height + 120).max(500).min(1400);
    let root = SVGBackend::new(&path, (1000, height)).into_drawing_area();
    root.fill(&WHITE)?;

    let x_max = points
        .iter()
        .flat_map(|p| [p.p90, p.o3])
        .fold(0.0f64, f64::max)
        * 1.15;
    let y_range = -0.5..(n as f64 + 0.5);

    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Random Search Time Distributions vs O3",
            ("sans-serif", TITLE_FONT),
        )
        .margin(15)
        .margin_right(30)
        .x_label_area_size(45)
        .y_label_area_size(200)
        .build_cartesian_2d(0.0..x_max, y_range)?;

    chart
        .configure_mesh()
        .disable_y_mesh()
        .x_label_style(("sans-serif", TICK_FONT))
        .y_label_style(("sans-serif", LABEL_FONT))
        .y_labels(n)
        .y_label_formatter(&|y| {
            let idx = y.round() as usize;
            let rev_idx = if idx < n { n - 1 - idx } else { return String::new() };
            points.get(rev_idx).map(|p| p.name.clone()).unwrap_or_default()
        })
        .x_desc("Time (ms)")
        .axis_desc_style(("sans-serif", AXIS_FONT))
        .draw()?;

    let blue = RGBColor(31, 119, 180);
    let red = RGBColor(214, 39, 40);

    for (i, p) in points.iter().enumerate() {
        let y = (n - 1 - i) as f64;

        // P10-P90 whisker line
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(p.p10, y), (p.p90, y)],
            blue.stroke_width(1),
        )))?;

        // Whisker end caps
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(p.p10, y - 0.15), (p.p10, y + 0.15)],
            blue.stroke_width(1),
        )))?;
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(p.p90, y - 0.15), (p.p90, y + 0.15)],
            blue.stroke_width(1),
        )))?;

        // P25-P75 box
        chart.draw_series(std::iter::once(Rectangle::new(
            [(p.p25, y - 0.32), (p.p75, y + 0.32)],
            blue.mix(0.25).filled(),
        )))?;
        chart.draw_series(std::iter::once(Rectangle::new(
            [(p.p25, y - 0.32), (p.p75, y + 0.32)],
            blue.stroke_width(1),
        )))?;

        // Median line (bold blue)
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(p.median, y - 0.32), (p.median, y + 0.32)],
            blue.stroke_width(3),
        )))?;

        // O3 baseline — red vertical line with diamond
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(p.o3, y - 0.38), (p.o3, y + 0.38)],
            red.stroke_width(2),
        )))?;
        let dx = x_max * 0.005;
        let dy = 0.12;
        chart.draw_series(std::iter::once(Polygon::new(
            vec![
                (p.o3, y + dy),
                (p.o3 + dx, y),
                (p.o3, y - dy),
                (p.o3 - dx, y),
            ],
            red.filled(),
        )))?;
    }

    // Legend
    chart
        .draw_series(std::iter::once(Rectangle::new(
            [(x_max, 0.0), (x_max, 0.0)],
            blue.mix(0.25).filled(),
        )))?
        .label("P25-P75 box (median = bold line)")
        .legend(move |(x, y)| {
            Rectangle::new([(x, y - 5), (x + 16, y + 5)], blue.mix(0.25).filled())
        });

    chart
        .draw_series(std::iter::once(Rectangle::new(
            [(x_max, 0.0), (x_max, 0.0)],
            red.filled(),
        )))?
        .label("O3 baseline")
        .legend(move |(x, y)| {
            PathElement::new(vec![(x + 3, y - 6), (x + 3, y + 6)], red.stroke_width(2))
        });

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperRight)
        .margin(15)
        .label_font(("sans-serif", LEGEND_FONT))
        .background_style(WHITE.mix(0.9))
        .border_style(BLACK.mix(0.4))
        .draw()?;

    root.present()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. IR feature heatmap
//    Rows = benchmarks (grouped by cluster), Columns = 18 IR features
//    Color = z-score intensity (blue = low, white = avg, red = high)
// ---------------------------------------------------------------------------

fn ir_heatmap(dir: &Path, rows: &[FeatureRow]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let path = dir.join("ir_features_heatmap.svg");
    let n = rows.len();
    let ndims = rows[0].values.len().min(FEATURE_NAMES.len());

    let cell_w = 44u32;
    let cell_h = 22u32;
    let left_margin = 210u32;
    let top_margin = 80u32;
    let right_margin = 120u32;
    let bottom_margin = 30u32;

    let width = left_margin + cell_w * ndims as u32 + right_margin;
    let height = top_margin + cell_h * n as u32 + bottom_margin;

    let root = SVGBackend::new(&path, (width, height)).into_drawing_area();
    root.fill(&WHITE)?;

    // Title
    root.draw(&Text::new(
        "Pre-optimization IR Feature Profiles (z-score)",
        (width as i32 / 2 - 200, 15),
        ("sans-serif", TITLE_FONT).into_font().color(&BLACK),
    ))?;

    // Column headers
    for (j, name) in FEATURE_NAMES.iter().enumerate().take(ndims) {
        let x = left_margin as i32 + j as i32 * cell_w as i32 + cell_w as i32 / 2 - 8;
        let y = top_margin as i32 - 8;
        root.draw(&Text::new(
            name.to_string(),
            (x, y),
            ("sans-serif", 11).into_font().color(&BLACK),
        ))?;
    }

    let cluster_colors = [
        RGBColor(31, 119, 180),
        RGBColor(255, 127, 14),
        RGBColor(44, 160, 44),
        RGBColor(214, 39, 40),
        RGBColor(148, 103, 189),
    ];

    // Draw cells + row labels
    for (i, row) in rows.iter().enumerate() {
        let y_px = top_margin as i32 + i as i32 * cell_h as i32;

        let cc = cluster_colors[row.cluster % cluster_colors.len()];

        // Cluster color dot
        root.draw(&Circle::new(
            (12, y_px + cell_h as i32 / 2),
            5,
            cc.filled(),
        ))?;

        // Name
        root.draw(&Text::new(
            row.name.clone(),
            (22, y_px + 4),
            ("sans-serif", LABEL_FONT - 1).into_font().color(&BLACK),
        ))?;

        // Feature cells
        for (j, &val) in row.values.iter().enumerate().take(ndims) {
            let x_px = left_margin as i32 + j as i32 * cell_w as i32;
            let color = zscore_color(val);

            root.draw(&Rectangle::new(
                [(x_px, y_px), (x_px + cell_w as i32, y_px + cell_h as i32)],
                color.filled(),
            ))?;
            root.draw(&Rectangle::new(
                [(x_px, y_px), (x_px + cell_w as i32, y_px + cell_h as i32)],
                RGBColor(220, 220, 220).stroke_width(1),
            ))?;

            if val.abs() > 0.5 {
                let text_color = if val.abs() > 1.5 { WHITE } else { BLACK };
                root.draw(&Text::new(
                    format!("{:.1}", val),
                    (x_px + 6, y_px + 4),
                    ("sans-serif", 10).into_font().color(&text_color),
                ))?;
            }
        }
    }

    // Color bar legend
    let bar_x = (left_margin + cell_w * ndims as u32 + 20) as i32;
    let bar_top = top_margin as i32 + 10;
    let bar_h = (n as i32 * cell_h as i32).min(200);
    let bar_w = 18;

    for py in 0..bar_h {
        let frac = py as f64 / bar_h as f64;
        let z = 3.0 - 6.0 * frac;
        let color = zscore_color(z);
        root.draw(&Rectangle::new(
            [(bar_x, bar_top + py), (bar_x + bar_w, bar_top + py + 1)],
            color.filled(),
        ))?;
    }
    root.draw(&Text::new(
        "High (+3\u{03C3})".to_string(),
        (bar_x + bar_w + 4, bar_top - 2),
        ("sans-serif", 11).into_font().color(&BLACK),
    ))?;
    root.draw(&Text::new(
        "Avg (0)".to_string(),
        (bar_x + bar_w + 4, bar_top + bar_h / 2 - 5),
        ("sans-serif", 11).into_font().color(&BLACK),
    ))?;
    root.draw(&Text::new(
        "Low (-3\u{03C3})".to_string(),
        (bar_x + bar_w + 4, bar_top + bar_h - 8),
        ("sans-serif", 11).into_font().color(&BLACK),
    ))?;

    root.present()?;
    Ok(())
}

/// Map z-score to a diverging blue-white-red color
fn zscore_color(z: f64) -> RGBColor {
    let z = z.clamp(-3.0, 3.0);
    if z >= 0.0 {
        let t = (z / 3.0).min(1.0);
        RGBColor(
            255,
            (255.0 * (1.0 - t * 0.7)) as u8,
            (255.0 * (1.0 - t * 0.85)) as u8,
        )
    } else {
        let t = (-z / 3.0).min(1.0);
        RGBColor(
            (255.0 * (1.0 - t * 0.85)) as u8,
            (255.0 * (1.0 - t * 0.55)) as u8,
            255,
        )
    }
}
