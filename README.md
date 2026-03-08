# llvm-lstm

A reinforcement learning agent that learns optimal LLVM pass ordering for C programs. An LSTM policy trained with PPO observes IR features after each applied pass and selects the next optimization pass to maximize runtime speedup.

## Overview

Modern compilers apply optimization passes in a fixed order determined by `-O2`/`-O3` flags. This project explores whether an RL agent can discover better orderings for specific programs by treating pass selection as a sequential decision problem.

### How it works

```
C source → clang-20 (emit IR) → [pass sequence] → llc-20 → binary → benchmark (ns)
                                       ↑
                              LSTM policy + PPO
                                       |
                              18-dim IR feature vector
                              + pass history
```

The agent operates episodically: observe current IR features, select a pass (or STOP), apply it via `opt-20`, re-extract features, and receive a reward proportional to speedup over the -O3 baseline.

## Prerequisites

- **LLVM 20**: `clang-20`, `opt-20`, `llc-20` on PATH
- **Rust 1.75+** (edition 2024)
- Linux (tested on Ubuntu)

```bash
# Ubuntu
sudo apt install llvm-20 clang-20
```

## Build

```bash
cargo build --release
```

To enable the extended 86-action pass set (vs default 29):
```bash
cargo build --release --features secondary_passes
```

## Benchmarks

32 self-contained C benchmark programs in `benchmarks/`. Each measures its own execution time internally using `clock_gettime` and prints the median over multiple iterations. Covers a range of compute patterns:

| Category | Examples |
|----------|---------|
| Linear algebra | `dot_product`, `matrix_multiply_tiled`, `convolution`, `stencil2d` |
| Sorting / searching | `mergesort`, `quicksort`, `binary_search`, `kmp_search` |
| Data structures | `binary_tree`, `hashtable`, `heap_ops` |
| Algorithms | `fft`, `karatsuba`, `levenshtein`, `nqueens`, `lz_compress` |
| Misc | `miniray`, `physics_sim`, `trig_approx`, `interpreter`, `regex_match` |

## Action Space

### Primary passes (default — 28 transforms + STOP = 29 actions)

High-impact passes that appear in LLVM's `-O3` inner optimization kernel, with broad demonstrated effect on C compute code:

`instcombine`, `inline`, `loop-unroll`, `licm`, `gvn`, `sroa`, `mem2reg`, `simplifycfg`, `dse`, `reassociate`, `jump-threading`, `loop-rotate`, `adce`, `early-cse`, `tailcallelim`, `loop-vectorize`, `slp-vectorize`, `sccp`, `correlated-propagation`, `loop-idiom`, `indvars`, `aggressive-instcombine`, `mldst-motion`, `newgvn`, `loop-deletion`, `merge-func`, `div-rem-pairs` + STOP

### Secondary passes (`--features secondary_passes` — 72 actions total)

Adds 42 interprocedural, module-level, and niche function passes. Enable when the primary set converges but performance headroom remains.

## IR Feature Vector (18 dimensions)

Extracted by fast text-parsing of `.ll` files (<50ms):

| Feature | Description |
|---------|-------------|
| `add_count` | add/fadd/sub/fsub instructions |
| `mul_count` | mul/div/rem variants |
| `load_count` | load instructions |
| `store_count` | store instructions |
| `br_count` | branch/switch instructions |
| `call_count` | call/invoke instructions |
| `phi_count` | phi nodes |
| `alloca_count` | stack allocations |
| `gep_count` | getelementptr instructions |
| `icmp_count` | integer comparisons |
| `fcmp_count` | float comparisons |
| `ret_count` | return instructions |
| `other_inst_count` | all other instructions |
| `basic_block_count` | total basic blocks |
| `total_instruction_count` | all instructions |
| `function_count` | defined functions |
| `loop_depth_approx` | back-edge count (loop proxy) |
| `load_store_ratio` | load/store ratio |

## Commands

### collect — Gather training data

