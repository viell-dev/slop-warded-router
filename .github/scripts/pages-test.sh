#!/usr/bin/env bash
# Tests pages-resolve.sh against fake gh and curl commands.
set -euo pipefail

scripts="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
scratch="$(mktemp -d)"
trap 'rm -rf -- "$scratch"' EXIT
cd "$scratch"

tag_commit=1111111111111111111111111111111111111111
branch_commit=2222222222222222222222222222222222222222
export GITHUB_REPOSITORY=example/warded GITHUB_OUTPUT="$scratch/output"
export TEST_INDEX="$scratch/index" TEST_COMPARE="$scratch/compare" TEST_JOBS="$scratch/jobs"
export TEST_TAG_COMMIT="$tag_commit" TEST_BRANCH_COMMIT="$branch_commit"

mkdir bin
export PATH="$scratch/bin:$PATH"
# gh api <path> [--jq <filter>] [--paginate], answering from the test files and variables.
cat > bin/gh <<'BASH'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == api ]] || exit 1
shift
filter=. path=
while (($#)); do
    case "$1" in
        --jq) filter="$2"; shift 2 ;;
        --paginate) shift ;;
        *) path="$1"; shift ;;
    esac
done
case "$path" in
    https://api.github.com/jobs) body="$(cat "$TEST_JOBS")" ;;
    "repos/$GITHUB_REPOSITORY/commits/refs/tags/0.1.0-beta.4")
        body="{\"sha\":\"$TEST_TAG_COMMIT\"}" ;;
    # GitHub answers the short tags/<tag> form with HTTP 422, which broke the first deployment.
    "repos/$GITHUB_REPOSITORY/commits/tags/"*)
        echo "gh: No commit found for SHA: ${path#*/commits/} (HTTP 422)" >&2
        exit 1 ;;
    "repos/$GITHUB_REPOSITORY/commits/refs/heads/site-0.1.0-beta.4")
        [[ -n "${TEST_NO_BRANCH:-}" ]] && exit 1
        body="{\"sha\":\"$TEST_BRANCH_COMMIT\"}" ;;
    "repos/$GITHUB_REPOSITORY/compare/$TEST_TAG_COMMIT...$TEST_BRANCH_COMMIT")
        body="$(cat "$TEST_COMPARE")" ;;
    *) exit 1 ;;
