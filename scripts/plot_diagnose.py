#!/usr/bin/env python3
"""Diagnose results plotter — dark/minimal."""
import argparse
import json
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np

parser = argparse.ArgumentParser()
parser.add_argument("--results", type=Path, default=Path("checkpoints/diagnose.json"))
args = parser.parse_args()

OUT = args.results.with_suffix(".png")

with open(args.results) as f:
    records = json.load(f)

if not records:
    print("No records in diagnose.json", file=sys.stderr)
    sys.exit(1)

# ── style ─────────────────────────────────────────────────────────────────────

BG    = "#0d0f14"
PANEL = "#13161e"
GRID  = "#1f2330"
TEXT  = "#c8ccd8"
WARN  = "#ff6b9d"

FUNC_PALETTE = {
    "array_reduction":  "#7eb3ff",
    "binary_tree":      "#ff8c69",
    "fft":              "#7ddc8b",
    "interpreter":      "#d49aff",
    "kmp_search":       "#ffd166",
    "polynomial_eval":  "#ff6b9d",
}
DEFAULT_COLOR = "#aaaaaa"

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

# Sort by measured mean descending for the bar chart.
records_sorted = sorted(records, key=lambda r: r["mean"], reverse=True)

labels  = [f"#{r['rank']} {r['func_name'].replace('_',' ')}" for r in records_sorted]
means   = [r["mean"]          for r in records_sorted]
stds    = [r["std"]           for r in records_sorted]
cached  = [r["cached_speedup"] for r in records_sorted]
colors  = [FUNC_PALETTE.get(r["func_name"], DEFAULT_COLOR) for r in records_sorted]

fig = plt.figure(figsize=(15, 9), facecolor=BG)
fig.suptitle("Diagnose — Top Sequence Re-benchmark", color=TEXT, fontsize=13, y=0.98)

gs = fig.add_gridspec(2, 2, hspace=0.45, wspace=0.35,
                      left=0.1, right=0.97, top=0.92, bottom=0.07)

ax_bar  = fig.add_subplot(gs[:, 0])   # tall left panel
ax_scat = fig.add_subplot(gs[0, 1])
ax_box  = fig.add_subplot(gs[1, 1])

# ── 1. Horizontal bar: mean ± std ─────────────────────────────────────────────
y = np.arange(len(records_sorted))
ax_bar.barh(y, means, xerr=stds, color=colors, alpha=0.85,
            error_kw=dict(ecolor=TEXT, alpha=0.5, linewidth=1.2), height=0.6)
ax_bar.axvline(0, color=WARN, linewidth=0.8, linestyle=":")
# Overlay cached speedup as a marker.
ax_bar.scatter(cached, y, color=TEXT, s=25, zorder=5, label="cached (IR frac)")
ax_bar.set_yticks(y)
ax_bar.set_yticklabels(labels, fontsize=8)
ax_bar.set_xlabel("speedup vs O3")
ax_bar.set_title("Mean ± Std  (bar) / Cached (dot)")
ax_bar.grid(True, axis="x")
ax_bar.legend(loc="lower right")

# ── 2. Scatter: cached vs measured mean ───────────────────────────────────────
ax_scat.scatter(cached, means, c=colors, s=55, alpha=0.9, zorder=3)
lo = min(min(cached), min(means)) - 0.05
hi = max(max(cached), max(means)) + 0.05
ax_scat.plot([lo, hi], [lo, hi], color=TEXT, linewidth=0.8, linestyle="--", alpha=0.4)
ax_scat.axhline(0, color=WARN, linewidth=0.6, linestyle=":")
ax_scat.axvline(0, color=WARN, linewidth=0.6, linestyle=":")
ax_scat.set_xlabel("cached speedup (IR fraction)")
ax_scat.set_ylabel("measured mean speedup")
ax_scat.set_title("Cached vs Measured")
ax_scat.grid(True)
ax_scat.set_facecolor(PANEL)

# ── 3. Box plot: per-sequence distribution ────────────────────────────────────
all_sp = [r["all_speedups"] for r in records_sorted]
short_labels = [f"#{r['rank']}" for r in records_sorted]
bp = ax_box.boxplot(
    all_sp,
    vert=True,
    patch_artist=True,
    tick_labels=short_labels,
    medianprops=dict(color=TEXT, linewidth=1.5),
    whiskerprops=dict(color=TEXT, linewidth=1.0),
    capprops=dict(color=TEXT, linewidth=1.0),
    flierprops=dict(marker=".", color=TEXT, alpha=0.4, markersize=4),
)
for patch, col in zip(bp["boxes"], colors):
    patch.set(facecolor=col, alpha=0.55)
ax_box.axhline(0, color=WARN, linewidth=0.6, linestyle=":")
ax_box.set_ylabel("speedup vs O3")
ax_box.set_title("Speedup Distribution per Sequence")
ax_box.grid(True, axis="y")
ax_box.set_facecolor(PANEL)

fig.savefig(OUT, dpi=150, bbox_inches="tight", facecolor=BG)
print(f"saved → {OUT}")
