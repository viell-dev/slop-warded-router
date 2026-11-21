# Tower and Axum

`Router`, from the `tower` feature, routes requests to [Tower](https://docs.rs/tower) services. It
is a Tower service itself, so it works with Axum, Hyper and other Tower-based servers. This page
builds an Axum application around a `Router` and explains what the router does with requests,
misses, errors and readiness.

## Dependencies

The examples on this site that run services use these crates besides Warded and `http`:

```toml
[dependencies]
axum = "0.8"
bytes = "1"
http-body-util = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
tower = { version = "0.5", features = ["util"] }
```

Warded itself does not depend on Axum or Tokio; use any Tower-compatible server.

## Routes and services

`Router::new().route(matcher, service)` adds a route whose destination is any Tower service that
is `Clone + Send + Sync + 'static` and returns a `Send` future. All destinations of one router share
three types:

- the request body type `B`, so each receives a `Request<B>`;
- the response type `Response<R>`;
- the error type `E`.

For each request the router selects a route with the same rules as a
[`Selector`](./selection.md), then calls that route's service with the original request: the same
URI, headers and unread body. Before the call it inserts two extensions:

- `Captures`, the selected route's raw [path captures](./paths.md);
- `Selection`, the route's declaration index together with the same captures.

The selected service's response is final. If it returns 404, an error, or stays pending, the router
does not try another route.

## An Axum application

This application selects routes with Warded and lets Axum method routers handle the selected
requests. The handler reads the captures from the request extensions:

```rust
use axum::{
    Extension,
    body::Body,
    error_handling::HandleError,
    http::{Request, StatusCode},
    routing::get,
};
use http_body_util::BodyExt;
use std::convert::Infallible;
use tower::ServiceExt;
use warded::{
    Captures, DefinitionError, DispatchError, MatchError, Matcher, Router,
    matchers::{exact_path, path},
    negotiation::accepts_media_type,
};

async fn show_order(Extension(captures): Extension<Captures>) -> String {
    format!("order {}", captures.get("id").unwrap_or_default())
}

async fn health() -> &'static str {
    "ok"
}

/// Axum serves only infallible services, so routing errors become responses here.
async fn routing_error(error: DispatchError<Infallible>) -> StatusCode {
    match error {
        DispatchError::Match(MatchError::InvalidHeader { .. }) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn application() -> Result<axum::Router, DefinitionError> {
    let json = accepts_media_type("application/json")?.invalid_is_error();
    let routes = Router::new()
        .route(path("/orders/{id}")?.and(json), get(show_order))
        .route(exact_path("/health"), get(health));
    Ok(axum::Router::new().fallback_service(HandleError::new(routes, routing_error)))
}

/// Serve the application; a real program calls this from `main`.
async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, application()?).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = application()?;
    let request = |method: &str, uri: &str, accept: &str| {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("accept", accept)
            .body(Body::empty())
    };

    let response = app
        .clone()
        .oneshot(request("GET", "/orders/42", "application/json")?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.into_body().collect().await?.to_bytes(), "order 42");

    // The Axum method router answers HEAD for GET, and 405 with Allow for other methods.
    let response = app
        .clone()
        .oneshot(request("HEAD", "/orders/42", "*/*")?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .clone()
        .oneshot(request("POST", "/orders/42", "*/*")?)
        .await?;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(response.headers()["allow"], "GET,HEAD");

    // A malformed Accept field is an error under invalid_is_error, mapped to 400.
    let response = app
        .clone()
        .oneshot(request("GET", "/orders/42", "text/html;q=2")?)
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // No route: the router's own 404.
    let response = app.oneshot(request("GET", "/elsewhere", "*/*")?).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    Ok(())
}
```

The pieces:

- **`axum::Router::new().fallback_service(...)`** hands every request to Warded. Axum layers, such
  as tracing or the [`TransportSecurity`](./transport-security.md) extension, go on this outer
  Axum router.
- **`HandleError`** turns the router's `DispatchError` into a response, because Axum only serves
  services that cannot fail. See [map errors to responses](#map-errors-to-responses).
- **`get(show_order)`** is an Axum `MethodRouter`, a Tower service. It brings Axum's method
  handling for the selected route: HEAD for GET, and 405 with `Allow` for other methods.
- **`Extension<Captures>`** reads Warded's captures. Axum's `Path` extractor does not see them,
  because Warded does not fill Axum's private path state.

## Mounting Axum routers

A destination can also be a whole `axum::Router`, with its own routes, state and extractors.
Warded then decides which application receives the request, and the Axum router routes it again
by its own rules, including `Path` extraction and its 404 and 405 responses:

```rust
use axum::{
    body::Body,
    extract::{Path, State},
    http::{Request, StatusCode},
};
use std::convert::Infallible;
use tower::ServiceExt;
use warded::{Router, matchers::header_eq};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let beta = axum::Router::new()
        .route(
            "/api/{id}",
            axum::routing::get(
                |State(name): State<&'static str>, Path(id): Path<String>| async move {
                    format!("{name}: {id}")
                },
            ),
        )
        .with_state("beta");
    let stable = axum::Router::new()
        .route("/api/{id}", axum::routing::get(|| async { "stable" }))
        .route("/status", axum::routing::get(|| async { "ok" }));

    let routes = Router::<Body, _, Infallible>::new()
        .route(header_eq("x-channel", "beta")?, beta)
        .route(warded::match_fn(|_| true), stable);

    let request = Request::builder()
        .uri("/api/7")
        .header("x-channel", "beta")
        .body(Body::empty())?;
    let response = routes.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);

    // The beta application has no /status, and its 404 is final: Warded does not try the
    // stable application, which would have answered.
    let request = Request::builder()
        .uri("/status")
        .header("x-channel", "beta")
        .body(Body::empty())?;
    assert_eq!(
        routes.oneshot(request).await?.status(),
        StatusCode::NOT_FOUND
    );
    Ok(())
}
```

The [server example] serves mounted Axum routers over HTTP/1.1 with trusted cleartext context,
error mapping and an outer `Vary` policy.

## Misses and fallbacks

When no route matches, the router has three behaviors:

1. **Default:** respond 404 with `R::default()` as the body. This needs a response body type that
   implements `Default`; whether that default is empty depends on the type.
2. **`.fallback(service)`:** call your service with the unmatched request. A fallback receives no
   `Captures` or `Selection` from this router.
3. **`.error_on_miss()`:** return a `RequestRouter`, whose error type
   `RequestDispatchError::NoMatch` hands back the unmatched request with its body unread. The
   response body type then needs no `Default`. A fallback set on it still takes precedence.

```rust
use http::{Request, Response, StatusCode};
use std::convert::Infallible;
use tower::{ServiceExt, service_fn};
use warded::{RequestDispatchError, Router, matchers::exact_path};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reply = |status: StatusCode| {
        service_fn(move |_: Request<String>| async move {
            let mut response = Response::new(String::new());
            *response.status_mut() = status;
            Ok::<_, Infallible>(response)
        })
    };
    let request = |uri: &str| {
        Request::builder()
            .uri(uri)
            .body(String::from("unread body"))
    };

    // A fallback answers every request that no route selects.
    let with_fallback = Router::new()
        .route(exact_path("/v2/orders"), reply(StatusCode::OK))
        .fallback(reply(StatusCode::GONE));
    let response = with_fallback.oneshot(request("/v1/orders")?).await?;
    assert_eq!(response.status(), StatusCode::GONE);

    // error_on_miss returns the unmatched request to the caller instead.
    let returning = Router::new()
        .route(exact_path("/v2/orders"), reply(StatusCode::OK))
        .error_on_miss();
    match returning.oneshot(request("/v1/orders")?).await {
        Err(RequestDispatchError::NoMatch(request)) => {
            assert_eq!(request.uri(), "/v1/orders");
            assert_eq!(request.into_body(), "unread body");
        }
        other => panic!("expected the unmatched request, got {other:?}"),
    }
    Ok(())
}
```

Use `error_on_miss` when the caller decides what happens next with the request, for example
passing it to an older application that has a different response type.

## Map errors to responses

`Router` fails with `DispatchError<E>`:

- `DispatchError::Match(MatchError)` when a condition could not be evaluated, such as missing
  [transport security](./transport-security.md#unknown-context) context, or a negotiation
  condition with `invalid_is_error`. No route was called.
- `DispatchError::Service(E)` when the selected service failed, either while becoming ready or
  while handling the request. No other route is tried.

`DispatchError` carries no request body, so it converts into Tower's `BoxError` when `E` does.
`RequestRouter` fails with `RequestDispatchError`, which is `NoMatch(request)` or
`Dispatch(DispatchError<E>)`. Both enums are non-exhaustive: match the variants you handle and add
a wildcard arm.

The server decides what an error means for the client. Typical mappings are 400 Bad Request for
`MatchError::InvalidHeader`, 500 for a missing security context, which indicates a deployment
mistake, and the service's own mapping for its errors. With Axum, wrap the router in `HandleError`
as above. With other servers, use `tower::ServiceExt::map_err` or a layer of your own.

## Readiness and admission

`Router` is always ready: its `poll_ready` does not wait for any destination. After selecting a
route, it clones that route's service, waits for that clone to become ready and calls the same
clone. An unrelated route that is not ready never delays a request.

The router therefore does not limit how many requests are in progress. Add an outer limit, such
as `tower::ServiceBuilder::new().concurrency_limit(256).service(router)` with Tower's `limit`
feature. The router has no queues, retries, timeouts or body buffering; dropping the response
future cancels the selected call.

## Nested routers

A `Router` can be the destination of another `Router`. Both use the same extension types, so:

- when the inner router selects a route, its `Captures` and `Selection` replace the outer ones;
- when the inner router falls back, the outer ones stay.

```rust
use bytes::Bytes;
use http::{Request, Response};
use http_body_util::{BodyExt, Full};
use std::convert::Infallible;
use tower::{ServiceExt, service_fn};
use warded::{Captures, Router, matchers::path};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reply with the label and the captures the service received.
    let describe = |label: &'static str| {
        service_fn(move |request: Request<()>| async move {
            let captures = request
                .extensions()
                .get::<Captures>()
                .cloned()
                .unwrap_or_default();
            let pairs: Vec<String> = captures
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect();
            let body = format!("{label}: {}", pairs.join(" "));
            Ok::<_, Infallible>(Response::new(Full::new(Bytes::from(body))))
        })
    };

    let inner = Router::new()
        .route(path("/api/{version}/users/{id}")?, describe("user"))
        .fallback(describe("inner fallback"));
    let outer = Router::new().route(path("/api/{version}/{*rest}")?, inner);

    let text = |response: Response<Full<Bytes>>| async move {
        Ok::<_, Infallible>(response.into_body().collect().await?.to_bytes())
    };
    let request = Request::builder().uri("/api/v2/users/7").body(())?;
    let response = outer.clone().oneshot(request).await?;
    assert_eq!(text(response).await?, "user: id=7 version=v2");

    let request = Request::builder().uri("/api/v2/teams").body(())?;
    let response = outer.oneshot(request).await?;
    assert_eq!(
        text(response).await?,
        "inner fallback: rest=teams version=v2"
    );
    Ok(())
}
```

To keep the outer captures available after an inner match, copy what you need into an extension
type of your own before the inner router runs. Warded does not strip a path prefix for the inner
router; its patterns match the full path.

## Cloning and inspection

Cloning a `Router` is cheap: clones share one route table, and dispatch clones only the selected
service. Adding a route or fallback to a clone copies the table first, so the original is
unchanged.

`router.select(&Head::from_request(&request))` returns the `Selection` the router would make,
without calling a service or touching the body. Use it in tests or to log routing decisions.

[server example]: https://github.com/viell-dev/slop-warded-router/blob/0.1.0-beta.3/examples/server.rs
