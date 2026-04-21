#!/usr/bin/env python3
"""Evaluation results plotter — per-function bar chart comparing baselines vs policy."""
import argparse
import json
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
import numpy as np

parser = argparse.ArgumentParser()
parser.add_argument("--input",  type=Path, required=True)
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()

with open(args.input) as f:
    records = json.load(f)

if not records:
    print("Empty eval JSON", file=sys.stderr)
    sys.exit(1)

# ── style ──────────────────────────────────────────────────────────────────
BG    = "#0d0f14"
PANEL = "#13161e"
GRID  = "#1f2330"
TEXT  = "#c8ccd8"

plt.rcParams.update({
    "figure.facecolor": BG,
    "axes.facecolor":   PANEL,
    "axes.edgecolor":   GRID,
    "axes.labelcolor":  TEXT,
    "axes.titlecolor":  TEXT,
    "xtick.color":      TEXT,
    "ytick.color":      TEXT,
    "text.color":       TEXT,
    "grid.color":       GRID,
    "grid.linestyle":   "--",
    "grid.linewidth":   0.5,
    "font.size":        9,
})

COLORS = {
    "O0":        "#555566",
    "O1":        "#7a7a99",
    "O2":        "#9999cc",
    "rand_mean": "#ff8c69",
    "rand_best": "#ffb347",
    "greedy":    "#7eb3ff",
    "samp_best": "#7ddc8b",
    "beam":      "#d49aff",
}

funcs    = [r["name"] for r in records]
n        = len(funcs)
x        = np.arange(n)

# Detect which optional columns exist.
has_samp = "sample_best_speedup" in records[0]
has_beam = "beam_speedup" in records[0]

cols = ["O0", "O1", "O2", "rand_mean", "rand_best", "greedy"]
keys = {
    "O0":        "o0_speedup",
    "O1":        "o1_speedup",
    "O2":        "o2_speedup",
    "rand_mean": "random_mean_speedup",
    "rand_best": "random_best_speedup",
    "greedy":    "greedy_speedup",
}
if has_samp:
    cols.append("samp_best")
    keys["samp_best"] = "sample_best_speedup"
if has_beam:
    cols.append("beam")
    keys["beam"] = "beam_speedup"

ncols   = len(cols)
width   = 0.8 / ncols
offsets = np.linspace(-(ncols - 1) / 2, (ncols - 1) / 2, ncols) * width

fig, axes = plt.subplots(2, 1, figsize=(max(10, n * 1.4), 9),
                         gridspec_kw={"height_ratios": [3, 1]})
fig.subplots_adjust(hspace=0.35)

ax_bar, ax_pass = axes

# ── bar chart ──────────────────────────────────────────────────────────────
for i, col in enumerate(cols):
    vals = [r[keys[col]] for r in records]
    bars = ax_bar.bar(x + offsets[i], vals, width=width,
                      color=COLORS[col], alpha=0.85, label=col, zorder=3)
    for bar, v in zip(bars, vals):
        if abs(v) > 0.005:
            ax_bar.text(bar.get_x() + bar.get_width() / 2,
                        bar.get_height() + (0.003 if v >= 0 else -0.012),
                        f"{v:+.3f}",
                        ha="center", va="bottom" if v >= 0 else "top",
                        fontsize=6.5, color=TEXT, alpha=0.8)

ax_bar.axhline(0, color=TEXT, linewidth=0.6, alpha=0.4)
ax_bar.set_xticks(x)
ax_bar.set_xticklabels(funcs, rotation=15, ha="right")
ax_bar.set_ylabel("Speedup vs O3")
ax_bar.set_title("Evaluation — speedup vs O3 (positive = faster)")
ax_bar.legend(loc="upper right", framealpha=0.3, fontsize=8)
ax_bar.grid(axis="y", zorder=0)
ax_bar.set_facecolor(PANEL)

# ── pass sequence table ────────────────────────────────────────────────────
ax_pass.set_facecolor(PANEL)
ax_pass.set_xlim(-0.5, n - 0.5)
ax_pass.set_ylim(-0.5, 0.5)
ax_pass.set_yticks([])
ax_pass.set_xticks(x)
ax_pass.set_xticklabels(funcs, rotation=15, ha="right")
ax_pass.set_title("Greedy policy pass sequence", pad=4)

for i, r in enumerate(records):
    passes = r.get("greedy_passes", [])
    seq    = " → ".join(passes) if passes else "(stop immediately)"
    ax_pass.text(i, 0, seq, ha="center", va="center",
                 fontsize=6, color=TEXT, alpha=0.85,
                 wrap=True, bbox=dict(boxstyle="round,pad=0.2",
                                      facecolor=GRID, alpha=0.5, linewidth=0))

# ── save ───────────────────────────────────────────────────────────────────
fig.savefig(args.output, dpi=150, bbox_inches="tight", facecolor=BG)
print(f"Saved → {args.output}")
