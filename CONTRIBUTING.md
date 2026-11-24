# Contributing

Warded is an early Rust library maintained by agents. Small, well-scoped reports, tests, and fixes
are welcome. Establish scope before proposing a new framework, server implementation, or unrelated
feature. Describe behavior with synthetic requests and expected outcomes.

Use a branch and pull request against `main`. Both `ci` and `leptos-fixture` are required.
The former includes Windows/Linux library and package checks, the Linux dependency policy
check and the documentation site build. The latter runs the full fixture when its inputs change
and verifies that decision on every pull request. The [release
procedure](./internals/release.md#before-rc) lists affected paths, manual dispatch and tag
triggers. After relevant tests and required checks pass, review the complete change and record
the platforms, Rust version, commands and evidence limits.

Run `bash scripts/check.sh` in Bash, which is Git Bash on Windows, for the library, package and
documentation example checks. To build the documentation site, use the Node.js version in
`docs/.nvmrc` and run `npm ci` and `npm run build` in `docs/`; `npm run dev` previews it.
[Verification](./internals/verification.md) describes each check.

Documentation has three areas: the consumer guides in [`docs/`](./docs/), published as the
[documentation site](https://viell-dev.github.io/slop-warded-router/); the API reference in
rustdoc, published on docs.rs; and the [maintainer notes](./internals/README.md). A change to
behavior updates the guides, rustdoc and README it affects in the same pull request. Every Rust
example on the site is a complete program that CI compiles and runs.

Releases are distributed through crates.io, with versioned API documentation on docs.rs and the
site built from each release's tag. Record unreleased user-facing changes in the changelog. The
[release procedure](./internals/release.md) covers registry verification, publishing and site
deployment.

Report suspected vulnerabilities using the [private GitHub route](./SECURITY.md). Security fixes
land on `main` in the current 0.1.0 prerelease line.

## Security advisories

A published advisory against a resolved Cargo or npm dependency takes precedence over routine update
batching and any release-age gate, only for the versions the fix needs:

1. Confirm that the advisory covers the resolved version. Confirm that the fixed release comes from
   the package's usual publisher and source, and that the advisory, changelog, or release notes
   identify it as the fix.
2. Take the smallest version that resolves the advisory, updating the package directly when a fixed
   release is within the declared range. If a dependant has not adopted the fix, override the
   transitive version with Cargo `[patch]` or npm `overrides` once the checks show that it works.
   Record the advisory and removal condition in a comment beside the override or in the pull
   request, and remove the override when the dependant releases a version with the fix. Cargo
   `[patch]` applies only to this workspace's builds and does not protect applications that depend
   on the published crate, so record that remaining exposure.
3. If no working fixed version exists, track the advisory in the [dependency
   overview](https://github.com/viell-dev/slop-warded-router/issues/4).
4. Decide whether the change needs an independent review of the published package contents. Get one
   for a less widely used package, after a recent change of maintainer or publishing method, or when
   the fix release has substantial unrelated changes.

Record the advisory, affected and fixed versions, dependency path, any bypassed gate, and the review
decision with its reason in the pull request. Required checks still apply.

## Upstream tests

During early releases, use existing routers' test suites as a recurring source of adversarial
inputs and missing scenarios. For substantial matching/dispatch changes and alpha/beta/RC
preparation, inspect the relevant upstream areas and record the pinned revisions, applicable
cases, and their dispositions. Reuse existing evidence for unchanged areas; a full rescan of every
upstream suite is unnecessary. Preserve Warded's documented semantics rather than importing another
router's defaults. See the [comparison record](./internals/upstream-tests.md). After the first stable
release, retire this routine comparison gate and use targeted upstream research when useful.

## Attribution, labels and licensing

Agent-authored issues, pull requests, and reviews must clearly disclose their authorship, the
submitting model and harness, submission on the account owner's behalf, and whether that person
reviewed the material. Authorization and automated or agent verification do not constitute human
review. Use an `Assisted-by: <model> via <harness>` commit trailer for agent assistance; do not add
agent `Co-Authored-By` or `Signed-off-by` trailers.

Keep one `Type:` label on every issue and pull request. An optional `Status:` label expresses the
current disposition; `Needs: Human` or `Needs: Owner` identifies a required response and must be
accompanied by an assignee and self-contained request. See the
[label catalog](https://github.com/viell-dev/slop-warded-router/blob/main/.github/labels.json)
for the catalog.

Contributions to original code are licensed under MIT OR Apache-2.0. Retain applicable notices and
provenance when adapting third-party source; original project licensing does not replace its terms.
