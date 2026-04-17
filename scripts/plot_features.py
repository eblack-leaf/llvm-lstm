#!/usr/bin/env python3
"""IR feature plotter — opcode chunk deltas + metadata reference deltas."""
import argparse
import json
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import matplotlib.gridspec as gridspec
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

if "op_deltas" not in records[0]:
    print("features.json is old format — re-run export-features", file=sys.stderr)
    sys.exit(1)

OP_CATS   = ["memory", "int_arith", "float_arith", "bitwise",
             "compare", "cast", "call", "control",
             "phi_select", "vector", "block", "other"]
META_CATS = ["tbaa", "loop", "alias_scope", "noalias"]
N_OP   = len(OP_CATS)
N_META = len(META_CATS)

funcs   = [r["name"] for r in records]
n_funcs = len(funcs)
k       = records[0]["ir_chunks"]
n_d     = k - 1

op_deltas   = np.array([r["op_deltas"]   for r in records]).reshape(n_funcs, n_d, N_OP)
meta_deltas = np.array([r["meta_deltas"] for r in records]).reshape(n_funcs, n_d, N_META)

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
    "legend.edgecolor": GRID,
})

OP_THRESH   = 0.02
META_THRESH = 0.005
op_vmax   = np.abs(op_deltas).max()
meta_vmax = np.abs(meta_deltas).max()

cmap = plt.get_cmap("RdBu_r").copy()
cmap.set_bad(color=PANEL)   # masked (near-zero) cells show as background

def draw_heatmap(ax, raw, vmax, thresh, labels, show_y, fmt):
    """Plot masked heatmap; annotate every visible cell with luminance-aware text."""
    masked = np.ma.masked_where(np.abs(raw) < thresh, raw)
    im = ax.imshow(masked, aspect="auto", cmap=cmap, vmin=-vmax, vmax=vmax)
    ax.set_facecolor(PANEL)
    ax.set_xticks(range(raw.shape[1]))
    ax.set_xticklabels(labels, rotation=45, ha="right", fontsize=7)
    ax.set_yticks(range(raw.shape[0]))
    ax.set_yticklabels(funcs if show_y else [], fontsize=8)
    # Annotate every cell that passes the threshold.
    for fi in range(raw.shape[0]):
        for ci in range(raw.shape[1]):
            v = raw[fi, ci]
            if abs(v) < thresh:
                continue
            # Luminance of the cell colour to pick readable text.
            r_, g_, b_, _ = cmap((v + vmax) / (2 * vmax))
            lum = 0.299 * r_ + 0.587 * g_ + 0.114 * b_
            txt = "#0d0f14" if lum > 0.45 else "#f0f2f8"
            ax.text(ci, fi, fmt.format(v), ha="center", va="center",
                    fontsize=8, fontweight="bold", color=txt)
    return im

fig = plt.figure(figsize=(5 * n_d, 9), facecolor=BG)
fig.suptitle("IR Chunk Deltas — opcode (top) · metadata refs (bottom)",
             color=TEXT, fontsize=12, y=1.01)

gs = gridspec.GridSpec(2, n_d, figure=fig, hspace=0.45, wspace=0.12,
                       top=0.94, bottom=0.08, left=0.10, right=0.93)

op_axes   = [fig.add_subplot(gs[0, d]) for d in range(n_d)]
meta_axes = [fig.add_subplot(gs[1, d]) for d in range(n_d)]

for d, ax in enumerate(op_axes):
    im_op = draw_heatmap(ax, op_deltas[:, d, :], op_vmax, OP_THRESH,
                         OP_CATS, d == 0, "{:+.2f}")
    ax.set_title(f"chunk {d}→{d+1}")

fig.colorbar(im_op, ax=op_axes[-1], fraction=0.04, pad=0.03, label="opcode Δ")

for d, ax in enumerate(meta_axes):
    im_meta = draw_heatmap(ax, meta_deltas[:, d, :], meta_vmax, META_THRESH,
                           [f"!{c}" for c in META_CATS], d == 0, "{:+.3f}")

fig.colorbar(im_meta, ax=meta_axes[-1], fraction=0.04, pad=0.03, label="meta ref rate Δ")

fig.savefig(OUT, dpi=150, bbox_inches="tight", facecolor=BG)
print(f"saved → {OUT}")
