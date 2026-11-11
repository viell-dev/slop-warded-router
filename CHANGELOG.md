# Changes

## 0.1.0-alpha.3 source candidate

This is unpublished source. Cargo publication remains disabled; beta scope assessment is pending.

- Add `Matcher::filter` for predicates over tentative raw captures, including optional newtype
  parsing, with ordinary miss/error semantics and no mandatory `Clone` or thread-safety bounds.
- Return a definition error above 25 ordinary path parameters; retain a terminal catch-all beyond
  that bound. Reject unmatched closing braces before dependency normalization can expose an extra
  parameter at an escaped-suffix boundary.
- Add upstream-inspired regression coverage, including actual HTTP/1.1 QUERY/extension-method
  body preservation, and recurring targeted comparison during early releases.
- Declare the tested Rust minimum as 1.99.1, matching the required verification toolchain.

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
