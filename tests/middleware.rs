//! Middleware behavior across successful and exhausted head-based routing.

#![cfg(feature = "tower")]
use bytes::Bytes;
use http::{
    Request, Response, StatusCode,
    header::{HeaderValue, VARY},
};
use http_body_util::Full;
use std::convert::Infallible;
use tower::{ServiceExt, service_fn};
use warded::{Router, matchers::header_eq};

#[tokio::test]
async fn outer_cache_policy_covers_header_dependent_success_and_miss() {
    let router = Router::new().route(
        header_eq("accept-language", "fr").unwrap(),
        service_fn(|_: Request<()>| async {
            Ok::<_, Infallible>(Response::new(Full::new(Bytes::from_static(b"bonjour"))))
        }),
    );
    let app = router.map_response(|mut response: Response<Full<Bytes>>| {
        response
            .headers_mut()
            .insert(VARY, HeaderValue::from_static("Accept-Language"));
        response
    });
    for (language, status) in [("fr", StatusCode::OK), ("en", StatusCode::NOT_FOUND)] {
        let request = Request::builder()
            .header("accept-language", language)
            .body(())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()[VARY], "Accept-Language");
    }
}
