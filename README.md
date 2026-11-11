# Warded

A Rust router that selects a service using the complete declared conditions on the request head:
method, path, headers, query, HTTP version, and trusted HTTPS context. A route need not constrain
the path. Selection and service execution are separate.

**Status:** `0.1.0-alpha.3` in development. There is no published crate or release yet. APIs and
behavior may change. This repository contains the router; it does not implement an HTTP server or
TLS stack.

Agents write and maintain Warded. No person reviews the code. See
[CONTRIBUTING.md](./CONTRIBUTING.md) for contribution and verification expectations.

## Try the source

The declared and tested minimum Rust version is 1.99.1. Clone this repository and run:

```sh
cargo run --example basic --locked
cargo test --all-features --locked
```

The basic example dispatches synthetic requests. For an ordinary Axum HTTP/1.1 host, run
`cargo run --example server --locked`, then request `/orders` with `Accept: application/json`.
`cargo run --example server --locked -- --check` runs its bounded localhost integration check.
To use unreleased source in another Cargo package:

```toml
[dependencies]
warded = { git = "https://github.com/viell-dev/slop-warded-router", branch = "main" }
```

Replace the moving branch with a tested commit `rev` when pinning an application. Crates.io
installation instructions will be added only after a package is published. Removing the dependency
from the consumer's `Cargo.toml` and updating its lockfile removes Warded; it installs no service or
global configuration.

## Match and select

```rust
use http::Request;
use warded::{Head, Matcher, Selector, matchers::{exact_path, get, header_eq}};

let routes = Selector::new()
    .route(get().and(exact_path("/orders")).and(header_eq("accept-language", "fr")?), "french")
    .route(get().and(exact_path("/orders")).and(header_eq("accept-language", "en")?), "english");

let request = Request::builder()
    .uri("/orders")
    .header("accept-language", "fr")
    .body(())?;
let selected = routes.select(&Head::from_request(&request))?.unwrap();
assert_eq!(*selected.target, "french");
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Selector` returns a target without executing it. The first complete match wins; failed headers,
methods, queries, or security conditions continue selection. A path condition is optional. A
selector miss is `None`; the optional `Router` Tower adapter instead supplies a 404 with the
response body's default value. A custom fallback replaces that response. `error_on_miss()` returns
a separate `RequestRouter` adapter whose `RequestDispatchError::NoMatch` retains the unread request
when there is no custom fallback.

`Selector` supports local custom matchers and stays local to a thread. Use `SyncSelector` for
thread-safe matchers and targets in shared server state. String-taking raw helpers own their
configuration, including when constructed from temporary borrowed strings.

Use `path("/orders/{id}")?` or `path("/assets/{*rest}")?` for raw captures. Exact paths and raw
query helpers do not decode percent escapes, fold case, clean slashes, or redirect. `header_eq`
requires exactly one equal raw field value; `header_any_eq` explicitly searches repeated values.
Semantic negotiation is a separate operation. If a destination later decodes or normalizes paths,
its interpretation can differ from a raw match; raw deny-style routes are not an authorization
boundary.

Refine a successful pattern with `filter` to select by captured values before committing to a
handler. A predicate can be a free function, closure, newtype parser, or precompiled regex:

```rust
use warded::{Matcher, Selector, matchers::path};

let routes = Selector::new()
    .route(path("/users/{id}")?.filter(|captures| {
        captures.get("id").is_some_and(|raw| raw.parse::<u64>().is_ok())
    }), "numeric user")
    .route(path("/users/{username}")?.filter(|captures| {
        captures.get("username").is_some_and(|raw| {
            !raw.is_empty() && raw.bytes().all(|byte| byte.is_ascii_alphabetic())
        })
    }), "named user");
# Ok::<(), warded::DefinitionError>(())
```

These are example domain policies; Warded does not assign priority to a type or numeric value.
False discards that candidate's captures and continues normal ordered selection. True keeps the
raw strings; parsed values are not stored. The predicate only sees captures from the matcher it
wraps, so refine a composition to compare several captures. Underlying errors propagate, and
missing capture names need an explicit caller policy. A compiled pattern supports up to 25 ordinary
parameters plus an optional terminal catch-all; excess capacity returns `DefinitionError`.

## Optional shortcuts and negotiation

`get()`, `post()`, and the other method helpers constrain only the method. `secured()` requires
trusted HTTPS context; `unsecure()` requires known non-HTTPS context. Unknown context is an error
unless `unknown_as_miss()` is explicitly selected. Server or trusted proxy integration supplies a
`TransportSecurity` request extension; forwarding headers and URI scheme are not trusted by the
router. HTTPS is distinct from authentication.

