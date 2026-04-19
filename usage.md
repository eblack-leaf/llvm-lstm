```shell
cargo run --release --features wgpu,auto-tfx -- train --returns episode --episodes 4 --mini-batch-size 4 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin 
cargo run -- plot-train
cp checkpoints/train_plots.png checkpoints/auto-tfx-episode-4ep.png
```

```shell
cargo run --release --features wgpu,auto-tfx -- train --returns episode --episodes 16 --mini-batch-size 16 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin 
cargo run -- plot-train
cp checkpoints/train_plots.png checkpoints/auto-tfx-episode-16ep.png
```