#!/usr/bin/env bash
set -e

# ── 1. Auto-TFX + episode + baseline  (primary result) ─────────── [x] done ────
#cargo run --release --features wgpu,auto-tfx -- train \
#    --returns episode \
#    --episodes 256 --mini-batch-size 256 \
#    --cache-file checkpoints/data.cache \
#    --sequences-file checkpoints/auto-tfx-episode-256ep-top.bin \
#    --checkpoint-dir checkpoints/auto-tfx-episode-256ep
#cargo run --release --features wgpu,auto-tfx -- plot-train \
#    --dir checkpoints/auto-tfx-episode-256ep
#cp checkpoints/auto-tfx-episode-256ep/train_plots.png checkpoints/auto-tfx-episode-256ep.png
#cargo run --release --features wgpu,auto-tfx -- diagnose \
#    --sequences checkpoints/auto-tfx-episode-256ep-top.bin \
#    --output checkpoints/auto-tfx-episode-256ep-diagnose.json
#
#cargo run --release --features wgpu,auto-tfx -- evaluate \
#    --model checkpoints/auto-tfx-episode-256ep/best \
#    --output checkpoints/auto-tfx-episode-256ep/eval.json
#cargo run --release --features wgpu,auto-tfx -- plot-eval \
#    --input checkpoints/auto-tfx-episode-256ep/eval.json \
#    --output checkpoints/auto-tfx-episode-256ep/eval.png

# ── 2. Auto-GRU + episode + baseline  (architecture comparison) ─── [ ] todo ────
#cargo run --release --features wgpu,auto-gru -- train \
#    --returns episode \
#    --episodes 256 --mini-batch-size 256 \
#    --cache-file checkpoints/data.cache \
#    --sequences-file checkpoints/auto-gru-episode-256ep-top.bin \
#    --checkpoint-dir checkpoints/auto-gru-episode-256ep
#cargo run --release --features wgpu,auto-gru -- plot-train \
#    --dir checkpoints/auto-gru-episode-256ep
#cp checkpoints/auto-gru-episode-256ep/train_plots.png checkpoints/auto-gru-episode-256ep.png
#cargo run --release --features wgpu,auto-gru -- diagnose \
#    --sequences checkpoints/auto-gru-episode-256ep-top.bin \
#    --output checkpoints/auto-gru-episode-256ep-diagnose.json

# ── 3. Auto-TFX + weighted + baseline  (shaped rewards) ─────────── [ ] todo ────
#cargo run --release --features wgpu,auto-tfx -- train \
#    --returns weighted \
#    --episodes 256 --mini-batch-size 256 \
#    --cache-file checkpoints/data.cache \
#    --sequences-file checkpoints/auto-tfx-weighted-256ep-top.bin \
#    --checkpoint-dir checkpoints/auto-tfx-weighted-256ep
#cargo run --release --features wgpu,auto-tfx -- plot-train \
#    --dir checkpoints/auto-tfx-weighted-256ep
#cp checkpoints/auto-tfx-weighted-256ep/train_plots.png checkpoints/auto-tfx-weighted-256ep.png
#cargo run --release --features wgpu,auto-tfx -- diagnose \
#    --sequences checkpoints/auto-tfx-weighted-256ep-top.bin \
#    --output checkpoints/auto-tfx-weighted-256ep-diagnose.json

# ── 4. Auto-GRU + weighted + baseline  (architecture comparison) ── [ ] todo ────
#cargo run --release --features wgpu,auto-gru -- train \
#    --returns weighted \
#    --episodes 256 --mini-batch-size 256 \
#    --cache-file checkpoints/data.cache \
#    --sequences-file checkpoints/auto-gru-weighted-256ep-top.bin \
#    --checkpoint-dir checkpoints/auto-gru-weighted-256ep
#cargo run --release --features wgpu,auto-gru -- plot-train \
#    --dir checkpoints/auto-gru-weighted-256ep
#cp checkpoints/auto-gru-weighted-256ep/train_plots.png checkpoints/auto-gru-weighted-256ep.png
#cargo run --release --features wgpu,auto-gru -- diagnose \
#    --sequences checkpoints/auto-gru-weighted-256ep-top.bin \
#    --output checkpoints/auto-gru-weighted-256ep-diagnose.json

# ── 5. Auto-TFX + ir-step  (return ablation, separate cache) ─────── [ ] todo ────
#cargo run --release --features wgpu,auto-tfx -- train \
#    --returns ir-step \
#    --episodes 256 --mini-batch-size 256 \
#    --cache-file checkpoints/ir-ablation.cache \
#    --sequences-file checkpoints/auto-tfx-irstep-256ep-top.bin \
#    --checkpoint-dir checkpoints/auto-tfx-irstep-256ep
#cargo run --release --features wgpu,auto-tfx -- plot-train \
#    --dir checkpoints/auto-tfx-irstep-256ep
#cp checkpoints/auto-tfx-irstep-256ep/train_plots.png checkpoints/auto-tfx-irstep-256ep.png
#cargo run --release --features wgpu,auto-tfx -- diagnose \
#    --sequences checkpoints/auto-tfx-irstep-256ep-top.bin \
#    --output checkpoints/auto-tfx-irstep-256ep-diagnose.json
