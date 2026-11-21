#!/usr/bin/env bash
# Choose what the Pages workflow builds: a release tag, or the documentation corrections on that
# release's site-<version> branch. The site describes the latest installable release, so the tag
# must name the newest version of Warded on crates.io that is not yanked.
#
# Environment: GITHUB_EVENT_NAME, GITHUB_REPOSITORY and GITHUB_OUTPUT, with GH_TOKEN for gh.
# A manual run sets TAG and CORRECTIONS (true or false). After a Release run, RELEASE_TAG,
# RELEASE_COMMIT and RELEASE_JOBS_URL describe that run.
set -euo pipefail

fail() {
    echo "$*" >&2
    exit 1
}

summary() {
    echo "$*"
    if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
        echo "$*" >> "$GITHUB_STEP_SUMMARY"
    fi
}

if [[ "$GITHUB_EVENT_NAME" == workflow_run ]]; then
    # The Release workflow uploads and confirms the package in its publish job. A run whose
    # publish job did not succeed leaves the site as it is.
    published="$(gh api --paginate "$RELEASE_JOBS_URL" \
        --jq '.jobs[] | select(.name == "publish") | .conclusion')" ||
        fail "Could not read the jobs of the Release run"
    if [[ "$published" != success ]]; then
        summary "The Release run did not publish; the site stays as it is."
        exit 0
    fi
    tag="$RELEASE_TAG"
    corrections=false
else
    tag="${TAG:-}"
    corrections="${CORRECTIONS:-false}"
fi

# The unprefixed version tags that the Release workflow publishes.
[[ "$tag" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] ||
    fail "'$tag' is not an unprefixed version tag"
tag_commit="$(gh api "repos/$GITHUB_REPOSITORY/commits/tags/$tag" --jq .sha)" ||
    fail "The tag $tag does not exist"
if [[ "$GITHUB_EVENT_NAME" == workflow_run && "$tag_commit" != "$RELEASE_COMMIT" ]]; then
    fail "The tag $tag names $tag_commit, not the released commit $RELEASE_COMMIT"
fi

index="$(mktemp)"
trap 'rm -f -- "$index"' EXIT
status="$(curl --silent --show-error --connect-timeout 10 --max-time 30 --retry 2 \
    --user-agent warded-pages --header 'Cache-Control: no-cache' \
    --output "$index" --write-out '%{http_code}' https://index.crates.io/wa/rd/warded)" ||
    fail "Could not read the crates.io index"
[[ "$status" == 200 ]] || fail "crates.io sparse index returned HTTP $status"
# Semantic Versioning precedence: a release outranks its prereleases, and prerelease identifiers
# compare numerically when they are numbers, below any alphanumeric identifier.
newest="$(jq -ers '
    def precedence:
        capture("^(?<major>[0-9]+)\\.(?<minor>[0-9]+)\\.(?<patch>[0-9]+)(-(?<pre>[^+]+))?")
        | [(.major, .minor, .patch | tonumber),
           (if .pre == null then [1]
            else [0] + (.pre | split(".") |
                map(if test("^[0-9]+$") then [0, tonumber] else [1, .] end))
            end)];
    map(select(.name == "warded" and .yanked == false) | .vers) | max_by(precedence)
' "$index")" || fail "The crates.io index lists no usable Warded version"
[[ "$newest" == "$tag" ]] || fail "$tag is not the newest published version, $newest"

commit="$tag_commit"
if [[ "$corrections" == true ]]; then
    # Corrections must build on the release and change nothing but its documentation, so the
    # site never describes unreleased behavior.
    branch="site-$tag"
    commit="$(gh api "repos/$GITHUB_REPOSITORY/commits/heads/$branch" --jq .sha)" ||
        fail "The branch $branch does not exist"
    comparison="$(gh api "repos/$GITHUB_REPOSITORY/compare/$tag_commit...$commit")" ||
        fail "Could not compare $branch with $tag"
    relation="$(jq -r .status <<< "$comparison")"
    case "$relation" in
        ahead | identical) ;;
        *) fail "$branch ($commit) does not descend from $tag ($tag_commit): $relation" ;;
    esac
    # The comparison lists at most 300 files.
    (($(jq '.files | length' <<< "$comparison") < 300)) || fail "$branch changes too many files"
    # A file renamed into docs/ also removes its old path.
    outside="$(jq -r '.files[] | .filename, (.previous_filename // empty) |
        select(startswith("docs/") | not)' <<< "$comparison")"
    [[ -z "$outside" ]] || fail "$branch changes files outside docs/: ${outside//$'\n'/, }"
    summary "$branch ($commit) holds documentation corrections on $tag."
elif [[ "$corrections" != false ]]; then
    fail "CORRECTIONS must be true or false, not '$corrections'"
fi

summary "Building the site of $tag, tag commit $tag_commit, from commit $commit."
printf 'tag=%s\ntag-commit=%s\ncommit=%s\n' "$tag" "$tag_commit" "$commit" >> "$GITHUB_OUTPUT"