`secured_post()` is just `secured().and(post())`; `secured_get_and_head()` is
`secured().and(get().or(head()))`. These optional compositions add no special registration rule.
GET alone excludes HEAD. The server still applies HEAD response-body rules.

Combinators evaluate left to right and errors stop selection. Scope strict checks deliberately:
`exact_path("/admin").and(secured())` misses `/public` before needing security context, while
`secured().and(exact_path("/admin"))` errors there if context is missing. Explicitly mapping an
unknown value to a miss also means negating that predicate matches the unknown value.

Negotiation reuses `parse-net` grammar and encoding negotiation, plus `oxilangtag` for
well-formed server tags. Warded supplies media precedence and language selection rules.
`accepts_media_type("application/json")?` tests
whether one representation is acceptable. To respect preference among several representations,
declare the same complete offer set for each alternative:

```rust
use warded::{Selector, negotiation::Offers};

let formats = Offers::media_types(["text/html", "application/json"])?;
let routes = Selector::new()
    .route(formats.selected("text/html")?, "html")
    .route(formats.selected("application/json")?, "json");
# Ok::<(), Box<dyn std::error::Error>>(())
```

With `Accept: application/json;q=1, text/html;q=0.5`, preferred-offer predicates select JSON.
Independent acceptability predicates can both match, so route declaration order still controls
them. Only group offers whose other applicability conditions are shared.

Language helpers take an explicit choice: lookup can select `fr` for `fr-CA`, while basic filtering
can select `fr-CA` for `fr`. Lookup tries client ranges by descending weight, retaining client order
for ties, and finishes each range's fallback before the next range. Wildcards provide a default
after explicit ranges. An equally or more specific zero-quality lookup range excludes a candidate;
a broader exclusion does not override an explicit positive match. Media, encoding, and basic
filtering ties use server offer order. Absent fields impose no preference; an offer set chooses its
first offer. No acceptable offer means a miss, so application defaults need their own final route.

Encoding helpers account for identity and absent/empty field differences. Malformed input ordinarily
misses; `invalid_is_error` exposes a parser diagnostic. Negating a malformed-as-miss helper matches
malformed input; choose strict errors when that is inappropriate. Nothing automatically enables
400, 405, or 406 responses. Apply
appropriate `Vary` or cache-control middleware around successes and fallbacks; helpers do not encode
responses or set their metadata.

Shared offer sets cache their complete choice within one immutable `Head`, keyed by set identity
and every configured limit. `negotiation::Limits` controls list members, parameters, and combined
field bytes; the combined-field default is 64 KiB. Independent sets still perform independent
negotiation. `Head` is
cloneable rather than `Copy`; a clone starts a fresh cache.

## Tower integration and scope

`Router::new().route(matcher, service).fallback(service)` accepts ordinary cloneable Tower services.
Selection does not poll or consume bodies. The selected service receives the original URI and body,
plus `Captures` and `Selection` extensions. Its returned 404 or error never retries another route.
Axum applications can retain their own path extraction and state as mounted destinations.

The adapter waits only for the selected destination's readiness, using the same instance it calls.
Apply an outer Tower concurrency limit for admission control. Services need common response/error
types. `Router` requires a `Default` response body for its implicit 404; `Default` does not itself
guarantee emptiness. `RequestRouter` has no such bound, including with a custom fallback.

Normal `DispatchError<E>` carries no body and can convert to Tower's boxed error when `E` has the
necessary bounds. Infallible hosts such as Axum still need an explicit error-to-response boundary;
the [server example](./examples/server.rs) demonstrates it, trusted cleartext context, preserved
HEAD/405 behavior, and outer `Vary` policy. Router clones share an immutable service table and only
clone the selected destination for dispatch. Editing a shared clone copies the table.

Nested routers currently share the same `Selection` and `Captures` extension types: an inner match
replaces them, while an inner fallback retains outer metadata. Preserve parent metadata in a
separate application extension when needed.

Default features are `tower` and `negotiation`. Disable defaults for standalone matching, or enable
either feature independently. See the [design and limits](./docs/design.md), the runnable
[example](./examples/basic.rs), and [verification commands](./docs/verification.md). Generate the API
reference with `cargo doc --all-features --no-deps --open`.

Report defects through [GitHub issues](https://github.com/viell-dev/slop-warded-router/issues), with
the commit, Rust version, features, synthetic request, expected result, and actual result. This
alpha does not promise framework-wide compatibility, an HTTP server, authentication, automatic
protocol response policy, or measured performance.

Licensed MIT OR Apache-2.0, at the recipient's option. See
[LICENSE-MIT](./LICENSE-MIT) and [LICENSE-APACHE](./LICENSE-APACHE).
