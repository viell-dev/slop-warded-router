#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all -- --check
cargo clippy --all-features --all-targets --locked -- -D warnings
cargo test --all-features --locked
cargo test --no-default-features --locked
cargo test --no-default-features --features tower --locked
cargo test --no-default-features --features negotiation --locked
export RUSTDOCFLAGS="-D warnings"
cargo doc --all-features --no-deps --locked
cargo run --example basic --locked
cargo run --example server --locked -- --check
cargo package --locked
cargo run --manifest-path target/package/warded-0.1.0-alpha.2/Cargo.toml --example basic --locked
cargo run --manifest-path target/package/warded-0.1.0-alpha.2/Cargo.toml --example server --locked -- --check
