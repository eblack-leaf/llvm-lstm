#!/usr/bin/env python3
"""Training diagnostics plotter — dark/minimal, seaborn."""
import argparse
import json
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import pandas as pd
import seaborn as sns

parser = argparse.ArgumentParser()
parser.add_argument("--dir", type=Path,
                    default=Path(__file__).parent.parent / "checkpoints",
                    help="checkpoint directory containing train.jsonl")
args = parser.parse_args()

JSONL = args.dir / "train.jsonl"
OUT   = args.dir / "train_plots.png"

FUNC_COLORS = [
    "#7eb3ff", "#ff8c69", "#7ddc8b", "#d49aff", "#ffd166", "#ff6b9d"
]

# ── load ──────────────────────────────────────────────────────────────────────

rows = []
with open(JSONL) as f:
    for line in f:
        line = line.strip()
        if line:
            rows.append(json.loads(line))

if not rows:
    print("No data in train.jsonl", file=sys.stderr)
    sys.exit(1)

df = pd.DataFrame(rows)
funcs = sorted(df["func_speedups"].iloc[0].keys())
for fn in funcs:
    df[f"fn_{fn}"] = df["func_speedups"].apply(lambda d: d[fn])

# ── style ─────────────────────────────────────────────────────────────────────

BG      = "#0d0f14"
PANEL   = "#13161e"
GRID    = "#1f2330"
TEXT    = "#c8ccd8"
ACCENT  = "#7eb3ff"
EMA_C   = "#ffd166"
WARN_C  = "#ff6b9d"

plt.rcParams.update({
    "figure.facecolor":  BG,
    "axes.facecolor":    PANEL,
    "axes.edgecolor":    GRID,
    "axes.labelcolor":   TEXT,
    "axes.titlecolor":   TEXT,
    "xtick.color":       TEXT,
    "ytick.color":       TEXT,
    "grid.color":        GRID,
    "grid.linewidth":    0.6,
    "text.color":        TEXT,
    "font.family":       "monospace",
    "font.size":         9,
    "axes.titlesize":    10,
    "legend.facecolor":  PANEL,
    "legend.edgecolor":  GRID,
    "legend.fontsize":   8,
    "lines.linewidth":   1.6,
})

ep = df["epoch"]

# ── layout ────────────────────────────────────────────────────────────────────

fig = plt.figure(figsize=(16, 10), facecolor=BG)
fig.suptitle("Training Diagnostics", color=TEXT, fontsize=13, y=0.98)

gs = fig.add_gridspec(3, 3, hspace=0.52, wspace=0.38,
                      left=0.06, right=0.97, top=0.93, bottom=0.07)

axes = {
    "speedup":   fig.add_subplot(gs[0, :2]),
    "func":      fig.add_subplot(gs[0, 2]),
    "ev":        fig.add_subplot(gs[1, 0]),
    "entropy":   fig.add_subplot(gs[1, 1]),
    "kl":        fig.add_subplot(gs[1, 2]),
    "losses":    fig.add_subplot(gs[2, 0]),
    "noop":      fig.add_subplot(gs[2, 1]),
    "ep_len":    fig.add_subplot(gs[2, 2]),
}

def style_ax(ax, title, ylabel="", ylim=None):
    ax.set_title(title, pad=5)
    ax.set_xlabel("epoch", labelpad=3)
    ax.set_ylabel(ylabel, labelpad=3)
    ax.grid(True, axis="y")
    ax.set_xlim(ep.min(), ep.max())
    if ylim:
        ax.set_ylim(*ylim)
    ax.xaxis.set_major_locator(ticker.MaxNLocator(integer=True, nbins=6))

# 1. Speedup + EMA ─────────────────────────────────────────────────────────────
ax = axes["speedup"]
ax.fill_between(ep, df["avg_final_speedup"], alpha=0.15, color=ACCENT)
ax.plot(ep, df["avg_final_speedup"], color=ACCENT, label="avg speedup")
ax.plot(ep, df["ema"],               color=EMA_C,  label="EMA",      linestyle="--")
ax.legend(loc="lower right")
style_ax(ax, "Speedup", "speedup ratio")

# 2. Per-function speedup ──────────────────────────────────────────────────────
ax = axes["func"]
for fn, col in zip(funcs, FUNC_COLORS):
    ax.plot(ep, df[f"fn_{fn}"], color=col, label=fn.replace("_", " "), linewidth=1.2)
ax.legend(loc="lower right", ncol=1)
style_ax(ax, "Per-function", "speedup")

# 3. Explained variance ────────────────────────────────────────────────────────
ax = axes["ev"]
ax.axhline(0, color=WARN_C, linewidth=0.8, linestyle=":")
ax.fill_between(ep, df["explained_variance"].clip(-1, 1), alpha=0.15, color=ACCENT)
ax.plot(ep, df["explained_variance"].clip(-1, 1), color=ACCENT)
style_ax(ax, "Explained Variance", "EV", ylim=(-1.1, 1.1))

# 4. Entropy ───────────────────────────────────────────────────────────────────
ax = axes["entropy"]
ax.plot(ep, df["entropy"],     color=ACCENT, label="entropy (nats)")
ax2 = ax.twinx()
ax2.plot(ep, df["entropy_pct"], color=EMA_C, label="entropy %", linestyle="--", linewidth=1.2)
ax2.set_ylabel("% of max", color=EMA_C, fontsize=8)
ax2.tick_params(colors=EMA_C)
ax2.spines["right"].set_color(GRID)
ax.legend(loc="upper right")
style_ax(ax, "Entropy", "nats")

# 5. KL divergence ─────────────────────────────────────────────────────────────
ax = axes["kl"]
ax.plot(ep, df["kl_div"], color=WARN_C)
ax.axhline(0.02, color=TEXT, linewidth=0.7, linestyle=":", alpha=0.5, label="target ~0.02")
ax.legend(loc="upper right")
style_ax(ax, "KL Divergence", "KL")

# 6. Losses + clip fraction ───────────────────────────────────────────────────
ax = axes["losses"]
ax.plot(ep, df["policy_loss"], color=ACCENT, label="policy")
ax.plot(ep, df["value_loss"],  color=EMA_C,  label="value")
ax.axhline(0, color=WARN_C, linewidth=0.6, linestyle=":")
if "clip_frac" in df.columns:
    ax2 = ax.twinx()
    ax2.plot(ep, df["clip_frac"] * 100, color=WARN_C, linestyle="--",
             linewidth=1.2, alpha=0.7, label="clip %")
    ax2.set_ylabel("clip %", color=WARN_C, fontsize=8)
    ax2.tick_params(colors=WARN_C)
    ax2.spines["right"].set_color(GRID)
    ax2.set_ylim(0, 100)
    ax2.legend(loc="upper left", fontsize=7)
ax.legend(loc="upper right")
style_ax(ax, "Losses + Clip %", "loss")

# 7. Noop % ────────────────────────────────────────────────────────────────────
ax = axes["noop"]
ax.fill_between(ep, df["noop_pct"], alpha=0.15, color=WARN_C)
ax.plot(ep, df["noop_pct"], color=WARN_C)
style_ax(ax, "Noop %", "%", ylim=(0, 100))

# 8. Episode length ─────────────────────────────────────────────────────────────
ax = axes["ep_len"]
ax.plot(ep, df["avg_episode_len"], color=ACCENT)
style_ax(ax, "Avg Episode Length", "steps")

# ── save ──────────────────────────────────────────────────────────────────────

fig.savefig(OUT, dpi=150, bbox_inches="tight", facecolor=BG)
print(f"saved → {OUT}")
