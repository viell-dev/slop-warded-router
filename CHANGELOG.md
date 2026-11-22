# Changes

## Unreleased

Changes on [`main`](https://github.com/viell-dev/slop-warded-router/blob/main/CHANGELOG.md#unreleased)
appear here before the next release.

- **Breaking:** `path` returns a `DefinitionError` for a parameter name that contains `:` or
  whitespace, such as `{id:[0-9]+}`, `{id:int}` or `{ id }`. Earlier versions accepted the whole
  text as the capture's name, so another router's constraint syntax silently matched any value.
  The message points at `Matcher::filter`. The
  [upgrading guide](https://viell-dev.github.io/slop-warded-router/upgrading.html) shows the
  change. [#39](https://github.com/viell-dev/slop-warded-router/issues/39)
- Document that `:name` and `*name` outside braces are literal path text, and add
  troubleshooting entries for other routers' path syntax and for Axum's `Path` extractor, which
  answers 500 `No paths parameters found for matched route` under a Warded route.
  [#39](https://github.com/viell-dev/slop-warded-router/issues/39)
- Extend the negotiation guide: offer `pt` beside a regional-only `pt-BR` so that lookup reaches
  it, let visitors choose a language with a query parameter or cookie declared before the
  negotiated routes, with the `Vary` values each route needs, and serve every page in each
  language from one route with a path capture. It also says that one malformed member makes the
  whole field malformed, so `fr, garbage!!` reaches the default.
  [#39](https://github.com/viell-dev/slop-warded-router/issues/39)
- Write the install command without quotes, as `cargo add warded@=<version> http@1.5`, which
  `cmd.exe` accepts as well as PowerShell and POSIX shells. Getting started also gives the
  dependencies of its Tower example their own `cargo add` commands.
  [#39](https://github.com/viell-dev/slop-warded-router/issues/39)
- Link Troubleshooting and Scope and limits from the site's top navigation and home page, give
  each site page its own description for search results, and reword dense API reference
  summaries, such as those of `Offers`, `Negotiated` and `Router`, in plain words. The crate's
  `homepage` is the documentation site. Library behavior is unchanged.
  [#39](https://github.com/viell-dev/slop-warded-router/issues/39)

## 0.1.0-beta.4

Adds the documentation site that the
[hands-on report in #26](https://github.com/viell-dev/slop-warded-router/issues/26) asked for before
RC, with one breaking rename and one change to encoding selection. The
[upgrading guide](https://viell-dev.github.io/slop-warded-router/upgrading.html) covers both.

- **Breaking:** rename the raw URI query-string helpers so a composition cannot mix them up with
  the QUERY method. Matching is unchanged: split on `&` and the first `=`, with no decoding. No
  deprecated aliases remain. [#30](https://github.com/viell-dev/slop-warded-router/issues/30)
  - `matchers::query_pair` → `matchers::param_pair`
  - `matchers::query_present` → `matchers::param_present`
  - `matchers::QueryMatcher` → `matchers::ParamMatcher`
- Add `matchers::query()`, which matches only the QUERY method
  ([RFC 10008](https://www.rfc-editor.org/rfc/rfc10008)), like `get()` and `post()`. Matchers still
  never read the request content. [#30](https://github.com/viell-dev/slop-warded-router/issues/30)
- **Behavior change from beta.3:** a shared `Offers::encodings` set ranks an `identity` coding that
  `Accept-Encoding` does not list below every coding it lists as acceptable. `gzip;q=0.9` with the
  offers `gzip` and `identity`, or `br;q=0.5, gzip;q=0.8` with `br`, `gzip` and `identity`, now
  prefers `gzip` instead of `identity`. Without an `identity` or `*` member, `identity` is preferred
  only when no listed offer is acceptable, as RFC 9110 section 12.5.3 asks. Explicit `identity` and
  `*` weights, refusals, empty and absent fields, and `accepts_encoding` are unchanged.
  [#33](https://github.com/viell-dev/slop-warded-router/issues/33)
- Add the [documentation site](https://viell-dev.github.io/slop-warded-router/), deployed from
  each release's tag, with guides for getting started, selection order, paths, request
  conditions, transport security, content negotiation, Tower and Axum, custom matchers,
  troubleshooting, limits, versions and upgrading. The negotiation guide works through languages
  with a default, lookup against basic filtering, weights and exclusions, absent, empty and
  malformed fields, `Vary`, media types, encodings and limits; the Tower guide serves such a page
  from an Axum application with `Content-Language` and `Vary` headers.
  [#29](https://github.com/viell-dev/slop-warded-router/issues/29)
- Rewrite the README around a language negotiation example, replacing an exact `Accept-Language`
  comparison, and add worked examples to the crate and `negotiation` rustdoc. Document existing
  behavior: a path segment holds at most one parameter.
- Every Rust example on the site compiles and runs in required CI on Windows and Linux. CI also
  builds the site, failing on links to missing pages or headings, and the release text check
  covers the site's install pins and versioned links. Maintainer notes move to `internals/`.
  These documentation changes leave library behavior unchanged.
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
