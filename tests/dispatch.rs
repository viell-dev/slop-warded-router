//! Service commitment, body preservation, readiness, and framework integration.

#![cfg(feature = "tower")]
use bytes::Bytes;
use http::{HeaderMap, Request, Response, StatusCode};
use http_body::{Body, Frame};
use http_body_util::{BodyExt, Full};
use std::{
    collections::VecDeque,
    convert::Infallible,
    future::{Future, Ready, ready},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};
use tower::{Service, ServiceExt, service_fn};
use warded::{
    Captures, DispatchError, RequestDispatchError, Router, Selection, TransportSecurity,
    matchers::*,
};

type Reply = Full<Bytes>;
fn response(status: StatusCode) -> Response<Reply> {
    let mut response = Response::new(Reply::default());
    *response.status_mut() = status;
    response
}

#[tokio::test]
async fn misses_use_404_or_consumer_fallback_and_explicit_error_preserves_request() {
    let router = Router::<String, Reply, Infallible>::new();
    assert_eq!(
        router
            .oneshot(Request::new("unread".into()))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let router =
        Router::new()
            .error_on_miss()
            .fallback(service_fn(|request: Request<String>| async move {
                assert_eq!(request.body(), "unread");
                Ok::<_, Infallible>(response(StatusCode::GONE))
            }));
    assert_eq!(
        router
            .oneshot(Request::new("unread".into()))
            .await
            .unwrap()
            .status(),
        StatusCode::GONE
    );
    let router = Router::<String, Reply, Infallible>::new().error_on_miss();
    match router
        .oneshot(
            Request::builder()
                .uri("/miss")
                .body("unread".into())
                .unwrap(),
        )
        .await
    {
        Err(RequestDispatchError::NoMatch(request)) => {
            assert_eq!(request.uri(), "/miss");
            assert_eq!(request.into_body(), "unread");
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
async fn selected_404_and_handler_error_never_retry_other_routes() {
    let later_calls = Arc::new(AtomicUsize::new(0));
    let calls = later_calls.clone();
    let router = Router::new()
        .route(
            get(),
            service_fn(|_: Request<()>| async {
                Ok::<_, Infallible>(response(StatusCode::NOT_FOUND))
            }),
        )
        .route(
            get(),
            service_fn(move |_: Request<()>| {
                calls.fetch_add(1, Ordering::SeqCst);
                async { Ok::<_, Infallible>(response(StatusCode::OK)) }
            }),
        );
    assert_eq!(
        router.oneshot(Request::new(())).await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(later_calls.load(Ordering::SeqCst), 0);
    let calls = later_calls.clone();
    let router = Router::new()
        .route(
            get(),
            service_fn(|request: Request<String>| async move {
                assert_eq!(request.into_body(), "malformed body");
                Err::<Response<Reply>, _>(std::io::Error::other("decoder failed"))
            }),
        )
        .fallback(service_fn(move |_: Request<String>| {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Ok(response(StatusCode::OK)) }
        }));
    assert!(matches!(
        router.oneshot(Request::new("malformed body".into())).await,
        Err(DispatchError::Service(_))
    ));
    assert_eq!(later_calls.load(Ordering::SeqCst), 0);
}

#[derive(Default)]
struct Frames {
    frames: VecDeque<Frame<Bytes>>,
    polls: Arc<AtomicUsize>,
}
impl Body for Frames {
    type Data = Bytes;
    type Error = Infallible;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        Poll::Ready(self.frames.pop_front().map(Ok))
    }
}

#[tokio::test]
async fn selection_and_dispatch_leave_body_frames_and_trailers_intact() {
    let polls = Arc::new(AtomicUsize::new(0));
    let mut trailers = HeaderMap::new();
    trailers.insert("x-complete", "yes".parse().unwrap());
    let body = Frames {
        frames: VecDeque::from([
            Frame::data(Bytes::from_static(b"first")),
            Frame::data(Bytes::from_static(b"second")),
            Frame::trailers(trailers),
        ]),
        polls: polls.clone(),
    };
    let request = Request::builder().uri("/items/a%2Fb").body(body).unwrap();
    let router = Router::new().route(
        path("/items/{id}").unwrap(),
        service_fn(|request: Request<Frames>| async move {
            assert_eq!(
                request.extensions().get::<Captures>().unwrap().get("id"),
                Some("a%2Fb")
            );
            assert_eq!(request.extensions().get::<Selection>().unwrap().index, 0);
            Ok::<_, Infallible>(Response::new(request.into_body()))
        }),
    );
    assert!(
        router
            .select(&warded::Head::from_request(&request))
            .unwrap()
            .is_some()
    );
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    let collected = response.into_body().collect().await.unwrap();
    assert_eq!(collected.trailers().unwrap()["x-complete"], "yes");
    assert_eq!(collected.to_bytes(), "firstsecond");
    assert!(polls.load(Ordering::SeqCst) > 0);
}

struct Readiness {
    pending: bool,
    ready_on_this_instance: bool,
    polls: Arc<AtomicUsize>,
}
impl Clone for Readiness {
    fn clone(&self) -> Self {
        Self {
            pending: self.pending,
            ready_on_this_instance: false,
            polls: self.polls.clone(),
        }
    }
}
impl Service<Request<()>> for Readiness {
    type Response = Response<Reply>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        if self.pending {
            Poll::Pending
        } else {
            self.ready_on_this_instance = true;
            Poll::Ready(Ok(()))
        }
    }
    fn call(&mut self, _: Request<()>) -> Self::Future {
        assert!(
            self.ready_on_this_instance,
            "a different cloned instance was polled ready"
        );
        ready(Ok(response(StatusCode::OK)))
    }
}

#[test]
fn unrelated_pending_destination_does_not_block_and_readiness_instance_is_preserved() {
    let unrelated = Arc::new(AtomicUsize::new(0));
    let selected = Arc::new(AtomicUsize::new(0));
    let router = Router::new()
        .route(
            exact_path("/slow"),
            Readiness {
                pending: true,
                ready_on_this_instance: false,
                polls: unrelated.clone(),
            },
        )
        .route(
            exact_path("/ready"),
            Readiness {
                pending: false,
                ready_on_this_instance: false,
                polls: selected.clone(),
            },
        );
    let mut future = Box::pin(router.oneshot(Request::builder().uri("/ready").body(()).unwrap()));
    assert!(matches!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(Ok(_))
    ));
    assert_eq!(unrelated.load(Ordering::SeqCst), 0);
    assert_eq!(selected.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn mounted_axum_keeps_state_path_extensions_and_its_head_and_method_policy() {
    #[derive(Clone)]
    struct StateValue(&'static str);
    #[derive(Clone)]
    struct Marker(&'static str);
    let app = axum::Router::new()
        .route(
            "/api/{id}",
            axum::routing::get(
                |axum::extract::State(state): axum::extract::State<StateValue>,
                 axum::extract::Path(id): axum::extract::Path<String>,
                 axum::Extension(marker): axum::Extension<Marker>| async move {
                    format!("{}:{id}:{}", state.0, marker.0)
                },
            ),
        )
        .with_state(StateValue("state"));
    let router = Router::new().route(header_eq("x-app", "yes").unwrap(), app);
    let mut request = Request::builder()
        .uri("/api/42")
        .header("x-app", "yes")
        .body(axum::body::Body::empty())
        .unwrap();
    request.extensions_mut().insert(Marker("extension"));
    request.extensions_mut().insert(TransportSecurity::Https);
    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_body().collect().await.unwrap().to_bytes(),
        "state:42:extension"
    );
    for (method, status) in [
        ("HEAD", StatusCode::OK),
        ("POST", StatusCode::METHOD_NOT_ALLOWED),
    ] {
        let mut request = Request::builder()
            .method(method)
            .uri("/api/42")
            .header("x-app", "yes")
            .body(axum::body::Body::empty())
            .unwrap();
        request.extensions_mut().insert(Marker("extension"));
        let response = router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), status);
        if method == "HEAD" {
            assert!(
                response
                    .into_body()
                    .collect()
                    .await
                    .unwrap()
                    .to_bytes()
                    .is_empty()
            );
        } else {
            assert_eq!(response.headers()["allow"], "GET,HEAD");
        }
    }
}

struct CloneCounter(Arc<AtomicUsize>);
impl Clone for CloneCounter {
    fn clone(&self) -> Self {
        self.0.fetch_add(1, Ordering::SeqCst);
        Self(self.0.clone())
    }
}
impl Service<Request<()>> for CloneCounter {
    type Response = Response<Reply>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, _: Request<()>) -> Self::Future {
        ready(Ok(response(StatusCode::OK)))
    }
}

#[tokio::test]
async fn cloning_a_router_shares_the_table_and_dispatch_clones_only_the_selected_service() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut router = Router::new();
    for i in 0..100 {
        router = router.route(exact_path(format!("/{i}")), CloneCounter(count.clone()));
    }
    let copy = router.clone();
    assert_eq!(count.load(Ordering::SeqCst), 0);
    let response = copy
        .oneshot(Request::builder().uri("/99").body(()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    let modified = router
        .clone()
        .route(exact_path("/new"), CloneCounter(count.clone()));
    assert!(
        modified
            .select(&warded::Head::from_request(
                &Request::builder().uri("/new").body(()).unwrap()
            ))
            .unwrap()
            .is_some()
    );
    assert!(
        router
            .select(&warded::Head::from_request(
                &Request::builder().uri("/new").body(()).unwrap()
            ))
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn default_dispatch_error_boxes_with_axum_bodies_and_works_under_timeout() {
    fn assert_box<E: Into<tower::BoxError>>() {}
    assert_box::<DispatchError<Infallible>>();
    let router = Router::<axum::body::Body, axum::body::Body, Infallible>::new();
    let service = tower::timeout::Timeout::new(router, std::time::Duration::from_secs(1));
    assert_eq!(
        service
            .oneshot(Request::new(axum::body::Body::empty()))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn request_returning_mode_and_its_fallback_accept_non_default_responses() {
    struct NonDefault;
    let router = Router::<(), NonDefault, Infallible>::new().error_on_miss();
    assert!(matches!(
        router.clone().oneshot(Request::new(())).await,
        Err(RequestDispatchError::NoMatch(_))
    ));
    let service = router.fallback(service_fn(|_: Request<()>| async {
        Ok::<_, Infallible>(Response::new(NonDefault))
    }));
    assert_eq!(
        service.oneshot(Request::new(())).await.unwrap().status(),
        StatusCode::OK
    );
}

struct ReadinessError;
impl Clone for ReadinessError {
    fn clone(&self) -> Self {
        Self
    }
}
impl Service<Request<()>> for ReadinessError {
    type Response = Response<Reply>;
    type Error = std::io::Error;
    type Future = Ready<Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Err(std::io::Error::other("not ready")))
    }
    fn call(&mut self, _: Request<()>) -> Self::Future {
        panic!("readiness failed")
    }
}

#[tokio::test]
async fn a_selected_readiness_error_never_calls_the_fallback() {
    let router = Router::new()
        .route(get(), ReadinessError)
        .fallback(service_fn(|_: Request<()>| async {
            panic!("a selected route must not retry fallback");
            #[expect(
                unreachable_code,
                reason = "provide the service's response/error type after the assertion"
            )]
            Ok::<_, std::io::Error>(response(StatusCode::OK))
        }));
    assert!(matches!(
        router.oneshot(Request::new(())).await,
        Err(DispatchError::Service(_))
    ));
}

struct Dropped(Arc<AtomicUsize>);
impl Drop for Dropped {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn cancellation_releases_outer_admission_without_retrying_a_destination() {
    let calls = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    let entered = calls.clone();
    let dropped = drops.clone();
    let router = Router::new()
        .route(
            get(),
            service_fn(move |_: Request<()>| {
                entered.fetch_add(1, Ordering::SeqCst);
                let guard = Dropped(dropped.clone());
                async move {
                    let _guard = guard;
                    std::future::pending::<Result<Response<Reply>, Infallible>>().await
                }
            }),
        )
        .fallback(service_fn(|_: Request<()>| async {
            panic!("no retry after cancellation");
            #[expect(
                unreachable_code,
                reason = "provide the service type after the assertion"
            )]
            Ok::<_, Infallible>(response(StatusCode::OK))
        }));
    let limited = tower::limit::ConcurrencyLimit::new(router, 1);
    let mut first = Box::pin(limited.clone().oneshot(Request::new(())));
    let mut second = Box::pin(limited.oneshot(Request::new(())));
    let mut context = Context::from_waker(Waker::noop());
    assert!(first.as_mut().poll(&mut context).is_pending());
    assert!(second.as_mut().poll(&mut context).is_pending());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    drop(first);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(second.as_mut().poll(&mut context).is_pending());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    drop(second);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}
