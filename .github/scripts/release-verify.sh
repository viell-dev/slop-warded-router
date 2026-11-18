#!/usr/bin/env bash
set -euo pipefail

fail() {
    echo "$*" >&2
    exit 1
}

version="$(cargo metadata --locked --no-deps --format-version 1 |
    jq -er '.packages[] | select(.name == "warded") | .version')"
[[ "$GITHUB_REF" == "refs/tags/$version" ]] ||
    fail "Expected the unprefixed version tag $version, got $GITHUB_REF"

commit="$(git rev-parse 'HEAD^{commit}')"
[[ "$commit" == "$(git rev-parse "$GITHUB_SHA^{commit}")" &&
    "$commit" == "$(git rev-parse "$GITHUB_REF^{commit}")" ]] ||
    fail "The checkout, event and tag must resolve to the same commit"
git merge-base --is-ancestor "$commit" refs/remotes/origin/main ||
    fail "$commit is not reachable from origin/main"

for check in ci leptos-fixture; do
    endpoint="repos/$GITHUB_REPOSITORY/actions/workflows/$check.yml/runs"
    # Only main runs for this SHA count. The fixture started by the tag is independent.
    run="$(gh api --paginate --slurp "$endpoint?branch=main&head_sha=$commit&per_page=100" |
        jq -ce --arg sha "$commit" '
            [.[].workflow_runs[] |
             select(.head_branch == "main" and .head_sha == $sha and
                    (.event == "push" or .event == "workflow_dispatch"))] |
            max_by(.run_number) // error("no push or manual main run for this commit")
        ')" || fail "Could not find the required $check workflow on main for $commit"
    read -r id attempt status conclusion < <(
        jq -er '[.id, .run_attempt, .status, (.conclusion // "pending")] | @tsv' <<< "$run"
    )
    run_url="$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$id"
    [[ "$status" == completed && "$conclusion" == success ]] ||
        fail "Required $check workflow is $status/$conclusion: $run_url"

    gh api --paginate --slurp \
        "repos/$GITHUB_REPOSITORY/actions/runs/$id/attempts/$attempt/jobs?per_page=100" |
        jq -e --arg name "$check" --arg sha "$commit" '
            [.[].jobs[] | select(.name == $name)] |
            length == 1 and all(.[]; .head_sha == $sha and
                                .status == "completed" and .conclusion == "success")
        ' >/dev/null || fail "Required $check job did not succeed in attempt $attempt: $run_url"
    echo "Verified $check at $commit: $run_url/attempts/$attempt"
done

printf 'version=%s\ncommit=%s\n' "$version" "$commit" >> "$GITHUB_OUTPUT"
