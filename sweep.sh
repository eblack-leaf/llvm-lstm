cargo run --release --features wgpu,auto-tfx -- train --returns episode --advantages grpo --value-coef 0.0 --episodes 256 --mini-batch-size 256 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/auto-tfx-grpo-256ep
cargo run -- plot-train --dir checkpoints/auto-tfx-grpo-256ep
cp checkpoints/auto-tfx-grpo-256ep/train_plots.png checkpoints/auto-tfx-grpo-256ep.png

cargo run --release --features wgpu,auto-tfx -- train --returns episode --advantages baseline --episodes 256 --mini-batch-size 256 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/auto-tfx-episode-256ep
cargo run -- plot-train --dir checkpoints/auto-tfx-episode-256ep
cp checkpoints/auto-tfx-episode-256ep/train_plots.png checkpoints/auto-tfx-episode-256ep.png


