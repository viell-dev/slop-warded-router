#!/usr/bin/env bash
set -euo pipefail

# Keep these paths aligned with the push filters in leptos-fixture.yml.
# Read all names even after finding a match so the Git producer can finish.
affected=false
while IFS= read -r -d '' path; do
    case "$path" in
        integration/leptos/*.md) ;;
        src/*|Cargo.toml|.cargo/*|rust-toolchain.toml|integration/leptos/*|\
        .github/workflows/leptos-fixture.yml|.github/scripts/fixture-affected.sh)
            affected=true
            ;;
    esac
done

printf 'affected=%s\n' "$affected"
