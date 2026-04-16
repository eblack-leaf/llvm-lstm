#!/usr/bin/env python3
"""BenchNoise results plotter — dark/minimal, report-ready."""
import argparse
import json
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np

parser = argparse.ArgumentParser()
parser.add_argument("--results", type=Path, default=Path("checkpoints/bench_noise.json"))
args = parser.parse_args()

OUT = args.results.with_suffix(".png")

with open(args.results) as f:
    data = json.load(f)

solo_ns     = data["solo_ns"]
parallel_ns = np.array(data["parallel_ns"], dtype=float)
workers     = data["workers"]
runs        = data["runs"]
iters       = data["iters"]
source      = Path(data.get("source", "")).name

# ── derived stats ─────────────────────────────────────────────────────────────
par_mean   = parallel_ns.mean()
par_std    = parallel_ns.std()
par_median = np.median(parallel_ns)
par_min    = parallel_ns.min()
par_max    = parallel_ns.max()
contention = par_mean / solo_ns          # ratio (1.0 = no overhead)
spread_pct = (par_max - par_min) / solo_ns * 100.0

# ── style ─────────────────────────────────────────────────────────────────────

BG     = "#0d0f14"
PANEL  = "#13161e"
GRID   = "#1f2330"
TEXT   = "#c8ccd8"
ACCENT = "#7eb3ff"
WARN   = "#ff6b9d"
EMA_C  = "#ffd166"

plt.rcParams.update({
    "figure.facecolor": BG,   "axes.facecolor":  PANEL,
    "axes.edgecolor":   GRID, "axes.labelcolor": TEXT,
    "axes.titlecolor":  TEXT, "xtick.color":     TEXT,
    "ytick.color":      TEXT, "grid.color":      GRID,
    "grid.linewidth":   0.6,  "text.color":      TEXT,
    "font.family":      "monospace", "font.size": 9,
    "axes.titlesize":   10,   "legend.facecolor": PANEL,
    "legend.edgecolor": GRID, "legend.fontsize":  8,
    "lines.linewidth":  1.6,
})

fig = plt.figure(figsize=(13, 7), facecolor=BG)
fig.suptitle(
    f"Benchmark Noise — {source}  ({workers} workers, {runs} runs × {iters} iters)",
    color=TEXT, fontsize=12, y=0.98
)

gs = fig.add_gridspec(1, 3, hspace=0.3, wspace=0.38,
                      left=0.08, right=0.97, top=0.90, bottom=0.12)

ax_bar  = fig.add_subplot(gs[0])
ax_dist = fig.add_subplot(gs[1])
ax_cont = fig.add_subplot(gs[2])

# ── 1. Bar: solo vs parallel mean ─────────────────────────────────────────────
categories = ["solo\n(serial)", f"parallel\n({workers} workers)"]
values     = [solo_ns, par_mean]
bar_colors = [ACCENT, WARN]
bars = ax_bar.bar(categories, values, color=bar_colors, alpha=0.8, width=0.5)
ax_bar.errorbar(1, par_mean, yerr=par_std,
                fmt="none", ecolor=TEXT, elinewidth=1.5, capsize=6)
# Annotate
for bar, val in zip(bars, values):
    ax_bar.text(bar.get_x() + bar.get_width() / 2, val * 1.01,
                f"{val/1e6:.3f} ms", ha="center", va="bottom", fontsize=8)
ax_bar.set_ylabel("mean time (ns)")
ax_bar.set_title("Solo vs Parallel Mean")
ax_bar.grid(True, axis="y")
ax_bar.yaxis.set_major_formatter(ticker.FuncFormatter(lambda x, _: f"{x/1e6:.1f}ms"))

# ── 2. Distribution of parallel measurements ──────────────────────────────────
ax_dist.hist(parallel_ns / 1e6, bins=max(6, workers // 2),
             color=WARN, alpha=0.75, edgecolor="none")
ax_dist.axvline(solo_ns / 1e6, color=ACCENT, linewidth=1.2,
                linestyle="--", label="solo")
ax_dist.axvline(par_mean / 1e6, color=EMA_C, linewidth=1.2,
                linestyle="--", label="par mean")
ax_dist.set_xlabel("time (ms)")
ax_dist.set_ylabel("workers")
ax_dist.set_title("Parallel Measurement Distribution")
ax_dist.grid(True, axis="y")
ax_dist.legend()

# ── 3. Contention summary ─────────────────────────────────────────────────────
metrics = {
    "contention\nratio":   contention,
    "spread\n% of solo":  spread_pct / 100.0,   # normalise for same axis
}
metric_labels = list(metrics.keys())
metric_vals   = list(metrics.values())
bar_cols = [WARN if v > 1.05 else ACCENT for v in [contention, spread_pct / 100.0]]
ax_cont.bar(metric_labels, metric_vals, color=bar_cols, alpha=0.8, width=0.4)
ax_cont.axhline(1.0, color=TEXT, linewidth=0.7, linestyle=":", alpha=0.5)
for i, (label, val) in enumerate(zip(metric_labels, [contention, spread_pct])):
    display = f"{contention:.3f}×" if i == 0 else f"{spread_pct:.1f}%"
    ax_cont.text(i, metric_vals[i] + 0.01, display,
                 ha="center", va="bottom", fontsize=9)
ax_cont.set_ylabel("ratio / normalised spread")
ax_cont.set_title("Contention Metrics")
ax_cont.grid(True, axis="y")

# Text summary for report
summary = (
    f"solo={solo_ns/1e6:.3f}ms  par_mean={par_mean/1e6:.3f}ms  "
    f"std={par_std/1e6:.3f}ms\n"
    f"contention={contention:.3f}×  spread={spread_pct:.1f}%  "
    f"workers={workers}"
)
fig.text(0.5, 0.02, summary, ha="center", color=TEXT, fontsize=8, alpha=0.7)

fig.savefig(OUT, dpi=150, bbox_inches="tight", facecolor=BG)
print(f"saved → {OUT}")
