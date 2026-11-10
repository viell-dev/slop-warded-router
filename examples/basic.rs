//! Run an ordinary Tower service through Warded without starting a server.

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::Full;
use std::convert::Infallible;
use tower::{ServiceExt, service_fn};
use warded::{
    Matcher, Router, TransportSecurity,
    matchers::{exact_path, secured_get_and_head},
    negotiation::accepts_media_type,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = secured_get_and_head()
        .and(exact_path("/orders"))
        .and(accepts_media_type("application/json")?);
    let router = Router::new()
        .route(
            orders,
            service_fn(|_: Request<()>| async {
                let mut response = Response::new(Full::new(Bytes::from_static(b"{\"orders\":[]}")));
                response
                    .headers_mut()
                    .insert("content-type", "application/json".parse().unwrap());
                Ok::<_, Infallible>(response)
            }),
        )
        .map_response(|mut response: Response<Full<Bytes>>| {
            response
                .headers_mut()
                .append("vary", "accept".parse().unwrap());
            response
        });
    let mut request = Request::builder()
        .uri("/orders")
        .header("accept", "application/json")
        .body(())?;
    // In a real application the trusted server/proxy integration supplies this.
    request.extensions_mut().insert(TransportSecurity::Https);
    let response = router.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    println!("{}", response.status());
    let mut miss = Request::builder().uri("/elsewhere").body(())?;
    miss.extensions_mut().insert(TransportSecurity::Https);
    assert_eq!(router.oneshot(miss).await?.status(), StatusCode::NOT_FOUND);
    Ok(())
}
