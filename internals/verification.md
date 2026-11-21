# Verification

Use Rust 1.99.1 from `rust-toolchain.toml`. Integration requires `ci` (Windows and Linux library
checks, Linux dependency policy and the documentation site build) and `leptos-fixture` (the
fixture gate).

From the repository root, run the same entry point as both CI platforms:

```sh
bash scripts/check.sh
```

The [script](../scripts/check.sh) anchors itself to the checkout root and derives the packaged
manifest from Cargo's current package identity. It first runs the [release text
check](../scripts/check-release-text.sh). That check requires the README and a site page to pin
that version, the install pins and version-pinned links of the README and every site page and the
latest changelog release heading to name only that version, and every README link to be absolute
or an in-page anchor. crates.io resolves relative README links against the default branch. The
script then runs formatting, Clippy on all targets of both workspace members (the library and
the site's `warded-docs` crate), the four feature configurations and doctests, the site's Rust
examples, documentation with warnings denied, both examples, package verification, and both
packaged examples. Package output uses `CARGO_TARGET_DIR` when set,
otherwise `target`; the package step chooses that output explicitly to avoid stale-path fallback.

A separate required Linux job runs `cargo deny --locked check` with cargo-deny 0.20.2, verified
by the release checksum recorded in the workflow. [deny.toml](../deny.toml) covers the all-features
library graph including dev dependencies. RustSec vulnerability, unmaintained and unsound
advisories fail the job throughout the resolved graph. Yanked versions, licenses outside the
allow list, wildcard requirements and non-crates.io sources also fail the job. Run the same
command locally after installing that cargo-deny version. The Leptos fixture has a separate
dependency graph. Dependabot proposes weekly Cargo and
GitHub Actions updates for the library; grouped patch updates and individual minor updates still
pass through ordinary review and CI.

Use Git Bash for this entry point on Windows. Individual Cargo commands remain usable in
PowerShell; set `$env:RUSTDOCFLAGS = "-D warnings"` before a documentation check. The MSVC runtime
libraries and Windows SDK must be installed and discoverable; a missing `msvcrt.lib` prevents tests
from linking before Warded runs.

The suite checks complete-condition fallthrough, overlap and declaration order, captures discarded
after failed alternatives and negation, collision errors, raw/repeated/invalid inputs, all required
matching dimensions, equivalent shortcut compositions, semantic negotiation, unread bodies and
trailers, committed service execution, no-match request preservation, readiness isolation, and an
Axum mounting boundary. A compile-fail doctest checks that matchers have no body accessor.

The [comparison harness](https://github.com/viell-dev/slop-warded-router/tree/main/research/comparison)
is an optional engineering probe,
independent of the library dependency graph. It records actual upstream behavior and negotiation
differences; it is not a benchmark or a required compatibility claim.

The server example additionally checks a real localhost HTTP/1.1 listener, host error conversion,
HEAD/405 behavior, known cleartext context, graceful shutdown, and Vary preservation. It does not
establish TLS/proxy trust, browser workflows, HTTP/2, or HTTP/3. The owner's hands-on pass is
required before the first RC. Independent agent usability reviews cover later candidates.

The [upstream-test comparison](./upstream-tests.md) records recurring early-release research and
the provenance and disposition of additional cases in `upstream_cases.rs`. These cover empty and
affixed path captures, nested rollback, raw namespace boundaries, atypical URI forms, method
extensions, snapshot isolation, and real HTTP/1.1 QUERY/custom-method body preservation. They run
through the ordinary feature matrix; the two dispatch/hosting cases require the Tower feature.

`path_capacity.rs` checks fallible construction at the ordinary-parameter boundary, including
escaped names, literal braces, affixes, and a catch-all beyond 25 ordinary parameters.
`path_escaped_suffix.rs` checks literal braces around captures, longer ASCII and Unicode names,
catch-all matching, raw values, definition errors, misses and capture rollback.
`refinement.rs` checks predicate/newtype alternatives, raw values and overflow, discarded captures,
ordered overlaps, error/lazy evaluation, conditional trait bounds, cross-thread selection, and
selected-service failure without retry. These tests run without default features except the
refinement dispatch check, which needs Tower.

`input_corpus.rs` adds deterministic generated raw header/query/path inputs and path-definition
boundaries, plus Accept-family malformed/policy/cache/limit checks. Seed and failing input are
reported for generated cases. Path controls assert static, catch-all and two-capture hits, preserve
the one-byte escaped-suffix case, and verify the 24, 25 and 26 ordinary-parameter boundaries.
Grammar-guided valid Accept-family fields compare every decided preferred and acceptable offer
with an independent oracle, at default and exactly sufficient bounds, and require every predicate
to refuse one below each bound. Absent, empty, standalone and repeated-field controls are separate,
including 16 accepted and 17 refused empty elements. Its eight test groups run in required CI
(four without negotiation); the negotiation groups print per-configuration winner, miss,
strict-refusal and undecided yields with `-- --nocapture`.
`lookup_truncation.rs` checks consecutive singleton subtags in positive lookup and
zero-quality exclusions, alongside the RFC private-use example.

The representative [Leptos fixture](../integration/leptos/README.md) runs with Axum and Warded on
Linux over localhost HTTP/1.1. Axum needs 3 Node HTTP-client passes and 5 browser passes per engine.
Warded needs 5 HTTP-client passes and 6 browser passes per engine. The broken ID predicate must
produce one specific failure in each of Chromium, Firefox and WebKit with no HTTP-client cases.
The runner checks the live server identity, binds reports to the exact source and verifies cleanup
after a forced failure. Hosted and local runs execute the same suite. It does not replace the
required Windows/Linux library CI or human release testing. On pull requests the required
`leptos-fixture` check runs the suite when the [fixture's inputs change](./release.md#before-rc).
The fixture builds Warded with only `tower`. Negotiation evidence comes from the library checks,
benchmarks, clean-consumer rehearsal and the server example.

## Documentation site

`docs/` is both the VitePress site and the unpublished `warded-docs` workspace member. Its
[`pages.rs`](../docs/pages.rs) includes each page as the documentation of an item that exists
only under `cfg(doctest)`, so `cargo test --package warded-docs --locked` compiles and runs every
`rust` code block on the site against the source in this checkout, on Windows and Linux. The
crate's unit tests fail when a page is missing from `pages.rs`, when a code block names a
language outside a short allowed list (rustdoc would silently skip a block marked `rs`), or when
a fence is indented or unclosed. The check reads the first word after the fence, so a rustdoc
attribute must follow the language after a space: `rust no_run` compiles a block without running
it, and VitePress still shows it as Rust. Rustdoc's usual comma form, `rust,no_run`, fails the
check. No page uses an attribute yet; examples are expected to run.
The crate package excludes `docs/`, so these checks never run from the package; the README's
examples run from the package's own doctests.

The required `ci` workflow's `site` job installs the pinned tooling with `npm ci` and runs
`npm run build` in `docs/`. VitePress fails on a link to a missing page, and the site
configuration's build hook fails on a link to a missing heading. Locally, use the Node.js major
version in `docs/.nvmrc` (`fnm use` or `nvm use` in `docs/`) and the same two commands; `npm run
dev` serves the site with live reload.

The [Pages resolution tests](../.github/scripts/pages-test.sh) run in the Linux `checks` job with
fake `gh` and `curl` commands. They cover SemVer precedence including prerelease numbers above 9,
yanked and missing versions, index failures, tag format, correction-branch ancestry, files and
renames outside `docs/`, and Release runs that did not publish. The Pages workflow itself runs only
on `main` after a release or by dispatch, so its first deployment is part of release
verification.

## Clean consumer

Use Bash with Rust 1.99.1, Git and HTTPS access to GitHub and crates.io. For an installable registry
version or an unreleased git revision, respectively:

```sh
bash scripts/check-clean-consumer.sh --registry <published-version> [previous-commit-sha-or-version]
bash scripts/check-clean-consumer.sh <pushed-commit-sha> [previous-release-sha]
```

The [script](../scripts/check-clean-consumer.sh) creates a disposable Cargo package outside the
source tree with its own target directory and uses the README's dependency snippets. Registry
mode first runs a fresh install of the exact version, checking its crates.io lockfile source and
checksum. It then tests an update from source beta.2 by default. Pass a previous published version
to exercise a registry-to-registry update. Git mode defaults to source beta.1 as its baseline.

Both modes update the baseline's existing lockfile, verify the candidate's exact version or
commit, run with defaults, without defaults and with each optional feature alone, then build
Warded's dependency rustdoc with warnings denied. They remove Warded, check the lockfile, and
restore and run the pinned candidate. Use `scripts/check.sh` for reconstructed package verification.

## Alpha.2 regression and cost probes

Six tests in `review_regressions.rs` were first run against unchanged alpha.1 (`f35a9ff`) in Linux
Docker, Rust 1.99.1: all failed for the reported media specificity/parameter ordering, lookup
priority/wildcards, concrete language definitions, and duplicate path captures. The same test bodies
remain as regression checks, alongside quoted/wildcard media cases, language exclusions, valid and
invalid tags, and field-byte limits with valid syntax and repeated-field separators.

The expanded suite checks borrowed configuration ownership, actual cross-thread selection,
constant-cost router cloning by service clone counts, copy-on-write isolation, boxed errors with
Axum bodies, non-default response types in request-returning mode, selected readiness failures,
cancellation, and outer concurrency admission. Shared-offer cache checks include differing sets,
heads, limits, and malformed-input policies. Strict operand-order behavior remains explicit.

Review regressions compare 50 escaped-name/catch-all combinations with matchit's decoded names,
reject malformed private-use tags while retaining valid private-use/grandfathered controls, and
exercise supported field-byte/member/parameter bounds across media, language, and encoding. The
unsupported token-limit setter was removed. Listener checks compare complete Vary tokens so
`accept-encoding` cannot satisfy an expected `accept` token.

Run `cargo bench --bench selection --locked` for the current focused cost probe: compiled path
tables with 16/128/1024 routes, independent versus shared media negotiation with 16/128 offers,
small and near-64-KiB fields, early/late/miss shapes, and near-64-KiB language lookup with
exclusions. A fresh head includes cache creation in each sample. Every sample asserts the target
or miss and, for path hits, the captures. The
[assessment](./beta-assessment.md#selection-cost-decision) records inputs, counts, environment,
measurements, limits and the indexing decision. This is not a CI timing threshold or throughput
promise. The earlier alpha.2 shared-offer probe used a different smaller input; retain its historical
evidence rather than presenting these different shapes as an implementation-only comparison.

## First alpha evidence

On 2026-11-10, the Linux Docker run with Rust 1.99.1 passed all commands above: 22 regression tests,
four doctests including the negative body-access check, each independent feature configuration,
Clippy with warnings denied, documentation, the example, and package reconstruction/execution.
The native Windows machine passed `cargo check`; its installed MSVC runtime libraries were
unavailable for linking. Hosted Windows CI supplies the required Windows execution evidence.

A negative control temporarily changed `Not` to accept a matching operand. The focused
`failed_branches_and_negation_cannot_publish_captures` test failed on its capture assertion, then
passed after the original source was restored. The mutation was made only in a disposable test
copy. The body-frame test also verifies that its poll counter advances when the returned body is
actually consumed, and the readiness fixture rejects calls on an unprepared clone.
