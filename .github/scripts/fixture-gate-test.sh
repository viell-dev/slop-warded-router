#!/usr/bin/env bash
set -euo pipefail

scripts="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/warded-fixture-gate.XXXXXXXX")"
trap 'rm -rf -- "$scratch"' EXIT
cd "$scratch"

git init -q
git config user.name "Fixture gate test"
git config user.email "fixture@example.invalid"
git config commit.gpgSign false
git config core.hooksPath "$scratch/.git/no-hooks"
git config core.quotePath true
git config diff.renames true
git config core.autocrlf false
mkdir -p integration/leptos/public docs src .github/workflows .github/scripts
printf 'asset\n' > integration/leptos/public/favicon.ico
printf '// old source\n' > src/old.rs
git add -- integration/leptos/public/favicon.ico src/old.rs
git commit -qm 'Base files'

expect_detection() {
    local label="$1" expected="affected=$2" actual
    git add -A
    git commit -q --allow-empty -m "$label"
    actual="$(bash "$scripts/fixture-affected.sh" HEAD^1 HEAD)"
    if [[ "$actual" != "$expected" ]]; then
        printf '%s: expected %s, got %s\n' "$label" "$expected" "$actual" >&2
        exit 1
    fi
    printf '%s: %s\n' "$label" "$actual"
}

git mv integration/leptos/public/favicon.ico docs/favicon.ico
expect_detection 'Asset rename out of the fixture' true
git mv src/old.rs docs/old.rs
expect_detection 'Source rename out of src' true
printf '// new source\n' > src/café.rs
expect_detection 'Non-ASCII source path' true
printf 'fixture notes\n' > integration/leptos/notes.md
expect_detection 'Fixture Markdown only' false
printf 'root lockfile\n' > Cargo.lock
expect_detection 'Library lockfile only' false
printf '# workflow change\n' > .github/workflows/leptos-fixture.yml
expect_detection 'Workflow only' true
printf '# gate script change\n' > .github/scripts/fixture-required.sh
expect_detection 'Gate script only' true
expect_detection 'Empty diff' false

if output="$(bash "$scripts/fixture-affected.sh" missing-revision HEAD 2>/dev/null)" ||
    [[ -n "$output" ]]; then
    echo 'Git error must fail without a decision' >&2
    exit 1
fi
echo 'Git error: failed with no decision'

expect_aggregate() {
    local expected="$1" actual=0
    shift
    bash "$scripts/fixture-required.sh" "$@" 2>/dev/null || actual=$?
    if [[ "$actual" != "$expected" ]]; then
        printf 'Aggregate inputs:' >&2
        printf ' %q' "$@" >&2
        printf '; expected exit %s, got %s\n' "$expected" "$actual" >&2
        exit 1
    fi
}

# Test every workflow result against true, false, empty and invalid decisions.
results=(success failure cancelled skipped '')
for changes in "${results[@]}"; do
    for affected in true false '' unknown; do
        for fixture in "${results[@]}"; do
            expected=1
            if [[ "$changes" == success ]] &&
                [[ "$affected" == true && "$fixture" == success ||
                    "$affected" == false && "$fixture" == skipped ]]; then
                expected=0
            fi
            expect_aggregate "$expected" "$changes" "$affected" "$fixture"
        done
    done
done
expect_aggregate 2
expect_aggregate 2 success
expect_aggregate 2 success true
echo 'Aggregate: passed all 100 state combinations and three missing-argument checks'
