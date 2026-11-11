# Upstream test comparison

> [!NOTE]
> Agent-authored and submitted on viell-dev's behalf by GPT-6.1 Sol via Codex. The owner has not
> reviewed this record.

Reviewed on 2026-11-10 to find useful edge cases for early releases. These are pinned snapshots of
upstream default branches, including unreleased work. We inspected relevant test areas and selected
cases; we did not run their complete suites or establish cross-router equivalence.

## Review snapshots

| Upstream | Revision | Areas inspected |
| --- | --- | --- |
| matchit | [`4b5a2c3`][matchit-match] | Matching/insertion tests: empty parameters, suffixes, catch-alls, overlaps and escaped braces |
| Axum | [`4739962`][axum-fallback] | Fallback, nesting, method/HEAD handling and state-clone tests |
| Warp | [`ff34d72`][warp-path] | Paths, filter alternatives, methods, headers and query tests |
| Actix | [`ae1d51a`][actix-resource] | Resource/router/path tests: rejected captures, recognition agreement, path boundaries and definitions |
| Rama | [`20abbf9`][rama-tests] | Boolean composition, unreachable alternatives and extension rollback |
| Salvo | [`b695140`][salvo-router] | Sibling/overwritten capture rollback, QUERY, path normalization and encoded separators |

This is a source of scenarios, not a requirement to adopt the same implementation or response
policy. The recurring review procedure is in [CONTRIBUTING](../CONTRIBUTING.md). For the next review,
inspect affected areas and upstream changes, update the snapshot/dispositions, and retain useful
failures. After the first stable release, this stops being a routine prerelease gate.

## Adopted cases

The independently written checks in [`tests/upstream_cases.rs`](../tests/upstream_cases.rs) first
passed against unchanged alpha.2 `0819b31` in Rust 1.99.1 Linux Docker: twelve tests with all
features, ten without default features. They close coverage gaps without requiring product changes.

| Source scenarios | Warded adaptation |
| --- | --- |
| [matchit matching][matchit-match] and [insertion cases][matchit-insert] | Empty captures, partial prefix/suffix matches, catch-all emptiness, query separation, and malformed parameter definitions |
| [matchit `overlapping_param_backtracking`][matchit-match] | An incomplete longer path or a failed header cannot hide a later overlapping candidate; assert only the winning captures |
| [Actix `capture_check_can_reject_a_match_without_mutating_path`][actix-resource], [Rama extension rollback][rama-tests], [Salvo sibling/overwritten rollback][salvo-router] | Rejected nested captures cannot overwrite an ancestor, create a spurious collision, or leak into a later alternative |
| [Salvo encoded-separator and dot-segment matrices][salvo-path] | Assert Warded preserves raw input and does not normalize it into another route; retain double encoding and encoded ordinary characters |
| [Warp full/raw-path][warp-path] and [raw-query cases][warp-query] | Absolute-form URIs match using their path while retaining the original authority/query; pathless CONNECT can use its authority-form URI |
| [Salvo QUERY method test][salvo-router], [Warp method tests][warp-method] | QUERY and an extension method match exactly, including case. A real bounded HTTP/1.1 test preserves query text and request bodies through ordinary Axum hosting |
| [Rama `test_and_or`, `test_and_never`, `test_or_never`][rama-tests] | Record evaluation order and prove unreachable boolean branches do not run |
| [Axum fallback/state-clone scenarios][axum-fallback] | Deriving routes and replacing a fallback on a cloned Warded router preserves the original routing/fallback snapshot |

These test bodies and local fixture data were written for Warded. No upstream test implementation
or complete test-vector dataset was copied. Source links retain scenario provenance; literal
source adaptation would additionally require its applicable licenses and notices.

A disposable negative control made a failed conjunction publish its left-hand captures. The new
nested-collision regression failed with `DuplicateCapture`; the original source passed again.
The mutation did not enter the repository. This verifies that the added case detects a broken
rollback contract.

## Existing coverage and different contracts

- Existing checks already cover escaped/decoded capture names, same-path/header alternatives,
  selection errors, discarded captures under negation, raw repeated fields, selected-instance
  readiness, clone counts, body frames/trailers, and cancellation. Upstream examples informed this
  review but do not justify duplicate tests for every equivalent spelling.
- matchit uses static-path precedence within a routing tree. Warded evaluates independently
  compiled patterns in declaration order across complete predicates. Adapt overlap inputs without
  importing trie precedence.
- Salvo's current path-state tests reject encoded separators and normalize some dot segments.
  Warded deliberately preserves raw paths. Its new tests assert that contract; they do not add
  decoding or sanitization. Downstream normalization can differ, so raw matching is not an
  authorization boundary.
- Warp aggregates some rejections into response-status priorities. Warded's ordinary mismatches
  continue, operational errors stop, and default exhaustion is 404. Do not import Warp's automatic
  400/405 expectations.
- Axum's GET-to-HEAD, 405/Allow, prefix stripping, and inherited nesting fallbacks are framework
  policies. Warded keeps them explicit or supplied by a mounted destination. Existing hosting tests
  cover that boundary; the new CONNECT head test does not establish tunnel or extended-CONNECT
  support.
- Actix resource tests can operate on strings containing literal newlines and on framework-specific
  prefix/regex state. Warded's head uses `http::Uri`; do not fabricate requests outside that type's
  input contract or add an unsupported prefix/regex API just to replay a foreign test.
