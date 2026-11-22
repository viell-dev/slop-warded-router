# Design and first-release boundaries

Warded evaluates synchronous conditions on a body-free view, selects the first complete candidate,
then executes exactly one service. This makes route applicability independent of response status
and body decoding. The type system excludes a body argument but cannot make arbitrary captured
state pure, so the [rules for custom matchers](../docs/custom-matchers.md#rules-for-custom-matchers)
are documented obligations rather than enforced ones.

This note records why the design is as it is: the alternatives, the evidence and the dependency
choices. The consumer documentation owns the resulting rules and their examples, and each section
links the pages that state them: [how selection works](../docs/selection.md),
[paths and captures](../docs/paths.md), [content negotiation](../docs/negotiation.md),
[Tower and Axum](../docs/tower.md) and [scope and limits](../docs/limits.md). When a rule changes,
change it there and keep only its rationale here.

`Selector`, `SyncSelector` and `Router` share one selection function and differ in their bounds
([the three types](../docs/selection.md#selector-syncselector-and-router)). Generic borrowed
targets keep local selection free of Tower, body and thread-safety requirements, so local-only
predicates work without the optional adapter; the [assessment](./beta-assessment.md) records that
API decision.

## Existing work and dependency choices

The [pinned comparison](https://github.com/viell-dev/slop-warded-router/tree/main/research/comparison)
and its executable results cover Actix
Web 4.19.0, Warp 0.4.3, Salvo 1.1.1, and Rama 0.4.0. Complete request predicates already exist in
those projects; Warded does not claim to invent them. Actix resource guards reproduce the basic
fallthrough, while route guards and Warp rejections illustrate different native status policies.
These frameworks remain usable destinations or comparison material without becoming Warded's
required runtime and request representation.

Rama's generic boolean matcher and ordered dispatch are the closest architectural precedent. The
router uses a small original evaluator to keep evaluation errors distinct from misses,
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
weights to the earlier offer as Warded does. Encoding selection keeps parse-net's weights and
acceptability rules, except that an unlisted `identity` ranks below every acceptable listed coding.
parse-net 0.10.0 weights it as `q=1`, a weight RFC 9110 §12.5.3 does not assign. Its documented
contract covers acceptability, which it meets, so Warded re-ranks that one choice locally rather
than reporting a defect ([#33](https://github.com/viell-dev/slop-warded-router/issues/33)).
`oxilangtag` validates concrete well-formed server tags independently of client negotiation.

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

Declaration order is the only priority, across every dimension and overlapping paths
([the rule](../docs/selection.md#the-first-complete-match-wins)). The router scans candidates
linearly and does not claim a performance advantage; the [selection-cost
decision](./beta-assessment.md#selection-cost-decision) records the measurements and the condition
for revisiting an index.

Errors stop selection instead of becoming misses, and a selected service's response, error or
pending state never retries another candidate ([misses and
errors](../docs/selection.md#misses-and-errors), [misses and
fallbacks](../docs/tower.md#misses-and-fallbacks)). Treating an error as a miss could select a
route the request should not reach, and a retry would make applicability depend on a response.

Captures are tentative per matcher until their route is selected ([captures in
combinations](../docs/selection.md#captures-in-combinations)). Custom matchers can produce capture
names that are known only during evaluation, so a collision between composed matchers is diagnosed
then, as `DuplicateCapture`; static path syntax errors still fail construction. Captures stay raw
owned strings. Axum's path extractor state is private, so Warded cannot fill it.

`Matcher::filter` ([refine captures](../docs/paths.md#refine-captures-with-filter)) validates
several captures after a composition without introducing typed capture storage or another path
grammar. Its `Clone` and thread-safety bounds follow the wrapped matcher and callback, so local-only
and non-cloneable callbacks remain usable with `Selector`. The predicate/newtype/regex comparison
and measurements are retained in the independent `experiment-path-constraints` branch and issue #9.

Construction rejects duplicate decoded names, unmatched closing braces and more than 25 ordinary
parameters before calling matchit ([pattern rules](../docs/paths.md#pattern-rules-and-limits)).
Version 0.9.2 panics while normalizing the 26th ordinary parameter, which sets that limit; a
terminal catch-all does not consume ordinary-parameter capacity. The closing-brace check first
contained the escape-marker defect below; after its repair it is defence in depth. Definition
errors from matchit are prefixed with the declared pattern.

Construction also rejects a name containing `:` or whitespace. matchit 0.9.2 accepts such names,
so another router's constraint syntax, such as `{id:[0-9]+}`, became a parameter that matched any
value; refusing it keeps `filter` the one place for value conditions
([#39](https://github.com/viell-dev/slop-warded-router/issues/39)).

matchit 0.9.2 can lose the escape marker for a literal opener immediately after a parameter
when normalization changes the parameter's byte length. Warded validates the affected names with
matchit, inserts a one-byte name at each affected position and restores the declared names in
captures. matchit still validates the complete pattern. For example, `/{id}{{x}}` matches
`/value{x}` with `id=value`. Unicode names and raw capture values retain their original meaning.
See [issue #13](https://github.com/viell-dev/slop-warded-router/issues/13). Remove or revise the
capacity guard only after a tested upstream change.

## Readiness and cancellation

The Tower adapter polls readiness only on a clone of the selected destination and calls that same
clone ([readiness and admission](../docs/tower.md#readiness-and-admission)), so an unrelated
destination that is not ready never delays a request. Tower's `Steer` takes the other approach:
its `poll_ready` waits for every service, because it cannot know which one the next request
needs. The consequence of Warded's choice is that its readiness is not admission control, so hosts
add an outer limit. Cancellation drops the selected future through normal Tower behavior; no retry
is attempted.

A Tower service has one response and error type, so all destinations share them
([routes and services](../docs/tower.md#routes-and-services)). `RequestRouter` exists for response
bodies without the `Default` that the implicit 404 needs, and for callers that take back an
unmatched request with its body unread. `DispatchError<E>` holds no body, so it converts into
Tower's `BoxError`.

## Negotiation and convenience helpers

Negotiation is a route condition, so it never reorders routes: declaration order stays the only
priority, and a shared `Offers` set only decides which of its routes can match ([two kinds of
conditions](../docs/negotiation.md#two-kinds-of-conditions)). The cost is that routes sharing a
set need the same other conditions, or the set can prefer an unavailable alternative ([shared
offers](../docs/negotiation.md#shared-offers-or-independent-conditions)).

Field handling is in [absent, empty and malformed
headers](../docs/negotiation.md#absent-empty-and-malformed-headers) and [limits and
cost](../docs/negotiation.md#limits-and-cost). Details the site leaves out:

- An empty line adds no member, but once repeated lines are combined it is an empty list element,
  and parse-net refuses more than 16 empty elements, a fixed inherited limit: such a field is
  malformed, not memberless.
- `Limits` configures combined field bytes, list members and parameters per member. The
  dependency's per-token length setting applies to other grammar paths, so Warded does not expose
  it.
- One immutable head caches complete choices for shared offer sets and limits, including parser
  failures before error/miss mapping. The cache retains each set to prevent address reuse and is
  thread-safe; cloning a head starts a new cache. Independent sets still parse independently.
  There is no route index.

The selection rules are in [weights, wildcards and
exclusions](../docs/negotiation.md#weights-wildcards-and-exclusions), [JSON or
HTML](../docs/negotiation.md#json-or-html-from-one-path) and [precompressed
files](../docs/negotiation.md#serve-precompressed-files). Their rationale:

- Media specificity counts parameters on either side of `q`, which parse-net 0.9.3 lost, and keeps
  counting past the 63 at which parse-net's own specificity stops.
- RFC 9110 §12.5.3 lets `identity;q=0`, or `*;q=0` without an `identity` entry, exclude an
  unlisted identity; Warded's policy also gives an unlisted identity a positive `*` weight.
- Without an `identity` entry or a `*` member, the same section gives identity no weight and asks
  for no coding when none of the available codings is listed as acceptable. Warded therefore uses
  identity only as that fallback: it never beats an acceptable listed coding on weight or offer
  order. Treating it as `q=1` would serve an uncompressed file to a client that listed only
  lowered codings.
- Lookup truncation follows RFC 4647 section 3.4, including consecutive one-character subtags
  ([issue #21](https://github.com/viell-dev/slop-warded-router/issues/21),
  `tests/lookup_truncation.rs`).
- Zero-quality ranges reject an equal or less specific lookup match (an exact range, then a
  truncated one, then `*`). This retains explicit exclusions without letting a broad wildcard
  erase a more specific positive preference.
- Registry validation of server language tags is outside the constructor's well-formed-syntax
  contract.

Convenience helpers such as `secured_post` and `secured_get_and_head` are ordinary compositions;
they add no dispatch behavior ([methods](../docs/conditions.md#methods)). Security conditions read
only trusted context, because a client can send the URI scheme and forwarding headers itself, and
unknown context is an error by default so a missing layer cannot silently serve or hide routes
([transport security](../docs/transport-security.md#unknown-context)).

### Negotiation defaults and compatibility

Language constructors require a mode ([lookup or basic
filtering](../docs/negotiation.md#lookup-or-basic-filtering)). Neither mode is right for every
offer set, and making the caller choose keeps the direction explicit at the route definition.

An invalid or over-limit field ordinarily misses, so a later explicit route can supply the
application's fallback. Treating malformed input as absent was rejected because it could give the
first offer a preference the client never expressed; an error by default would stop later routes
from running. `invalid_is_error` serves hosts that want a diagnostic and want selection to stop.

The [default limits](../docs/negotiation.md#limits-and-cost) bound accepted input size per
operation. Work also depends on how many ranges, truncations and offers are compared and how many
predicates the request reaches. Each zero-quality lookup exclusion check compares an offer against
a single range in time linear in the offer length. The [selection-cost
decision](./beta-assessment.md#selection-cost-decision) records the measurements.

After the 0.1 compatibility freeze, tighter defaults could turn a previously eligible route into
a fallback or a strict matching error. Changing malformed-as-miss to an error could stop later
routes from running. Such changes alter documented selection semantics. Use per-route settings
for stricter application policy and assess an intentional breaking default change against the
0.2.0 version boundary.

## Integration and limits

The adapter mounts ordinary Tower services and the regression suite mounts Axum with its own state,
path extraction, and extensions. This does not establish compatibility with every framework,
server transport, or UI runtime. [What your application
provides](../docs/limits.md#what-your-application-provides) lists the concerns left to the server
and destinations.

Host conditions use [custom matchers](../docs/custom-matchers.md#a-host-condition-with-match_fn).
HTTP/1.1 `Host` and HTTP/2 `:authority` are distinct protocol inputs, and `Host` cannot replace an
available authority, so no dedicated host helper is part of 0.1. Real localhost HTTP/1.1 is
exercised by the hosting tests and representative Leptos fixture. HTTP/2/3, TLS negotiation and
proxy trust remain outside this evidence ([tested
environments](../docs/limits.md#tested-environments)). See the
[freeze assessment](./beta-assessment.md) for API and cost decisions and the
[release procedure](./release.md) for the agreed compatibility/stage policy.
