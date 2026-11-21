# Changes

## Unreleased

Changes on `main` appear here before the next release.

- **Breaking:** rename the raw URI query-string helpers so a composition cannot mix them up with
  the QUERY method. Matching is unchanged: split on `&` and the first `=`, with no decoding. No
  deprecated aliases remain. [#30](https://github.com/viell-dev/slop-warded-router/issues/30)
  - `matchers::query_pair` → `matchers::param_pair`
  - `matchers::query_present` → `matchers::param_present`
  - `matchers::QueryMatcher` → `matchers::ParamMatcher`
- Add `matchers::query()`, which matches only the QUERY method
  ([RFC 10008](https://www.rfc-editor.org/rfc/rfc10008)), like `get()` and `post()`. Matchers still
  never read the request content. [#30](https://github.com/viell-dev/slop-warded-router/issues/30)
- Release tooling: the repository check also rejects a missing README version pin, stale pins in
  any form, a missing changelog release heading and relative README links outside code blocks.
  The release workflow installs the toolchain `rust-toolchain.toml` names, and its publication
  confirmation stays within the job timeout. The release tests cover index name and checksum
  validation and the README rules.

## 0.1.0-beta.3

- Prepare the first [crates.io publication](https://crates.io/crates/warded/0.1.0-beta.3)
  and [versioned API reference](https://docs.rs/warded/0.1.0-beta.3) for the
  [hands-on test in #26](https://github.com/viell-dev/slop-warded-router/issues/26).
- Limit the crate package to the library, runnable examples, tests, benchmark, README and licenses.
  Add exact-commit release checks, checksum verification and trusted publishing for later versions.
  Require the README install pin, version-pinned links and changelog heading to match the package.
- Document registry installation, updates, removal and recovery, and add a clean registry-consumer
  check. Library behavior is unchanged from source beta.2.

## 0.1.0-beta.2 source candidate

The second source beta for the agreed router-only first release. Cargo publication remains disabled.

- Remove every trailing one-character subtag during language lookup truncation, as RFC 4647
  section 3.4 requires: `x-a-b-cc` no longer reaches the offer `x-a`, as a match or as an exclusion.
  [#23](https://github.com/viell-dev/slop-warded-router/pull/23)
- Accept escaped literal suffixes after longer ASCII and Unicode path parameter names. Preserve
  declared capture names and raw values while handling matchit 0.9.2's normalization locally.
  [#18](https://github.com/viell-dev/slop-warded-router/pull/18)
- Add a security policy with GitHub private vulnerability reporting as the private route.
- Check RustSec advisories, licenses, wildcard requirements and dependency sources with a pinned,
  checksum-verified cargo-deny in required CI. The policy configures zero advisory ignores.
- Configure weekly Dependabot updates for the library's Cargo dependencies and GitHub Actions.
  [#17](https://github.com/viell-dev/slop-warded-router/pull/17) covers these dependency and
  reporting changes.
- Run the representative Leptos fixture in GitHub Actions for affected changes and source release
  tags, gated by a required `leptos-fixture` check that every pull request reports. Report
  HTTP-client and browser results separately, check selected page elements and live router
  identity, and retain exact-revision reports, traces and server logs.
  [#20](https://github.com/viell-dev/slop-warded-router/pull/20)
- Add grammar-guided valid Accept, Accept-Language and Accept-Encoding corpus cases whose preferred
  and acceptable offers come from an independent oracle written from RFC 9110, RFC 4647 and the
  documented rules. Separate absent, empty, standalone and repeated-field controls, and report
  per-configuration winner, miss, refusal and undecided yields.
- Document existing behavior: a present `Accept` or `Accept-Language` field that parses as a list
  without members accepts no offer, also in strict mode; empty list elements, including combined
  empty lines, are limited to 16; basic filtering weights a tag by its longest matching range; and
  a positive `Accept-Encoding` wildcard weight also applies to an unlisted `identity`.
- Update parse-net from 0.9.3 to 0.10.0, still pinned exactly. Reuse its corrected media-range
  parameters and matching while preserving Warded's media precedence and language selection.
- Document that a negotiation member carries at most 64 parameters, counting `q`, whatever the
  configured parameter bound. This limit is inherited.
  [#22](https://github.com/viell-dev/slop-warded-router/pull/22) covers the parser and oracle work.
- Make each zero-quality lookup exclusion check linear in the offer length. Add a
  correctness-checked near-bound language lookup measurement.
- Detect fixture-affecting deletions and non-ASCII paths, reject missing gate output, and keep
  separate CI and fixture runs for non-PR triggers. Fail on unmaintained and unsound advisories
  throughout the library dependency graph.
- Verify a disposable consumer's pinned git install, feature builds, dependency rustdoc, update,
  removal and recovery using the README dependency snippet. Document the targeted upstream
  review, negotiation defaults and the proposed RC source and documentation channels.
  [#24](https://github.com/viell-dev/slop-warded-router/pull/24) covers these promotion changes.

## 0.1.0-beta.1 source candidate

The first source beta for the agreed router-only release integrated through
[#16](https://github.com/viell-dev/slop-warded-router/pull/16) at `59710b0`.

- Maintain a representative Leptos application using Warded as its outer HTTP router, compared with
  an Axum baseline across three browser engines. Cover dynamic captures, ID/username predicates,
  header/query alternatives, real forms/server functions, SSR streaming, hydration and failures.
- Assert complete browser reports, a broken-predicate negative control and success/failure cleanup.
- Add a deterministic seeded input corpus to ordinary required CI, covering raw matching, compiled
  paths, path-definition boundaries, negotiation limits, miss/error policies and cache consistency.
- Record a consumer-informed API freeze assessment, reproducible selection/negotiation measurements,
  dependency dispositions, and explicit host/method/raw-query/transport limits.
- Document matchit 0.9.2's escaped-suffix limitation and its verified one-byte ASCII-name
  workaround.

## 0.1.0-alpha.3 source candidate

This is unpublished source. Cargo publication remains disabled. The [first-release goal](https://github.com/viell-dev/slop-warded-router/issues/8)
is agreed; its beta implementation gates remain unfinished.

- Add `Matcher::filter` for predicates over tentative raw captures, including optional newtype
  parsing, with ordinary miss/error semantics and no mandatory `Clone` or thread-safety bounds.
- Return a definition error above 25 ordinary path parameters; retain a terminal catch-all beyond
  that bound. Reject unmatched closing braces before dependency normalization can expose an extra
  parameter at an escaped-suffix boundary.
- Add upstream-inspired regression coverage, including actual HTTP/1.1 QUERY/extension-method
  body preservation, and recurring targeted comparison during early releases.
- Declare the tested Rust minimum as 1.99.1, matching the required verification toolchain.
- Derive packaged-source verification paths from Cargo and run the same entry point in local
  checks and Windows/Linux CI, avoiding stale example paths after version changes.

## 0.1.0-alpha.2 source candidate

This is unpublished source. Cargo publication remains disabled.

- Correct media-range parameter specificity and parameters on either side of `q`.
- Correct weighted language lookup and wildcard defaults; validate concrete server tags with an
  existing syntax-only language-tag parser, including private-use subtag boundaries.
- Cache complete choices for shared offer sets within one immutable head and limits.
- Own borrowed configuration in named raw path/header/query matcher types; add `SyncSelector` while
  preserving local-only custom matchers in `Selector`.
- Reject duplicate decoded capture names within one compiled path at construction, respecting
  escaped opening and closing braces in names.
- Share service tables when cloning routers, with copy-on-write when editing a shared clone.
- Separate ordinary `DispatchError<E>` from request-preserving misses. `error_on_miss()` now returns
  `RequestRouter` with `RequestDispatchError<B, E>` and supports non-default response bodies.
- Add a real Axum HTTP/1.1 hosting example with error mapping, trusted cleartext context, HEAD/405
  preservation, and outer Vary policy.

Breaking alpha API changes: `Head` is cloneable rather than `Copy`; language matching and limits
use Warded-owned types; string-taking helper return types are named; no-match errors and their
adapter are separate from ordinary dispatch errors. Strict ordered error propagation is retained.

## 0.1.0-alpha.1

Initial unpublished source alpha: ordered complete-predicate selection, isolated captures,
body-free matching, raw path/header/query helpers, optional negotiation and Tower dispatch,
selected-only readiness, and explicit fallback/no-match policies.
