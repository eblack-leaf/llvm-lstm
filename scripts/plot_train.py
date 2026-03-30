"""
plot_train.py  <checkpoint_dir>

Reads train_metrics.json written by training_tfx::train and produces
matplotlib figures in the same directory.

Figures generated:
  train_return.png      — mean EMA + per-function EMA over iterations
  train_policy.png      — policy loss (raw + rolling avg)
  train_entropy_kl.png  — entropy fraction and KL divergence
  train_signal.png      — advantage std and g0 spread
  train_baselines.png   — per-function % vs O0 / O2 / O3 over iterations
"""

import json
import sys
from pathlib import Path

import numpy as np

try:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    import matplotlib.ticker as ticker
except ImportError:
    print("plot_train: matplotlib not available, skipping plots", file=sys.stderr)
    sys.exit(0)

# ── Load data ─────────────────────────────────────────────────────────────────

def load_metrics(checkpoint_dir: Path):
    path = checkpoint_dir / "train_metrics.json"
    if not path.exists():
        print(f"plot_train: {path} not found", file=sys.stderr)
        sys.exit(1)
    with open(path) as f:
        records = json.load(f)
    if not records:
        print("plot_train: train_metrics.json is empty", file=sys.stderr)
        sys.exit(1)
    return records


def extract(records, key):
    return [r[key] for r in records]


def rolling_mean(xs, w=5):
    result = []
    for i in range(len(xs)):
        lo = max(0, i - w + 1)
        result.append(float(np.mean(xs[lo : i + 1])))
    return result


COLORS = [
    "#4e79a7", "#f28e2b", "#e15759", "#76b7b2",
    "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
]

def fn_color(name, names_sorted):
    idx = names_sorted.index(name) if name in names_sorted else 0
    return COLORS[idx % len(COLORS)]


# ── Figures ───────────────────────────────────────────────────────────────────

def fig_return(records, out_dir):
    iters = extract(records, "iteration")
    mean_ema = extract(records, "ema_mean")

    # Collect per-function EMA series
    fn_names = sorted({k for r in records for k in r.get("fn_ema", {})})
    fn_series = {
        fn: [r["fn_ema"].get(fn, float("nan")) for r in records]
        for fn in fn_names
    }

    fig, ax = plt.subplots(figsize=(10, 5))

    for i, fn in enumerate(fn_names):
        ax.plot(iters, fn_series[fn], color=COLORS[i % len(COLORS)],
                alpha=0.45, linewidth=1.0, label=fn)

    ax.plot(iters, mean_ema, color="white", linewidth=2.5, zorder=5)
    ax.plot(iters, mean_ema, color="#4e79a7", linewidth=1.8,
            zorder=6, label="mean EMA", linestyle="--")

    ax.axhline(0.0, color="gray", linewidth=0.6, linestyle=":")
    ax.set_xlabel("iteration")
    ax.set_ylabel("EMA return (relative to baselines)")
    ax.set_title("Return quality — EMA over training")
    ax.legend(fontsize=8, loc="upper left")
    _style(ax)
    fig.tight_layout()
    fig.savefig(out_dir / "train_return.png", dpi=150)
    plt.close(fig)
    print(f"  wrote {out_dir / 'train_return.png'}")


def fig_policy(records, out_dir):
    iters  = extract(records, "iteration")
    ploss  = extract(records, "policy_loss")
    ravg   = rolling_mean(ploss, w=10)

    fig, ax = plt.subplots(figsize=(10, 4))
    ax.plot(iters, ploss, color="#aaaaaa", linewidth=0.8, alpha=0.6, label="policy loss")
    ax.plot(iters, ravg,  color="#e15759", linewidth=1.8, label="rolling avg (10)")
    ax.axhline(0.0, color="gray", linewidth=0.6, linestyle=":")
    ax.set_xlabel("iteration")
    ax.set_ylabel("policy loss")
    ax.set_title("Policy loss over training")
    ax.legend(fontsize=9)
    _style(ax)
    fig.tight_layout()
    fig.savefig(out_dir / "train_policy.png", dpi=150)
    plt.close(fig)
    print(f"  wrote {out_dir / 'train_policy.png'}")


def fig_entropy_kl(records, out_dir):
    iters    = extract(records, "iteration")
    ent_frac = [v * 100.0 for v in extract(records, "entropy_frac")]
    kl       = extract(records, "kl")
    clip     = [v * 100.0 for v in extract(records, "clip_fraction")]

    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(10, 6), sharex=True)

    ax1.plot(iters, ent_frac, color="#59a14f", linewidth=1.4)
    ax1.axhline(35.0, color="orange", linewidth=0.8, linestyle="--", alpha=0.7, label="35% warning")
    ax1.axhline(20.0, color="red",    linewidth=0.8, linestyle="--", alpha=0.7, label="20% critical")
    ax1.set_ylabel("entropy (% of max)")
    ax1.set_title("Entropy & update diagnostics")
    ax1.set_ylim(bottom=0)
    ax1.legend(fontsize=8, loc="upper right")
    _style(ax1)

    ax2.plot(iters, kl,   color="#4e79a7", linewidth=1.2, label="KL")
    ax2.plot(iters, [c / 100.0 for c in clip], color="#f28e2b",
             linewidth=1.0, alpha=0.7, label="clip frac")
    ax2.axhline(0.15, color="red",    linewidth=0.7, linestyle="--", alpha=0.6, label="KL skip threshold")
    ax2.set_xlabel("iteration")
    ax2.set_ylabel("KL / clip fraction")
    ax2.legend(fontsize=8)
    _style(ax2)

    fig.tight_layout()
    fig.savefig(out_dir / "train_entropy_kl.png", dpi=150)
    plt.close(fig)
    print(f"  wrote {out_dir / 'train_entropy_kl.png'}")


