#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
export CI=1
export LEPTOS_OUTPUT_NAME=warded-leptos-probe
export LEPTOS_SITE_ROOT=target/site
export LEPTOS_SITE_ADDR=127.0.0.1:0
export LEPTOS_SITE_PKG_DIR=pkg
if [[ ! "${WARDED_FIXTURE_REVISION:-}" =~ ^(commit:[a-f0-9]{40}|archive-sha256:[a-f0-9]{64})$ ]]; then
    echo "Set WARDED_FIXTURE_REVISION to commit:<Git SHA> or archive-sha256:<archive SHA-256>" >&2
    exit 2
fi
export WARDED_FIXTURE_REVISION

server_pid=""
server_url=""
stop_server() {
    if [[ -z "$server_pid" ]]; then return 0; fi
    local status=0
    kill "$server_pid" 2>/dev/null || true
    if ! timeout 10s tail --pid="$server_pid" -f /dev/null; then
        kill -KILL "$server_pid" 2>/dev/null || true
        status=1
    fi
    wait "$server_pid" 2>/dev/null || true
    if kill -0 "$server_pid" 2>/dev/null; then
        echo "Fixture server $server_pid survived cleanup" >&2
        status=1
    fi
    if [[ "$status" == 0 ]]; then echo "Verified server cleanup: $server_pid"; fi
    server_pid=""
    return "$status"
}
cleanup() {
    local status=$?
    trap - EXIT
    trap '' INT TERM
    stop_server || status=1
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

run() {
    local mode="$1" mutation="$2" variant="$3" status=0 identity="" expected_identity="$1"
    local variant_dir="$artifact_dir/$variant"
    mkdir "$variant_dir"
    export WARDED_FIXTURE_ROUTER="$mode" WARDED_FIXTURE_REPORT="$variant_dir/results.json"
    if [[ -n "$mutation" ]]; then
        export WARDED_FIXTURE_MUTATION="$mutation"
        expected_identity="$mode/$mutation"
    else
        unset WARDED_FIXTURE_MUTATION
    fi

    local log="$variant_dir/server.log"
    target/debug/warded-leptos-probe >"$log" 2>&1 &
    server_pid=$!
    server_url=""
    for attempt in {1..100}; do
        kill -0 "$server_pid" 2>/dev/null || break
        server_url="$(sed -n 's/^fixture listening on \(http:\/\/127\.0\.0\.1:[1-9][0-9]*\)$/\1/p' "$log")"
        if [[ -n "$server_url" ]] && identity="$(curl --noproxy '*' --fail --silent --max-time 1 "$server_url/__test/router")"; then
            break
        fi
        sleep 0.1
    done
    if [[ "$identity" != "$expected_identity" ]]; then
        echo "Expected live router $expected_identity, received '${identity}'" >&2
        cat "$log" >&2
        return 1
    fi
    export WARDED_FIXTURE_URL="$server_url" WARDED_FIXTURE_IDENTITY="$identity"
    printf 'revision=%s\nrouter=%s\nurl=%s\npid=%s\n' \
        "$WARDED_FIXTURE_REVISION" "$identity" "$server_url" "$server_pid" | tee "$variant_dir/server-identity.txt"

    if [[ "$variant" == cleanup ]]; then
        exit 97
    fi
    if [[ "$variant" == negative ]]; then
        (cd end2end && timeout 120s npx --no-install playwright test --grep 'value predicates select ID or username') || status=$?
        [[ "$status" == 1 ]] || { echo "Negative control exited $status, expected 1" >&2; return 1; }
    else
        (cd end2end && timeout 300s npx --no-install playwright test) || status=$?
    fi
    node end2end/check-report.mjs "$WARDED_FIXTURE_REPORT" "$variant"
    if [[ "$variant" != negative && "$status" != 0 ]]; then return "$status"; fi
    kill -0 "$server_pid"
    stop_server
}

case "${1:-}" in
    --failure-cleanup-control)
        artifact_dir="$2"
        run warded "" cleanup
        ;;
    "")
        mkdir -p artifacts
        artifact_dir="$(mktemp -d "$PWD/artifacts/run.XXXXXXXX")"
        printf '%s\n' "$WARDED_FIXTURE_REVISION" | tee "$artifact_dir/source-revision.txt"
        node_version="$(node --version)"
        expected_node_version="v$(cat .nvmrc)"
        if [[ "$node_version" != "$expected_node_version" ]]; then
            echo "Fixture requires Node $expected_node_version, found $node_version" >&2
            exit 1
        fi
        cargo fmt --check
        cargo clippy --locked --no-default-features --features ssr --bin warded-leptos-probe -- -D warnings
        cargo clippy --locked --no-default-features --features hydrate --target wasm32-unknown-unknown --lib -- -D warnings
        cargo leptos build --cargo-locked
        (cd end2end && npm ci --no-audit --no-fund && npx --no-install tsc --noEmit)
        run axum "" axum
        run warded "" warded
        run warded reject-id negative

        # The supervisor stays in the same container so teardown cannot mask a leak.
        status=0
        bash check.sh --failure-cleanup-control "$artifact_dir" || status=$?
        [[ "$status" == 97 ]] || { echo "Cleanup control exited $status, expected 97" >&2; exit 1; }
        cleanup_identity="$artifact_dir/cleanup/server-identity.txt"
        leaked_pid="$(sed -n 's/^pid=//p' "$cleanup_identity")"
        cleanup_url="$(sed -n 's/^url=//p' "$cleanup_identity")"
        if kill -0 "$leaked_pid" 2>/dev/null; then
            echo "Failure cleanup left server $leaked_pid alive" >&2
            exit 1
        fi
        status=0
        timeout 2s bash -c 'exec 3<>"/dev/tcp/127.0.0.1/$1"' _ "${cleanup_url##*:}" 2>/dev/null || status=$?
        [[ "$status" == 1 ]] || { echo "Failure cleanup did not prove $cleanup_url closed" >&2; exit 1; }
        echo "Verified failing-run cleanup, closed port and preserved exit 97" | tee "$artifact_dir/cleanup/result.txt"
        ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
esac