esac
jq -r "$filter" <<< "$body"
BASH
# curl ... --output <file> ... <URL>, copying TEST_INDEX and printing TEST_STATUS.
cat > bin/curl <<'BASH'
#!/usr/bin/env bash
set -euo pipefail
[[ "${!#}" == https://index.crates.io/wa/rd/warded ]]
[[ "${TEST_STATUS:-200}" != fail ]] || exit 28
for ((i = 1; i < $#; i++)); do
    if [[ "${!i}" == --output ]]; then
        output=$((i + 1))
        cp "$TEST_INDEX" "${!output}"
    fi
done
echo "${TEST_STATUS:-200}"
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

published() {
    for version in "$@"; do
        local yanked=false
        if [[ "$version" == *! ]]; then
            version="${version%!}"
            yanked=true
        fi
        jq -nc --arg version "$version" --argjson yanked "$yanked" \
            '{name:"warded",vers:$version,cksum:("0" * 64),yanked:$yanked}'
    done > "$TEST_INDEX"
}
changes() {
    jq -n --arg status "$1" '{status:$status,files:[]}' > "$TEST_COMPARE"
    shift
    for file in "$@"; do
        jq --arg file "$file" '
            .files += [($file | split(" from ")) as [$to, $from] |
                {filename:$to} + (if $from then {previous_filename:$from} else {} end)]
        ' "$TEST_COMPARE" > "$TEST_COMPARE.new"
        mv "$TEST_COMPARE.new" "$TEST_COMPARE"
    done
}
resolve() {
    : > "$GITHUB_OUTPUT"
    bash "$scripts/pages-resolve.sh"
}

export GITHUB_EVENT_NAME=workflow_dispatch TAG=0.1.0-beta.4 CORRECTIONS=false

# A manual run builds the newest release's tag. Prereleases order by SemVer precedence, and
# yanked versions do not count.
published 0.1.0-alpha.3 0.1.0-beta.3 0.1.0-beta.4 0.1.0-beta.10! 0.0.9
resolve
grep -Fx "tag=0.1.0-beta.4" "$GITHUB_OUTPUT"
grep -Fx "tag-commit=$tag_commit" "$GITHUB_OUTPUT"
grep -Fx "commit=$tag_commit" "$GITHUB_OUTPUT"
published 0.1.0-beta.4 0.1.0-beta.10
expect_failure '0.1.0-beta.4 is not the newest published version, 0.1.0-beta.10' resolve
published 0.1.0-beta.4 0.1.0-rc.1
expect_failure 'not the newest published version, 0.1.0-rc.1' resolve
published 0.1.0-beta.4 0.1.0
expect_failure 'not the newest published version, 0.1.0' resolve
published 0.1.0-beta.4 0.1.0-beta.4.1
expect_failure 'not the newest published version, 0.1.0-beta.4.1' resolve
published 0.1.0-beta.3
expect_failure 'not the newest published version, 0.1.0-beta.3' resolve
published 0.1.0-beta.4!
expect_failure 'lists no usable Warded version' resolve
published 0.1.0-beta.4
TEST_STATUS=503 expect_failure 'returned HTTP 503' resolve
TEST_STATUS=fail expect_failure 'Could not read the crates.io index' resolve

TAG=v0.1.0-beta.4 expect_failure "'v0.1.0-beta.4' is not an unprefixed version tag" resolve
TAG=0.1.0-beta.5 expect_failure 'The tag 0.1.0-beta.5 does not exist' resolve
TAG='' expect_failure 'is not an unprefixed version tag' resolve
CORRECTIONS=yes expect_failure "CORRECTIONS must be true or false, not 'yes'" resolve

# Corrections build the site-<version> branch when it descends from the tag and changes only
# docs/, including the old path of a renamed file.
export CORRECTIONS=true
changes ahead docs/negotiation.md docs/.vitepress/config.mts
resolve
grep -Fx "tag-commit=$tag_commit" "$GITHUB_OUTPUT"
grep -Fx "commit=$branch_commit" "$GITHUB_OUTPUT"
changes identical
resolve
changes ahead docs/negotiation.md README.md
expect_failure 'changes files outside docs/: README.md' resolve
changes ahead "docs/guide.md from internals/guide.md"
expect_failure 'changes files outside docs/: internals/guide.md' resolve
changes diverged docs/negotiation.md
expect_failure 'does not descend from 0.1.0-beta.4' resolve
changes behind
expect_failure 'does not descend from 0.1.0-beta.4' resolve
changes ahead
jq '.files = [range(300) | {filename: "docs/page\(.).md"}]' "$TEST_COMPARE" > many
mv many "$TEST_COMPARE"
expect_failure 'changes too many files' resolve
TEST_NO_BRANCH=1 expect_failure 'The branch site-0.1.0-beta.4 does not exist' resolve

# After a Release run, the released tag is built if the run published it.
export GITHUB_EVENT_NAME=workflow_run TAG='' CORRECTIONS=''
export RELEASE_TAG=0.1.0-beta.4 RELEASE_COMMIT="$tag_commit"
export RELEASE_JOBS_URL=https://api.github.com/jobs
jobs() {
    jq -n --arg publish "$1" '{jobs:[{name:"verify",conclusion:"success"},
        {name:"publish",conclusion:$publish}]}' > "$TEST_JOBS"
}
jobs success
resolve
grep -Fx "commit=$tag_commit" "$GITHUB_OUTPUT"
for conclusion in skipped failure cancelled; do
    jobs "$conclusion"
    resolve | grep -F 'The Release run did not publish'
    [[ ! -s "$GITHUB_OUTPUT" ]]
done
jobs success
RELEASE_COMMIT="$branch_commit" expect_failure 'not the released commit' resolve
echo "Pages resolution tests passed"
