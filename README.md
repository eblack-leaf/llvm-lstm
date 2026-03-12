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

### Current results (from 76,000 random sequences)

| Metric | Value |
|--------|-------|
| Benchmarks that beat -O3 | 15/38 (best-of-2000 random search) |
| Reachable (<20% gap to O3) | 16/38 |
| Total trainable targets | 31/38 |
| Top pass enrichment | `inline` 2.37x, `sroa` 2.02x, `mem2reg` 1.86x |

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

38 self-contained C benchmark programs in `benchmarks/`. Each measures its own execution time internally using `clock_gettime` and prints the median over multiple iterations. Covers a range of compute patterns:

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

### eda — Analysis and visualization

Analyzes collected data and generates a report with SVG visualizations.

```bash
cargo run --release -- eda \
  --input data/exploratory/ \
  --output eda_output/ \
  --functions benchmarks/
```

The `--functions` flag is optional — when provided, it extracts IR features from each benchmark, runs t-SNE dimensionality reduction, and clusters benchmarks by IR similarity.

Outputs:
- `report.txt` — Full analysis with 6 sections (see below)
- `baselines.json` — O0/O2/O3 per benchmark with speedup ratios
- `ceiling.json` — Best-of-random-search vs O3/O2/O0 per benchmark
- `distributions.json` — Quantile stats (P10/P25/Med/P75/P90, CV%)
- `pass_enrichment.json` — Per-pass enrichment in top-10% sequences
- `ir_features.json` — Pre-optimization IR features with t-SNE + cluster assignments
- `tsne.csv` — t-SNE coordinates for external plotting
- `tsne_clusters.svg` — t-SNE scatter colored by cluster, shaped by reachability
- `ceiling_gaps.svg` — Horizontal bar chart: gap% vs O3 and O2
- `pass_enrichment.svg` — Enrichment ratio per pass with 1.0x reference line
- `distributions.svg` — Box-plot style time distributions with O3 markers

Report sections:
1. **Baseline Landscape** — O0/O2/O3 times and speedup ratios per benchmark
2. **Ceiling Analysis** — Best random search result vs all baselines, achievability classification
3. **Performance Distributions** — Quantiles and CV% showing sensitivity to pass choice
4. **Pass Enrichment** — Which passes appear disproportionately in top-performing sequences
5. **IR Feature Landscape** — Pre-optimization IR features, k-means clusters, t-SNE embedding
6. **Actionable Summary** — Benchmarks classified as beats-O3 / reachable / needs-pipeline-work

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
benchmarks/         38 self-contained C benchmark programs
src/
  pass_menu.rs      Pass definitions + opt pipeline builder
  pipeline.rs       clang → opt → llc → link → benchmark
  ir_features.rs    Text-based IR feature extraction
  env.rs            RL environment
  dataset.rs        Random data collection
  eda.rs            Analysis and reporting
  plots.rs          SVG visualization (plotters)
  evaluation.rs     Baseline comparison harness
  model/            LSTM policy + value network
  ppo.rs            PPO implementation
  rollout.rs        Rollout buffer
  training.rs       Training loop
data/               Collected datasets (gitignored)
eda_output/         Analysis results + SVG plots
results/            Evaluation results
```

## Dependencies

- [burn](https://github.com/tracel-ai/burn) — deep learning framework (NDArray backend)
- [rayon](https://github.com/rayon-rs/rayon) — data parallelism
- [plotters](https://github.com/plotters-rs/plotters) — SVG chart generation
- [bhtsne](https://github.com/frjnn/bhtsne) — t-SNE dimensionality reduction
- [clap](https://github.com/clap-rs/clap) — CLI
- [serde](https://serde.rs/) / serde_json — serialization

## Tips

- Close other applications during data collection for cleaner timing
- Add new benchmarks by dropping `.c` files into `benchmarks/` — all commands auto-discover them
- `--runs 1` is sufficient for exploratory collection (each binary internally takes a 50-run median)
- Baselines default to 5 external runs x 50 internal iterations
- The pipeline uses `clang -O3 -Xclang -disable-llvm-optzns` to get frontend-annotated IR (TBAA, lifetime markers) without applying LLVM optimization passes — this is the pre-optimization IR the model sees
