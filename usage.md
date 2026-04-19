# sweep grid — arch × returns @ 256 episodes

mb rule: episodes/2, floor 4, cap 128
ir / ir-step use data-ir.cache + bench-top-ir.bin
predictor needs predictor_checkpoints to exist

---

## seq (default arch)

### episode

```shell
cargo run --release --features wgpu -- train --returns episode --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/seq-episode-256ep
cargo run -- plot-train --dir checkpoints/seq-episode-256ep
cp checkpoints/seq-episode-256ep/train_plots.png checkpoints/seq-episode-256ep.png
```

### weighted

```shell
cargo run --release --features wgpu -- train --returns weighted --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/seq-weighted-256ep
cargo run -- plot-train --dir checkpoints/seq-weighted-256ep
cp checkpoints/seq-weighted-256ep/train_plots.png checkpoints/seq-weighted-256ep.png
```

### proxy

```shell
cargo run --release --features wgpu -- train --returns proxy --proxy-alpha 0.5 --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/seq-proxy-256ep
cargo run -- plot-train --dir checkpoints/seq-proxy-256ep
cp checkpoints/seq-proxy-256ep/train_plots.png checkpoints/seq-proxy-256ep.png
```

### predictor

```shell
cargo run --release --features wgpu -- train --returns predictor --predictor-checkpoint predictor_checkpoints --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/seq-predictor-256ep
cargo run -- plot-train --dir checkpoints/seq-predictor-256ep
cp checkpoints/seq-predictor-256ep/train_plots.png checkpoints/seq-predictor-256ep.png
```

### ir

```shell
cargo run --release --features wgpu -- train --returns ir --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data-ir.cache --sequences-file checkpoints/bench-top-ir.bin --checkpoint-dir checkpoints/seq-ir-256ep
cargo run -- plot-train --dir checkpoints/seq-ir-256ep
cp checkpoints/seq-ir-256ep/train_plots.png checkpoints/seq-ir-256ep.png
```

### ir-step

```shell
cargo run --release --features wgpu -- train --returns ir-step --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data-ir.cache --sequences-file checkpoints/bench-top-ir.bin --checkpoint-dir checkpoints/seq-ir-step-256ep
cargo run -- plot-train --dir checkpoints/seq-ir-step-256ep
cp checkpoints/seq-ir-step-256ep/train_plots.png checkpoints/seq-ir-step-256ep.png
```

---

## auto-tfx

### episode

```shell
cargo run --release --features wgpu,auto-tfx -- train --returns episode --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/auto-tfx-episode-256ep
cargo run -- plot-train --dir checkpoints/auto-tfx-episode-256ep
cp checkpoints/auto-tfx-episode-256ep/train_plots.png checkpoints/auto-tfx-episode-256ep.png
```

### weighted

```shell
cargo run --release --features wgpu,auto-tfx -- train --returns weighted --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/auto-tfx-weighted-256ep
cargo run -- plot-train --dir checkpoints/auto-tfx-weighted-256ep
cp checkpoints/auto-tfx-weighted-256ep/train_plots.png checkpoints/auto-tfx-weighted-256ep.png
```

### proxy

```shell
cargo run --release --features wgpu,auto-tfx -- train --returns proxy --proxy-alpha 0.5 --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/auto-tfx-proxy-256ep
cargo run -- plot-train --dir checkpoints/auto-tfx-proxy-256ep
cp checkpoints/auto-tfx-proxy-256ep/train_plots.png checkpoints/auto-tfx-proxy-256ep.png
```

### predictor

```shell
cargo run --release --features wgpu,auto-tfx -- train --returns predictor --predictor-checkpoint predictor_checkpoints --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/auto-tfx-predictor-256ep
cargo run -- plot-train --dir checkpoints/auto-tfx-predictor-256ep
cp checkpoints/auto-tfx-predictor-256ep/train_plots.png checkpoints/auto-tfx-predictor-256ep.png
```

### ir

```shell
cargo run --release --features wgpu,auto-tfx -- train --returns ir --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data-ir.cache --sequences-file checkpoints/bench-top-ir.bin --checkpoint-dir checkpoints/auto-tfx-ir-256ep
cargo run -- plot-train --dir checkpoints/auto-tfx-ir-256ep
cp checkpoints/auto-tfx-ir-256ep/train_plots.png checkpoints/auto-tfx-ir-256ep.png
```

### ir-step

```shell
cargo run --release --features wgpu,auto-tfx -- train --returns ir-step --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data-ir.cache --sequences-file checkpoints/bench-top-ir.bin --checkpoint-dir checkpoints/auto-tfx-ir-step-256ep
cargo run -- plot-train --dir checkpoints/auto-tfx-ir-step-256ep
cp checkpoints/auto-tfx-ir-step-256ep/train_plots.png checkpoints/auto-tfx-ir-step-256ep.png
```

