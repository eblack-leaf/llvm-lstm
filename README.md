# llvm-lstm

Reinforcement learning for LLVM pass sequencing. A PPO agent learns to select sequences of optimisation passes that produce binaries faster than `-O3`. The agent observes a compact IR feature vector at each step and outputs one of 28 passes or a Stop token. Two autoregressive architectures are provided: a transformer (Auto-TFX) and a GRU (Auto-GRU).

## Requirements

- Rust toolchain (stable)
- `clang-20` and `opt-20` on `PATH`
- A GPU with Vulkan support (used via the `wgpu` backend)
- Python 3 with matplotlib, numpy, and seaborn for plotting:

```shell
python3 -m venv .venv
.venv/bin/pip install matplotlib numpy seaborn pandas
```

## Training functions

The `benchmarks/` directory contains six functions used for training:

| File | Description |
|---|---|
| `array_reduction.c` | Sum reduction over an integer array |
| `binary_tree.c` | Recursive binary-tree traversal |
| `fft.c` | Cooley-Tukey FFT |
| `interpreter.c` | Bytecode dispatch loop |
| `kmp_search.c` | Knuth-Morris-Pratt string search |
| `polynomial_eval.c` | Horner polynomial evaluation |

The `pool/` directory contains 38 functions used for the generalisation experiment (run 6).

---

## Run 1 — Auto-TFX + episode return

The primary result. The episode return assigns the terminal benchmark speedup uniformly across all steps in a sequence.

```shell
cargo run --release --features wgpu -- train \
    --returns episode \
    --episodes 64 --mini-batch-size 64 \
    --cache-file checkpoints/data.cache \
    --sequences-file checkpoints/auto-tfx-episode-top.bin \
    --checkpoint-dir checkpoints/auto-tfx-episode

cargo run --release --features wgpu -- plot-train \
    --dir checkpoints/auto-tfx-episode
cp checkpoints/auto-tfx-episode/train_plots.png checkpoints/auto-tfx-episode.png

cargo run --release --features wgpu -- diagnose \
    --sequences checkpoints/auto-tfx-episode-top.bin \
    --output checkpoints/auto-tfx-episode-diagnose.json
cargo run --release --features wgpu -- plot-diagnose \
    --results checkpoints/auto-tfx-episode-diagnose.json

cargo run --release --features wgpu -- evaluate \
    --model checkpoints/auto-tfx-episode/best \
    --output checkpoints/auto-tfx-episode/eval.json
cargo run --release --features wgpu -- plot-eval \
    --input checkpoints/auto-tfx-episode/eval.json \
    --output checkpoints/auto-tfx-episode/eval.png
```

Outputs:
- `checkpoints/auto-tfx-episode.png` — training curves
- `checkpoints/auto-tfx-episode-diagnose.png` — top-sequence re-benchmark
- `checkpoints/auto-tfx-episode/eval.png` — speedup bar chart vs O3

---

## Run 2 — Auto-GRU + episode return

Architecture comparison. GRU uses a recurrent hidden state instead of attention; requires `--no-default-features` to disable the default `auto-tfx` feature.

```shell
cargo run --release --no-default-features --features wgpu,auto-gru -- train \
    --returns episode \
    --episodes 64 --mini-batch-size 64 \
    --cache-file checkpoints/data.cache \
    --sequences-file checkpoints/auto-gru-episode-top.bin \
    --checkpoint-dir checkpoints/auto-gru-episode

cargo run --release --no-default-features --features wgpu,auto-gru -- plot-train \
    --dir checkpoints/auto-gru-episode
cp checkpoints/auto-gru-episode/train_plots.png checkpoints/auto-gru-episode.png

cargo run --release --no-default-features --features wgpu,auto-gru -- diagnose \
    --sequences checkpoints/auto-gru-episode-top.bin \
    --output checkpoints/auto-gru-episode-diagnose.json
cargo run --release --no-default-features --features wgpu,auto-gru -- plot-diagnose \
    --results checkpoints/auto-gru-episode-diagnose.json
```

---

## Run 3 — Auto-TFX + weighted return

The weighted return redistributes credit proportionally to per-step instruction reduction. Steps that do not reduce the instruction count are penalised as no-ops.

