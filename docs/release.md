# Release procedure

Distribute Warded through [crates.io](https://crates.io/crates/warded). The
[docs.rs API reference](https://docs.rs/warded) and README are the documentation site. Rustdoc
provides searchable, versioned API documentation built from the package. The README covers
installation, examples, feature selection and support limits. A separate site would duplicate
these surfaces for a library of this size.

Link to docs.rs for the exact published version, using the latest installable release as the
public default. Check the version selector when working with a prerelease. The README on `main`
and the [Unreleased changelog](../CHANGELOG.md#unreleased) identify behavior awaiting publication.
Released READMEs link behavior guides and examples at their source tag. Corrections to released
documentation ship in the next version. To try unreleased work, use the README's git dependency
with a full commit SHA and build rustdoc from that revision.

## Release stages

The first-release scope is agreed in [#8](https://github.com/viell-dev/slop-warded-router/issues/8).
It covers the router, capture predicates, bounded negotiation, optional Tower/Axum integration and
one representative Leptos application. The server, indexing, typed query/capture storage and
universal adapters are outside that scope. Stable 0.1.x preserves the public API, documented
routing semantics, feature names and Rust minimum 1.99.1. Intentional breaking changes go to 0.2.0.

- **Alpha** allows changes to scope, core behavior and workflows.
- **Beta** starts when the agreed feature scope and gates below are met, then focuses on fixes and
  polish. A material change to the first-release goal needs an owner decision.
- **RC** identifies a concrete candidate intended to become stable. The owner must explicitly
  approve the first RC after review and the hands-on session in
  [#26](https://github.com/viell-dev/slop-warded-router/issues/26). Publish `0.1.0-beta.3` as the
  installable candidate for that session. Record the owner's report, resolve blockers and return
  to [#25](https://github.com/viell-dev/slop-warded-router/issues/25) for RC approval.
- **Stable** requires separate explicit owner approval after review. After the hands-on beta,
  independent agent usability reviews cover later candidates and substantial workflow changes.
  Review the published install path, main tasks, documentation and recovery, and resolve blockers
  before promotion. The owner's later comments or issues are welcome as optional feedback.
  [#25](https://github.com/viell-dev/slop-warded-router/issues/25) records this update to the
  original human-use gate in [#8](https://github.com/viell-dev/slop-warded-router/issues/8).

### Beta gates

1. Complete declared request-head routing, refinement, negotiation and dispatch behavior. Resolve
   release-blocking correctness and security defects and disposition dependencies and alpha
   constraints in [#4](https://github.com/viell-dev/slop-warded-router/issues/4) and
   [#5](https://github.com/viell-dev/slop-warded-router/issues/5).
2. A representative fixture on the candidate with the workflows and exact counts in the
   [Leptos README](../integration/leptos/README.md), without skips, retries or flakes. Require
   the broken-predicate control, retained reports and success/failure cleanup. Run the seeded input
   corpus in CI and retain failing inputs.
3. A consumer-informed API freeze assessment and correctness-checked measurement to decide whether
   indexing needs an API change before the freeze. See the [assessment](./beta-assessment.md).
4. Accurate public claims, targeted upstream-test refresh, the Windows/Linux feature and package
   matrix, and proportionate independent review of the coherent candidate on its exact revision.

## Before RC

Finish beta findings and compatibility work. Verify the [private security reporting route](../SECURITY.md).
Run the verification below and independently review the exact RC proposal, including evidence
limits and remaining findings. Link the owner's hands-on report and ask for explicit approval in
the assigned owner issue before promoting.

Every pull request needs `ci` and `leptos-fixture`. The latter runs the full fixture when library
source, the root `Cargo.toml`, Cargo configuration, the Rust toolchain, non-Markdown fixture
files, or fixture workflow and gate scripts change. A cheap production gate compares the PR merge
revision with its base and checks the affected or unaffected result. The same paths trigger the
main-push fixture, and every unprefixed version tag triggers a separate full fixture. Use manual
dispatch on `main` if the release commit needs fixture evidence. The fixture uses its own lockfile
and checks both compilation modes.

## Verify the exact release commit

1. Prepare a release PR with the version in Cargo.toml and both lockfiles, the README status,
   install commands and versioned links, and the changelog. For `0.1.0-beta.3` the release notes
   link the [hands-on request](https://github.com/viell-dev/slop-warded-router/issues/26). Review
   the complete PR and pass required CI on its final head. After integration, verify the commit
   and tree against the reviewed head and refresh any evidence invalidated by integration.
2. Run [repository verification](./verification.md) on an archive of the release commit.
   Check the package list and run `cargo publish --dry-run --locked` from a clean Git checkout
   of the exact commit. Run the git-source clean-consumer check against that pushed commit.
   Record commit, tree, toolchain, commands, checks, package files and artifacts with the release.
3. Require both GitHub Actions checks on the exact release commit in `main`. The
   [release gate](../.github/scripts/release-verify.sh) selects the highest run number per
   workflow file among `main` pushes and manual `main` runs for that SHA. It requires the selected
   run and its named gate job in the current attempt to have succeeded. Missing, pending, skipped
   or failed evidence blocks release. PR merge refs, other commits and the separate tag-triggered
   fixture run are excluded. A docs-only release commit may need a manual fixture run on `main`.

Package on Linux with the version in `rust-toolchain.toml`. Use a clean Git checkout for the
release artifact so Cargo records the source commit in `.cargo_vcs_info.json`. A package from a
source archive omits that provenance and can have a different checksum. Keep Cargo's build
verification enabled.

## First publish (0.1.0-beta.3)

The first version must use a crates.io API token authorized to create a crate. This is a one-time
`cargo publish` from a Cargo session signed in to crates.io. Use a Linux checkout with the pinned
toolchain and a working linker. The lead publishes after the release PR merges and the required
checks pass on its exact `main` commit.

Use Bash with GitHub CLI access and `jq`. Set `release_commit` to the verified full SHA on `main`.
Fetch `main` and check out that commit in a clean Git checkout. Create the annotated
tag locally and leave it unpushed until trusted publishing is configured:

```sh
git fetch origin refs/heads/main:refs/remotes/origin/main
git switch --detach "$release_commit"
version=0.1.0-beta.3
git tag -a "$version" -m "Warded $version" "$release_commit"

gate_output="$(mktemp)"
GITHUB_REF="refs/tags/$version" GITHUB_SHA="$release_commit" \
  GITHUB_REPOSITORY=viell-dev/slop-warded-router GITHUB_SERVER_URL=https://github.com \
  GITHUB_OUTPUT="$gate_output" bash .github/scripts/release-verify.sh
rm -- "$gate_output"
```

Continue only after the gate succeeds. With GitHub CLI access and the release commit verified,
package and compare the index before uploading:

```sh
export CARGO_HTTP_USER_AGENT=warded-release
cargo package --locked --target-dir target
checksum="$(sha256sum "target/package/warded-$version.crate" | cut -d ' ' -f 1)"
printf 'Warded %s SHA-256 %s\n' "$version" "$checksum"
index_output="$(mktemp)"
GITHUB_OUTPUT="$index_output" bash .github/scripts/release-index.sh check "$version" "$checksum"
cat "$index_output"
rm -- "$index_output"
```

Only `published=false` permits the upload. A matching existing version means the upload already
succeeded. Investigate any checksum mismatch before proceeding.

```sh
cargo publish --locked --registry crates-io --target-dir target
bash .github/scripts/release-index.sh confirm "$version" "$checksum"
```

## Trusted publishing

After the crate exists, create the GitHub environment `crates-io`. Under **Deployment branches and
tags**, choose **Selected branches and tags**, add a **Tag** rule `[0-9]*.[0-9]*.[0-9]*`, and allow
no branch deployments. Keep authority to create release tags with trusted maintainers. No registry
token secret is needed.

In the crate's [Trusted Publishing settings](https://crates.io/docs/trusted-publishing), add a
GitHub Actions publisher with owner `viell-dev`, repository `slop-warded-router`, workflow filename
`release.yml` and environment `crates-io`.

Push the verified annotated tag with `git push origin "refs/tags/$version"`. On beta.3, the
[workflow](../.github/workflows/release.yml) will compare the manually published bytes and exercise
the token exchange. For later releases follow the exact-commit verification above, then create
and push the annotated tag. The workflow checks that the tag equals the Cargo version and that its
commit is reachable from `main`. Only the environment's publish job can request a registry token.
It authenticates after a matching existing publication too, so a successful run also verifies the
trusted-publisher configuration. An existing version only succeeds when its index checksum matches.
The workflow uploads only when the version is missing, then confirms the checksum.

## Verify the publication

- Check the exact version page on crates.io, its README links and the sparse index checksum against
  the recorded Linux package. Confirm the downloadable archive's checksum too. Use a generic
  User-Agent for registry requests.
- Check the same version on docs.rs, including a successful build and the Tower and negotiation
  APIs. Check the source tag resolves to the recorded commit.
- Run the registry consumer check from a fresh Cargo environment. For beta.3 use
  `bash scripts/check-clean-consumer.sh --registry 0.1.0-beta.3`. It tests fresh installation,
  an update from source beta.2, all four feature combinations, rustdoc, removal and recovery.
  Pass the previous published version for later registry-to-registry updates.
- Record evidence in the release tracker. Mark the beta.3 hands-on request ready with verified
  download, documentation and tag links when these checks pass. Keep the request open until the
  owner's report and resulting work are handled.

## Recovery

If an upload or workflow is interrupted, compare the index with the package and rerun the release
workflow on the same tag after resolving the cause. A matching checksum confirms the original
upload. A mismatch blocks the release. If a required check is pending, let it finish before
rerunning; if a check failed, diagnose it and retest the release commit or prepare a corrected
release with a new version.

While trusted publishing is being established, an authorized maintainer can use a crates.io API
token scoped to publishing Warded updates and the manual commands above for an absent version.
Require the same exact-commit checks and post-publication verification. Keep tokens out of the
repository and Actions secrets.

If a published build must be withdrawn, use `cargo yank --version "$version" warded` with an
authorized Cargo session. Yanking prevents new resolutions but existing lockfiles may continue to
use it. Record the impact and ship a corrected version. Never reuse a published version, replace
its bytes, or move a published tag. Documentation fixes use the same next-version procedure.
