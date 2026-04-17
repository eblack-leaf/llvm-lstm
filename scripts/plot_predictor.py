#!/usr/bin/env python3
"""Predictor training diagnostics — dark/minimal."""
import argparse
import json
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np

parser = argparse.ArgumentParser()
parser.add_argument("--log", type=Path,
                    default=Path("predictor_checkpoints/train.jsonl"))
args = parser.parse_args()

OUT = args.log.with_suffix(".png")

rows = []
with open(args.log) as f:
    for line in f:
        line = line.strip()
        if line:
            rows.append(json.loads(line))

if not rows:
    print("No data", file=sys.stderr)
    sys.exit(1)

import pandas as pd
df = pd.DataFrame(rows)
ep = df["epoch"]
best_epoch = df[df["is_best"]]["epoch"].max()

BG    = "#0d0f14"
PANEL = "#13161e"
GRID  = "#1f2330"
TEXT  = "#c8ccd8"
TRAIN = "#7eb3ff"
VAL   = "#ffd166"
WARN  = "#ff6b9d"

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

def style(ax, title, ylabel=""):
    ax.set_title(title, pad=5)
    ax.set_xlabel("epoch", labelpad=3)
    ax.set_ylabel(ylabel, labelpad=3)
    ax.grid(True, axis="y")
    ax.set_xlim(ep.min(), ep.max())
    ax.xaxis.set_major_locator(ticker.MaxNLocator(integer=True, nbins=6))
    ax.axvline(best_epoch, color=VAL, linewidth=1.0, linestyle="--", alpha=0.5, label=f"best (e{best_epoch})")

fig = plt.figure(figsize=(16, 8), facecolor=BG)
fig.suptitle("Predictor Training", color=TEXT, fontsize=13, y=0.98)

gs = fig.add_gridspec(2, 3, hspace=0.48, wspace=0.35,
                      left=0.06, right=0.97, top=0.92, bottom=0.08)

ax_rmse  = fig.add_subplot(gs[0, 0])
ax_r2    = fig.add_subplot(gs[0, 1])
ax_bias  = fig.add_subplot(gs[0, 2])
ax_mae   = fig.add_subplot(gs[1, 0])
ax_gap   = fig.add_subplot(gs[1, 1])
ax_pstd  = fig.add_subplot(gs[1, 2])

# ── RMSE ──────────────────────────────────────────────────────────────────────
ax_rmse.plot(ep, df["tr_rmse"], color=TRAIN, label="train")
ax_rmse.plot(ep, df["va_rmse"], color=VAL,   label="val",  linestyle="--")
ax_rmse.legend()
style(ax_rmse, "RMSE", "rmse")

# ── R² ────────────────────────────────────────────────────────────────────────
ax_r2.plot(ep, df["tr_r2"], color=TRAIN, label="train")
ax_r2.plot(ep, df["va_r2"], color=VAL,   label="val",  linestyle="--")
ax_r2.axhline(0, color=WARN, linewidth=0.7, linestyle=":")
ax_r2.axhline(1, color=TEXT, linewidth=0.5, linestyle=":", alpha=0.3)
ax_r2.legend()
style(ax_r2, "R²", "r²")

# ── Bias ──────────────────────────────────────────────────────────────────────
ax_bias.plot(ep, df["tr_bias"], color=TRAIN, label="train")
ax_bias.plot(ep, df["va_bias"], color=VAL,   label="val",  linestyle="--")
ax_bias.axhline(0, color=WARN, linewidth=0.7, linestyle=":")
ax_bias.legend()
style(ax_bias, "Mean Bias (+ = over-predict)", "bias")

# ── MAE ───────────────────────────────────────────────────────────────────────
ax_mae.plot(ep, df["tr_mae"], color=TRAIN, label="train")
ax_mae.plot(ep, df["va_mae"], color=VAL,   label="val",  linestyle="--")
ax_mae.legend()
style(ax_mae, "MAE", "mae")

# ── Val/Train gap ─────────────────────────────────────────────────────────────
ax_gap.plot(ep, df["gap"], color=WARN)
ax_gap.axhline(1.0, color=TEXT, linewidth=0.7, linestyle=":", alpha=0.5)
style(ax_gap, "Val/Train Loss Gap", "ratio")

# ── Pred std ──────────────────────────────────────────────────────────────────
ax_pstd.plot(ep, df["tr_pred_std"], color=TRAIN)
ax_pstd.axhline(0, color=WARN, linewidth=0.7, linestyle=":")
style(ax_pstd, "Prediction Std (train)", "std")

fig.savefig(OUT, dpi=150, bbox_inches="tight", facecolor=BG)
print(f"saved → {OUT}")
