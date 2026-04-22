#!/usr/bin/env python3
import argparse, json
from pathlib import Path
import matplotlib; matplotlib.use("Agg")
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
import numpy as np

parser = argparse.ArgumentParser()
parser.add_argument("--input",    type=Path, required=True)
parser.add_argument("--output",   type=Path, required=True)
parser.add_argument("--show-all", action="store_true")
args = parser.parse_args()

with open(args.input) as f:
    records = json.load(f)

Y_MIN = -0.38

has_samp = "sample_best_speedup" in records[0]
cols = (["O0", "O1", "O2", "rand_mean", "rand_best", "greedy"] if args.show_all
        else ["O2", "rand_mean", "rand_best", "greedy"])
if has_samp:
    cols.append("samp_best")

keys   = {"O0": "o0_speedup", "O1": "o1_speedup", "O2": "o2_speedup",
          "rand_mean": "random_mean_speedup", "rand_best": "random_best_speedup",
          "greedy": "greedy_speedup", "samp_best": "sample_best_speedup"}
colors = {"O0": "#aaaaaa", "O1": "#888888", "O2": "#555555",
          "rand_mean": "#e07b39", "rand_best": "#e8b84b",
          "greedy": "#4c8be0", "samp_best": "#3ab87d"}
labels = {"O0": "O0", "O1": "O1", "O2": "O2",
          "rand_mean": "rand (mean)", "rand_best": "rand (best)",
          "greedy": "greedy", "samp_best": "sample (best)"}

funcs   = [r["name"].replace("_", " ") for r in records]
n       = len(funcs)
x       = np.arange(n)
ncols   = len(cols)
width   = 0.72 / ncols
offsets = np.linspace(-(ncols-1)/2, (ncols-1)/2, ncols) * width

fig, ax = plt.subplots(figsize=(max(7.0, n * 0.55), 3.8))
fig.subplots_adjust(left=0.10, right=0.78, top=0.88, bottom=0.22)

# Alternating background bands to separate function groups.
for idx in range(n):
    if idx % 2 == 0:
        ax.axvspan(idx - 0.5, idx + 0.5, color="#f0f0f0", zorder=0)

for i, col in enumerate(cols):
    vals = [r[keys[col]] for r in records]
    ax.bar(x + offsets[i], vals, width=width, color=colors[col],
           label=labels[col], edgecolor="white", linewidth=0.3, zorder=3,
           clip_on=True)

ax.axhline(0, color="#333333", linewidth=0.8, zorder=4)
ax.text(n - 0.45, 0.005, "-O3", fontsize=7, color="#333333", va="bottom")
ax.set_ylim(Y_MIN, 0.36)
ax.set_xticks(x)

# 3-level vertical stagger: tick labels sit at three offsets so
# adjacent names never overlap, no rotation needed.
LEVELS = 3
STEP   = 10   # points per level
ax.set_xticklabels([])   # hide default tick labels
for idx, name in enumerate(funcs):
    offset_pts = (idx % LEVELS) * STEP
    ax.annotate(name, xy=(idx, 0), xycoords=("data", "axes fraction"),
                xytext=(0, -(18 + offset_pts)), textcoords="offset points",
                ha="center", va="top", fontsize=8, annotation_clip=False)

ax.set_ylabel("Speedup vs -O3", fontsize=9)
ax.set_title("Policy evaluation — speedup relative to -O3  (positive = faster)", fontsize=9)
ax.yaxis.grid(True, linestyle="--", linewidth=0.5, alpha=0.5, zorder=0)
ax.set_axisbelow(True)
ax.spines[["top", "right"]].set_visible(False)
ax.legend(handles=[mpatches.Patch(color=colors[c], label=labels[c]) for c in cols],
          loc="upper left", bbox_to_anchor=(1.01, 1.0),
          frameon=True, framealpha=0.9, fontsize=8,
          title="method", title_fontsize=8)

fig.savefig(args.output, dpi=200, bbox_inches="tight")
print(f"Saved -> {args.output}")
