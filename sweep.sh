cargo run --release --features wgpu,auto-tfx -- train --returns weighted --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/auto-tfx-weighted-256ep
cargo run -- plot-train --dir checkpoints/auto-tfx-weighted-256ep
cp checkpoints/auto-tfx-weighted-256ep/train_plots.png checkpoints/auto-tfx-weighted-256ep.png

cargo run --release --features wgpu,auto-tfx -- train --returns predictor --predictor-checkpoint predictor_checkpoints --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data.cache --sequences-file checkpoints/bench-top.bin --checkpoint-dir checkpoints/auto-tfx-predictor-256ep
cargo run -- plot-train --dir checkpoints/auto-tfx-predictor-256ep
cp checkpoints/auto-tfx-predictor-256ep/train_plots.png checkpoints/auto-tfx-predictor-256ep.png

cargo run --release --features wgpu,auto-tfx -- train --returns ir-step --episodes 256 --mini-batch-size 128 --cache-file checkpoints/data-ir.cache --sequences-file checkpoints/bench-top-ir.bin --checkpoint-dir checkpoints/auto-tfx-ir-step-256ep
cargo run -- plot-train --dir checkpoints/auto-tfx-ir-step-256ep
cp checkpoints/auto-tfx-ir-step-256ep/train_plots.png checkpoints/auto-tfx-ir-step-256ep.png

