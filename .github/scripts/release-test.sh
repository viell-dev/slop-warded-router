#!/usr/bin/env bash
set -euo pipefail

scripts="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
scratch="$(mktemp -d)"
trap 'rm -rf -- "$scratch"' EXIT
cd "$scratch"

git_commit() {
    git -c user.name=Release -c user.email=release@example.com "$@"
}

git init -q
git_commit commit -qm base --allow-empty
git update-ref refs/remotes/origin/main HEAD
git_commit tag -am release 0.1.0-beta.3
GITHUB_SHA="$(git rev-parse HEAD)"
export GITHUB_SHA
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
        } | jq -s --argjson pages "${TEST_PAGES:-1}" '
            # A second page holds the eligible main run, as --paginate --slurp returns it.
            if $pages == 2 then [{workflow_runs:.[:3]}, {workflow_runs:.[3:]}]
            else [{workflow_runs:.}] end'
        ;;
    "repos/$GITHUB_REPOSITORY/actions/runs/1/attempts/2/jobs?per_page=100")
        jobs=(ci leptos-fixture)
        if [[ "$TEST_RUN" == no-gate ]]; then jobs=(other); fi
        if [[ "$TEST_RUN" == duplicate-gate ]]; then jobs=(ci ci leptos-fixture leptos-fixture); fi
        for job in "${jobs[@]}"; do
            jq -n --arg name "$job" --arg sha "${TEST_JOB_SHA:-$GITHUB_SHA}" \
                --arg conclusion "${TEST_JOB:-success}" \
                '{name:$name,head_sha:$sha,status:"completed",conclusion:$conclusion}'
        done | jq -s '[{jobs:.}]'
        ;;
    *) exit 1 ;;