def fig_signal(records, out_dir):
    iters     = extract(records, "iteration")
    adv_std   = extract(records, "adv_std")
    g0_spread = extract(records, "g0_spread")
    has_ev    = any("explained_var" in r for r in records)
    ev        = [r.get("explained_var", float("nan")) for r in records] if has_ev else None

    n_rows = 3 if has_ev else 2
    fig, axes = plt.subplots(n_rows, 1, figsize=(10, 3 * n_rows), sharex=True)

    axes[0].plot(iters, adv_std, color="#b07aa1", linewidth=1.4)
    axes[0].axhline(0.015, color="orange", linewidth=0.8, linestyle="--",
                    alpha=0.7, label="0.015 weak signal")
    axes[0].axhline(0.005, color="red",    linewidth=0.8, linestyle="--",
                    alpha=0.7, label="0.005 dead signal")
    axes[0].set_ylabel("advantage std")
    axes[0].set_title("Signal quality")
    axes[0].legend(fontsize=8, loc="upper right")
    _style(axes[0])

    axes[1].plot(iters, g0_spread, color="#76b7b2", linewidth=1.4)
    axes[1].axhline(0.05, color="orange", linewidth=0.8, linestyle="--",
                    alpha=0.7, label="0.05 low spread")
    axes[1].set_ylabel("g0 spread (max−min)")
    axes[1].legend(fontsize=8, loc="upper right")
    _style(axes[1])

    if has_ev:
        axes[2].plot(iters, ev, color="#edc948", linewidth=1.4)
        axes[2].axhline(0.5, color="green",  linewidth=0.8, linestyle="--", alpha=0.7, label="0.5 good")
        axes[2].axhline(0.0, color="red",    linewidth=0.8, linestyle="--", alpha=0.6, label="0 = mean baseline")
        axes[2].set_ylabel("explained variance")
        axes[2].legend(fontsize=8, loc="lower right")
        _style(axes[2])

    axes[-1].set_xlabel("iteration")
    fig.tight_layout()
    fig.savefig(out_dir / "train_signal.png", dpi=150)
    plt.close(fig)
    print(f"  wrote {out_dir / 'train_signal.png'}")


def fig_baselines(records, out_dir):
    iters = extract(records, "iteration")

    fn_names = sorted({k for r in records for k in r.get("fn_vs_o3", {})})
    if not fn_names:
        return

    fig, axes = plt.subplots(len(fn_names), 1,
                              figsize=(10, 3 * len(fn_names)),
                              sharex=True, squeeze=False)

    for row, fn in enumerate(fn_names):
        ax = axes[row][0]
        color = COLORS[row % len(COLORS)]

        vs_o0 = [r.get("fn_vs_o0", {}).get(fn, float("nan")) for r in records]
        vs_o2 = [r.get("fn_vs_o2", {}).get(fn, float("nan")) for r in records]
        vs_o3 = [r.get("fn_vs_o3", {}).get(fn, float("nan")) for r in records]

        ax.plot(iters, vs_o0, color="#aaaaaa", linewidth=1.0, label="vs O0", alpha=0.6)
        ax.plot(iters, vs_o2, color="#f28e2b", linewidth=1.0, label="vs O2")
        ax.plot(iters, vs_o3, color=color,     linewidth=1.5, label="vs O3")
        ax.axhline(0.0, color="gray", linewidth=0.5, linestyle=":")
        ax.set_ylabel("% speedup")
        ax.set_title(fn)
        ax.legend(fontsize=8, loc="upper left")
        _style(ax)

    axes[-1][0].set_xlabel("iteration")
    fig.suptitle("Per-function performance vs baselines", fontsize=11, y=1.01)
    fig.tight_layout()
    fig.savefig(out_dir / "train_baselines.png", dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  wrote {out_dir / 'train_baselines.png'}")


# ── Shared style ──────────────────────────────────────────────────────────────

def _style(ax):
    ax.set_facecolor("#1a1a2e")
    ax.figure.set_facecolor("#12121f")
    ax.tick_params(colors="#cccccc", labelsize=8)
    ax.xaxis.label.set_color("#cccccc")
    ax.yaxis.label.set_color("#cccccc")
    ax.title.set_color("#eeeeee")
    for spine in ax.spines.values():
        spine.set_edgecolor("#333355")
    ax.grid(True, color="#2a2a4a", linewidth=0.5)
    ax.legend_ and ax.legend(
        facecolor="#1a1a2e", edgecolor="#333355",
        labelcolor="#cccccc", fontsize=8,
    )


# ── Entry point ───────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) < 2:
        print("usage: plot_train.py <checkpoint_dir>", file=sys.stderr)
        sys.exit(1)

    checkpoint_dir = Path(sys.argv[1])
    records = load_metrics(checkpoint_dir)
    print(f"plot_train: {len(records)} iterations loaded from {checkpoint_dir}")

    fig_return(records, checkpoint_dir)
    fig_policy(records, checkpoint_dir)
    fig_entropy_kl(records, checkpoint_dir)
    fig_signal(records, checkpoint_dir)
    fig_baselines(records, checkpoint_dir)


if __name__ == "__main__":
    main()