```shell
cargo run --release --features wgpu -- train \
    --returns weighted \
    --episodes 64 --mini-batch-size 64 \
    --cache-file checkpoints/data.cache \
    --sequences-file checkpoints/auto-tfx-weighted-top.bin \
    --checkpoint-dir checkpoints/auto-tfx-weighted

cargo run --release --features wgpu -- plot-train \
    --dir checkpoints/auto-tfx-weighted
cp checkpoints/auto-tfx-weighted/train_plots.png checkpoints/auto-tfx-weighted.png

cargo run --release --features wgpu -- diagnose \
    --sequences checkpoints/auto-tfx-weighted-top.bin \
    --output checkpoints/auto-tfx-weighted-diagnose.json
cargo run --release --features wgpu -- plot-diagnose \
    --results checkpoints/auto-tfx-weighted-diagnose.json
```

---

## Run 4 — Auto-GRU + weighted return

```shell
cargo run --release --no-default-features --features wgpu,auto-gru -- train \
    --returns weighted \
    --episodes 64 --mini-batch-size 64 \
    --cache-file checkpoints/data.cache \
    --sequences-file checkpoints/auto-gru-weighted-top.bin \
    --checkpoint-dir checkpoints/auto-gru-weighted

cargo run --release --no-default-features --features wgpu,auto-gru -- plot-train \
    --dir checkpoints/auto-gru-weighted
cp checkpoints/auto-gru-weighted/train_plots.png checkpoints/auto-gru-weighted.png

cargo run --release --no-default-features --features wgpu,auto-gru -- diagnose \
    --sequences checkpoints/auto-gru-weighted-top.bin \
    --output checkpoints/auto-gru-weighted-diagnose.json
cargo run --release --no-default-features --features wgpu,auto-gru -- plot-diagnose \
    --results checkpoints/auto-gru-weighted-diagnose.json
```

---

## Run 5 — Auto-TFX + IR-step return (ablation)

The IR-step return gives a dense per-step reward based on instruction-count reduction, skipping benchmarking entirely. This makes training much faster but the policy is not evaluated against real runtimes — it is included as an ablation to isolate the effect of the dense signal.

```shell
cargo run --release --features wgpu -- train \
    --returns ir-step \
    --episodes 64 --mini-batch-size 64 \
    --cache-file checkpoints/ir-ablation.cache \
    --sequences-file checkpoints/auto-tfx-irstep-top.bin \
    --checkpoint-dir checkpoints/auto-tfx-irstep

cargo run --release --features wgpu -- plot-train \
    --dir checkpoints/auto-tfx-irstep
cp checkpoints/auto-tfx-irstep/train_plots.png checkpoints/auto-tfx-irstep.png

cargo run --release --features wgpu -- diagnose \
    --sequences checkpoints/auto-tfx-irstep-top.bin \
    --output checkpoints/auto-tfx-irstep-diagnose.json
cargo run --release --features wgpu -- plot-diagnose \
    --results checkpoints/auto-tfx-irstep-diagnose.json
```

Note: uses a separate cache (`ir-ablation.cache`) to avoid polluting the benchmark cache used by runs 1–4.

---

## Run 6 — Auto-TFX + episode return + full pool (generalisation)

Trains on all 38 functions in `pool/` to test how well the policy generalises beyond the six training functions.

```shell
cargo run --release --features wgpu -- train \
    --returns episode \
    --directory pool \
    --episodes 64 --mini-batch-size 64 \
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
```

---

## Dataset inspection

After training, the benchmark cache can be dumped to a JSONL file and plotted to inspect the distribution of speedups, sequence lengths, and pass frequencies the agent explored:

```shell
cargo run --release --features wgpu -- collect-dataset \
    --cache-file checkpoints/data.cache \
    --output dataset.jsonl

cargo run --release --features wgpu -- plot-dataset \
    --data dataset.jsonl
```

---

## IR feature inspection

To inspect what the agent observes — the chunked IR delta features — for any set of functions:

```shell
cargo run --release --features wgpu -- export-features \
    --directory benchmarks \
    --output features.json

cargo run --release --features wgpu -- plot-features \
    --features features.json
```

---

## All runs in sequence

`sweep.sh` runs whichever blocks are uncommented. Edit the `EPISODES` and `MINI_BATCH` variables at the top to change scale, then uncomment the runs you want:

```shell
bash sweep.sh
```
