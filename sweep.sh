#!/usr/bin/env bash
set -e

# ── 1. Auto-TFX + GRPO  (primary result) ───────────────────────── [ ] done ────
cargo run --release --features wgpu,auto-tfx -- train \
    --returns episode --advantages grpo --value-coef 0.0 \
    --episodes 256 --mini-batch-size 256 \
    --cache-file checkpoints/data.cache \
    --sequences-file checkpoints/auto-tfx-grpo-256ep-top.bin \
    --checkpoint-dir checkpoints/auto-tfx-grpo-256ep
cargo run --release -- plot-train --dir checkpoints/auto-tfx-grpo-256ep
cp checkpoints/auto-tfx-grpo-256ep/train_plots.png checkpoints/auto-tfx-grpo-256ep.png
cargo run --release -- diagnose \
    --sequences checkpoints/auto-tfx-grpo-256ep-top.bin \
    --output checkpoints/auto-tfx-grpo-256ep-diagnose.json

cargo run --release --features wgpu,auto-tfx -- evaluate \
    --model checkpoints/auto-tfx-grpo-256ep/best \
    --output checkpoints/auto-tfx-grpo-256ep/eval.json
cargo run --release -- plot-eval \
    --input checkpoints/auto-tfx-grpo-256ep/eval.json \
    --output eval.png

