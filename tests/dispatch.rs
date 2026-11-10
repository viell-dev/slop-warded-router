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
use warded::{Captures, DispatchError, Matcher, Router, Selection, TransportSecurity, matchers::*};

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
        Err(DispatchError::NoMatch(request)) => {
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
async fn mounted_axum_keeps_its_state_extractors_uri_extensions_and_policy() {
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
    let router = Router::new().route(get().and(header_eq("x-app", "yes").unwrap()), app);
    let mut request = Request::builder()
        .uri("/api/42")
        .header("x-app", "yes")
        .body(axum::body::Body::empty())
        .unwrap();
    request.extensions_mut().insert(Marker("extension"));
    request.extensions_mut().insert(TransportSecurity::Https);
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_body().collect().await.unwrap().to_bytes(),
        "state:42:extension"
    );
}
