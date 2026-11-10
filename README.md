# Warded

A Rust router that selects a service using the complete declared conditions on the request head:
method, path, headers, query, HTTP version, and trusted HTTPS context. A route need not constrain
the path. Selection and service execution are separate.

**Status:** `0.1.0-alpha.1` in development. There is no published crate or release yet. APIs and
behavior may change. This repository contains the router; it does not implement an HTTP server or
TLS stack.

Agents write and maintain Warded. No person reviews the code. See
[CONTRIBUTING.md](./CONTRIBUTING.md) for contribution and verification expectations.

## Try the source

Use Rust 1.99.1, clone this repository, and run:

```sh
cargo run --example basic --locked
cargo test --all-features --locked
```

The example dispatches synthetic requests without starting a server. To use unreleased source in
another Cargo package:

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
selector miss is `None`; the optional `Router` Tower adapter instead supplies an empty 404 by
default. A custom fallback replaces that response. `error_on_miss()` preserves the unread request
in a `DispatchError::NoMatch` when there is no custom fallback.

Use `path("/orders/{id}")?` or `path("/assets/{*rest}")?` for raw captures. Exact paths and raw
query helpers do not decode percent escapes, fold case, clean slashes, or redirect. `header_eq`
requires exactly one equal raw field value; `header_any_eq` explicitly searches repeated values.
Semantic negotiation is a separate operation.

## Optional shortcuts and negotiation

`get()`, `post()`, and the other method helpers constrain only the method. `secured()` requires
trusted HTTPS context; `unsecure()` requires known non-HTTPS context. Unknown context is an error
unless `unknown_as_miss()` is explicitly selected. Server or trusted proxy integration supplies a
`TransportSecurity` request extension; forwarding headers and URI scheme are not trusted by the
router. HTTPS is distinct from authentication.

`secured_post()` is just `secured().and(post())`; `secured_get_and_head()` is
`secured().and(get().or(head()))`. These optional compositions add no special registration rule.
GET alone excludes HEAD. The server still applies HEAD response-body rules.

Negotiation uses the existing `parse-net` package. `accepts_media_type("application/json")?` tests
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

Language helpers take an explicit lookup or basic-filtering choice. Encoding helpers account for
identity and absent/empty field differences. Malformed input ordinarily misses; `invalid_is_error`
exposes a parser diagnostic. Nothing automatically enables 400, 405, or 406 responses. Apply
appropriate `Vary` or cache-control middleware around successes and fallbacks; helpers do not encode
responses or set their metadata.

## Tower integration and scope

`Router::new().route(matcher, service).fallback(service)` accepts ordinary cloneable Tower services.
Selection does not poll or consume bodies. The selected service receives the original URI and body,
plus `Captures` and `Selection` extensions. Its returned 404 or error never retries another route.
Axum applications can retain their own path extraction and state as mounted destinations.

The adapter waits only for the selected destination's readiness, using the same instance it calls.
Apply an outer Tower concurrency limit for admission control. Services need common response/error
types, and the response body needs `Default` for the implicit empty 404. Custom matchers in a
standalone `Selector` need neither `Send` nor `Sync`.

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
