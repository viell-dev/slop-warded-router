#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "Usage: fixture-affected.sh <base> <head>" >&2
    exit 2
fi

classify_paths() {
    # Keep these paths aligned with the push filters in leptos-fixture.yml.
    # Read all names even after finding a match so the Git producer can finish.
    local affected=false path
    while IFS= read -r -d '' path; do
        case "$path" in
            integration/leptos/*.md) ;;
            src/*|Cargo.toml|.cargo/*|rust-toolchain.toml|integration/leptos/*|\
            .github/workflows/leptos-fixture.yml|.github/scripts/fixture-*.sh)
                affected=true
                ;;
        esac
    done
    printf '%s\n' "$affected"
}

# List both sides of renames separately and preserve raw path bytes.
decision="$(git -c core.quotePath=false diff --no-renames -z --name-only "$1" "$2" -- |
    classify_paths)"
printf 'affected=%s\n' "$decision"