Runs random pass sequences on all benchmarks in parallel, recording IR features and timing at each step.

```bash
cargo run --release -- collect \
  --functions benchmarks/ \
  --num-sequences 800 \
  --output data/exploratory/

# Recommended: set CPU to performance mode first
sudo cpupower frequency-set -g performance
```

| Flag | Default | Description |
|------|---------|-------------|
| `--num-sequences` | `200` | Random sequences per benchmark |
| `--runs` | `3` | External binary launches per sequence |
| `--baseline-runs` | `5` | External launches per baseline (-O0/-O2/-O3) |
| `--bench-iters` | `51` | Internal timing iterations per launch |
| `--threads` | `0` (all cores) | Parallel worker count |

### eda — Exploratory data analysis

```bash
cargo run --release -- eda \
  --input data/exploratory/ \
  --output eda_output/
```

Outputs:
- `report.txt` — Human-readable summary with tables
- `function_stats.json` — Per-benchmark descriptive stats + baselines
- `pass_impact.json` — Per-pass avg time with vs without
- `pass_ordering.json` — Pairwise A→B vs B→A ordering effects
- `pass_ordering_triples.json` — All 6 permutations of 3-pass combos
- `ir_features_summary.json` — Feature vectors per benchmark

### baseline — Compute -O0/-O2/-O3 baselines

```bash
cargo run --release -- baseline \
  --functions benchmarks/ \
  --output data/baselines/
```

### evaluate — Compare methods against baselines

```bash
# Baselines + random search + greedy
cargo run --release -- evaluate \
  --functions benchmarks/ \
  --output results/ \
  --random-trials 100

# With a trained model
cargo run --release -- evaluate \
  --functions benchmarks/ \
  --output results/ \
  --model checkpoints/best
```

Results are cached — subsequent runs with a new `--model` reuse baseline/random caches instantly.

| Flag | Default | Description |
|------|---------|-------------|
| `--random-trials` | `50` | Random sequences to try per benchmark |
| `--model` | (none) | Path to trained model checkpoint |
| `--rerun-baselines` | false | Force recompute all caches |

### ordering-study — Pass ordering experiments

```bash
cargo run --release -- ordering-study \
  --functions benchmarks/ \
  --output eda_output/ordering/ \
  --experiments all
```

### test-pipeline — Smoke test compilation on a single file

```bash
cargo run --release -- test-pipeline \
  --file benchmarks/dot_product.c \
  --passes instcombine,sroa,simplifycfg
```

### features — Inspect IR feature vector

```bash
cargo run --release -- features --file benchmarks/fft.c
```

## Project Structure

```
benchmarks/         32 self-contained C benchmark programs
src/
  pass_menu.rs      Pass definitions + opt pipeline builder
  pipeline.rs       clang → opt → llc → link → benchmark
  ir_features.rs    Text-based IR feature extraction
  env.rs            RL environment
  dataset.rs        Random data collection
  eda.rs            Exploratory data analysis
  evaluation.rs     Baseline comparison harness
  ordering_study.rs Pass ordering experiments
  model/            LSTM policy + value network
  ppo.rs            PPO implementation
  rollout.rs        Rollout buffer
  training.rs       Training loop
data/               Collected datasets (gitignored)
eda_output/         EDA results
results/            Evaluation results
```

## Dependencies

- [burn](https://github.com/tracel-ai/burn) — deep learning framework (NDArray backend)
- [rayon](https://github.com/rayon-rs/rayon) — data parallelism
- [clap](https://github.com/clap-rs/clap) — CLI
- [serde](https://serde.rs/) / serde_json — serialization
- [statrs](https://github.com/statrs-dev/statrs) — statistics

## Tips

- Close other applications during data collection for cleaner timing
- Add new benchmarks by dropping `.c` files into `benchmarks/` — all commands auto-discover them
- `--runs 1` is sufficient for exploratory collection (each binary internally takes a 50-run median)
- Baselines default to 5 external runs × 50 internal iterations → median of 5 medians-of-50
