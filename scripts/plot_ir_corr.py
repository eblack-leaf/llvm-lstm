#!/usr/bin/env python3
import argparse
import json
import pathlib

import matplotlib.pyplot as plt
import numpy as np


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--results", required=True)
    ap.add_argument("--output", default=None)
    args = ap.parse_args()

    with open(args.results) as f:
        records = json.load(f)

    ir_pct = [r["ir_reduction"] * 100 for r in records]
    speedups = [r["speedup"] for r in records]
    labels = [f"{r['func_name']} #{r['rank']}" for r in records]

    fig, ax = plt.subplots(figsize=(9, 6))
    ax.scatter(ir_pct, speedups, zorder=3, s=60)
    for x, y, lbl in zip(ir_pct, speedups, labels):
        ax.annotate(lbl, (x, y), textcoords="offset points", xytext=(5, 4), fontsize=7)

    if len(ir_pct) >= 2:
        m, b = np.polyfit(ir_pct, speedups, 1)
        xs = np.linspace(min(ir_pct), max(ir_pct), 200)
        ax.plot(xs, m * xs + b, "--", color="red", linewidth=1, label=f"fit (slope={m:.4f})")
        r = float(np.corrcoef(ir_pct, speedups)[0, 1])
        ax.set_title(f"IR reduction vs speedup vs O3   (Pearson r = {r:.4f}, n={len(ir_pct)})")
        ax.legend(fontsize=9)
    else:
        ax.set_title("IR reduction vs speedup vs O3")

    ax.set_xlabel("IR instruction reduction (%)")
    ax.set_ylabel("speedup vs O3")
    ax.axhline(0, color="black", linewidth=0.5, linestyle=":")
    ax.axvline(0, color="black", linewidth=0.5, linestyle=":")
    ax.grid(True, alpha=0.3)
    plt.tight_layout()

    out = args.output or str(pathlib.Path(args.results).with_suffix(".png"))
    plt.savefig(out, dpi=150)
    print(f"saved → {out}")


if __name__ == "__main__":
    main()
