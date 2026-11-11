#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
export CI=1
export LEPTOS_OUTPUT_NAME=warded-leptos-probe
export LEPTOS_SITE_ROOT=target/site
export LEPTOS_SITE_ADDR=127.0.0.1:3000
export LEPTOS_SITE_PKG_DIR=pkg
mkdir -p artifacts
[[ "$(node --version)" == "v$(cat .nvmrc)" ]]
cargo fmt --check
cargo clippy --locked --no-default-features --features ssr --bin warded-leptos-probe -- -D warnings
cargo clippy --locked --no-default-features --features hydrate --target wasm32-unknown-unknown --lib -- -D warnings
cargo leptos build --cargo-locked
(cd end2end && npm ci --no-audit --no-fund && npx tsc --noEmit)

server_pid=""
cleanup() {
    local status=$?
    trap - EXIT INT TERM
    if [[ -n "$server_pid" ]]; then
        kill "$server_pid" 2>/dev/null || true
        if ! timeout 10s tail --pid="$server_pid" -f /dev/null; then
            kill -KILL "$server_pid" 2>/dev/null || true
            status=1
        fi
        wait "$server_pid" 2>/dev/null || true
        if kill -0 "$server_pid" 2>/dev/null; then
            echo "fixture server survived cleanup" >&2
            status=1
        fi
        echo "Verified server cleanup: $server_pid"
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

run() {
    local mode="$1" mutation="$2" report="$PWD/artifacts/$3.json" status=0 ready=0
    rm -f -- "$report"
    export WARDED_FIXTURE_ROUTER="$mode" WARDED_FIXTURE_REPORT="$report"
    if [[ -n "$mutation" ]]; then
        export WARDED_FIXTURE_MUTATION="$mutation"
    else
        unset WARDED_FIXTURE_MUTATION
    fi
    target/debug/warded-leptos-probe >"artifacts/$3-server.log" 2>&1 &
    server_pid=$!
    for attempt in {1..100}; do
        kill -0 "$server_pid" 2>/dev/null || break
        if curl --fail --silent --max-time 1 http://127.0.0.1:3000/ >/dev/null; then
            ready=1
            break
        fi
        sleep 0.1
    done
    if [[ "$ready" != 1 ]]; then
        cat "artifacts/$3-server.log" >&2
        return 1
    fi
    if [[ "${4:-}" == force-failure ]]; then
        printf '%s\n' "$server_pid" >artifacts/failure-cleanup.pid
        exit 97
    fi
    if [[ -n "$mutation" ]]; then
        (cd end2end && timeout 120s npx playwright test --grep 'value predicates select ID or username') || status=$?
        [[ "$status" == 1 ]] || { echo "negative control did not fail as expected: $status" >&2; return 1; }
        node end2end/check-report.mjs "$report" negative
    else
        (cd end2end && timeout 300s npx playwright test) || status=$?
        [[ "$status" == 0 ]] || return "$status"
        node end2end/check-report.mjs "$report" "$mode"
    fi
    kill "$server_pid"
    timeout 10s tail --pid="$server_pid" -f /dev/null
    wait "$server_pid" 2>/dev/null || true
    if kill -0 "$server_pid" 2>/dev/null; then return 1; fi
    echo "Verified server cleanup: $server_pid"
    server_pid=""
}

case "${1:-}" in
    --failure-cleanup-control)
        run warded "" cleanup force-failure
        ;;
    "")
        run axum "" axum
        run warded "" warded
        run warded reject-id negative
        # The supervisor remains alive in the same container: container teardown
        # cannot mask a server leaked by the child's failing EXIT path.
        status=0
        bash check.sh --failure-cleanup-control || status=$?
        [[ "$status" == 97 ]] || { echo "cleanup control exit: $status" >&2; exit 1; }
        leaked_pid="$(cat artifacts/failure-cleanup.pid)"
        if kill -0 "$leaked_pid" 2>/dev/null; then
            echo "failure cleanup left server $leaked_pid alive" >&2
            exit 1
        fi
        if curl --silent --max-time 1 http://127.0.0.1:3000/ >/dev/null; then
            echo "failure cleanup left the fixture port open" >&2
            exit 1
        fi
        echo "Verified failing-run cleanup and preserved exit 97"
        ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
esac
