# Design and alpha boundaries

Warded evaluates synchronous conditions on a body-free view, selects the first complete candidate,
then executes exactly one service. This makes route applicability independent of response status
and body decoding. Custom predicates should be bounded and free of observable effects; the type
system excludes a body argument but cannot make arbitrary captured state pure.

The matching trait and `Selector` have no thread-safety requirement. The optional `Router` adapter
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
candidate cannot suppress a later parameter route. `parse-net` supplies Accept-family parsing and
negotiation, including strict quality values, media parameters, language schemes, and identity
encoding rules. Warded configures the parser's refusal policy and maps it to a miss or an explicit
matching error. It does not build a new Accept parser or inherit a framework's 400/406 policy.
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

Custom matcher and missing-context errors stop selection. A selected service's error, 404, or
pending state never retries another candidate. A configured consumer fallback supersedes ordinary
404 and error-on-miss; explicit no-match errors retain the unread request.

## Readiness and cancellation

The Tower adapter accepts a request and waits only for its selected destination, polling readiness
on the same cloned instance it subsequently calls. This avoids coupling unrelated destinations'
readiness. The router's ready result is not admission control: apply an outer Tower concurrency
limit. Cancellation drops the selected future through normal Tower behavior; no retry is attempted.

The alpha requires a common response body and service error type, and the response body must
implement `Default` for the implicit empty 404. Convert heterogeneous service responses or errors
before mounting them. It provides no queues, retry middleware, timeout policy, or automatic body
buffering.

## Negotiation and convenience helpers

`accepts_media_type`, `accepts_language`, and `accepts_encoding` each evaluate one offered value.
`Offers` compares one shared complete set and allows a route to require the preferred result.
Offers must have the same non-negotiation applicability; otherwise an unavailable handler could
win the offer comparison. Routing itself remains ordered.

Language matching chooses lookup or basic filtering explicitly. Missing fields use field-specific
protocol semantics; for example, absent and empty Accept-Encoding differ. Repeated values are
combined as ordered lists. Invalid syntax ordinarily misses; `invalid_is_error` exposes the parser
diagnostic. The alpha bounds combined field bytes to 64 KiB, exposes parser limits, and performs
bounded negotiation per predicate evaluation. It does not yet cache parsed results across routes.

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
Trusted transport/proxy integration must populate `TransportSecurity` correctly. Real HTTP/1.1,
HTTP/2, and HTTP/3 server negotiation remains outside this alpha's evidence.
