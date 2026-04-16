#!/usr/bin/env python3
"""Dataset stats plotter — dark/minimal."""
import argparse
import json
import sys
from pathlib import Path
from collections import Counter

import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np

parser = argparse.ArgumentParser()
parser.add_argument("--data", type=Path, default=Path("dataset.jsonl"))
args = parser.parse_args()

OUT = args.data.with_suffix(".png")

rows = []
with open(args.data) as f:
    for line in f:
        line = line.strip()
        if line:
            rows.append(json.loads(line))

if not rows:
    print("No data", file=sys.stderr)
    sys.exit(1)

# ── style ─────────────────────────────────────────────────────────────────────

BG    = "#0d0f14"
PANEL = "#13161e"
GRID  = "#1f2330"
TEXT  = "#c8ccd8"
ACCENT = "#7eb3ff"
WARN  = "#ff6b9d"
EMA_C = "#ffd166"

FUNC_PALETTE = {
    "array_reduction":  "#7eb3ff",
    "binary_tree":      "#ff8c69",
    "fft":              "#7ddc8b",
    "interpreter":      "#d49aff",
    "kmp_search":       "#ffd166",
    "polynomial_eval":  "#ff6b9d",
}

plt.rcParams.update({
    "figure.facecolor": BG,   "axes.facecolor":  PANEL,
    "axes.edgecolor":   GRID, "axes.labelcolor": TEXT,
    "axes.titlecolor":  TEXT, "xtick.color":     TEXT,
    "ytick.color":      TEXT, "grid.color":      GRID,
    "grid.linewidth":   0.6,  "text.color":      TEXT,
    "font.family":      "monospace", "font.size": 9,
    "axes.titlesize":   10,   "legend.facecolor": PANEL,
    "legend.edgecolor": GRID, "legend.fontsize":  8,
})

funcs = sorted(set(r["func_name"] for r in rows))
speedups_all = [r["speedup"] for r in rows]
seq_lens     = [len(r["passes"]) for r in rows]

# Flatten step deltas across all samples.
step_deltas_all = [d for r in rows for d in r["step_deltas"]]

fig = plt.figure(figsize=(16, 9), facecolor=BG)
fig.suptitle(f"Predictor Dataset  ({len(rows):,} samples, {len(funcs)} functions)", color=TEXT, fontsize=13, y=0.98)

gs = fig.add_gridspec(2, 3, hspace=0.45, wspace=0.38,
                      left=0.07, right=0.97, top=0.92, bottom=0.08)

ax_hist  = fig.add_subplot(gs[0, 0])
ax_viol  = fig.add_subplot(gs[0, 1])
ax_slen  = fig.add_subplot(gs[0, 2])
ax_delta = fig.add_subplot(gs[1, 0])
ax_scat  = fig.add_subplot(gs[1, 1])
ax_pass  = fig.add_subplot(gs[1, 2])

# ── 1. Speedup histogram (all) ────────────────────────────────────────────────
ax_hist.hist(speedups_all, bins=60, color=ACCENT, alpha=0.75, edgecolor="none")
ax_hist.axvline(0, color=WARN, linewidth=0.9, linestyle=":")
ax_hist.set_xlabel("speedup")
ax_hist.set_ylabel("count")
ax_hist.set_title("Speedup Distribution")
ax_hist.grid(True, axis="y")

# ── 2. Speedup violin per function ────────────────────────────────────────────
func_speedups = [
    [r["speedup"] for r in rows if r["func_name"] == fn]
    for fn in funcs
]
vp = ax_viol.violinplot(func_speedups, positions=range(len(funcs)),
                         showmedians=True, showextrema=False)
for body, fn in zip(vp["bodies"], funcs):
    body.set(facecolor=FUNC_PALETTE.get(fn, ACCENT), alpha=0.6)
vp["cmedians"].set(color=TEXT, linewidth=1.5)
ax_viol.axhline(0, color=WARN, linewidth=0.8, linestyle=":")
ax_viol.set_xticks(range(len(funcs)))
ax_viol.set_xticklabels([f.replace("_", "\n") for f in funcs], fontsize=7)
ax_viol.set_ylabel("speedup")
ax_viol.set_title("Per-function Speedup")
ax_viol.grid(True, axis="y")

# ── 3. Sequence length histogram ──────────────────────────────────────────────
ax_slen.hist(seq_lens, bins=range(1, max(seq_lens) + 2), color=EMA_C,
             alpha=0.75, edgecolor="none", align="left")
ax_slen.set_xlabel("sequence length (passes)")
ax_slen.set_ylabel("count")
ax_slen.set_title("Sequence Length Distribution")
ax_slen.grid(True, axis="y")
ax_slen.xaxis.set_major_locator(ticker.MaxNLocator(integer=True))

# ── 4. Step delta distribution ────────────────────────────────────────────────
ax_delta.hist(step_deltas_all, bins=80, color="#d49aff", alpha=0.75, edgecolor="none")
ax_delta.axvline(0, color=WARN, linewidth=0.9, linestyle=":")
ax_delta.set_xlabel("step delta (IR reduction fraction)")
ax_delta.set_ylabel("count")
ax_delta.set_title("Step Delta Distribution")
ax_delta.grid(True, axis="y")

# ── 5. Speedup vs sequence length scatter ─────────────────────────────────────
colors_scat = [FUNC_PALETTE.get(r["func_name"], ACCENT) for r in rows]
ax_scat.scatter(seq_lens, speedups_all, c=colors_scat, s=4, alpha=0.25)
ax_scat.axhline(0, color=WARN, linewidth=0.8, linestyle=":")
ax_scat.set_xlabel("sequence length")
ax_scat.set_ylabel("speedup")
ax_scat.set_title("Speedup vs Seq Length")
ax_scat.grid(True)

# ── 6. Pass frequency bar chart ────────────────────────────────────────────────
pass_counts: Counter = Counter()
for r in rows:
    for p in r["passes"]:
        pass_counts[p] += 1
top_passes = pass_counts.most_common(15)
pass_names = [p for p, _ in top_passes]
pass_vals  = [c for _, c in top_passes]
y = np.arange(len(pass_names))
ax_pass.barh(y, pass_vals, color=ACCENT, alpha=0.8)
ax_pass.set_yticks(y)
ax_pass.set_yticklabels(pass_names, fontsize=7)
ax_pass.set_xlabel("occurrences")
ax_pass.set_title("Top 15 Passes (all samples)")
ax_pass.grid(True, axis="x")

fig.savefig(OUT, dpi=150, bbox_inches="tight", facecolor=BG)
print(f"saved → {OUT}")
