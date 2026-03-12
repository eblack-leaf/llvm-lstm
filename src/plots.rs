use std::fmt::Write as _;
use std::path::Path;

use anyhow::Result;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn generate_all(output_dir: &Path, data: &PlotData) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;
    ceiling_chart(output_dir, &data.ceiling)?;
    enrichment_chart(output_dir, &data.enrichment)?;
    distribution_chart(output_dir, &data.distributions)?;
    ir_heatmap(output_dir, &data.ir_features)?;
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

pub struct FeatureRow {
    pub name: String,
    pub cluster: usize,
    pub values: Vec<f64>,
}

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
// SVG helpers — hex colors go through hex() to avoid # in format!() macros
// ---------------------------------------------------------------------------

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Returns e.g. "#1f77b4" — kept out of format!() raw strings where # is
/// parsed as a Rust 2021 prefix literal.
fn hex(rgb: &str) -> String {
    format!("#{rgb}")
}

fn svg_header(buf: &mut String, width: u32, height: u32) {
    let _ = write!(buf,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" font-family="sans-serif">"#,
    );
    let _ = write!(buf, r#"<rect width="{width}" height="{height}" fill="{c}"/>"#, c = hex("fff"));
}

fn svg_footer(buf: &mut String) {
    buf.push_str("</svg>");
}

/// Pick a "nice" tick step for an axis range.
fn nice_step(range: f64, target_ticks: usize) -> f64 {
    let raw = range / target_ticks as f64;
    let mag = 10.0f64.powf(raw.log10().floor());
    let norm = raw / mag;
    let nice = if norm < 1.5 { 1.0 } else if norm < 3.5 { 2.0 } else if norm < 7.5 { 5.0 } else { 10.0 };
    nice * mag
}

/// Map z-score to "r,g,b" for use in fill="rgb(…)".
fn zscore_rgb(z: f64) -> String {
    let z = z.clamp(-3.0, 3.0);
    let (r, g, b) = if z >= 0.0 {
        let t = z / 3.0;
        (255, (255.0 * (1.0 - t * 0.7)) as u8, (255.0 * (1.0 - t * 0.85)) as u8)
    } else {
        let t = -z / 3.0;
        ((255.0 * (1.0 - t * 0.85)) as u8, (255.0 * (1.0 - t * 0.55)) as u8, 255)
    };
    format!("rgb({r},{g},{b})")
}

// ---------------------------------------------------------------------------
// 1. Ceiling gap chart
// ---------------------------------------------------------------------------

fn ceiling_chart(dir: &Path, pts: &[CeilingPoint]) -> Result<()> {
    if pts.is_empty() { return Ok(()); }
    let n = pts.len();
    let row_h = 26.0_f64;
    let left = 200.0_f64;
    let top = 50.0_f64;
    let plot_w = 700.0_f64;
    let width = (left + plot_w + 30.0) as u32;
    let height = (top + n as f64 * row_h + 50.0) as u32;

    let x_min = pts.iter().map(|p| p.gap_vs_o3.min(p.gap_vs_o2)).fold(f64::MAX, f64::min).min(-5.0) - 5.0;
    let x_max = pts.iter().map(|p| p.gap_vs_o3.max(p.gap_vs_o2)).fold(f64::MIN, f64::max).max(5.0) + 15.0;
    let x_range = x_max - x_min;
    let mx = |v: f64| left + (v - x_min) / x_range * plot_w;
    let my = |i: usize| top + (i as f64 + 0.5) * row_h;

    let mut s = String::with_capacity(8192);
    svg_header(&mut s, width, height);

    let _ = write!(s, r#"<text x="{:.0}" y="30" text-anchor="middle" font-size="18" font-weight="bold">Best Random Search vs Baselines (% gap)</text>"#, width as f64 / 2.0);

    // zero line
    let bot = top + n as f64 * row_h;
    let _ = write!(s, r#"<line x1="{:.1}" y1="{top}" x2="{:.1}" y2="{bot:.1}" stroke="{c}" stroke-width="1" stroke-dasharray="4,3"/>"#, mx(0.0), mx(0.0), c = hex("888"));

    // x ticks
    let step = nice_step(x_range, 8);
    let mut t = (x_min / step).ceil() * step;
    while t <= x_max {
        let tx = mx(t);
        let _ = write!(s, r#"<line x1="{tx:.1}" y1="{bot:.1}" x2="{tx:.1}" y2="{:.1}" stroke="{c}" stroke-width="1"/>"#, bot + 5.0, c = hex("ccc"));
        let _ = write!(s, r#"<text x="{tx:.1}" y="{:.1}" text-anchor="middle" font-size="11">{t:.0}</text>"#, bot + 18.0);
        t += step;
    }
    let _ = write!(s, r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="12" fill="{c}">% gap (negative = beats baseline)</text>"#, left + plot_w / 2.0, bot + 38.0, c = hex("555"));

    let green = hex("2ca02c");
    let blue = hex("1f77b4");
    let orange = hex("ff8c00");

    for (i, p) in pts.iter().enumerate() {
        let cy = my(i);
        let bh = row_h * 0.65;
        let bx1 = mx(0.0);
        let bx2 = mx(p.gap_vs_o3.clamp(x_min, x_max));
        let (bl, br) = if bx1 < bx2 { (bx1, bx2) } else { (bx2, bx1) };
        let col = if p.gap_vs_o3 < 0.0 { &green } else { &blue };
        let _ = write!(s, r#"<rect x="{bl:.1}" y="{:.1}" width="{:.1}" height="{bh:.1}" fill="{col}" opacity="0.8"/>"#, cy - bh / 2.0, br - bl);

        let dx = mx(p.gap_vs_o2.clamp(x_min, x_max));
        let ds = 5.0;
        let _ = write!(s, r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="{orange}"/>"#, dx, cy - ds, dx + ds, cy, dx, cy + ds, dx - ds, cy);

        let _ = write!(s, r#"<text x="{:.1}" y="{cy:.1}" text-anchor="end" font-size="12" dominant-baseline="middle">{}</text>"#, left - 8.0, xml_escape(&p.name));
    }

    // legend
    let lx = left + plot_w - 200.0;
    let ly = top + 10.0;
    let _ = write!(s, r#"<rect x="{lx:.0}" y="{ly:.0}" width="195" height="50" fill="{c1}" stroke="{c2}" rx="4"/>"#, c1 = hex("fff"), c2 = hex("ccc"));
    let _ = write!(s, r#"<rect x="{:.0}" y="{:.0}" width="14" height="10" fill="{blue}" opacity="0.8"/>"#, lx + 8.0, ly + 8.0);
    let _ = write!(s, r#"<text x="{:.0}" y="{:.0}" font-size="11">Gap vs O3 (green = beats)</text>"#, lx + 28.0, ly + 17.0);
    let _ = write!(s, r#"<polygon points="{:.0},{:.0} {:.0},{:.0} {:.0},{:.0} {:.0},{:.0}" fill="{orange}"/>"#, lx + 15.0, ly + 26.0, lx + 20.0, ly + 31.0, lx + 15.0, ly + 36.0, lx + 10.0, ly + 31.0);
    let _ = write!(s, r#"<text x="{:.0}" y="{:.0}" font-size="11">Gap vs O2</text>"#, lx + 28.0, ly + 35.0);

    svg_footer(&mut s);
    std::fs::write(dir.join("ceiling_gaps.svg"), &s)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Pass enrichment chart
// ---------------------------------------------------------------------------

fn enrichment_chart(dir: &Path, pts: &[EnrichPoint]) -> Result<()> {
    if pts.is_empty() { return Ok(()); }
    let n = pts.len();
    let row_h = 24.0_f64;
    let left = 220.0_f64;
    let top = 50.0_f64;
    let plot_w = 500.0_f64;
    let width = (left + plot_w + 30.0) as u32;
    let height = (top + n as f64 * row_h + 50.0) as u32;
    let x_max = pts.iter().map(|p| p.enrichment).fold(0.0f64, f64::max) + 0.3;
    let mx = |v: f64| left + v / x_max * plot_w;
    let my = |i: usize| top + (i as f64 + 0.5) * row_h;

    let mut s = String::with_capacity(4096);
    svg_header(&mut s, width, height);

    let _ = write!(s, r#"<text x="{:.0}" y="30" text-anchor="middle" font-size="18" font-weight="bold">Pass Enrichment in Top-10% Sequences</text>"#, width as f64 / 2.0);

    let bot = top + n as f64 * row_h;
    let _ = write!(s, r#"<line x1="{:.1}" y1="{top}" x2="{:.1}" y2="{bot:.1}" stroke="{c}" stroke-width="1" stroke-dasharray="4,3"/>"#, mx(1.0), mx(1.0), c = hex("888"));

    let step = nice_step(x_max, 6);
    let mut t = 0.0;
    while t <= x_max {
        let tx = mx(t);
        let _ = write!(s, r#"<line x1="{tx:.1}" y1="{bot:.1}" x2="{tx:.1}" y2="{:.1}" stroke="{c}" stroke-width="1"/>"#, bot + 5.0, c = hex("ccc"));
        let _ = write!(s, r#"<text x="{tx:.1}" y="{:.1}" text-anchor="middle" font-size="11">{t:.1}x</text>"#, bot + 18.0);
        t += step;
    }
    let _ = write!(s, r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="12" fill="{c}">&gt;1.0 = overrepresented in good sequences</text>"#, left + plot_w / 2.0, bot + 38.0, c = hex("555"));

    let green = hex("2ca02c");
    let gray = hex("a0a0a0");
    for (i, p) in pts.iter().enumerate() {
        let cy = my(i);
        let bh = row_h * 0.7;
        let bw = mx(p.enrichment) - left;
        let col = if p.enrichment > 1.2 { &green } else { &gray };
        let _ = write!(s, r#"<rect x="{left:.1}" y="{:.1}" width="{bw:.1}" height="{bh:.1}" fill="{col}" opacity="0.7"/>"#, cy - bh / 2.0);
        let _ = write!(s, r#"<text x="{:.1}" y="{cy:.1}" text-anchor="end" font-size="11" dominant-baseline="middle">{} ({:.2}x)</text>"#, left - 8.0, xml_escape(&p.name), p.enrichment);
    }

    svg_footer(&mut s);
    std::fs::write(dir.join("pass_enrichment.svg"), &s)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Distribution chart (box-plot style)
// ---------------------------------------------------------------------------

fn distribution_chart(dir: &Path, pts: &[DistPoint]) -> Result<()> {
    if pts.is_empty() { return Ok(()); }
    let n = pts.len();
    let row_h = 26.0_f64;
    let left = 200.0_f64;
    let top = 50.0_f64;
    let plot_w = 700.0_f64;
    let width = (left + plot_w + 30.0) as u32;
    let height = (top + n as f64 * row_h + 50.0) as u32;
    let x_max = pts.iter().flat_map(|p| [p.p90, p.o3]).fold(0.0f64, f64::max) * 1.15;
    let mx = |v: f64| left + v / x_max * plot_w;
    let my = |i: usize| top + (i as f64 + 0.5) * row_h;

    let mut s = String::with_capacity(8192);
    svg_header(&mut s, width, height);

    let _ = write!(s, r#"<text x="{:.0}" y="30" text-anchor="middle" font-size="18" font-weight="bold">Random Search Time Distributions vs O3</text>"#, width as f64 / 2.0);

    let bot = top + n as f64 * row_h;
    let step = nice_step(x_max, 8);
    let mut t = 0.0;
    let eee = hex("eee");
    while t <= x_max {
        let tx = mx(t);
        let _ = write!(s, r#"<line x1="{tx:.1}" y1="{top}" x2="{tx:.1}" y2="{bot:.1}" stroke="{eee}" stroke-width="1"/>"#);
        let _ = write!(s, r#"<text x="{tx:.1}" y="{:.1}" text-anchor="middle" font-size="11">{t:.1}</text>"#, bot + 18.0);
        t += step;
    }
    let _ = write!(s, r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="12" fill="{c}">Time (ms)</text>"#, left + plot_w / 2.0, bot + 38.0, c = hex("555"));

    let blue = hex("1f77b4");
    let red = hex("d62728");

    for (i, p) in pts.iter().enumerate() {
        let cy = my(i);
        let bh = row_h * 0.55;

        // whisker P10–P90
        let _ = write!(s, r#"<line x1="{:.1}" y1="{cy:.1}" x2="{:.1}" y2="{cy:.1}" stroke="{blue}" stroke-width="1"/>"#, mx(p.p10), mx(p.p90));
        for &v in &[p.p10, p.p90] {
            let vx = mx(v);
            let _ = write!(s, r#"<line x1="{vx:.1}" y1="{:.1}" x2="{vx:.1}" y2="{:.1}" stroke="{blue}" stroke-width="1"/>"#, cy - 4.0, cy + 4.0);
        }

        // box P25–P75
        let bx1 = mx(p.p25);
        let bx2 = mx(p.p75);
        let _ = write!(s, r#"<rect x="{bx1:.1}" y="{:.1}" width="{:.1}" height="{bh:.1}" fill="{blue}" fill-opacity="0.2" stroke="{blue}" stroke-width="1"/>"#, cy - bh / 2.0, bx2 - bx1);

        // median
        let mmx = mx(p.median);
        let _ = write!(s, r#"<line x1="{mmx:.1}" y1="{:.1}" x2="{mmx:.1}" y2="{:.1}" stroke="{blue}" stroke-width="3"/>"#, cy - bh / 2.0, cy + bh / 2.0);

        // O3 diamond
        let ox = mx(p.o3);
        let ds = 5.0;
        let _ = write!(s, r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="{red}"/>"#, ox, cy - ds, ox + ds, cy, ox, cy + ds, ox - ds, cy);

        let _ = write!(s, r#"<text x="{:.1}" y="{cy:.1}" text-anchor="end" font-size="12" dominant-baseline="middle">{}</text>"#, left - 8.0, xml_escape(&p.name));
    }

    // legend
    let lx = left + plot_w - 250.0;
    let ly = top + 10.0;
    let _ = write!(s, r#"<rect x="{lx:.0}" y="{ly:.0}" width="245" height="50" fill="{c1}" stroke="{c2}" rx="4"/>"#, c1 = hex("fff"), c2 = hex("ccc"));
    let _ = write!(s, r#"<rect x="{:.0}" y="{:.0}" width="14" height="10" fill="{blue}" fill-opacity="0.2" stroke="{blue}"/>"#, lx + 8.0, ly + 8.0);
    let _ = write!(s, r#"<text x="{:.0}" y="{:.0}" font-size="11">P25-P75 box (bold = median)</text>"#, lx + 28.0, ly + 17.0);
    let _ = write!(s, r#"<polygon points="{:.0},{:.0} {:.0},{:.0} {:.0},{:.0} {:.0},{:.0}" fill="{red}"/>"#, lx + 15.0, ly + 26.0, lx + 20.0, ly + 31.0, lx + 15.0, ly + 36.0, lx + 10.0, ly + 31.0);
    let _ = write!(s, r#"<text x="{:.0}" y="{:.0}" font-size="11">O3 baseline</text>"#, lx + 28.0, ly + 35.0);

    svg_footer(&mut s);
    std::fs::write(dir.join("distributions.svg"), &s)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. IR feature heatmap
// ---------------------------------------------------------------------------

fn ir_heatmap(dir: &Path, rows: &[FeatureRow]) -> Result<()> {
    if rows.is_empty() { return Ok(()); }
    let n = rows.len();
    let ndims = rows[0].values.len().min(FEATURE_NAMES.len());
    let cw = 42.0_f64;
    let ch = 22.0_f64;
    let left = 180.0_f64;
    let top = 70.0_f64;
    let width = (left + cw * ndims as f64 + 110.0) as u32;
    let height = (top + ch * n as f64 + 20.0) as u32;

    let mut s = String::with_capacity(16384);
    svg_header(&mut s, width, height);

    let _ = write!(s, r#"<text x="{:.0}" y="28" text-anchor="middle" font-size="18" font-weight="bold">Pre-optimization IR Feature Profiles (z-score)</text>"#, width as f64 / 2.0);

    // column headers
    for (j, name) in FEATURE_NAMES.iter().enumerate().take(ndims) {
        let cx = left + j as f64 * cw + cw / 2.0;
        let _ = write!(s, r#"<text x="{cx:.1}" y="{:.1}" text-anchor="middle" font-size="11">{name}</text>"#, top - 6.0);
    }

    let cluster_colors = [hex("1f77b4"), hex("ff7f0e"), hex("2ca02c"), hex("d62728"), hex("9467bd")];
    let ddd = hex("ddd");

    for (i, row) in rows.iter().enumerate() {
        let ry = top + i as f64 * ch;
        let cc = &cluster_colors[row.cluster % cluster_colors.len()];

        let _ = write!(s, r#"<circle cx="12" cy="{:.1}" r="5" fill="{cc}"/>"#, ry + ch / 2.0);
        let _ = write!(s, r#"<text x="22" y="{:.1}" font-size="12" dominant-baseline="middle">{}</text>"#, ry + ch / 2.0, xml_escape(&row.name));

        for (j, &val) in row.values.iter().enumerate().take(ndims) {
            let cx = left + j as f64 * cw;
            let fill = zscore_rgb(val);
            let _ = write!(s, r#"<rect x="{cx:.1}" y="{ry:.1}" width="{cw}" height="{ch}" fill="{fill}" stroke="{ddd}" stroke-width="0.5"/>"#);

            if val.abs() > 0.5 {
                let tc = if val.abs() > 1.5 { hex("fff") } else { hex("000") };
                let _ = write!(s, r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="10" fill="{tc}" dominant-baseline="middle">{val:.1}</text>"#, cx + cw / 2.0, ry + ch / 2.0);
            }
        }
    }

    // color bar
    let bx = left + cw * ndims as f64 + 20.0;
    let bt = top + 10.0;
    let bh = (n as f64 * ch).min(200.0);
    let bw = 16.0;
    let steps = 40;
    let sh = bh / steps as f64;
    for si in 0..steps {
        let z = 3.0 - 6.0 * si as f64 / steps as f64;
        let fill = zscore_rgb(z);
        let sy = bt + si as f64 * sh;
        let _ = write!(s, r#"<rect x="{bx:.1}" y="{sy:.1}" width="{bw}" height="{:.1}" fill="{fill}"/>"#, sh + 0.5);
    }
    let lx = bx + bw + 6.0;
    let _ = write!(s, r#"<text x="{lx:.1}" y="{:.1}" font-size="10">+3σ</text>"#, bt + 4.0);
    let _ = write!(s, r#"<text x="{lx:.1}" y="{:.1}" font-size="10">0</text>"#, bt + bh / 2.0 + 3.0);
    let _ = write!(s, r#"<text x="{lx:.1}" y="{:.1}" font-size="10">-3σ</text>"#, bt + bh);

    svg_footer(&mut s);
    std::fs::write(dir.join("ir_features_heatmap.svg"), &s)?;
    Ok(())
}
