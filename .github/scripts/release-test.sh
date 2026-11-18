#!/usr/bin/env bash
set -euo pipefail

scripts="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
scratch="$(mktemp -d)"
trap 'rm -rf -- "$scratch"' EXIT
cd "$scratch"

git init -q
git -c user.name=Release -c user.email=release@example.com commit -qm base --allow-empty
git update-ref refs/remotes/origin/main HEAD
git -c user.name=Release -c user.email=release@example.com tag -am release 0.1.0-beta.3
export GITHUB_SHA="$(git rev-parse HEAD)"
export GITHUB_REF=refs/tags/0.1.0-beta.3 GITHUB_REPOSITORY=example/warded
export GITHUB_SERVER_URL=https://github.com GITHUB_OUTPUT="$scratch/output"
export TEST_VERSION=0.1.0-beta.3 TEST_RUN=success
export TEST_CHECKSUM
printf -v TEST_CHECKSUM '%064d' 1

mkdir bin
export PATH="$scratch/bin:$PATH"
cat > bin/cargo <<'BASH'
#!/usr/bin/env bash
[[ "$1" == metadata ]] || exit 1
jq -n --arg version "$TEST_VERSION" '{packages:[{name:"warded",version:$version}]}'
BASH
cat > bin/gh <<'BASH'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1 $2 $3" == 'api --paginate --slurp' ]] || exit 1
run() {
    jq -n --arg branch "$1" --arg sha "$2" --arg event "$3" --argjson number "$4" \
        --arg status "$5" --arg result "$6" '
        {head_branch:$branch,head_sha:$sha,event:$event,run_number:$number,id:$number,
         run_attempt:2,status:$status,conclusion:(if $result == "" then null else $result end)}'
}
case "$4" in
    "repos/$GITHUB_REPOSITORY/actions/workflows/"*".yml/runs?branch=main&head_sha=$GITHUB_SHA&per_page=100")
        {
            check=ci
            [[ "$4" != *leptos-fixture.yml* ]] || check=leptos-fixture
            run "$TEST_VERSION" "$GITHUB_SHA" push 90 completed success
            run main 0000000000000000000000000000000000000000 push 89 completed success
            run main "$GITHUB_SHA" pull_request 88 completed success
            if [[ "$check" != leptos-fixture || "$TEST_RUN" != missing ]]; then
                run main "$GITHUB_SHA" push 1 completed success
            fi
            if [[ "$check" == ci && "$TEST_RUN" == pending ]]; then
                run main "$GITHUB_SHA" workflow_dispatch 2 in_progress ''
            elif [[ "$check" == ci && "$TEST_RUN" == failed ]]; then
                run main "$GITHUB_SHA" workflow_dispatch 2 completed failure
            fi
        } | jq -s '[{workflow_runs:.}]'
        ;;
    "repos/$GITHUB_REPOSITORY/actions/runs/1/attempts/2/jobs?per_page=100")
        jobs=(ci leptos-fixture)
        if [[ "$TEST_RUN" == no-gate ]]; then jobs=(other); fi
        for job in "${jobs[@]}"; do
            jq -n --arg name "$job" --arg sha "$GITHUB_SHA" --arg conclusion "${TEST_JOB:-success}" \
                '{name:$name,head_sha:$sha,status:"completed",conclusion:$conclusion}'
        done | jq -s '[{jobs:.}]'
        ;;
    *) exit 1 ;;
esac
BASH
cat > bin/curl <<'BASH'
#!/usr/bin/env bash
set -euo pipefail
for ((i = 1; i < $#; i++)); do
    if [[ "${!i}" == --output ]]; then
        output_arg=$((i + 1))
        cp "$TEST_INDEX" "${!output_arg}"
    fi
done
[[ "${!#}" == https://index.crates.io/wa/rd/warded ]]
echo "$TEST_STATUS"
BASH
chmod +x bin/*

expect_failure() {
    local message=$1
    shift
    if "$@" >"$scratch/log" 2>&1; then
        echo "Expected failure: $*" >&2
        exit 1
    fi
    grep -F -- "$message" "$scratch/log"
}

verify=(bash "$scripts/release-verify.sh")
"${verify[@]}"
grep -Fx "version=$TEST_VERSION" "$GITHUB_OUTPUT"
grep -Fx "commit=$GITHUB_SHA" "$GITHUB_OUTPUT"
TEST_VERSION=0.1.0-beta.4 expect_failure 'Expected the unprefixed version tag' "${verify[@]}"
GITHUB_REF=refs/heads/main expect_failure 'Expected the unprefixed version tag' "${verify[@]}"
TEST_RUN=missing expect_failure 'Could not find the required leptos-fixture workflow' "${verify[@]}"
TEST_RUN=pending expect_failure 'in_progress/pending' "${verify[@]}"
TEST_RUN=failed expect_failure 'completed/failure' "${verify[@]}"
TEST_RUN=no-gate expect_failure 'Required ci job did not succeed' "${verify[@]}"
TEST_JOB=skipped expect_failure 'Required ci job did not succeed' "${verify[@]}"

git -c user.name=Release -c user.email=release@example.com commit -qm unmerged --allow-empty
expect_failure 'checkout, event and tag must resolve to the same commit' "${verify[@]}"
export GITHUB_SHA="$(git rev-parse HEAD)"
git -c user.name=Release -c user.email=release@example.com tag -fam unmerged "$TEST_VERSION"
expect_failure 'is not reachable from origin/main' "${verify[@]}"

export TEST_INDEX="$scratch/index" TEST_STATUS=200
jq -nc --arg version "$TEST_VERSION" --arg hash "$TEST_CHECKSUM" \
    '{name:"warded",vers:$version,cksum:$hash,yanked:false}' > "$TEST_INDEX"
index=(bash "$scripts/release-index.sh")
"${index[@]}" check "$TEST_VERSION" "$TEST_CHECKSUM"
grep -Fx 'published=true' "$GITHUB_OUTPUT"
"${index[@]}" confirm "$TEST_VERSION" "$TEST_CHECKSUM"
expect_failure 'already has a different checksum' "${index[@]}" check "$TEST_VERSION" "$(printf '%064d' 2)"

"${index[@]}" check 0.1.0-beta.4 "$TEST_CHECKSUM"
grep -Fx 'published=false' "$GITHUB_OUTPUT"
: > "$GITHUB_OUTPUT"
TEST_STATUS=404 "${index[@]}" check "$TEST_VERSION" "$TEST_CHECKSUM"
grep -Fx 'published=false' "$GITHUB_OUTPUT"
TEST_STATUS=503 expect_failure 'HTTP 503' "${index[@]}" check "$TEST_VERSION" "$TEST_CHECKSUM"

cp "$TEST_INDEX" duplicate
cat duplicate >> "$TEST_INDEX"
expect_failure 'duplicate Warded version' "${index[@]}" check "$TEST_VERSION" "$TEST_CHECKSUM"
printf '[]\n' > "$TEST_INDEX"
expect_failure 'invalid Warded index response' "${index[@]}" check "$TEST_VERSION" "$TEST_CHECKSUM"
echo "Release verification and index tests passed"
