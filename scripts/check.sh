#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Cargo's qualified package ID ends with #name@version (or #version).
package_id="$(cargo pkgid --locked --package warded)"
package_id="${package_id%$'\r'}"
package_version="${package_id##*[@#]}"
package_target="${CARGO_TARGET_DIR:-target}"
package_manifest="$package_target/package/warded-$package_version/Cargo.toml"

# A published README cannot be corrected, so it must pin the version being packaged, and its
# install pins, version-pinned links and the changelog's latest release heading must name only
# that version. crates.io resolves relative README links against the default branch, so every
# link must be absolute or an in-page anchor.
release_versions="$(
    {
        grep -oE "warded@=[^' ]+|warded = (\{ *version = )?\"=[^\"]+\"" README.md |
            grep -oE '[0-9][^"]*'
        grep -oE 'slop-warded-router/(blob|tree)/[0-9][^/)#]*|(crates|docs\.rs)/warded/[0-9][^/)#]*' \
            README.md | sed -E 's#.*/##; s/[.,;:]+$//'
        grep -m 1 -E '^## [0-9]' CHANGELOG.md | sed -E 's/^## ([^ ]+).*/\1/'
    } | sort -u
)"
if [[ "$release_versions" != "$package_version" ]] ||
    ! grep -qF "warded = \"=$package_version\"" README.md; then
    printf 'README and CHANGELOG must pin and name only %s; found:\n%s\n' "$package_version" \
        "$release_versions" >&2
    exit 1
fi
relative_links="$(
    {
        grep -oE '\]\([^)]+\)' README.md | sed -E 's/^\]\(//; s/\)$//'
        grep -oE '^\[[^]]+\]: +[^ ]+' README.md | sed -E 's/^\[[^]]+\]: +//'
    } | grep -vE '^(https?://|#|mailto:)' || true
)"
if [[ -n "$relative_links" ]]; then
    printf 'README links must be absolute for crates.io; found:\n%s\n' "$relative_links" >&2
    exit 1
fi

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
