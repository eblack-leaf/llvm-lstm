#!/usr/bin/env bash
set -e

EPISODES=64
MINI_BATCH=64

# ── 1. Auto-TFX + episode + baseline  (primary result) ─────────── [x] done ────
#cargo run --release --features wgpu -- train \
#    --returns episode \
#    --episodes $EPISODES --mini-batch-size $MINI_BATCH \
#    --cache-file checkpoints/data.cache \
#    --sequences-file checkpoints/auto-tfx-episode-top.bin \
#    --checkpoint-dir checkpoints/auto-tfx-episode
#cargo run --release --features wgpu -- plot-train \
#    --dir checkpoints/auto-tfx-episode
#cp checkpoints/auto-tfx-episode/train_plots.png checkpoints/auto-tfx-episode.png
#cargo run --release --features wgpu -- diagnose \
#    --sequences checkpoints/auto-tfx-episode-top.bin \
#    --output checkpoints/auto-tfx-episode-diagnose.json
#cargo run --release --features wgpu -- plot-diagnose \
#    --results checkpoints/auto-tfx-episode-diagnose.json
#
#cargo run --release --features wgpu -- evaluate \
#    --model checkpoints/auto-tfx-episode/best \
#    --output checkpoints/auto-tfx-episode/eval.json
#cargo run --release --features wgpu -- plot-eval \
#    --input checkpoints/auto-tfx-episode/eval.json \
#    --output checkpoints/auto-tfx-episode/eval.png

# ── 2. Auto-GRU + episode + baseline  (architecture comparison) ─── [ ] todo ────
#cargo run --release --no-default-features --features wgpu,auto-gru -- train \
#    --returns episode \
#    --episodes $EPISODES --mini-batch-size $MINI_BATCH \
#    --cache-file checkpoints/data.cache \
#    --sequences-file checkpoints/auto-gru-episode-top.bin \
#    --checkpoint-dir checkpoints/auto-gru-episode
#cargo run --release --no-default-features --features wgpu,auto-gru -- plot-train \
#    --dir checkpoints/auto-gru-episode
#cp checkpoints/auto-gru-episode/train_plots.png checkpoints/auto-gru-episode.png
#cargo run --release --no-default-features --features wgpu,auto-gru -- diagnose \
#    --sequences checkpoints/auto-gru-episode-top.bin \
#    --output checkpoints/auto-gru-episode-diagnose.json
#cargo run --release --no-default-features --features wgpu,auto-gru -- plot-diagnose \
#    --results checkpoints/auto-gru-episode-diagnose.json

# ── 3. Auto-TFX + weighted + baseline  (shaped rewards) ─────────── [ ] todo ────
#cargo run --release --features wgpu -- train \
#    --returns weighted \
#    --episodes $EPISODES --mini-batch-size $MINI_BATCH \
#    --cache-file checkpoints/data.cache \
#    --sequences-file checkpoints/auto-tfx-weighted-top.bin \
#    --checkpoint-dir checkpoints/auto-tfx-weighted
#cargo run --release --features wgpu -- plot-train \
#    --dir checkpoints/auto-tfx-weighted
#cp checkpoints/auto-tfx-weighted/train_plots.png checkpoints/auto-tfx-weighted.png
#cargo run --release --features wgpu -- diagnose \
#    --sequences checkpoints/auto-tfx-weighted-top.bin \
#    --output checkpoints/auto-tfx-weighted-diagnose.json
#cargo run --release --features wgpu -- plot-diagnose \
#    --results checkpoints/auto-tfx-weighted-diagnose.json

# ── 4. Auto-GRU + weighted + baseline  (architecture comparison) ── [ ] todo ────
#cargo run --release --no-default-features --features wgpu,auto-gru -- train \
#    --returns weighted \
#    --episodes $EPISODES --mini-batch-size $MINI_BATCH \
#    --cache-file checkpoints/data.cache \
#    --sequences-file checkpoints/auto-gru-weighted-top.bin \
#    --checkpoint-dir checkpoints/auto-gru-weighted
#cargo run --release --no-default-features --features wgpu,auto-gru -- plot-train \
#    --dir checkpoints/auto-gru-weighted
#cp checkpoints/auto-gru-weighted/train_plots.png checkpoints/auto-gru-weighted.png
#cargo run --release --no-default-features --features wgpu,auto-gru -- diagnose \
#    --sequences checkpoints/auto-gru-weighted-top.bin \
#    --output checkpoints/auto-gru-weighted-diagnose.json
#cargo run --release --no-default-features --features wgpu,auto-gru -- plot-diagnose \
#    --results checkpoints/auto-gru-weighted-diagnose.json

# ── 5. Auto-TFX + ir-step  (return ablation, separate cache) ─────── [ ] todo ────
#cargo run --release --features wgpu -- train \
#    --returns ir-step \
#    --episodes $EPISODES --mini-batch-size $MINI_BATCH \
#    --cache-file checkpoints/ir-ablation.cache \
#    --sequences-file checkpoints/auto-tfx-irstep-top.bin \
#    --checkpoint-dir checkpoints/auto-tfx-irstep
#cargo run --release --features wgpu -- plot-train \
#    --dir checkpoints/auto-tfx-irstep
#cp checkpoints/auto-tfx-irstep/train_plots.png checkpoints/auto-tfx-irstep.png
#cargo run --release --features wgpu -- diagnose \
#    --sequences checkpoints/auto-tfx-irstep-top.bin \
#    --output checkpoints/auto-tfx-irstep-diagnose.json
#cargo run --release --features wgpu -- plot-diagnose \
#    --results checkpoints/auto-tfx-irstep-diagnose.json

# ── 6. Auto-TFX + episode + full pool  (generalisation train) ────── [ ] todo ────
cargo run --release --features wgpu -- train \
    --returns episode \
    --directory pool \
    --episodes $EPISODES --mini-batch-size $MINI_BATCH \
    --cache-file checkpoints/pool.cache \
    --sequences-file checkpoints/auto-tfx-pool-top.bin \
    --checkpoint-dir checkpoints/auto-tfx-pool
cargo run --release --features wgpu -- plot-train \
    --dir checkpoints/auto-tfx-pool
cp checkpoints/auto-tfx-pool/train_plots.png checkpoints/auto-tfx-pool.png
cargo run --release --features wgpu -- diagnose \
    --sequences checkpoints/auto-tfx-pool-top.bin \
    --directory pool \
    --output checkpoints/auto-tfx-pool/diagnose.json
cargo run --release --features wgpu -- plot-diagnose \
    --results checkpoints/auto-tfx-pool/diagnose.json
cargo run --release --features wgpu -- evaluate \
    --model checkpoints/auto-tfx-pool/best \
    --directory pool \
    --output checkpoints/auto-tfx-pool/eval.json
cargo run --release --features wgpu -- plot-eval \
    --input checkpoints/auto-tfx-pool/eval.json \
    --output checkpoints/auto-tfx-pool/eval.png
