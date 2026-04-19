#!/usr/bin/env bash
set -euo pipefail

# ── Sweep configuration ────────────────────────────────────────────────────────
# Architectures: name → cargo feature flag (added after "wgpu,"; "" = default seq)
declare -A ARCH_FEATURES=(
    ["seq"]=""
    ["auto-tfx"]="auto-tfx"
    ["auto-gru"]="auto-gru"
    ["conclave"]="conclave"
)
ARCH_ORDER=("seq" "auto-tfx" "auto-gru" "conclave")

EPISODES=(256)

# Returns to sweep:
#   episode   — uniform terminal speedup
#   proxy     — blended instr-count + terminal speedup (--proxy-alpha 0.5)
#   weighted  — terminal weighted by per-slot instr reduction
#   predictor — per-step marginal from pretrained SpeedupPredictor
#   ir        — terminal IR-count reduction (no benchmark)
#   ir-step   — per-step IR-count delta (dense; good for auto-tfx/gru)
RETURNS=("episode" "weighted" "proxy" "predictor" "ir" "ir-step")

# Cache/sequences split by reward type.
BENCH_CACHE="checkpoints/data.cache"
BENCH_SEQS="checkpoints/bench-top.bin"
IR_CACHE="checkpoints/data-ir.cache"
IR_SEQS="checkpoints/bench-top-ir.bin"

PREDICTOR_CKPT="predictor_checkpoints"

# Mini-batch = episodes / 2, floored at 4, capped at 128.
mb_for() {
    local ep=$1
    local mb=$(( ep / 2 ))
    [[ $mb -lt 4   ]] && mb=4
    [[ $mb -gt 128 ]] && mb=128
    echo $mb
}

# ── Run sweep ──────────────────────────────────────────────────────────────────
TOTAL=$(( ${#ARCH_ORDER[@]} * ${#EPISODES[@]} * ${#RETURNS[@]} ))
RUN=0

for ARCH in "${ARCH_ORDER[@]}"; do
    FEAT="${ARCH_FEATURES[$ARCH]}"
    FEATURES="wgpu${FEAT:+,$FEAT}"

    for EP in "${EPISODES[@]}"; do
        MB=$(mb_for "$EP")

        for RET in "${RETURNS[@]}"; do
            RUN=$(( RUN + 1 ))
            CKPT="checkpoints/${ARCH}-${RET}-${EP}ep"
            LABEL="${ARCH}-${RET}-${EP}ep"

            echo ""
            echo "── [$RUN/$TOTAL]  arch=${ARCH}  returns=${RET}  episodes=${EP}  mb=${MB} ──"

            EXTRA_ARGS=()

            case "$RET" in
                ir|ir-step)
                    EXTRA_ARGS+=(--cache-file "$IR_CACHE" --sequences-file "$IR_SEQS")
                    ;;
                predictor)
                    EXTRA_ARGS+=(
                        --cache-file "$BENCH_CACHE"
                        --sequences-file "$BENCH_SEQS"
                        --predictor-checkpoint "$PREDICTOR_CKPT"
                    )
                    ;;
                proxy)
                    EXTRA_ARGS+=(
                        --cache-file "$BENCH_CACHE"
                        --sequences-file "$BENCH_SEQS"
                        --proxy-alpha 0.5
                    )
                    ;;
                *)
                    EXTRA_ARGS+=(--cache-file "$BENCH_CACHE" --sequences-file "$BENCH_SEQS")
                    ;;
            esac

            cargo run --release --features "$FEATURES" -- train \
                --returns "$RET" \
                --episodes "$EP" \
                --mini-batch-size "$MB" \
                --checkpoint-dir "$CKPT" \
                "${EXTRA_ARGS[@]}"

            cargo run -- plot-train --dir "$CKPT"

            cp "${CKPT}/train_plots.png" "checkpoints/${LABEL}.png"
            echo "  → checkpoints/${LABEL}.png"
        done
    done
done

echo ""
echo "Sweep complete ($TOTAL runs). PNGs:"
ls checkpoints/*.png
