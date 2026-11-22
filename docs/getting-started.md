# Getting started

Warded chooses one destination for an HTTP request by checking conditions on the request head: the
method, path, headers, query string, HTTP version and whether the connection is known to be HTTPS.
You list routes in order, each a set of conditions and a destination, and the first route whose
conditions all hold is selected. Matching never reads the request body.

This page installs Warded, declares a first route table and shows the two ways to use it: selecting
a target yourself, or dispatching requests to Tower services.

## Install

Warded needs Rust 1.99.1 or later. In your Cargo project, add the exact release and the `http`
crate, whose request types the examples use:

```sh
cargo add warded@=0.1.0-beta.4 http@1.5
```

This adds the following to `Cargo.toml`:

```toml
[dependencies]
http = "1.5"
warded = "=0.1.0-beta.4"
```

The `=` pins this exact prerelease. Betas can still change behavior between versions, so you choose
when to move to the next one; see [versions](./versions.md).

Two optional features are enabled by default:

| Feature       | Provides                                                                    |
| ------------- | --------------------------------------------------------------------------- |
| `tower`       | `Router`, which dispatches requests to [Tower](./tower.md) services          |
| `negotiation` | `Accept`, `Accept-Language` and `Accept-Encoding` [negotiation](./negotiation.md) |

Matching and selection work without either. To choose features yourself, turn off the defaults:

```toml
[dependencies]
warded = { version = "=0.1.0-beta.4", default-features = false, features = ["negotiation"] }
```

## A first route table

A route pairs a matcher with a target. Matchers come from `warded::matchers` and combine with
`and`, `or` and `not`, which are methods of the `Matcher` trait, so import the trait too. A
`Selector` keeps the routes in declaration order and returns the first target whose matcher
accepts the request:

```rust
use http::Request;
use warded::{
    Head, Matcher, Selector,
    matchers::{exact_path, get, path, post},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let routes = Selector::new()
        .route(get().and(path("/orders/{id}")?), "show order")
        .route(post().and(exact_path("/orders")), "create order");

    // A GET for one order selects the first route and captures the id.
    let request = Request::builder().uri("/orders/42").body(())?;
    let selected = routes
        .select(&Head::from_request(&request))?
        .expect("the first route matches");
    assert_eq!(*selected.target, "show order");
    assert_eq!(selected.selection.captures.get("id"), Some("42"));

    // No route accepts a DELETE, so nothing is selected.
    let request = Request::builder()
        .method("DELETE")
        .uri("/orders/42")
        .body(())?;
    assert!(routes.select(&Head::from_request(&request))?.is_none());
    Ok(())
}
```

`Head::from_request` borrows the method, URI, version and headers of a request without touching its
body. `select` returns:

- `Ok(Some(selected))` with your target, its declaration index and the route's path captures;
- `Ok(None)` when no route matches, a _miss_;
- `Err(error)` when a condition could not be evaluated, such as an HTTPS check without any
  information about the connection. Selection stops there; see [misses and
  errors](./selection.md#misses-and-errors).

`path` and other constructors that parse a definition return a `Result`, so a malformed route fails
when you build the table rather than when a request arrives.

## Selecting or dispatching

A target can be any value: a string, an enum of pages, a handler function or a service. There are
two ways to use the selection:

- **Select and call it yourself.** `Selector` returns a reference to the target and does nothing
  else. It works without Tower and without thread-safety requirements, so custom matchers may use
  local state such as `Rc`. Use `SyncSelector` for a table shared between threads.
- **Dispatch to Tower services.** `Router`, from the `tower` feature, is itself a Tower service.
  It selects a route, adds the captures to the request's extensions and calls that route's
  service. Without a match it responds 404, unless you set a fallback.

The same route as a `Router`. This example, like the other examples on this site that run
services, also uses `tokio`, `tower`, `bytes` and `http-body-util`. Add them with:

```sh
cargo add tokio@1 --features macros,rt-multi-thread
cargo add tower@0.5 --features util
cargo add bytes@1 http-body-util@0.1
```

[Tower and Axum](./tower.md#dependencies) lists the resulting `Cargo.toml` entries, together with
Axum for the examples that use it.

```rust
use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use std::convert::Infallible;
use tower::{ServiceExt, service_fn};
use warded::{
    Captures, Matcher, Router,
    matchers::{get, path},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new().route(
        get().and(path("/orders/{id}")?),
        service_fn(|request: Request<()>| async move {
            let captures = request.extensions().get::<Captures>();
            let id = captures
                .and_then(|captures| captures.get("id"))
                .unwrap_or_default();
            let body = Full::new(Bytes::from(format!("order {id}")));
            Ok::<_, Infallible>(Response::new(body))
        }),
    );

    let request = Request::builder().uri("/orders/42").body(())?;
    let response = router.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.into_body().collect().await?.to_bytes(), "order 42");

    let request = Request::builder().uri("/elsewhere").body(())?;
    let response = router.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    Ok(())
}
```

A selected service's own response is final: if it returns 404 or an error, Warded does not try
the routes after it. [Tower and Axum](./tower.md) covers fallbacks, error mapping and serving an
Axum application.

## Update or remove

To update, change the exact version in `Cargo.toml`, run `cargo update --workspace`, and read
[upgrading](./upgrading.md) for the steps a release needs. To remove Warded, delete its calls and
imports, then run `cargo remove warded`, which also refreshes the lockfile. To recover a working
state, restore your code and add back the exact version it used.

## Try the repository examples

The repository has two runnable examples. Check out the [source tag] of this release, then run:

```sh
cargo run --example basic --locked
cargo run --example server --locked
```

The `basic` example dispatches synthetic requests through a `Router`. The `server` example serves
an Axum application on `http://127.0.0.1:3000`; request `/orders` with `Accept: application/json`.
With `-- --check` it runs its own bounded localhost check instead.

To try an unreleased commit in another Cargo package, depend on its full commit SHA, which
`git rev-parse HEAD` prints in the checkout you tested:

```toml
[dependencies]
warded = { git = "https://github.com/viell-dev/slop-warded-router", rev = "<full commit SHA>" }
http = "1.5"
```

Run `cargo build` to resolve it, and change `rev` followed by `cargo update --workspace` to move to
another commit. The [changelog on the default branch][unreleased] lists unreleased changes; this
site describes the latest release.

## Next steps

- [How selection works](./selection.md): declaration order, misses, errors and combinators.
- [Content negotiation](./negotiation.md): languages, JSON or HTML, and encodings.
- [Tower and Axum](./tower.md): serving routes from a real HTTP server.
- The [API reference] on docs.rs lists every type and function.

[source tag]: https://github.com/viell-dev/slop-warded-router/tree/0.1.0-beta.4
[API reference]: https://docs.rs/warded/0.1.0-beta.4/warded/
[unreleased]: https://github.com/viell-dev/slop-warded-router/blob/main/CHANGELOG.md#unreleased
