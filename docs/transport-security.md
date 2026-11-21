# Transport security

`secured()` matches requests that arrived over HTTPS, and `unsecure()` matches requests known to
have arrived without it. Warded cannot see the connection, so it relies on a fact your server
supplies: a `TransportSecurity` value in the request's extensions. When that fact is missing, both
conditions report an error rather than guess.

HTTPS protects the connection; it does not say who the client is. Check authentication and
authorization separately.

## Supplying the context

`TransportSecurity` has three values:

| Value      | Meaning                                                         |
| ---------- | --------------------------------------------------------------- |
| `Https`    | Trusted information says this request arrived over HTTPS        |
| `Unsecure` | Trusted information says this request arrived without HTTPS     |
| `Unknown`  | No trustworthy information; also the value when none was set    |

Insert it where the fact is known: in the listener that terminates TLS, or in middleware that
trusts your reverse proxy. Warded never reads the URI scheme, `Forwarded` or `X-Forwarded-Proto`,
because a client can send those itself.

With Axum, a layer on each listener's application can insert the value. An application served by
a TLS listener inserts `Https`; one bound to a plain listener inserts `Unsecure`:

```rust
use axum::{Extension, body::Body, http::StatusCode};
use http::Request;
use std::convert::Infallible;
use tower::{ServiceExt, service_fn};
use warded::{
    DispatchError, Matcher, Router, TransportSecurity,
    matchers::{exact_path, secured},
};

/// The application for one listener, which knows how its connections are secured.
fn application(security: TransportSecurity) -> axum::Router {
    let admin = service_fn(|_: Request<Body>| async {
        Ok::<_, Infallible>(axum::response::Response::new(Body::from("admin")))
    });
    let routes = Router::new().route(exact_path("/admin").and(secured()), admin);
    // Axum serves only infallible services; turn routing errors into a response.
    let routes =
        axum::error_handling::HandleError::new(routes, |_error: DispatchError<Infallible>| async {
            StatusCode::INTERNAL_SERVER_ERROR
        });
    axum::Router::new()
        .fallback_service(routes)
        .layer(Extension(security))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let request = || Request::builder().uri("/admin").body(Body::empty());

    let tls = application(TransportSecurity::Https);
    assert_eq!(tls.oneshot(request()?).await?.status(), StatusCode::OK);

    // The plain listener's requests reach no route, so the router answers 404.
    let plain = application(TransportSecurity::Unsecure);
    assert_eq!(
        plain.oneshot(request()?).await?.status(),
        StatusCode::NOT_FOUND
    );
    Ok(())
}
```

The repository's [server example] serves a cleartext listener this way.

## Behind a reverse proxy

When a proxy terminates TLS, the application sees a plain connection, and only the proxy knows
how the client connected. Set `TransportSecurity` from the proxy's report only for connections
that come from the proxy, and only from a header the proxy always overwrites. A connection from
anywhere else is cleartext as far as the application can tell:

```rust
use axum::{body::Body, extract::ConnectInfo};
use http::Request;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use warded::TransportSecurity;

/// The address of the reverse proxy that terminates TLS for this application.
const PROXY: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

/// Record how the request reached the proxy, trusting only the proxy's own header.
fn set_transport_security(mut request: Request<Body>) -> Request<Body> {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip());
    let security = if peer == Some(PROXY) {
        match request
            .headers()
            .get("x-forwarded-proto")
            .map(|value| value.as_bytes())
        {
            Some(b"https") => TransportSecurity::Https,
            Some(b"http") => TransportSecurity::Unsecure,
            _ => TransportSecurity::Unknown,
        }
    } else {
        TransportSecurity::Unsecure
    };
    request.extensions_mut().insert(security);
    request
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let from = |address: [u8; 4], proto: &str| -> Result<Request<Body>, http::Error> {
        let mut request = Request::builder()
            .header("x-forwarded-proto", proto)
            .body(Body::empty())?;
        let peer = SocketAddr::from((address, 40_000));
        request.extensions_mut().insert(ConnectInfo(peer));
        Ok(set_transport_security(request))
    };
    let security =
        |request: Request<Body>| request.extensions().get::<TransportSecurity>().copied();

    assert_eq!(
        security(from([10, 0, 0, 2], "https")?),
        Some(TransportSecurity::Https)
    );
    // Another client cannot claim HTTPS by sending the header itself.
    assert_eq!(
        security(from([203, 0, 113, 9], "https")?),
        Some(TransportSecurity::Unsecure)
    );
    Ok(())
}
```

Apply the function as a layer, `.layer(tower::util::MapRequestLayer::new(set_transport_security))`,
and serve the application with `into_make_service_with_connect_info::<SocketAddr>()` so the peer
address is available. Adapt the
header and the trust rule to what your proxy guarantees; the important property is that a value a
client can forge never becomes `Https`.

## Unknown context

`secured()` and `unsecure()` return `MatchError::MissingSecurityContext` when the context is
`Unknown`. That stops selection, so a forgotten layer shows up as an error instead of silently
serving or hiding routes.

Two choices control where that error can happen:

- **Evaluation order.** In `exact_path("/admin").and(secured())`, the security check runs only for
  `/admin`; other requests miss on the path first. In `secured().and(exact_path("/admin"))` every
  request is checked. See [how selection works](./selection.md#combinators-and-evaluation-order).
- **`unknown_as_miss()`.** `secured().unknown_as_miss()` treats unknown context as a miss, and
  selection continues. Use it when some deployments legitimately supply no context and the route
  should then simply not apply.

`not()` inverts a miss into a match, so `secured().unknown_as_miss().not()` matches unknown context
as well as known cleartext. To match only known cleartext, use `unsecure()`.

```rust
use http::Request;
use warded::{
    Head, MatchError, Matcher, TransportSecurity,
    matchers::{secured, unsecure},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut request = Request::builder().uri("/").body(())?;

    // Nothing inserted: the context is unknown.
    let head = Head::from_request(&request);
    assert_eq!(
        secured().evaluate(&head),
        Err(MatchError::MissingSecurityContext)
    );
    assert_eq!(
        unsecure().evaluate(&head),
        Err(MatchError::MissingSecurityContext)
    );
    assert!(secured().unknown_as_miss().evaluate(&head)?.is_none());
    assert!(secured().unknown_as_miss().not().evaluate(&head)?.is_some());

    request.extensions_mut().insert(TransportSecurity::Unsecure);
    let head = Head::from_request(&request);
    assert!(secured().evaluate(&head)?.is_none());
    assert!(unsecure().evaluate(&head)?.is_some());
    Ok(())
}
```

## Redirecting cleartext requests

A common policy sends every cleartext request to HTTPS. Declare an `unsecure()` route first and
point it at a service that answers with a redirect; Warded does not create redirects itself. When
every listener supplies the context, the first route sees every request, and later routes can
assume HTTPS.

[server example]: https://github.com/viell-dev/slop-warded-router/blob/0.1.0-beta.3/examples/server.rs