---

## auto-gru

### episode

```shell
cargo run --release --features wgpu,auto-gru -- train --returns episode --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/auto-gru-episode-256ep
cargo run -- plot-train --dir checkpoints/auto-gru-episode-256ep
cp checkpoints/auto-gru-episode-256ep/train_plots.png checkpoints/auto-gru-episode-256ep.png
```

### weighted

```shell
cargo run --release --features wgpu,auto-gru -- train --returns weighted --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/auto-gru-weighted-256ep
cargo run -- plot-train --dir checkpoints/auto-gru-weighted-256ep
cp checkpoints/auto-gru-weighted-256ep/train_plots.png checkpoints/auto-gru-weighted-256ep.png
```

### proxy

```shell
cargo run --release --features wgpu,auto-gru -- train --returns proxy --proxy-alpha 0.5 --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/auto-gru-proxy-256ep
cargo run -- plot-train --dir checkpoints/auto-gru-proxy-256ep
cp checkpoints/auto-gru-proxy-256ep/train_plots.png checkpoints/auto-gru-proxy-256ep.png
```

### predictor

```shell
cargo run --release --features wgpu,auto-gru -- train --returns predictor --predictor-checkpoint predictor_checkpoints --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/auto-gru-predictor-256ep
cargo run -- plot-train --dir checkpoints/auto-gru-predictor-256ep
cp checkpoints/auto-gru-predictor-256ep/train_plots.png checkpoints/auto-gru-predictor-256ep.png
```

### ir

```shell
cargo run --release --features wgpu,auto-gru -- train --returns ir --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data-ir.cache --sequences-file checkpoints/bench-top-ir.bin --checkpoint-dir checkpoints/auto-gru-ir-256ep
cargo run -- plot-train --dir checkpoints/auto-gru-ir-256ep
cp checkpoints/auto-gru-ir-256ep/train_plots.png checkpoints/auto-gru-ir-256ep.png
```

### ir-step

```shell
cargo run --release --features wgpu,auto-gru -- train --returns ir-step --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data-ir.cache --sequences-file checkpoints/bench-top-ir.bin --checkpoint-dir checkpoints/auto-gru-ir-step-256ep
cargo run -- plot-train --dir checkpoints/auto-gru-ir-step-256ep
cp checkpoints/auto-gru-ir-step-256ep/train_plots.png checkpoints/auto-gru-ir-step-256ep.png
```

---

## conclave

### episode

```shell
cargo run --release --features wgpu,conclave -- train --returns episode --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/conclave-episode-256ep
cargo run -- plot-train --dir checkpoints/conclave-episode-256ep
cp checkpoints/conclave-episode-256ep/train_plots.png checkpoints/conclave-episode-256ep.png
```

### weighted

```shell
cargo run --release --features wgpu,conclave -- train --returns weighted --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/conclave-weighted-256ep
cargo run -- plot-train --dir checkpoints/conclave-weighted-256ep
cp checkpoints/conclave-weighted-256ep/train_plots.png checkpoints/conclave-weighted-256ep.png
```

### proxy

```shell
cargo run --release --features wgpu,conclave -- train --returns proxy --proxy-alpha 0.5 --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/conclave-proxy-256ep
cargo run -- plot-train --dir checkpoints/conclave-proxy-256ep
cp checkpoints/conclave-proxy-256ep/train_plots.png checkpoints/conclave-proxy-256ep.png
```

### predictor

```shell
cargo run --release --features wgpu,conclave -- train --returns predictor --predictor-checkpoint predictor_checkpoints --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/conclave-predictor-256ep
cargo run -- plot-train --dir checkpoints/conclave-predictor-256ep
cp checkpoints/conclave-predictor-256ep/train_plots.png checkpoints/conclave-predictor-256ep.png
```

### ir

```shell
cargo run --release --features wgpu,conclave -- train --returns ir --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data-ir.cache --sequences-file checkpoints/bench-top-ir.bin --checkpoint-dir checkpoints/conclave-ir-256ep
cargo run -- plot-train --dir checkpoints/conclave-ir-256ep
cp checkpoints/conclave-ir-256ep/train_plots.png checkpoints/conclave-ir-256ep.png
```

### ir-step

```shell
cargo run --release --features wgpu,conclave -- train --returns ir-step --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data-ir.cache --sequences-file checkpoints/bench-top-ir.bin --checkpoint-dir checkpoints/conclave-ir-step-256ep
cargo run -- plot-train --dir checkpoints/conclave-ir-step-256ep
cp checkpoints/conclave-ir-step-256ep/train_plots.png checkpoints/conclave-ir-step-256ep.png
```