esac
BASH
# Each call takes the next status from TEST_STATUS, a space-separated list whose last entry
# repeats. "fail" stands for a transport error such as a timeout.
cat > bin/curl <<'BASH'
#!/usr/bin/env bash
set -euo pipefail
read -ra statuses <<< "$TEST_STATUS"
calls=0
[[ ! -f "$TEST_CALLS" ]] || calls="$(< "$TEST_CALLS")"
echo $((calls + 1)) > "$TEST_CALLS"
printf '%s\n' "$*" > "$TEST_CALLS.args"
status="${statuses[calls]:-${statuses[-1]}}"
[[ "${!#}" == https://index.crates.io/wa/rd/warded ]]
[[ "$status" != fail ]] || exit 28
for ((i = 1; i < $#; i++)); do
    if [[ "${!i}" == --output ]]; then
        output_arg=$((i + 1))
        cp "$TEST_INDEX" "${!output_arg}"
    fi
done
echo "$status"
BASH
# Confirm mode waits between index reads; the tests need no real delay.
printf '#!/usr/bin/env bash\n' > bin/sleep
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
: > "$GITHUB_OUTPUT"
TEST_PAGES=2 "${verify[@]}"
grep -Fx "commit=$GITHUB_SHA" "$GITHUB_OUTPUT"
TEST_VERSION=0.1.0-beta.4 expect_failure 'Expected the unprefixed version tag' "${verify[@]}"
GITHUB_REF=refs/heads/main expect_failure 'Expected the unprefixed version tag' "${verify[@]}"
git_commit tag -am prefixed "v$TEST_VERSION"
GITHUB_REF="refs/tags/v$TEST_VERSION" expect_failure 'Expected the unprefixed version tag' \
    "${verify[@]}"
TEST_RUN=missing expect_failure 'Could not find the required leptos-fixture workflow' "${verify[@]}"
TEST_RUN=pending expect_failure 'in_progress/pending' "${verify[@]}"
TEST_RUN=failed expect_failure 'completed/failure' "${verify[@]}"
TEST_RUN=no-gate expect_failure 'Required ci job did not succeed' "${verify[@]}"
TEST_RUN=duplicate-gate expect_failure 'Required ci job did not succeed' "${verify[@]}"
TEST_JOB=skipped expect_failure 'Required ci job did not succeed' "${verify[@]}"
TEST_JOB_SHA=0000000000000000000000000000000000000000 \
    expect_failure 'Required ci job did not succeed' "${verify[@]}"

# Each of the checkout, event and tag commits must agree with the other two.
other="$(git_commit commit-tree -p HEAD -m other 'HEAD^{tree}')"
GITHUB_SHA="$other" \
    expect_failure 'checkout, event and tag must resolve to the same commit' "${verify[@]}"
git_commit tag -fam moved "$TEST_VERSION" "$other"
expect_failure 'checkout, event and tag must resolve to the same commit' "${verify[@]}"
git_commit tag -fam release "$TEST_VERSION" HEAD
"${verify[@]}"

git_commit commit -qm unmerged --allow-empty
expect_failure 'checkout, event and tag must resolve to the same commit' "${verify[@]}"
GITHUB_SHA="$(git rev-parse HEAD)"
git_commit tag -fam unmerged "$TEST_VERSION"
expect_failure 'is not reachable from origin/main' "${verify[@]}"

export TEST_INDEX="$scratch/index" TEST_CALLS="$scratch/calls" TEST_STATUS=200
published() {
    jq -nc --arg version "$1" --arg hash "$2" \
        '{name:"warded",vers:$version,cksum:$hash,yanked:false}'
}
index() {
    rm -f -- "$TEST_CALLS"
    bash "$scripts/release-index.sh" "$@"
}
published "$TEST_VERSION" "$TEST_CHECKSUM" > "$TEST_INDEX"
index check "$TEST_VERSION" "$TEST_CHECKSUM"
grep -Fx 'published=true' "$GITHUB_OUTPUT"
grep -F -- '--retry 2 ' "$TEST_CALLS.args"
index confirm "$TEST_VERSION" "$TEST_CHECKSUM"
grep -F -- '--retry 0 ' "$TEST_CALLS.args"
expect_failure 'already has a different checksum' index check "$TEST_VERSION" "$(printf '%064d' 2)"
expect_failure 'Invalid package checksum' index check "$TEST_VERSION" not-a-checksum

index check 0.1.0-beta.4 "$TEST_CHECKSUM"
grep -Fx 'published=false' "$GITHUB_OUTPUT"
: > "$GITHUB_OUTPUT"
TEST_STATUS=404 index check "$TEST_VERSION" "$TEST_CHECKSUM"
grep -Fx 'published=false' "$GITHUB_OUTPUT"
TEST_STATUS=503 expect_failure 'HTTP 503' index check "$TEST_VERSION" "$TEST_CHECKSUM"
TEST_STATUS=fail expect_failure 'Could not read the crates.io index' \
    index check "$TEST_VERSION" "$TEST_CHECKSUM"

# After an upload, read failures and a delayed index are retried until the version appears.
TEST_STATUS='404 fail 503 200' index confirm "$TEST_VERSION" "$TEST_CHECKSUM"
[[ "$(< "$TEST_CALLS")" == 4 ]]
TEST_STATUS=404 expect_failure 'did not appear in a readable index' \
    index confirm "$TEST_VERSION" "$TEST_CHECKSUM"
[[ "$(< "$TEST_CALLS")" == 18 ]]
TEST_STATUS='503 200' expect_failure 'already has a different checksum' \
    index confirm "$TEST_VERSION" "$(printf '%064d' 2)"
[[ "$(< "$TEST_CALLS")" == 2 ]]
expect_failure 'already has a different checksum' index confirm "$TEST_VERSION" "$(printf '%064d' 2)"
[[ "$(< "$TEST_CALLS")" == 1 ]]

cp "$TEST_INDEX" duplicate
cat duplicate >> "$TEST_INDEX"
expect_failure 'duplicate Warded version' index check "$TEST_VERSION" "$TEST_CHECKSUM"
printf '[]\n' > "$TEST_INDEX"
expect_failure 'invalid Warded index response' index check "$TEST_VERSION" "$TEST_CHECKSUM"
: > "$TEST_INDEX"
expect_failure 'invalid Warded index response' index check "$TEST_VERSION" "$TEST_CHECKSUM"
published "$TEST_VERSION" "$TEST_CHECKSUM" | jq -c '.name = "other"' > "$TEST_INDEX"
expect_failure 'invalid Warded index response' index check "$TEST_VERSION" "$TEST_CHECKSUM"
published "$TEST_VERSION" "$(printf '%064d' 1 | tr 0 A)" > "$TEST_INDEX"
expect_failure 'invalid Warded index response' index check "$TEST_VERSION" "$TEST_CHECKSUM"

# The release text check runs on copies of the repository's README and changelog.
root="$(cd "$scripts/../.." && pwd)"
release_version="$(sed -nE 's/^version = "([^"]+)"$/\1/p' "$root/Cargo.toml" | head -n 1)"
text_case() {
    rm -rf -- "$scratch/text"
    mkdir "$scratch/text"
    cp "$root/README.md" "$root/CHANGELOG.md" "$scratch/text/"
}
text_check() {
    (cd "$scratch/text" && bash "$root/scripts/check-release-text.sh" "$release_version")
}
readme_append() {
    printf '%b' "$1" >> "$scratch/text/README.md"
}
absolute() {
    text_case
    readme_append "$1"
    text_check
}
relative() {
    text_case
    readme_append "$1"
    expect_failure 'README links must be absolute' text_check
}
stale() {
    text_case
    "$@"
    expect_failure 'must pin and name only' text_check
}

fence='```'
absolute ''
absolute "\n${fence}md\nSee [x](docs/x.md)\n${fence}\n"
absolute '\n~~~\n[x](docs/x.md)\n~~~\n'
# A shorter or different fence inside a code block is content, not a closing fence.
absolute "\n${fence}\`md\n${fence}\n[x](docs/x.md)\n${fence}\n${fence}\`\n"
absolute "\n~~~\n${fence}\n[x](docs/x.md)\n${fence}\n~~~\n"
absolute '\n[Guide](<https://example.com/guide>)\n'
absolute '\n[x](https://example.com "Title")\n'
absolute "\nSee https://docs.rs/warded/$release_version.\n"
relative '\nSee [design](./docs/design.md).\n'
relative '\n[ref]: docs/design.md\n'
relative '\n[guide]:\n  docs/guide.md\n'
relative '\n[guide]:\t\n  docs/guide.md\n'
relative '\n[Guide](<docs/guide.md>)\n'
relative '\n![logo](logo.png)\n'
relative "\n~~~\n${fence}\n~~~\nSee [x](docs/x.md)\n"
stale readme_append "\n${fence}toml\nwarded = { version = \"=0.0.0-stale\" }\n${fence}\n"
stale readme_append \
    "\n${fence}toml\nwarded = { default-features = false, version = \"=0.0.0-stale\" }\n${fence}\n"
stale readme_append "\n${fence}toml\n  warded = \"=0.0.0-stale\"\n${fence}\n"
stale readme_append '\nSee https://docs.rs/warded/0.0.0-stale.\n'
stale sed -i "s/warded = \"=$release_version\"/warded = \"0.1\"/g" "$scratch/text/README.md"
stale sed -i -E "s/^## $release_version( |\$)/## 0.0.0-stale\\1/" "$scratch/text/CHANGELOG.md"
only_unreleased() {
    printf '# Changes\n\n## Unreleased\n' > "$scratch/text/CHANGELOG.md"
}
stale only_unreleased
text_case
rm "$scratch/text/README.md"
expect_failure 'README.md' text_check
text_case
rm "$scratch/text/CHANGELOG.md"
expect_failure 'CHANGELOG.md' text_check

# A grep failure on a readable file, such as a read error, must stop the check rather than read
# as "no match". The wrapper fails the calls whose arguments include FAIL_GREP_ON.
mkdir "$scratch/failing-grep"
REAL_GREP="$(command -v grep)"
export REAL_GREP
cat > "$scratch/failing-grep/grep" <<'BASH'
#!/usr/bin/env bash
if [[ " $* " == *" $FAIL_GREP_ON "* ]]; then
    echo "grep: simulated read error" >&2
    exit 2
fi
exec "$REAL_GREP" "$@"
BASH
chmod +x "$scratch/failing-grep/grep"
for option in -oE -vE; do
    text_case
    FAIL_GREP_ON="$option" PATH="$scratch/failing-grep:$PATH" \
        expect_failure 'simulated read error' text_check
done
echo "Release verification, index and release text tests passed"