## ── 2. Auto-TFX + episode + baseline  (EV scatter) ─────────────── [ ] done ────
#cargo run --release --features wgpu,auto-tfx -- train \
#    --returns episode --advantages baseline \
#    --episodes 256 --mini-batch-size 256 \
#    --cache-file checkpoints/data.cache \
#    --sequences-file checkpoints/auto-tfx-episode-256ep-top.bin \
#    --checkpoint-dir checkpoints/auto-tfx-episode-256ep
#cargo run --release -- plot-train --dir checkpoints/auto-tfx-episode-256ep
#cp checkpoints/auto-tfx-episode-256ep/train_plots.png checkpoints/auto-tfx-episode-256ep.png
#cargo run --release -- diagnose \
#    --sequences checkpoints/auto-tfx-episode-256ep-top.bin \
#    --output checkpoints/auto-tfx-episode-256ep-diagnose.json
#
## ── 3. Auto-TFX + weighted + baseline  (shaped rewards) ────────── [ ] done ────
#cargo run --release --features wgpu,auto-tfx -- train \
#    --returns weighted --advantages baseline \
#    --episodes 256 --mini-batch-size 256 \
#    --cache-file checkpoints/data.cache \
#    --sequences-file checkpoints/auto-tfx-weighted-256ep-top.bin \
#    --checkpoint-dir checkpoints/auto-tfx-weighted-256ep
#cargo run --release -- plot-train --dir checkpoints/auto-tfx-weighted-256ep
#cp checkpoints/auto-tfx-weighted-256ep/train_plots.png checkpoints/auto-tfx-weighted-256ep.png
#cargo run --release -- diagnose \
#    --sequences checkpoints/auto-tfx-weighted-256ep-top.bin \
#    --output checkpoints/auto-tfx-weighted-256ep-diagnose.json
#
## ── 4. Auto-TFX + ir-step  (pipeline ablation, separate cache) ─── [ ] done ────
#cargo run --release --features wgpu,auto-tfx -- train \
#    --returns ir-step --advantages baseline \
#    --episodes 256 --mini-batch-size 256 \
#    --cache-file checkpoints/ir-ablation.cache \
#    --sequences-file checkpoints/auto-tfx-irstep-256ep-top.bin \
#    --checkpoint-dir checkpoints/auto-tfx-irstep-256ep
#cargo run --release -- plot-train --dir checkpoints/auto-tfx-irstep-256ep
#cp checkpoints/auto-tfx-irstep-256ep/train_plots.png checkpoints/auto-tfx-irstep-256ep.png
#cargo run --release -- diagnose \
#    --sequences checkpoints/auto-tfx-irstep-256ep-top.bin \
#    --output checkpoints/auto-tfx-irstep-256ep-diagnose.json
#
## ── 5. Auto-GRU + GRPO  (architecture comparison) ──────────────── [ ] done ────
#cargo run --release --features wgpu,auto-gru -- train \
#    --returns episode --advantages grpo --value-coef 0.0 \
#    --episodes 256 --mini-batch-size 256 \
#    --cache-file checkpoints/data.cache \
#    --sequences-file checkpoints/auto-gru-grpo-256ep-top.bin \
#    --checkpoint-dir checkpoints/auto-gru-grpo-256ep
#cargo run --release -- plot-train --dir checkpoints/auto-gru-grpo-256ep
#cp checkpoints/auto-gru-grpo-256ep/train_plots.png checkpoints/auto-gru-grpo-256ep.png
#cargo run --release -- diagnose \
#    --sequences checkpoints/auto-gru-grpo-256ep-top.bin \
#    --output checkpoints/auto-gru-grpo-256ep-diagnose.json
#
## ── 6. Seq  (parallel model, episode-only bandit) ───────────────── [ ] done ────
#cargo run --release --features wgpu -- train \
#    --returns episode --advantages baseline \
#    --episodes 256 --mini-batch-size 256 \
#    --cache-file checkpoints/data.cache \
#    --sequences-file checkpoints/seq-episode-256ep-top.bin \
#    --checkpoint-dir checkpoints/seq-episode-256ep
#cargo run --release -- plot-train --dir checkpoints/seq-episode-256ep
#cp checkpoints/seq-episode-256ep/train_plots.png checkpoints/seq-episode-256ep.png
#cargo run --release -- diagnose \
#    --sequences checkpoints/seq-episode-256ep-top.bin \
#    --output checkpoints/seq-episode-256ep-diagnose.json
#
## ── 7. Conclave  (parallel model with slot attention) ───────────── [ ] done ────
#cargo run --release --features wgpu,conclave -- train \
#    --returns episode --advantages baseline \
#    --episodes 256 --mini-batch-size 256 \
#    --cache-file checkpoints/data.cache \
#    --sequences-file checkpoints/conclave-episode-256ep-top.bin \
#    --checkpoint-dir checkpoints/conclave-episode-256ep
#cargo run --release -- plot-train --dir checkpoints/conclave-episode-256ep
#cp checkpoints/conclave-episode-256ep/train_plots.png checkpoints/conclave-episode-256ep.png
#cargo run --release -- diagnose \
#    --sequences checkpoints/conclave-episode-256ep-top.bin \
#    --output checkpoints/conclave-episode-256ep-diagnose.json
#
## ── 8. Collect dataset from bench-cache  (requires 1-7 complete) ── [ ] done ────
#cargo run --release -- collect \
#    --cache-file checkpoints/data.cache \
#    --output checkpoints/dataset.jsonl
#
## ── 9. Train SpeedupPredictor ────────────────────────────────────── [ ] done ────
#cargo run --release -- train-predictor \
#    --data checkpoints/dataset.jsonl \
#    --checkpoint-dir checkpoints/predictor
#
## ── 10. Auto-TFX + predictor + baseline  (dense reward) ─────────── [ ] done ────
#cargo run --release --features wgpu,auto-tfx -- train \
#    --returns predictor --predictor-checkpoint checkpoints/predictor \
#    --advantages baseline \
#    --episodes 256 --mini-batch-size 256 \
#    --cache-file checkpoints/data.cache \
#    --sequences-file checkpoints/auto-tfx-predictor-256ep-top.bin \
#    --checkpoint-dir checkpoints/auto-tfx-predictor-256ep
#cargo run --release -- plot-train --dir checkpoints/auto-tfx-predictor-256ep
#cp checkpoints/auto-tfx-predictor-256ep/train_plots.png checkpoints/auto-tfx-predictor-256ep.png
#cargo run --release -- diagnose \
#    --sequences checkpoints/auto-tfx-predictor-256ep-top.bin \
#    --output checkpoints/auto-tfx-predictor-256ep-diagnose.json
