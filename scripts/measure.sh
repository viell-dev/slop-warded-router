#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
cd "$(dirname "$0")/.."
mkdir -p results
rustc --version > results/environment.txt
uname -a >> results/environment.txt
printf 'Timing: release, default features, system allocator, 100000 operations/sample, 9 samples/process, 3 processes; warmup 5000/scenario.\n' >> results/environment.txt
cargo build --locked --release
for run in 1 2 3; do
  ./target/release/warded-path-constraint-probe bench > "results/timing-$run.csv"
done
awk -f scripts/summarize.awk results/timing-1.csv results/timing-2.csv results/timing-3.csv > results/summary.csv
cargo run --locked --release --features count-allocs -- alloc > results/allocations.csv
