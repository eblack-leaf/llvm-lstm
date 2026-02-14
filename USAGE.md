# llvm-lstm Usage

## Build

```bash
cargo build --release
```

## Commands

### collect — Gather exploratory data

Runs random pass sequences on all benchmarks in parallel (one thread per function), records timing + IR features.

```bash
cargo run --release -- collect \
  --functions benchmarks/ \
  --num-sequences 800 \
  --output data/exploratory/ \
  --runs 1
```

| Flag | Default | Description |
|------|---------|-------------|
| `--functions` | `benchmarks/` | Directory of `.c` benchmark files |
| `--num-sequences` | `200` | Random pass sequences per function |
| `--output` | `data/exploratory/` | Output dir (writes `exploratory.jsonl` + `baselines.jsonl`) |
| `--runs` | `3` | Times to launch each compiled binary per sequence (1 is fine — C code already takes median of 50 internally) |
| `--baseline-runs` | `50` | Times to launch each baseline binary (-O0/-O2/-O3). Higher = more stable reference. |

Collection is parallelized across functions using rayon. Each function gets its own work directory. Baselines run sequentially for stable timing.

### eda — Exploratory data analysis

Reads collected data, produces analysis JSON files and a human-readable report.

```bash
cargo run --release -- eda \
  --input data/exploratory/ \
  --output eda_output/
```

Outputs:
- `report.txt` — Human-readable summary with tables and findings
- `function_stats.json` — Per-function descriptive stats + baselines
- `pass_impact.json` — Per-pass avg time with vs without
- `pass_ordering.json` — A→B vs B→A pairwise ordering effects (min 10 samples per direction)
- `pass_ordering_triples.json` — All 6 permutations of 3-pass combinations (min 10 samples per permutation)
- `ir_features_summary.json` — IR feature vectors per function

### baseline — Compute baselines only

```bash
cargo run --release -- baseline \
  --functions benchmarks/ \
  --output data/baselines/
```

| Flag | Default | Description |
|------|---------|-------------|
| `--functions` | `benchmarks/` | Directory of `.c` benchmark files |
| `--output` | `data/baselines/` | Output directory |
| `--baseline-runs` | `50` | Times to launch each baseline binary |

Writes `baselines.jsonl` with -O0, -O2, -O3 times for each function.

### test-pipeline — Test compilation pipeline on a single file

```bash
cargo run --release -- test-pipeline \
  --file benchmarks/dot_product.c \
  --passes instcombine,sroa,simplifycfg
```

Runs the full pipeline (emit IR → optimize → compile → benchmark) and prints results. Useful for verifying passes work.

Available passes: `instcombine`, `inline`, `loop-unroll`, `licm`, `gvn`, `sroa`, `mem2reg`, `simplifycfg`, `dse`, `reassociate`, `jump-threading`, `loop-rotate`, `adce`, `early-cse`, `tailcallelim`

### features — Extract IR features from a file

```bash
# From C source (emits IR first)
cargo run --release -- features --file benchmarks/fft.c

# From .ll file directly
cargo run --release -- features --file /tmp/llvm-lstm-test/fft_opt.ll
```

Prints the 18-dimensional feature vector as JSON.

### evaluate — Compare methods against baselines

```bash
# First run: computes baselines + random + greedy
cargo run --release -- evaluate \
  --functions benchmarks/ \
  --output results/ \
  --random-trials 50

# Subsequent runs: reuses cached baselines
cargo run --release -- evaluate \
  --functions benchmarks/ \
  --output results/ \
  --random-trials 100

# Force recompute baselines
cargo run --release -- evaluate \
  --functions benchmarks/ \
  --output results/ \
  --rerun-baselines

# With trained model (once implemented)
cargo run --release -- evaluate \
  --functions benchmarks/ \
  --output results/ \
  --model checkpoints/best
```

| Flag | Default | Description |
|------|---------|-------------|
| `--functions` | `benchmarks/` | Directory of `.c` benchmark files |
| `--output` | `results/` | Output dir for results and caches |
| `--random-trials` | `50` | Random sequences to try per function |
| `--model` | (none) | Path to trained model checkpoint |
| `--rerun-baselines` | `false` | Force recompute all caches |

All comparison methods are cached after first run:
- `baselines_cache.json` — -O0/-O2/-O3 results
- `greedy_cache.json` — best single-pass results
- `random_50_cache.json` — random search results (filename includes trial count)

Subsequent `--model` runs load caches instantly and only run model inference.

### train — Train the agent (not yet implemented)

```bash
cargo run --release -- train --config configs/train.toml
```

## Tips

- Set CPU to performance mode before collecting data:
  ```bash
  sudo cpupower frequency-set -g performance
  ```
- Close other applications during data collection for cleaner timing
- Add new benchmarks by dropping `.c` files into `benchmarks/` — all commands auto-discover them
- Use `--runs 1` for exploratory data collection; `--runs 3` for final evaluation
- Baselines default to 50 runs for stable reference numbers (each run internally does 50 iterations with warmup)
