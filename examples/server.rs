//! Ordinary Axum hosting, trusted transport context, error mapping, and Vary.
//! Run with `--check` for a bounded localhost HTTP/1.1 integration check.

use axum::{Extension, error_handling::HandleError, routing::get};
use http::{
    Response, StatusCode,
    header::{CONTENT_TYPE, HeaderValue, VARY},
};
use std::{convert::Infallible, error::Error, net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};
use tower::ServiceExt;
use warded::{
    DispatchError, MatchError, Matcher, Router, TransportSecurity, matchers::*,
    negotiation::accepts_media_type, try_match_fn,
};

fn application() -> axum::Router {
    let orders = axum::Router::new().route(
        "/orders",
        get(|| async {
            (
                [
                    (CONTENT_TYPE, "application/json"),
                    (VARY, "accept-encoding"),
                ],
                "{\"orders\":[]}",
            )
        }),
    );
    let health = axum::Router::new().route("/health", get(|| async { "ok" }));
    let router = Router::new()
        // Leave method dispatch to the mounted application, including HEAD/405.
        .route(
            exact_path("/orders").and(accepts_media_type("application/json").unwrap()),
            orders.clone(),
        )
        .route(exact_path("/health"), health)
        .route(exact_path("/secure").and(secured()), orders.clone())
        .route(
            exact_path("/error").and(try_match_fn(|_| {
                Err(MatchError::Custom("demonstrate host error mapping".into()))
            })),
            orders,
        );
    let service = HandleError::new(router, |_error: DispatchError<Infallible>| async {
        StatusCode::INTERNAL_SERVER_ERROR
    })
    .map_response(|mut response: Response<axum::body::Body>| {
        // Appending a list-valued field preserves a destination's existing Vary.
        // This outer policy covers ordinary misses and mapped errors too.
        response
            .headers_mut()
            .append(VARY, HeaderValue::from_static("accept"));
        response
    });
    axum::Router::new()
        .fallback_service(service)
        // This example binds cleartext directly. A TLS listener or trusted proxy
        // adapter must establish its own security fact instead of copying this.
        .layer(Extension(TransportSecurity::Unsecure))
}

async fn request(
    address: SocketAddr,
    method: &str,
    path: &str,
    accept: &str,
) -> Result<String, Box<dyn Error>> {
    let mut socket = TcpStream::connect(address).await?;
    socket.write_all(format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nAccept: {accept}\r\nConnection: close\r\n\r\n").as_bytes()).await?;
    let mut response = String::new();
    socket.read_to_string(&mut response).await?;
    Ok(response)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let check = std::env::args().any(|arg| arg == "--check");
    let listener = TcpListener::bind(if check {
        "127.0.0.1:0"
    } else {
        "127.0.0.1:3000"
    })
    .await?;
    let address = listener.local_addr()?;
    let (shutdown, stopped) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, application())
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
    });
    if !check {
        println!("Listening on http://{address}; try GET /orders with Accept: application/json");
        let _shutdown = shutdown;
        server.await??;
        return Ok(());
    }
    tokio::time::timeout(Duration::from_secs(10), async {
        for (method, path, accept, status) in [
            ("GET", "/orders", "application/json", 200),
            ("HEAD", "/orders", "application/json", 200),
            ("POST", "/orders", "application/json", 405),
            ("GET", "/orders", "text/html", 404),
            ("GET", "/secure", "*/*", 404),
            ("GET", "/error", "*/*", 500),
            ("GET", "/health", "*/*", 200),
        ] {
            let response = request(address, method, path, accept).await?;
            assert!(
                response.starts_with(&format!("HTTP/1.1 {status} ")),
                "{method} {path}: {response}"
            );
            let (headers, body) = response.split_once("\r\n\r\n").unwrap();
            let vary_tokens: Vec<_> = headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .filter(|(name, _)| name.eq_ignore_ascii_case("vary"))
                .flat_map(|(_, value)| value.split(',').map(str::trim))
                .collect();
            assert!(
                vary_tokens
                    .iter()
                    .any(|token| token.eq_ignore_ascii_case("accept")),
                "{headers}"
            );
            if method == "HEAD" {
                assert!(body.is_empty());
            }
            if path == "/orders" && status == 200 {
                assert!(
                    vary_tokens
                        .iter()
                        .any(|token| token.eq_ignore_ascii_case("accept-encoding")),
                    "{headers}"
                );
            }
        }
        Ok::<_, Box<dyn Error>>(())
    })
    .await??;
    shutdown.send(()).unwrap();
    server.await??;
    println!("HTTP/1.1 host integration passed");
    Ok(())
}
