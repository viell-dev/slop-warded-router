#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Cargo's qualified package ID ends with #name@version (or #version).
package_id="$(cargo pkgid --locked --package warded)"
package_id="${package_id%$'\r'}"
package_version="${package_id##*[@#]}"
package_target="${CARGO_TARGET_DIR:-target}"
package_manifest="$package_target/package/warded-$package_version/Cargo.toml"

# Print matching lines, or nothing when none match; other grep failures still stop the check.
matches() {
    grep "$@" || (($? == 1))
}

# Link destinations outside fenced code: inline links and images, and reference definitions,
# including one whose destination is on the next line. Angle brackets and titles are dropped.
readme_links() {
    awk '
        function emit(destination) {
            sub(/^</, "", destination)
            sub(/>$/, "", destination)
            print destination
        }
        /^ *(```|~~~)/ { fenced = !fenced; next }
        fenced { next }
        pending { if (NF) { emit($1); pending = 0 }; next }
        /^ *\[[^]]+\]:/ {
            rest = $0
            sub(/^ *\[[^]]+\]: */, "", rest)
            if (rest == "") { pending = 1 } else { split(rest, field, " "); emit(field[1]) }
            next
        }
        {
            line = $0
            while (match(line, /\]\([^)]*\)/)) {
                split(substr(line, RSTART + 2, RLENGTH - 3), field, " ")
                emit(field[1])
                line = substr(line, RSTART + RLENGTH)
            }
        }
    ' README.md
}

# A published README cannot be corrected, so it must pin the version being packaged, and its
# install pins, version-pinned links and the changelog's latest release heading must name only
# that version. crates.io resolves relative README links against the default branch, so every
# link must be absolute or an in-page anchor.
pin_versions="$(
    matches -oE "warded@=[^' ]+|^warded *= *\"=[^\"]+\"|^warded *= *\{.*version *= *\"=[^\"]+\"" \
        README.md | sed -E 's/.*=//; s/"$//'
)"
link_versions="$(
    matches -oE 'slop-warded-router/(blob|tree)/[0-9][^/)#>]*|(crates|docs\.rs)/warded/[0-9][^/)#>]*' \
        README.md | sed -E 's#.*/##; s/[.,;:]+$//'
)"
changelog_version="$(matches -m 1 -E '^## [0-9]' CHANGELOG.md | sed -E 's/^## ([^ ]+).*/\1/')"
release_versions="$(
    printf '%s\n' "$pin_versions" "$link_versions" "$changelog_version" | sed '/^$/d' | sort -u
)"
if [[ "$release_versions" != "$package_version" ]] ||
    ! grep -qF "warded = \"=$package_version\"" README.md; then
    printf 'README and CHANGELOG must pin and name only %s; found:\n%s\n' "$package_version" \
        "$release_versions" >&2
    exit 1
fi
links="$(readme_links)"
relative_links="$(matches -vE '^(https?://|#|mailto:)' <<< "$links")"
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
