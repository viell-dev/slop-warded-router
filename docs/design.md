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
quality values, quoting, media-range parameters and matching, and identity encoding rules.
Alpha.1 counterexamples exposed incorrect parameter specificity, loss of parameters following `q`,
and lookup priority in its 0.9.3 negotiation scorer, so alpha.2 added a small media/language
selection layer. Version 0.10.0 fixes those reported cases, and its own selection passes the
retained counterexamples. Warded now reuses its corrected range parameters and matching, but keeps
its own media precedence and language selection, because 0.10.0 chooses differently in documented
edge cases: a zero-quality lookup range refuses only an equal offer, lookup counts `*` only as the
last range in priority, basic filtering breaks equal weights by the longer range, and media
specificity stops counting at 63 parameters. The lookup and specificity rules are documented
choices; the basic-filtering tie departs from parse-net's own description, which gives equal
weights to the earlier offer as Warded does. `oxilangtag` validates concrete well-formed server
tags independently of client negotiation.

The initial alpha.2 candidate used `language-tags`, whose private-use branch accepted malformed
subtags. Its own README recommends `oxilangtag` for syntax-only validation. The latter requires no
registry database or runtime dependencies, parses borrowed tags without allocation, and passes the
retained private-use and grandfathered controls. Dependency defects are tracked in the pinned
[overview](https://github.com/viell-dev/slop-warded-router/issues/4).

The parser dependency is young and still changes selection answers between releases; its
version is pinned exactly, its types are kept out of Warded's public API, and upgrades need the
retained protocol counterexamples and the grammar-guided corpus. Reconsider the local selection
rules only together with Warded's documented semantics. This avoids maintaining a separate field
grammar or forking an entire parser to correct selection rules. Warded maps malformed fields to a
miss or an explicit matching error and inherits no framework 400/406 policy.
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

Construction rejects duplicate decoded names, unmatched closing braces and more than 25 ordinary
parameters before calling matchit. Version 0.9.2 panics while normalizing the 26th ordinary
parameter. A terminal catch-all does not consume ordinary-parameter capacity. The closing-brace
check first contained the escape-marker defect below; after its repair it is defence in depth.
Definition errors from matchit are prefixed with the declared pattern.

matchit 0.9.2 can lose the escape marker for a literal opener immediately after a parameter
when normalization changes the parameter's byte length. Warded validates the affected names with
matchit, inserts a one-byte name at each affected position and restores the declared names in
captures. matchit still validates the complete pattern. For example, `/{id}{{x}}` matches
`/value{x}` with `id=value`. Unicode names and raw capture values retain their original meaning.
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
protocol semantics. An absent field accepts every offer, so a set chooses its first. A present
field that parses as a list without members (an empty value, or only empty elements or empty
lines) accepts no media type or language, so even a strict predicate misses; such an
Accept-Encoding accepts only identity. Repeated values are combined as ordered lists. An empty line
adds no member, but once lines are combined it is an empty list element, and parsing refuses more
than 16 empty elements, a fixed inherited limit: such a field is malformed, not memberless.
Invalid syntax ordinarily misses; `invalid_is_error` exposes the parser
diagnostic. Combined field bytes default to 64 KiB and can be configured with Warded's own `Limits`,
along with list-member and parameter counts. No per-token length control is exposed: the
dependency's similarly named setting applies to other grammar paths.
One immutable head caches complete choices for shared offer sets and limits, including parser
failures before error/miss mapping. The cache retains each set to prevent address reuse and is
thread-safe; cloning a head starts a new cache. Independent sets still parse independently. There is
no route index, and raw path interpretation and ordered error propagation remain unchanged.

Media specificity compares wildcard/type specificity and then the number of applicable parameters;
`q` is a weight wherever it appears. Basic filtering weights a tag by its longest matching range,
and `*` covers only tags no other range matches. An encoding offer takes the weight of its listed
coding, otherwise of `*`. RFC 9110 §12.5.3 lets `identity;q=0`, or `*;q=0` without an `identity`
entry, exclude an unlisted identity; Warded's policy also gives an unlisted identity a positive
`*` weight. Media, encoding, and basic filtering ties use offer order. Lookup processes positive
client ranges by descending weight/client order, completing truncation before the next range, and
uses a wildcard only as a final default. As in RFC 4647 section 3.4, a one-character subtag is
removed together with its closest trailing subtag, so `x-a-b-cc` never truncates to `x-a`.
Zero-quality ranges reject an equal or less specific lookup match (an exact range, then a truncated
one, then `*`), retaining explicit exclusions without letting a broad wildcard erase a more specific
positive preference. Registry validation of server language tags is outside the constructor's
well-formed-syntax contract.

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
