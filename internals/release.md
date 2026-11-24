# Release procedure

Distribute Warded through [crates.io](https://crates.io/crates/warded). Consumers read three
documentation surfaces, described in the [maintainer notes index](./README.md):

- The [documentation site](https://viell-dev.github.io/slop-warded-router/), built from `docs/`
  with VitePress 2 and deployed to GitHub Pages. It holds the task guides: installation, selection,
  negotiation, Tower integration, troubleshooting, limits, versions and upgrading.
- The [docs.rs API reference](https://docs.rs/warded), built from each published package.
- The README, which crates.io shows for each published version.

## Documentation versions

- **The site's public default is the latest installable release.** Only the
  [Pages workflow](../.github/workflows/pages.yml) deploys it, from that release's tag or from the
  release's `site-<version>` correction branch, and it refuses a version that is not the newest
  non-yanked version on crates.io. Its `version.json` records the version, the tag's commit and the
  commit the site was built from.
- **`docs/` on `main` describes `main`**, including unreleased changes, as the README on `main`
  does. The [Unreleased changelog](../CHANGELOG.md#unreleased) identifies them. CI builds the site
  and runs its examples on every pull request, but nothing deploys from `main`.
- **docs.rs is versioned by the package.** Link to the exact published version from the README
  and the site. Rustdoc corrections ship in the next version.
- **Released READMEs and site pages link guides and examples at their source tag.** The release
  text check in `scripts/check.sh` requires install pins and version-pinned links in both to name
  the package version.
- **Unreleased work** is tried with the README's git dependency on a full commit SHA. Build its
  rustdoc with `cargo doc --open`, and its site with `npm ci` and `npm run dev` in `docs/`.

Site corrections between releases follow [correcting the released
site](#correcting-the-released-site) without a new version or a moved tag.

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
  approve the first RC after review, in
  [#25](https://github.com/viell-dev/slop-warded-router/issues/25). The owner's hands-on session
  with `0.1.0-beta.3` is recorded in
  [#26](https://github.com/viell-dev/slop-warded-router/issues/26).
- **Stable** requires separate explicit owner approval after review. The hands-on session is the
  human use the stable gate requires while RC and stable stay representative of the tested build.
  After it, independent agent usability reviews cover later candidates and substantial workflow
  changes. Review the published install path, main tasks, documentation and recovery, and resolve
  blockers before promotion. The owner's later comments or issues are welcome as optional feedback.
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

Finish beta findings and compatibility work. Verify the [private security reporting
route](../SECURITY.md). Run the verification below and independently review the exact RC proposal,
including evidence limits and remaining findings. Link the owner's hands-on report and ask for
explicit approval in the assigned owner issue before promoting.

Every pull request needs `ci` and `leptos-fixture`. The latter runs the full fixture when library
source, the root `Cargo.toml`, Cargo configuration, the Rust toolchain, non-Markdown fixture
files, or fixture workflow and gate scripts change. A cheap production gate compares the PR merge
revision with its base and checks the affected or unaffected result. The same paths trigger the
main-push fixture, and every unprefixed version tag triggers a separate full fixture. Use manual
dispatch on `main` if the release commit needs fixture evidence. The fixture uses its own lockfile
and checks both compilation modes.

## Verify the exact release commit

1. Prepare a release PR with the version in Cargo.toml and both lockfiles, the README status,
   install commands and versioned links, the site pages' install pins and versioned links, the
   [upgrading page](../docs/upgrading.md) for any change that needs consumer action, and the
   changelog; `scripts/check.sh` fails if they name another version or a README link is
   relative. The changelog entry links any request issue the release serves, as those of beta.3
   to beta.5 link the
   [hands-on request](https://github.com/viell-dev/slop-warded-router/issues/26). Review the
   complete PR and pass required CI on its final head. After integration, verify the commit and
   tree against the reviewed head and refresh any evidence invalidated by integration.
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
source archive omits that provenance and can have a different checksum. A rebase merge gives the
release a new commit, so package the merged `main` commit rather than the reviewed head.

## Trusted publishing

`warded` publishes through the [release workflow](../.github/workflows/release.yml) using crates.io
[Trusted Publishing](https://crates.io/docs/trusted-publishing). The crate's trusted-publishing-only
setting makes crates.io refuse API token uploads.

The GitHub environment `crates-io` allows only tags matching `[0-9]*.[0-9]*.[0-9]*` and no branch
deployments; no registry token secret is configured. The crates.io Trusted Publishing settings
authorize owner `viell-dev`, repository `slop-warded-router`, workflow `release.yml`, and
environment `crates-io`. A repository ruleset blocks updates and deletions of version tags, so a
published tag cannot move.

Pushing a version tag is the release decision: any such tag runs its own commit's workflow in this
environment, and the gate then requires a verified `main` commit. Keep tag creation with trusted
maintainers.

Use Bash with GitHub CLI access and `jq`, in a clean checkout. After the exact-commit verification
above passes, set `release_commit` to the verified full SHA on `main` and `version` to its Cargo
version. Create the annotated tag locally and run the release gate before pushing it, because a
pushed version tag cannot move:

```sh
git fetch origin refs/heads/main:refs/remotes/origin/main
git switch --detach "$release_commit"
git tag -a "$version" -m "Warded $version" "$release_commit"

gate_output="$(mktemp)"
GITHUB_REF="refs/tags/$version" GITHUB_SHA="$release_commit" \
  GITHUB_REPOSITORY=viell-dev/slop-warded-router GITHUB_SERVER_URL=https://github.com \
  GITHUB_OUTPUT="$gate_output" bash .github/scripts/release-verify.sh
rm -- "$gate_output"
```

Continue only after the gate succeeds: `git push origin "refs/tags/$version"`.

The workflow checks that the tag equals the Cargo version
and that its commit is reachable from `main`. Only the environment's publish job can request a
registry token. It authenticates after a matching existing publication too, so a successful run also
verifies the trusted-publisher configuration. An existing version only succeeds when its index
checksum matches. The workflow uploads only when the version is missing, then confirms the checksum.

## Verify the publication

- Check the exact version page on crates.io, its README links and the sparse index checksum against
  the recorded Linux package. Confirm the downloadable archive's checksum too. Use a generic
  User-Agent for registry requests.
- Check the same version on docs.rs, including a successful build and the Tower and negotiation
  APIs. Check the source tag resolves to the recorded commit.
- Run the registry consumer check from a fresh Cargo environment with the new and previous
  published versions, for example
  `bash scripts/check-clean-consumer.sh --registry 0.1.0-beta.4 0.1.0-beta.3`. It tests fresh
  installation, the update from the previous version, all four feature combinations, rustdoc,
  removal and recovery. Without a previous version it updates from source beta.2.
- Check the [deployed site](#documentation-site): its `version.json` names the version and the
  release commit, the navigation bar shows the version, and the getting-started page installs it.
  Keep the repository's About website pointing at the site, which leads to the current release's
  guides and links its docs.rs reference.
- Record evidence in the release tracker. When these checks pass, give any request issue that
  waits on the release, such as a hands-on or RC approval request, the verified download,
  documentation and tag links. Keep it open until its answer and resulting work are handled.

## Documentation site

The [Pages workflow](../.github/workflows/pages.yml) builds `docs/` with the VitePress version
pinned in `docs/package-lock.json` and the Node.js version in `docs/.nvmrc`, writes `version.json`
and deploys the result. Its [resolution script](../.github/scripts/pages-resolve.sh) and
[tests](../.github/scripts/pages-test.sh) define what may be deployed. The script refuses:

- a tag that is not an unprefixed version, or does not exist;
- a version that is not the newest non-yanked Warded version in the crates.io index, so the site
  cannot go back to an older release or describe an unpublished one;
- after a Release run, a run whose `publish` job did not succeed, or a tag that names another
  commit than the run;
- for corrections, a `site-<version>` branch that does not descend from the tag, or that changes,
  adds, removes or renames any file outside `docs/`.

### Settings

Configure these before the first deployment. Each is reversible; none affects releases or tags.

1. **Pages:** under **Settings → Pages → Build and deployment**, set **Source** to **GitHub
   Actions**. This creates the `github-pages` environment.
2. **Environment:** under **Settings → Environments → github-pages → Deployment branches and
   tags**, allow only the `main` branch. Both triggers run the workflow on `main`; only its
   checkout uses the tag or correction branch.
3. **Correction branches:** add a branch ruleset for `site-*` that requires pull requests and the
   `ci` status check, blocks force pushes and deletions, and has no bypass. The `leptos-fixture`
   workflow runs only for pull requests into `main`, so do not require it here. Creating the
   branch from the tag is the one direct push the ruleset allows.
4. **About:** set the repository's website to <https://viell-dev.github.io/slop-warded-router/>
   once the first deployment succeeds.

### Deploy a release's site

A successful Release run that published its version starts the workflow through `workflow_run`.
It builds the released tag and deploys it. To deploy by hand, for example after fixing a failed
run, dispatch **Pages** on `main` with `tag` set to the newest release's version and `corrections`
left off.

After the run, open the site and its `version.json`, and check the version, tag commit and built
commit against the release record.

### Correcting the released site

Corrections fix the released site without a new version. They may change only `docs/`: page
text, examples and the site's own tooling. Each page's "suggest a change" link opens its source
on `main`, because pull requests go there; when such a change corrects the released site, apply
it to the correction branch too.

1. If the release has no correction branch yet, create one at its tag's commit:
   `git push origin "refs/tags/$version^{commit}:refs/heads/site-$version"`. The `^{commit}`
   matters: a version tag is annotated, and GitHub rejects a branch that would name the tag
   object instead of its commit.
2. Open a pull request into `site-$version` from a task branch whose name does not start with
   `site-`. CI builds the site and runs its examples against the release's source.
3. After it merges, dispatch **Pages** on `main` with `tag` set to the version and `corrections`
   enabled. The `version.json` then names the correction branch's commit beside the tag's.
4. Apply the same correction to `main` in a separate pull request, unless it started there, so
   the next release keeps it.

The next release's site replaces the corrected one. Keep correction branches: they record what
was deployed. A Pages failure leaves the previous deployment in place; fix the cause and run the
workflow again.

## Recovery

If an upload or workflow is interrupted, compare the index with the package and rerun the release
workflow on the same tag after resolving the cause. A matching checksum confirms the original
upload. A mismatch blocks the release. If a required check is pending, let it finish before
rerunning; if a check failed, diagnose it and retest the release commit or prepare a corrected
release with a new version.

Later versions publish through the workflow. If it cannot publish a verified version, for example
because the trusted-publisher configuration fails, fix the cause and rerun it. A manual token
upload of such a version needs the owner's recorded go-ahead, the same exact-commit checks and
post-publication verification, and the trusted-publishing-only setting turned off for the upload
and on again afterwards. Package on Linux as above and check the index first:

```sh
cargo package --locked --target-dir target
checksum="$(sha256sum "target/package/warded-$version.crate" | cut -d ' ' -f 1)"
index_output="$(mktemp)"
GITHUB_OUTPUT="$index_output" bash .github/scripts/release-index.sh check "$version" "$checksum"
cat "$index_output"
rm -- "$index_output"
```

Only `published=false` permits the upload; a checksum mismatch blocks it. Then upload the verified
bytes and confirm them:

```sh
cargo publish --locked --no-verify --registry crates-io --target-dir target
bash .github/scripts/release-index.sh confirm "$version" "$checksum"
```

Keep tokens out of the repository and Actions secrets.

If a published build must be withdrawn, the owner yanks it on crates.io or with
`cargo yank --version "$version" warded` from a session whose token has the yank scope; the
publishing credentials do not include it. Yanking prevents new resolutions, but existing lockfiles
may continue to use it. Record the impact and ship a corrected version. Never reuse a published
version, replace its bytes, or move a published tag. README and rustdoc fixes ship in the next
version; site fixes can also use [correcting the released site](#correcting-the-released-site).
