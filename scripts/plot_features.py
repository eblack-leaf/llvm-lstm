#!/usr/bin/env python3
"""IR feature plotter — chunk deltas."""
import argparse
import json
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

parser = argparse.ArgumentParser()
parser.add_argument("--features", type=Path, default=Path("features.json"))
args = parser.parse_args()

OUT = args.features.with_suffix(".png")

with open(args.features) as f:
    records = json.load(f)

if not records:
    print("No records", file=sys.stderr)
    sys.exit(1)

if "deltas" not in records[0]:
    print("features.json is old format — re-run export-features", file=sys.stderr)
    sys.exit(1)

CATS = ["memory", "int_arith", "float_arith", "bitwise",
        "compare", "cast", "call", "control",
        "phi_select", "vector", "block", "other"]
N_CAT   = len(CATS)
funcs   = [r["name"] for r in records]
n_funcs = len(funcs)
k       = records[0]["ir_chunks"]
n_deltas = k - 1

hists  = np.array([r["histogram"] for r in records]).reshape(n_funcs, k, N_CAT)
deltas = np.array([r["deltas"]    for r in records]).reshape(n_funcs, n_deltas, N_CAT)

FUNC_PALETTE = ["#7eb3ff", "#ff8c69", "#7ddc8b", "#d49aff", "#ffd166", "#ff6b9d"]

BG    = "#0d0f14"
PANEL = "#13161e"
GRID  = "#1f2330"
TEXT  = "#c8ccd8"

plt.rcParams.update({
    "figure.facecolor": BG,   "axes.facecolor":  PANEL,
    "axes.edgecolor":   GRID, "axes.labelcolor": TEXT,
    "axes.titlecolor":  TEXT, "xtick.color":     TEXT,
    "ytick.color":      TEXT, "text.color":      TEXT,
    "font.family":      "monospace", "font.size": 8,
    "axes.titlesize":   9,    "legend.facecolor": PANEL,
    "legend.edgecolor": GRID, "legend.fontsize":  7,
    "grid.color":       GRID, "grid.linewidth":   0.6,
})

fig, axes = plt.subplots(1, n_deltas, figsize=(5 * n_deltas, 5), facecolor=BG)
if n_deltas == 1:
    axes = [axes]
fig.suptitle("IR Chunk Deltas — per-function category shift", color=TEXT, fontsize=12, y=1.02)

vmax = np.abs(deltas).max()

for d, ax in enumerate(axes):
    data = deltas[:, d, :]   # (n_funcs, N_CAT)
    im = ax.imshow(data, aspect="auto", cmap="RdBu_r", vmin=-vmax, vmax=vmax)
    ax.set_xticks(range(N_CAT))
    ax.set_xticklabels(CATS, rotation=45, ha="right", fontsize=7.5)
    ax.set_yticks(range(n_funcs))
    ax.set_yticklabels(funcs if d == 0 else [], fontsize=8)
    ax.set_title(f"chunk {d} → {d+1}")
    for fi in range(n_funcs):
        for ci in range(N_CAT):
            v = data[fi, ci]
            if abs(v) > 0.02:
                ax.text(ci, fi, f"{v:+.2f}", ha="center", va="center",
                        fontsize=6, color="white" if abs(v) > vmax * 0.6 else TEXT)

fig.colorbar(im, ax=axes[-1], fraction=0.03, pad=0.02, label="category weight change")

plt.tight_layout()
fig.savefig(OUT, dpi=150, bbox_inches="tight", facecolor=BG)
print(f"saved → {OUT}")
