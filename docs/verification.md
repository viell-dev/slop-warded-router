# Verification

Use Rust 1.99.1 from `rust-toolchain.toml`. CI requires Windows and Linux checks and a successful
aggregate `ci` result before integration.

From the repository root, run the same entry point as both CI platforms:

```sh
bash scripts/check.sh
```

The [script](../scripts/check.sh) anchors itself to the checkout root and derives the packaged
manifest from Cargo's current package identity. It runs formatting, Clippy on all targets,
the four feature configurations and doctests, documentation with warnings denied, both examples,
package verification, and both packaged examples. Package output uses `CARGO_TARGET_DIR` when set,
otherwise `target`; the package step chooses that output explicitly to avoid stale-path fallback.

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
`refinement.rs` checks predicate/newtype alternatives, raw values and overflow, discarded captures,
ordered overlaps, error/lazy evaluation, conditional trait bounds, cross-thread selection, and
selected-service failure without retry. These tests run without default features except the
refinement dispatch check, which needs Tower.

`input_corpus.rs` adds deterministic generated raw header/query/path inputs and path-definition
boundaries, plus Accept-family malformed/policy/cache/limit checks. Seed and failing input are
reported; panics fail the test. Its four test groups run in required CI (two without negotiation).
The verified one-character-name escaped-suffix control covers the documented dependency workaround.

Run `bash integration/leptos/check.sh` in the provisioned Linux Docker image for the representative
application; its [README](../integration/leptos/README.md) gives exact container commands, triggers,
counts, negative control, cleanup checks and evidence limits. It uses the enclosing source, compares
21 Axum cases with 27 Warded cases across Chromium/Firefox/WebKit, and requires three deliberately
broken-predicate failures. It is a manual beta gate; hosted affected-change/release execution is
required before RC. The browser check does not replace ordinary required library CI or human use.

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
