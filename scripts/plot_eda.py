#!/usr/bin/env python3
"""
EDA visualisation for llvm-lstm.

Reads the JSON files written by eda.rs and produces matplotlib/seaborn figures.
Usage:  python3 scripts/plot_eda.py <output_dir>
"""

import json
import sys
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
import matplotlib.ticker as ticker
import numpy as np

try:
    import seaborn as sns
    sns.set_theme(style="whitegrid", palette="muted", font_scale=0.95)
    HAS_SEABORN = True
except ImportError:
    HAS_SEABORN = False

# ── colour scheme ────────────────────────────────────────────────────────────
TIER_COLORS = {
    "beats-O3": "#2ca02c",
    "reachable": "#1f77b4",
    "gap":       "#ff7f0e",
    "hard":      "#d62728",
    "unknown":   "#9467bd",
}
TIER_ORDER = ["beats-O3", "reachable", "gap", "hard"]


def load(path: Path):
    if not path.exists():
        return None
    with open(path) as f:
        return json.load(f)


def savefig(fig, path: Path, tight=True):
    if tight:
        fig.tight_layout()
    fig.savefig(path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  wrote {path.name}")


# ── 1. Ceiling gaps ──────────────────────────────────────────────────────────

def plot_ceiling_gaps(data: list, out: Path):
    """Horizontal bar: best-random gap vs O3, colored by difficulty tier."""
    data = sorted(data, key=lambda x: x["gap_vs_o3_pct"])
    names   = [d["function"]      for d in data]
    gap_o3  = [d["gap_vs_o3_pct"] for d in data]
    gap_o2  = [d["gap_vs_o2_pct"] for d in data]
    t10_gap = [d["top10_gap_vs_o3_pct"] for d in data]

    def tier(g):
        if g < 0:    return "beats-O3"
        if g < 20:   return "reachable"
        if g < 100:  return "gap"
        return "hard"

    colors = [TIER_COLORS[tier(g)] for g in gap_o3]
    n = len(names)
    ys = np.arange(n)

    fig, ax = plt.subplots(figsize=(10, max(4, n * 0.38 + 1.2)))

    bars = ax.barh(ys, gap_o3, color=colors, alpha=0.85, height=0.55, label="Best vs O3")
    ax.scatter(gap_o2, ys, marker="D", color="#ff8c00", s=28, zorder=5, label="Best vs O2")
    ax.scatter(t10_gap, ys, marker="|", color="#666", s=40, linewidths=1.5,
               zorder=4, label="Top-10% median vs O3")

    ax.axvline(0, color="#333", linewidth=1.2, linestyle="--", alpha=0.6)
    ax.set_yticks(ys)
    ax.set_yticklabels(names, fontsize=8.5)
    ax.set_xlabel("% gap vs baseline  (negative = beats baseline)")
    ax.set_title("Best Random-Search Result vs Baselines")

    # tier legend patches
    patches = [mpatches.Patch(color=c, label=t, alpha=0.85)
               for t, c in TIER_COLORS.items() if t != "unknown"]
    handles, labels = ax.get_legend_handles_labels()
    ax.legend(handles=handles + patches, fontsize=7.5,
              loc="lower right", framealpha=0.9)

    ax.invert_yaxis()
    savefig(fig, out / "ceiling_gaps.png")


# ── 2. Distributions normalised to O3 ───────────────────────────────────────

def plot_distributions(dist_data: list, ceiling_data: list, out: Path):
    """Box-plot of per-function time distributions, x-axis normalised to O3 = 1."""
    o3_map = {d["function"]: d["o3_ns"] for d in ceiling_data}
    # sort by median / O3 ratio
    rows = []
    for d in dist_data:
        o3 = o3_map.get(d["function"], 0)
        if o3 == 0:
            continue
        rows.append({
            "function": d["function"],
            "p10": d["p10_ns"] / o3,
            "p25": d["p25_ns"] / o3,
            "median": d["median_ns"] / o3,
            "p75": d["p75_ns"] / o3,
            "p90": d["p90_ns"] / o3,
            "cv_pct": d["cv_pct"],
        })
    if not rows:
        return
    rows.sort(key=lambda r: r["median"])

    names = [r["function"] for r in rows]
    n = len(names)
    ys = np.arange(n)

    fig, axes = plt.subplots(1, 2, figsize=(14, max(4, n * 0.38 + 1.2)),
                              gridspec_kw={"width_ratios": [3, 1]})
    ax, ax_cv = axes

    for i, r in enumerate(rows):
        # whisker
        ax.plot([r["p10"], r["p90"]], [i, i], color="#1f77b4", lw=1.0, alpha=0.7)
        for v in (r["p10"], r["p90"]):
            ax.plot([v, v], [i - 0.12, i + 0.12], color="#1f77b4", lw=1.0, alpha=0.7)
        # box
        ax.barh(i, r["p75"] - r["p25"], left=r["p25"],
                height=0.5, color="#1f77b4", alpha=0.25, linewidth=0)
        ax.barh(i, r["p75"] - r["p25"], left=r["p25"],
                height=0.5, fill=False, edgecolor="#1f77b4", linewidth=0.8)
        # median
        ax.plot([r["median"], r["median"]], [i - 0.25, i + 0.25],
                color="#1f77b4", lw=2.2)

    ax.axvline(1.0, color="#d62728", lw=1.4, linestyle="--", label="O3 baseline")
    ax.set_yticks(ys)
    ax.set_yticklabels(names, fontsize=8.5)
    ax.set_xlabel("Execution time / O3 time  (1.0 = matches O3)")
    ax.set_title("Random-Search Time Distributions (normalised to O3)")
    ax.legend(fontsize=8)
    ax.invert_yaxis()

    # CV bar chart on the right
    cvs = [r["cv_pct"] for r in rows]
    ax_cv.barh(ys, cvs, color="#9467bd", alpha=0.75, height=0.55)
    ax_cv.set_yticks(ys)
    ax_cv.set_yticklabels([])
    ax_cv.set_xlabel("CV %")
    ax_cv.set_title("Pass Sensitivity")
    ax_cv.invert_yaxis()

    savefig(fig, out / "distributions.png")


# ── 3. Pass enrichment ───────────────────────────────────────────────────────

def plot_pass_enrichment(data: list, out: Path):
    """Grouped bar: top-10% presence vs overall, with enrichment ratio overlay."""
    data = sorted(data, key=lambda x: -x["enrichment"])[:30]
    if not data:
        return

    names   = [d["pass_name"]            for d in data]
    top10   = [d["presence_in_top10pct"] * 100 for d in data]
    overall = [d["presence_overall"]     * 100 for d in data]
    enrich  = [d["enrichment"]           for d in data]

    n = len(names)
    ys = np.arange(n)
    bh = 0.38

    fig, ax1 = plt.subplots(figsize=(11, max(5, n * 0.42 + 1.5)))
    ax2 = ax1.twiny()

    ax1.barh(ys + bh / 2, top10,   height=bh, color="#2ca02c", alpha=0.8, label="Top-10% presence")
    ax1.barh(ys - bh / 2, overall, height=bh, color="#aec7e8", alpha=0.8, label="Overall presence")
    ax1.axvline(0, color="#333", lw=0.8)

    ax2.scatter(enrich, ys, marker="o", color="#d62728", s=22, zorder=5, label="Enrichment ratio")
    ax2.axvline(1.0, color="#d62728", lw=1.0, linestyle=":", alpha=0.6)
    ax2.set_xlabel("Enrichment ratio (red dots)", color="#d62728")
    ax2.tick_params(axis="x", colors="#d62728")

    ax1.set_yticks(ys)
    ax1.set_yticklabels(names, fontsize=8)
    ax1.set_xlabel("Presence rate (%)")
    ax1.set_title("Pass Presence: Top-10% Sequences vs Overall  (top 30 by enrichment)")
    ax1.invert_yaxis()

    h1, l1 = ax1.get_legend_handles_labels()
    h2, l2 = ax2.get_legend_handles_labels()
    ax1.legend(h1 + h2, l1 + l2, fontsize=8, loc="lower right")

    savefig(fig, out / "pass_enrichment.png")


# ── 4. Pass impact ───────────────────────────────────────────────────────────

def plot_pass_impact(data: list, out: Path):
    """Horizontal bar of geometric-mean speedup with vs without each pass."""
    data = sorted(data, key=lambda x: -x["geo_mean_speedup"])[:25]
    if not data:
        return

    names    = [d["pass_name"]             for d in data]
    speedup  = [d["geo_mean_speedup"]      for d in data]
    speedup_t = [d["geo_mean_speedup_top10"] for d in data]
    n = len(names)
    ys = np.arange(n)
    bh = 0.38

    fig, ax = plt.subplots(figsize=(10, max(4, n * 0.4 + 1.5)))
    colors = ["#2ca02c" if s > 1.0 else "#d62728" for s in speedup]
    ax.barh(ys + bh / 2, speedup,   height=bh, color=colors,   alpha=0.8, label="All sequences")
    ax.barh(ys - bh / 2, speedup_t, height=bh, color="#9467bd", alpha=0.65, label="Top-10% only")
    ax.axvline(1.0, color="#333", lw=1.2, linestyle="--", alpha=0.7)

    ax.set_yticks(ys)
    ax.set_yticklabels(names, fontsize=8.5)
    ax.set_xlabel("Geometric-mean speedup  (>1 = pass helps)")
    ax.set_title("Per-Pass Impact: median speedup when pass is present vs absent")
    ax.legend(fontsize=8)
    ax.invert_yaxis()
    savefig(fig, out / "pass_impact.png")


# ── 5. Pass co-occurrence heatmap ────────────────────────────────────────────

def plot_cooccurrence(data: list, out: Path):
    """Heatmap of lift values for the most synergistic pass pairs."""
    if not data or len(data) < 4:
        return

    data = sorted(data, key=lambda x: -x["lift"])[:40]

    # Collect unique passes
    passes = []
    seen = set()
    for d in data:
        for p in (d["pass_a"], d["pass_b"]):
            if p not in seen:
                passes.append(p)
                seen.add(p)
    passes = passes[:20]  # cap matrix size

    idx = {p: i for i, p in enumerate(passes)}
    mat = np.ones((len(passes), len(passes)))
    for d in data:
        a, b = d["pass_a"], d["pass_b"]
        if a in idx and b in idx:
            v = d["lift"]
            mat[idx[a], idx[b]] = v
            mat[idx[b], idx[a]] = v

    fig, ax = plt.subplots(figsize=(max(7, len(passes) * 0.55 + 2),
                                    max(6, len(passes) * 0.55 + 1.5)))
    vmax = np.percentile(mat[mat > 1], 95) if (mat > 1).any() else 2.0
    im = ax.imshow(mat, cmap="YlOrRd", aspect="auto", vmin=1.0, vmax=vmax)
    plt.colorbar(im, ax=ax, label="Lift (P(A∩B) / P(A)·P(B))")

    ax.set_xticks(range(len(passes)))
    ax.set_yticks(range(len(passes)))
    ax.set_xticklabels(passes, rotation=45, ha="right", fontsize=7.5)
    ax.set_yticklabels(passes, fontsize=7.5)
    ax.set_title("Pass Co-occurrence Lift in Top-10% Sequences")
    savefig(fig, out / "pass_cooccurrence.png")


# ── 6. IR feature heatmap ────────────────────────────────────────────────────

FEATURE_KEYS = [
    ("add_count",             "add"),
    ("mul_count",             "mul"),
    ("load_count",            "load"),
    ("store_count",           "store"),
    ("br_count",              "br"),
    ("call_count",            "call"),
    ("phi_count",             "phi"),
    ("alloca_count",          "alloca"),
    ("gep_count",             "gep"),
    ("icmp_count",            "icmp"),
    ("fcmp_count",            "fcmp"),
    ("ret_count",             "ret"),
    ("other_inst_count",      "other"),
    ("basic_block_count",     "bb"),
    ("total_instruction_count","insts"),
    ("function_count",        "fns"),
    ("loop_depth_approx",     "loops"),
    ("load_store_ratio",      "ld/st"),
]


def plot_ir_heatmap(features: list, out: Path):
    """Heatmap of z-scored IR features sorted by gap_vs_o3."""
    if not features:
        return

    features = sorted(features, key=lambda x: x.get("gap_vs_o3_pct", 0.0))
    names  = [f["function"]    for f in features]
    tiers  = [f["difficulty"]  for f in features]
    gaps   = [f.get("gap_vs_o3_pct", 0.0) for f in features]
    keys   = [k for k, _ in FEATURE_KEYS]
    labels = [l for _, l in FEATURE_KEYS]

    mat = np.array([[f.get(k, 0.0) for k in keys] for f in features], dtype=float)

    # z-score per column
    means = mat.mean(axis=0)
    stds  = mat.std(axis=0)
    stds[stds < 1e-8] = 1.0
    mat_z = (mat - means) / stds

    n_rows, n_cols = mat_z.shape
    fig_h = max(5, n_rows * 0.38 + 2.5)
    fig_w = max(9, n_cols * 0.55 + 3.5)
    fig, ax = plt.subplots(figsize=(fig_w, fig_h))

    vabs = min(3.0, np.abs(mat_z).max())
    im = ax.imshow(mat_z, cmap="RdBu_r", aspect="auto", vmin=-vabs, vmax=vabs)
    plt.colorbar(im, ax=ax, label="z-score", fraction=0.03, pad=0.02)

    # annotate cells with z > 0.5 abs
    for i in range(n_rows):
        for j in range(n_cols):
            v = mat_z[i, j]
            if abs(v) > 0.5:
                tc = "white" if abs(v) > 1.5 else "black"
                ax.text(j, i, f"{v:.1f}", ha="center", va="center",
                        fontsize=6.5, color=tc)

    ax.set_xticks(range(n_cols))
    ax.set_xticklabels(labels, rotation=45, ha="right", fontsize=8)
    ax.set_yticks(range(n_rows))

    # y-tick labels: colour by difficulty tier
    ax.set_yticklabels(
        [f"{n}  ({g:+.0f}%)" for n, g in zip(names, gaps)],
        fontsize=7.5
    )
    for ytick, tier in zip(ax.get_yticklabels(), tiers):
        ytick.set_color(TIER_COLORS.get(tier, "#333"))

    ax.set_title("IR Feature Profiles (z-scored, sorted by gap vs O3 — colour = difficulty tier)")
    savefig(fig, out / "ir_features_heatmap.png")


# ── 7. Feature-performance correlations ─────────────────────────────────────

def plot_feature_correlations(corr: list, out: Path):
    """Bar chart of Pearson r between each IR feature and gap_vs_o3."""
    corr = sorted(corr, key=lambda x: x["pearson_r"])
    if not corr:
        return

    names = [c["feature"]   for c in corr]
    rs    = [c["pearson_r"] for c in corr]
    colors = ["#d62728" if r > 0 else "#1f77b4" for r in rs]

    fig, ax = plt.subplots(figsize=(8, max(4, len(names) * 0.45 + 1.5)))
    ys = np.arange(len(names))
    ax.barh(ys, rs, color=colors, alpha=0.8)
    ax.axvline(0, color="#333", lw=1.0)
    ax.set_yticks(ys)
    ax.set_yticklabels(names, fontsize=9)
    ax.set_xlabel("Pearson r with gap_vs_o3_pct")
    ax.set_title("IR Feature Correlation with Difficulty\n"
                 "(positive = harder benchmarks have more of this feature)")
    ax.invert_yaxis()

    # draw r values
    for i, r in enumerate(rs):
        ha = "left" if r >= 0 else "right"
        offset = 0.005 if r >= 0 else -0.005
        ax.text(r + offset, i, f"{r:+.3f}", va="center", ha=ha, fontsize=7.5)

    savefig(fig, out / "feature_correlations.png")


# ── 8. Sequence length by difficulty tier ───────────────────────────────────

def plot_seq_lengths(tiers: list, ceiling: list, out: Path):
    """Violin / strip of best sequence lengths, grouped by difficulty tier."""
    tier_map = {
        d["function"]: (
            "beats-O3" if d["gap_vs_o3_pct"] < 0 else
            "reachable" if d["gap_vs_o3_pct"] < 20 else
            "gap"       if d["gap_vs_o3_pct"] < 100 else
            "hard"
        )
        for d in ceiling
    }
    groups: dict[str, list] = {t: [] for t in TIER_ORDER}
    for d in ceiling:
        t = tier_map.get(d["function"], "hard")
        groups[t].append(d["best_seq_len"])

    active = [(t, groups[t]) for t in TIER_ORDER if groups[t]]
    if not active:
        return

    fig, ax = plt.subplots(figsize=(8, 5))
    positions = np.arange(len(active))
    for pos, (tier, vals) in enumerate(active):
        col = TIER_COLORS[tier]
        if len(vals) >= 4:
            parts = ax.violinplot([vals], positions=[pos], widths=0.6,
                                  showmedians=True, showextrema=False)
            for pc in parts["bodies"]:
                pc.set_facecolor(col)
                pc.set_alpha(0.6)
            parts["cmedians"].set_color(col)
            parts["cmedians"].set_linewidth(2)
        # jitter points
        jitter = np.random.default_rng(0).uniform(-0.15, 0.15, len(vals))
        ax.scatter(pos + jitter, vals, color=col, s=20, alpha=0.7, zorder=3)

    ax.set_xticks(positions)
    ax.set_xticklabels([t for t, _ in active])
    ax.set_ylabel("Best sequence length (# passes)")
    ax.set_title("Best-Found Sequence Length by Difficulty Tier")

    # annotate medians
    for pos, (_, vals) in enumerate(active):
        med = sorted(vals)[len(vals) // 2]
        ax.text(pos, med + 0.3, f"med={med}", ha="center", va="bottom", fontsize=8)

    savefig(fig, out / "seq_length_by_tier.png")


# ── 9. CV vs gap scatter ─────────────────────────────────────────────────────

def plot_cv_vs_gap(dist_data: list, ceiling_data: list, out: Path):
    """Scatter: coefficient of variation vs gap_vs_o3 per benchmark."""
    gap_map = {d["function"]: d["gap_vs_o3_pct"] for d in ceiling_data}
    tier_map = {
        d["function"]: (
            "beats-O3" if d["gap_vs_o3_pct"] < 0 else
            "reachable" if d["gap_vs_o3_pct"] < 20 else
            "gap"       if d["gap_vs_o3_pct"] < 100 else
            "hard"
        )
        for d in ceiling_data
    }

    xs, ys, cols, names = [], [], [], []
    for d in dist_data:
        fn = d["function"]
        if fn not in gap_map:
            continue
        xs.append(gap_map[fn])
        ys.append(d["cv_pct"])
        cols.append(TIER_COLORS.get(tier_map.get(fn, "unknown"), "#888"))
        names.append(fn)

    if not xs:
        return

    fig, ax = plt.subplots(figsize=(8, 5))
    ax.scatter(xs, ys, c=cols, s=55, alpha=0.85, edgecolors="white", linewidths=0.5)

    for x, y, name in zip(xs, ys, names):
        ax.annotate(name, (x, y), textcoords="offset points", xytext=(4, 3),
                    fontsize=7, alpha=0.8)

    ax.axvline(0, color="#333", lw=1.0, linestyle="--", alpha=0.5)
    ax.set_xlabel("gap_vs_o3_pct  (negative = beats O3)")
    ax.set_ylabel("Coefficient of variation  (%)")
    ax.set_title("Pass-Sensitivity vs Difficulty\n"
                 "(high CV = outcome strongly depends on pass choice)")

    patches = [mpatches.Patch(color=c, label=t)
               for t, c in TIER_COLORS.items() if t != "unknown"]
    ax.legend(handles=patches, fontsize=8, framealpha=0.9)
    savefig(fig, out / "cv_vs_gap.png")


# ── main ─────────────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) < 2:
        print("Usage: plot_eda.py <output_dir>", file=sys.stderr)
        sys.exit(1)

    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)

    ceiling  = load(out / "ceiling.json")      or []
    dist     = load(out / "distributions.json") or []
    enrich   = load(out / "pass_enrichment.json") or []
    impact   = load(out / "pass_impact.json")   or []
    cooccur  = load(out / "pass_cooccurrence.json") or []
    features = load(out / "ir_features.json")   or []
    corr     = load(out / "feature_correlations.json") or []
    seq_tiers = load(out / "seq_length_tiers.json") or []

    print(f"Generating plots in {out}/")

    if ceiling:
        plot_ceiling_gaps(ceiling, out)

    if dist and ceiling:
        plot_distributions(dist, ceiling, out)

    if enrich:
        plot_pass_enrichment(enrich, out)

    if impact:
        plot_pass_impact(impact, out)

    if cooccur:
        plot_cooccurrence(cooccur, out)

    if features:
        plot_ir_heatmap(features, out)

    if corr:
        plot_feature_correlations(corr, out)

    if ceiling:
        plot_seq_lengths(seq_tiers, ceiling, out)
        plot_cv_vs_gap(dist, ceiling, out)

    print("Done.")


if __name__ == "__main__":
    main()
