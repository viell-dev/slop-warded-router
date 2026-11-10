#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test --locked --release --all-features
cargo test --locked --release --no-default-features
cargo clippy --locked --all-features --all-targets -- -D warnings
cargo fmt --all -- --check
