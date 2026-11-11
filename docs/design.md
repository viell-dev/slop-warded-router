# Design and alpha boundaries

Warded evaluates synchronous conditions on a body-free view, selects the first complete candidate,
then executes exactly one service. This makes route applicability independent of response status
and body decoding. Custom predicates should be bounded and free of observable effects; the type
system excludes a body argument but cannot make arbitrary captured state pure.

The matching trait and local `Selector` have no thread-safety requirement; `SyncSelector` retains
`Send + Sync` bounds for a shared table without requiring Tower. The optional `Router` adapter
uses cloneable `Send + Sync` Tower services and matchers. A selector returns a borrowed target and
owned selection metadata. Tower dispatch stores the selected `Selection` and `Captures` in request
extensions and preserves the request URI and body.

## Existing work and dependency choices

The [pinned comparison](https://github.com/viell-dev/slop-warded-router/tree/main/research/comparison)
and its executable results cover Actix
Web 4.19.0, Warp 0.4.3, Salvo 1.1.1, and Rama 0.4.0. Complete request predicates already exist in
those projects; Warded does not claim to invent them. Actix resource guards reproduce the basic
fallthrough, while route guards and Warp rejections illustrate different native status policies.
These frameworks remain usable destinations or comparison material without becoming Warded's
required runtime and request representation.

Rama's generic boolean matcher and ordered dispatch are the closest architectural precedent. The
alpha instead uses a small original evaluator to keep evaluation errors distinct from misses,
exclude bodies from the public predicate interface, support local-only predicates, and provide
capture-free negation. It does not adapt or copy upstream matcher source.

`http` supplies request-head types. `matchit` compiles each path independently: a failed static-path
candidate cannot suppress a later parameter route. `parse-net` supplies grammar parsing, strict
quality values, quoting, and identity encoding rules. Alpha.1 counterexamples exposed incorrect
parameter specificity, loss of parameters following `q`, and lookup priority in its negotiation
scorer. Alpha.2 retains the grammar parsers and supplies a small media/language selection layer,
reading all parameters through the existing parameter parser. `oxilangtag` validates concrete
well-formed server tags independently of client negotiation.

The initial alpha.2 candidate used `language-tags`, whose private-use branch accepted malformed
subtags. Its own README recommends `oxilangtag` for syntax-only validation. The latter requires no
registry database or runtime dependencies, parses borrowed tags without allocation, and passes the
retained private-use and grandfathered controls. Dependency defects are tracked in the pinned
[overview](https://github.com/viell-dev/slop-warded-router/issues/4).

The parser dependency is young; its version is pinned, its types are kept out of Warded's public
API, and upgrades need the retained protocol counterexamples. Reassess the selection workaround
when a verified upstream release handles them. This avoids maintaining a separate field grammar or
forking an entire parser to correct selection rules. Warded maps malformed fields to a miss or an
explicit matching error and inherits no framework 400/406 policy.
Tower supplies service boxing and readiness-aware `oneshot` dispatch.

These are ordinary dependencies with their original license terms; see Cargo's resolved graph
and the upstream packages for their notices. Warded's original code uses MIT OR Apache-2.0.

## Selection and captures

All dimensions are peers. Undeclared conditions impose no restriction. Declaration order applies
even when paths overlap or an earlier route has no path condition. The alpha scans candidates
linearly and does not claim a performance advantage.

Each matcher returns its own tentative captures. Conjunction merges only successful results;
alternatives commit the first success; negation discards operand captures. A collision is an
explicit evaluation error. Captures are raw owned strings and do not automatically populate Axum's
private path extractor state. Static path syntax errors fail construction; name collisions between
composed matchers are diagnosed during evaluation.

`Matcher::filter` refines a successful match through a read-only predicate on its tentative
captures. Rejection is an ordinary miss, accepted captures stay raw, and errors from the wrapped
matcher propagate without invoking the predicate. It can validate several captures after a
composition without introducing typed capture storage or another path grammar. Conditional
`Clone`/thread-safety bounds follow the wrapped matcher and callback; local-only and non-cloneable
callbacks remain usable with `Selector`. The predicate/newtype/regex comparison and measurements
are retained in the independent `experiment-path-constraints` branch and issue #9.

Construction preflights decoded names and ordinary-parameter capacity before calling matchit.
Version 0.9.2 panics while normalizing the 26th ordinary parameter; Warded instead returns a
definition error. A terminal catch-all is not normalized and does not count toward the 25-parameter
limit. Unmatched closing braces are rejected before dependency normalization: matchit can lose an
escaped opener's marker at a parameter splice boundary, exposing an extra parameter from an invalid
suffix. matchit still validates the full pattern grammar. Escaped braces and affixed parameters
retain their existing meaning, with an inherited escaped-adjacency limit: `/{id}{{x}}` is rejected
by matchit 0.9.2, while the verified one-character-name workaround `/{a}{{x}}` matches `/value{x}`.
See [issue #13](https://github.com/viell-dev/slop-warded-router/issues/13). Remove or revise the
capacity guard only after a tested upstream change.

Custom matcher and missing-context errors stop selection. A selected service's error, 404, or
pending state never retries another candidate. A configured consumer fallback supersedes ordinary
404 and error-on-miss; explicit no-match errors retain the unread request.

## Readiness and cancellation

The Tower adapter accepts a request and waits only for its selected destination, polling readiness
on the same cloned instance it subsequently calls. This avoids coupling unrelated destinations'
readiness. The router's ready result is not admission control: apply an outer Tower concurrency
limit. Cancellation drops the selected future through normal Tower behavior; no retry is attempted.

The alpha requires a common response body and service error type. `Router` needs a `Default` body
for its implicit 404, whose body is that default value. `RequestRouter` removes that bound and uses
`RequestDispatchError` to preserve unmatched requests. Ordinary `DispatchError<E>` contains no body.
Convert heterogeneous service responses or errors before mounting them. Router clones share an
immutable table; changing a shared clone copies the table. Dispatch clones only its destination.
There are no queues, retry middleware, timeout policy, or automatic body buffering.

## Negotiation and convenience helpers

`accepts_media_type`, `accepts_language`, and `accepts_encoding` each evaluate one offered value.
`Offers` compares one shared complete set and allows a route to require the preferred result.
Offers must have the same non-negotiation applicability; otherwise an unavailable handler could
win the offer comparison. Routing itself remains ordered.

Language matching chooses lookup or basic filtering explicitly. Missing fields use field-specific
protocol semantics; for example, absent and empty Accept-Encoding differ. Repeated values are
combined as ordered lists. Invalid syntax ordinarily misses; `invalid_is_error` exposes the parser
diagnostic. Combined field bytes default to 64 KiB and can be configured with Warded's own `Limits`,
along with list-member and parameter counts. No per-token length control is exposed: the
dependency's similarly named setting applies to other grammar paths.
One immutable head caches complete choices for shared offer sets and limits, including parser
failures before error/miss mapping. The cache retains each set to prevent address reuse and is
thread-safe; cloning a head starts a new cache. Independent sets still parse independently. There is
no route index, and raw path interpretation and ordered error propagation remain unchanged.

Media specificity compares wildcard/type specificity and then the number of applicable parameters;
`q` is a weight wherever it appears. Media, encoding, and basic filtering ties use offer order.
Lookup processes positive client ranges by descending weight/client order, completing truncation
before the next range, and uses a wildcard only as a final default. Zero-quality ranges reject an
equal or less specific lookup match, retaining explicit exclusions without letting a broad wildcard
erase a more specific positive preference. Registry validation of server language tags is outside
the constructor's well-formed-syntax contract.

Method helpers match only their named method. `secured` and `unsecure` inspect trusted contextual
data, not URI scheme or forwarding headers. Unknown context errors unless explicitly made a miss.
`secured_post` and `secured_get_and_head` are ordinary optional compositions; they add no special
dispatch behavior. HTTPS is not authentication, and a GET-only condition does not match HEAD.

## Integration and limits

The adapter mounts ordinary Tower services and the regression suite mounts Axum with its own state,
path extraction, and extensions. This does not establish compatibility with every framework,
server transport, or UI runtime. The router implements no TLS, protocol negotiation, authentication,
automatic 405/OPTIONS/CORS response, redirect, compression, or body validation. HEAD response-body
rules remain the HTTP stack's responsibility.

Header-based variants and fallbacks need appropriate `Vary` or cache-control middleware around the
router. Negotiation helpers do not generate response metadata or encode a selected representation.
Trusted transport/proxy integration must populate `TransportSecurity` correctly. Host conditions
use custom URI/header predicates; HTTP/1.1 `Host` and HTTP/2 `:authority` are distinct protocol inputs,
and `Host` cannot replace an available authority. No dedicated host helper is part of 0.1.
Real localhost HTTP/1.1 is exercised by the hosting tests and representative Leptos fixture.
HTTP/2/3, TLS negotiation and proxy trust remain outside this evidence. See the
[freeze assessment](./beta-assessment.md) for API and cost decisions and the
[release procedure](./release.md) for the agreed compatibility/stage policy.
