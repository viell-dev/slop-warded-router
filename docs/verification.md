# Verification

Use Rust 1.99.1 from `rust-toolchain.toml`. Integration requires `ci` (Windows and Linux library
checks and Linux dependency policy) and `leptos-fixture` (the fixture gate).

From the repository root, run the same entry point as both CI platforms:

```sh
bash scripts/check.sh
```

The [script](../scripts/check.sh) anchors itself to the checkout root and derives the packaged
manifest from Cargo's current package identity. It runs formatting, Clippy on all targets,
the four feature configurations and doctests, documentation with warnings denied, both examples,
package verification, and both packaged examples. Package output uses `CARGO_TARGET_DIR` when set,
otherwise `target`; the package step chooses that output explicitly to avoid stale-path fallback.

A separate required Linux job runs `cargo deny --locked check` with cargo-deny 0.20.2, verified
by the release checksum recorded in the workflow. [deny.toml](../deny.toml) covers the all-features
library graph including dev dependencies: RustSec vulnerability, unmaintained, unsound and yanked
advisories fail the job, as do licenses outside the allow list, wildcard requirements and sources
other than crates.io. Run the same command locally after installing that cargo-deny version. The
Leptos fixture's separate lockfile is not part of this graph. Dependabot proposes weekly Cargo and
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
establish TLS/proxy trust, browser workflows, HTTP/2, or HTTP/3. Human use of a representative
candidate remains required before a stable release.

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

The representative [Leptos fixture](../integration/leptos/README.md) runs with Axum and Warded on
Linux over localhost HTTP/1.1. Axum needs 3 Node HTTP-client passes and 5 browser passes per engine.
Warded needs 5 HTTP-client passes and 6 browser passes per engine. The broken ID predicate must
produce one specific failure in each of Chromium, Firefox and WebKit with no HTTP-client cases.
The runner checks the live server identity, binds reports to the exact source and verifies cleanup
after a forced failure. Hosted and local runs execute the same suite. It does not replace the
required Windows/Linux library CI or human release testing. On pull requests the required
`leptos-fixture` check runs the suite when the [fixture's inputs change](./release.md#before-rc).

## Clean git-source consumer

Use a Bash environment with Rust 1.99.1, Git and HTTPS access to GitHub and crates.io. Pass the
full SHA of a pushed revision:

```sh
bash scripts/check-clean-consumer.sh <pushed-commit-sha>
```

The [script](../scripts/check-clean-consumer.sh) creates a disposable Cargo package outside the
source tree with its own target directory. It first runs against beta.1, then changes the `rev`
and updates the existing lockfile. It checks the exact resolved commit, runs matching and
example-equivalent feature checks with defaults, without defaults and with each optional feature
alone, then builds Warded's dependency rustdoc with warnings denied. Finally it removes Warded,
checks that its package and source are gone from the lockfile, and restores and runs the pinned
candidate. Both revisions are resolved through Git. Use `scripts/check.sh` separately for
reconstructed Cargo package verification.

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
tables with 16/128/1024 routes, independent versus shared negotiation with 16/128 offers, small and
near-64-KiB fields, and early/late/miss shapes. A fresh head includes cache creation in each sample;
every sample asserts the target or miss and path captures. The
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
