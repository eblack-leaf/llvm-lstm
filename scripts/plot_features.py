#!/usr/bin/env python3
"""IR feature plotter — two images: opcode chunk deltas and metadata ref deltas."""
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

stem = args.features.with_suffix("")
OUT_OP   = Path(str(stem) + "_op.png")
OUT_META = Path(str(stem) + "_meta.png")

with open(args.features) as f:
    records = json.load(f)

if not records:
    print("No records", file=sys.stderr)
    sys.exit(1)
if "op_deltas" not in records[0]:
    print("features.json is old format — re-run export-features", file=sys.stderr)
    sys.exit(1)

ALL_OP_CATS   = ["memory", "int_arith", "float_arith", "bitwise",
                 "compare", "cast", "call", "control",
                 "phi_select", "vector", "block", "other"]
ALL_META_CATS = ["tbaa", "loop", "alias_scope", "noalias"]
N_OP_ALL   = len(ALL_OP_CATS)
N_META_ALL = len(ALL_META_CATS)

funcs   = [r["name"] for r in records]
n_funcs = len(funcs)
k       = records[0]["ir_chunks"]
n_d     = k - 1

op_deltas   = np.array([r["op_deltas"]   for r in records]).reshape(n_funcs, n_d, N_OP_ALL)
meta_deltas = np.array([r["meta_deltas"] for r in records]).reshape(n_funcs, n_d, N_META_ALL)

OP_THRESH   = 0.02
META_THRESH = 0.005

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
    "axes.titlesize":   9,
})

cmap = plt.get_cmap("RdBu_r").copy()
cmap.set_bad(color="#252830")   # dim grey = zero/empty

def draw_panel(ax, raw, vmax, thresh, col_labels, row_labels):
    masked = np.ma.masked_where(np.abs(raw) < thresh, raw)
    im = ax.imshow(masked, aspect="auto", cmap=cmap, vmin=-vmax, vmax=vmax)
    ax.set_facecolor(PANEL)
    # Minor grid for cell borders.
    ax.set_xticks(np.arange(-0.5, raw.shape[1], 1), minor=True)
    ax.set_yticks(np.arange(-0.5, raw.shape[0], 1), minor=True)
    ax.grid(which="minor", color=BG, linewidth=1.0)
    ax.tick_params(which="minor", length=0)
    ax.set_xticks(range(raw.shape[1]))
    ax.set_xticklabels(col_labels, rotation=45, ha="right", fontsize=8)
    ax.set_yticks(range(raw.shape[0]))
    ax.set_yticklabels(row_labels, fontsize=max(5, min(9, int(200 / n_funcs))))
    # Annotate visible cells.
    ann_fs = max(5, min(8, int(170 / n_funcs)))
    fmt = "{:+.2f}" if vmax >= 0.05 else "{:+.3f}"
    for fi in range(raw.shape[0]):
        for ci in range(raw.shape[1]):
            v = raw[fi, ci]
            if abs(v) < thresh:
                continue
            r_, g_, b_, _ = cmap((v + vmax) / (2 * vmax))
            lum = 0.299 * r_ + 0.587 * g_ + 0.114 * b_
            txt = "#0d0f14" if lum > 0.45 else "#f0f2f8"
            ax.text(ci, fi, fmt.format(v), ha="center", va="center",
                    fontsize=ann_fs, fontweight="bold", color=txt)
    return im

def make_figure(data, vmax, thresh, all_col_cats, col_fmt, title, out_path):
    """Save one figure: n_d panels side by side."""
    n_cats = data.shape[2]
    # Width: label column + n_d panels, each wide enough for its categories.
    panel_w = max(2.5, n_cats * 0.55)
    fig_w   = min(18, panel_w * n_d + 1.8)
    # Height: enough rows to be readable.
    row_h   = max(0.28, min(0.55, 10.0 / n_funcs))
    fig_h   = min(16, max(4, n_funcs * row_h + 2.0))

    fig, axes = plt.subplots(1, n_d, figsize=(fig_w, fig_h),
                             gridspec_kw={"wspace": 0.08},
                             facecolor=BG)
    if n_d == 1:
        axes = [axes]
    fig.suptitle(title, color=TEXT, fontsize=11, y=0.995)

    for d, ax in enumerate(axes):
        row_labels = [r["name"] for r in records] if d == 0 else []
        im = draw_panel(ax, data[:, d, :], vmax, thresh,
                        [col_fmt.format(c) for c in all_col_cats], row_labels)
        ax.set_title(f"chunk {d}→{d+1}")

    fig.colorbar(im, ax=axes[-1], fraction=0.035, pad=0.04)
    fig.savefig(out_path, dpi=150, bbox_inches="tight", facecolor=BG)
    plt.close(fig)
    print(f"saved → {out_path}")

op_vmax   = max(np.abs(op_deltas).max(), 1e-6)
meta_vmax = max(np.abs(meta_deltas).max(), 1e-6)

make_figure(op_deltas,   op_vmax,   OP_THRESH,   ALL_OP_CATS,
            "{}",        "Opcode chunk deltas",        OUT_OP)
make_figure(meta_deltas, meta_vmax, META_THRESH, ALL_META_CATS,
            "!{}",       "Metadata ref rate deltas",   OUT_META)