- Typed query extraction, route generation, mutation/removal APIs, automatic method discovery, and
  framework-specific state machines need their own scope decision. They are not additions implied
  by this comparison. Capture-predicate refinement remains separate work in issue #9.

## Capacity validation and retained baseline failure

The capacity-validation family also reinforces the known fallible-path-construction issue
[#7](https://github.com/viell-dev/slop-warded-router/issues/7). A fresh baseline probe confirmed that
25 ordinary named parameters construct successfully, while 26 panic inside matchit. It failed with
exit 101. This is a recorded failure, not a passing test or a new defect discovered here.

The original test-only comparison kept that issue open for a targeted fix before beta. It did not
catch/ignore the panic or add a `should_panic` assertion that blesses it. No new product defect was
found in the twelve adopted cases.

The subsequent beta-hardening change reproduces that failure on `bfea539`, then moves the existing
decoded-name scan before insertion and rejects excess ordinary parameters. New public-boundary
checks cover 25 versus 26/27/100, a terminal catch-all, escaped literal braces and names, Unicode,
and affixed parameters. Relevant upstream capacity/escaping and rejection-rollback evidence above
is reused; capture refinement also checks rejected-capture isolation before later alternatives.
The earlier failed probe remains historical evidence; `tests/path_capacity.rs` requires an error
rather than a panic from the corrected constructor.

Independent review identified `{{extra}` immediately after the 25th ordinary parameter as another
capacity bypass. The exact probe panics on both `bfea539` and the initial guard head `6496fad`:
dependency normalization can lose the escaped opener's marker at the prior parameter's end.
Rejecting unmatched closing braces before insertion contains that malformed definition. The
permanent regression retains the input unchanged alongside successful capacity/escaping controls.
The finding and dependency disposition are tracked in issue #7 and the overview #4.

## Beta preparation refresh

On 2026-11-11, re-read Axum's fallback/state/method-not-allowed tests and Actix's rejected-capture,
raw-encoding, segment-boundary, matching-agreement and capacity cases. The current file contents
are unchanged from the earlier pinned snapshots: Axum file blob
`81204077fc5595ea21d5ef41ea4315a469440e08`, latest file commit
[`ab593bb`](https://github.com/tokio-rs/axum/blob/ab593bbab7e0e3203d8e15e09208f3cd8bd43fb4/axum/src/routing/tests/fallback.rs);
Actix file blob `0d9cabab29ec3be9eb4776f369249c19cfcebb86`, latest file commit
[`9fb5964`](https://github.com/actix/actix-web/blob/9fb59640aebc6a7c87bb79ead55b5730a5425ba4/actix-router/src/resource.rs).

The maintained Leptos fixture supplies real framework evidence for fallback, explicit method policy,
dynamic value rejection and selected failure; its extra rejection route prevents the framework's
renderer from re-admitting an invalid value. Do not import Axum's automatic method/nesting policy
or Actix's regex grammar and definition panics. The seeded corpus adds raw reachable inputs and
matching/definition/cache consistency checks without copying foreign test bodies or datasets.

The pinned `leptos_axum` 0.8.10 source was inspected for public SSR/context, server-function routing,
and file/error handlers before mounting them. Its route-list mode/methods and browser-navigation
boundary are asserted by the fixture. The matchit 0.9.2 escaped-marker splice remains unchanged;
retain the capacity regressions and the successful `/{a}{{x}}` workaround control for #13 rather
than asserting that rejecting a valid longer-name pattern is intended.

Boolean composition, dispatch readiness/cancellation, QUERY/extensions, malformed language/media
counterexamples and other raw-path contracts are unchanged by beta preparation. Reuse the earlier
Warp/Rama/Salvo/matchit comparison and retained failures for those areas. The preparation changes
tests, consumer evidence and public limits rather than importing another router's API/defaults.

[matchit-match]: https://github.com/ibraheemdev/matchit/blob/4b5a2c356022f544187d6e67325d8e98dd7f11ca/tests/match.rs
[matchit-insert]: https://github.com/ibraheemdev/matchit/blob/4b5a2c356022f544187d6e67325d8e98dd7f11ca/tests/insert.rs
[axum-fallback]: https://github.com/tokio-rs/axum/blob/47399621c4017601ea493cd63c3af581f4fb33e3/axum/src/routing/tests/fallback.rs
[warp-path]: https://github.com/seanmonstar/warp/blob/ff34d7213ed55ec342304aa7ff6ac4b351da9e66/tests/path.rs
[warp-query]: https://github.com/seanmonstar/warp/blob/ff34d7213ed55ec342304aa7ff6ac4b351da9e66/tests/query.rs
[warp-method]: https://github.com/seanmonstar/warp/blob/ff34d7213ed55ec342304aa7ff6ac4b351da9e66/tests/method.rs
[actix-resource]: https://github.com/actix/actix-web/blob/ae1d51adc04ae0a8cc06b396329fea67cbd4483d/actix-router/src/resource.rs
[rama-tests]: https://github.com/plabayo/rama/blob/20abbf9b4bfcfdc8bbac26f7c434e12e245d296a/rama-core/src/matcher/test.rs
[salvo-router]: https://github.com/salvo-rs/salvo/blob/b695140a8d2399be4e5fffe6faeaf834c8038bde/crates/core/src/routing/router.rs
[salvo-path]: https://github.com/salvo-rs/salvo/blob/b695140a8d2399be4e5fffe6faeaf834c8038bde/crates/core/src/routing/path_state.rs
