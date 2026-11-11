#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Cargo's qualified package ID ends with #name@version (or #version).
package_id="$(cargo pkgid --locked --package warded)"
package_id="${package_id%$'\r'}"
package_version="${package_id##*[@#]}"
package_target="${CARGO_TARGET_DIR:-target}"
package_manifest="$package_target/package/warded-$package_version/Cargo.toml"

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
cargo package --locked --target-dir "$package_target"
cargo run --manifest-path "$package_manifest" --example basic --locked
cargo run --manifest-path "$package_manifest" --example server --locked -- --check
