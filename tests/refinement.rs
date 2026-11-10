//! Capture-predicate refinement through the public selection and dispatch APIs.

use http::Request;
use std::{cell::Cell, rc::Rc, str::FromStr};
use warded::{
    Captures, Head, MatchError, Matcher, Selector, SyncSelector, match_fn, matchers::*,
    try_match_fn,
};

#[derive(Debug, PartialEq)]
struct UserId(u64);

impl FromStr for UserId {
    type Err = ();

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(());
        }
        raw.parse().map(Self).map_err(|_| ())
    }
}

fn username(raw: &str) -> bool {
    let mut bytes = raw.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[test]
fn predicates_and_newtype_parsers_distinguish_overlapping_paths() {
    let selector = Selector::new()
        .route(
            get().and(path("/users/{id}").unwrap().filter(|captures| {
                captures
                    .get("id")
                    .is_some_and(|raw| raw.parse::<UserId>().is_ok())
            })),
            "id",
        )
        .route(
            get().and(
                path("/users/{username}")
                    .unwrap()
                    .filter(|captures| captures.get("username").is_some_and(username)),
            ),
            "username",
        );
    for (value, expected) in [
        ("0", Some(("id", "id"))),
        ("18446744073709551615", Some(("id", "id"))),
        ("18446744073709551616", None),
        ("alice_7", Some(("username", "username"))),
        ("00042", Some(("id", "id"))),
        ("%34%32", None),
        ("a%2Fb", None),
        ("+42", None),
    ] {
        let request = Request::builder()
            .uri(format!("/users/{value}"))
            .body(())
            .unwrap();
        let selected = selector.select(&Head::from_request(&request)).unwrap();
        assert_eq!(
            selected.as_ref().map(|s| *s.target),
            expected.map(|(target, _)| target)
        );
        if let (Some(selected), Some((_, name))) = (selected, expected) {
            assert_eq!(selected.selection.captures.get(name), Some(value));
            assert_eq!(selected.selection.captures.iter().count(), 1);
        }
    }
}

#[test]
fn refinement_is_lazy_preserves_errors_and_keeps_accepted_raw_captures() {
    let calls = Rc::new(Cell::new(0));
    let predicate = {
        let calls = calls.clone();
        move |_: &Captures| {
            calls.set(calls.get() + 1);
            true
        }
    };
    let request = Request::builder().uri("/a%2Fb").body(()).unwrap();
    let head = Head::from_request(&request);
    assert!(
        exact_path("/elsewhere")
            .filter(predicate.clone())
            .evaluate(&head)
            .unwrap()
            .is_none()
    );
    let error = MatchError::Custom("preserve me".into());
    assert_eq!(
        try_match_fn(|_| Err(error.clone()))
            .filter(predicate.clone())
            .evaluate(&head),
        Err(error.clone()),
    );
    assert_eq!(calls.get(), 0);
    let captures = path("/{id}")
        .unwrap()
        .filter(predicate)
        .evaluate(&head)
        .unwrap()
        .unwrap();
    assert_eq!(captures.get("id"), Some("a%2Fb"));
    assert_eq!(calls.get(), 1);
}

#[test]
fn rejected_refinement_cannot_leak_or_collide_with_later_branches() {
    let matcher = path("/{id}/{action}").unwrap().and(
        path("/{discard}/{id}")
            .unwrap()
            .filter(|_| false)
            .or(exact_path("/a/b")),
    );
    let request = Request::builder().uri("/a/b").body(()).unwrap();
    let captures = matcher
        .evaluate(&Head::from_request(&request))
        .unwrap()
        .unwrap();
    assert_eq!(captures.get("id"), Some("a"));
    assert_eq!(captures.get("action"), Some("b"));
    assert_eq!(captures.get("discard"), None);
    assert!(
        path("/{id}/{action}")
            .unwrap()
            .filter(|_| false)
            .not()
            .evaluate(&Head::from_request(&request))
            .unwrap()
            .unwrap()
            .iter()
            .next()
            .is_none()
    );
}

#[test]
fn overlap_remains_ordered_and_refining_a_composition_sees_its_captures() {
    let left = path("/{id}/part").unwrap().filter(|_| true);
    let right = path("/value/{name}").unwrap().filter(|_| true);
    let request = Request::builder().uri("/value/part").body(()).unwrap();
    for (selector, expected) in [
        (
            Selector::new()
                .route(left.clone(), 1)
                .route(right.clone(), 2),
            1,
        ),
        (Selector::new().route(right, 2).route(left, 1), 2),
    ] {
        assert_eq!(
            *selector
                .select(&Head::from_request(&request))
                .unwrap()
                .unwrap()
                .target,
            expected
        );
    }
    let composed = path("/{one}/part")
        .unwrap()
        .and(path("/value/{two}").unwrap())
        .filter(|c| c.get("one") == Some("value") && c.get("two") == Some("part"));
    assert!(
        composed
            .evaluate(&Head::from_request(&request))
            .unwrap()
            .is_some()
    );
    assert!(
        exact_path("/value/part")
            .filter(|c| c.get("missing").is_some())
            .evaluate(&Head::from_request(&request))
            .unwrap()
            .is_none()
    );
}

#[test]
fn local_predicates_need_neither_clone_nor_thread_safety() {
    struct NotClone(Cell<usize>);
    let state = Rc::new(NotClone(Cell::new(0)));
    let selector = Selector::new().route(
        match_fn(|_| true).filter({
            let state = state.clone();
            move |_| {
                state.0.set(state.0.get() + 1);
                true
            }
        }),
        (),
    );
    let request = Request::new(());
    assert!(
        selector
            .select(&Head::from_request(&request))
            .unwrap()
            .is_some()
    );
    assert_eq!(state.0.get(), 1);
    // Box<dyn Fn> is not Clone, independently of closure capture precision.
    let predicate: Box<dyn Fn(&Captures) -> bool> = Box::new(|_| true);
    assert!(
        match_fn(|_| true)
            .filter(predicate)
            .evaluate(&Head::from_request(&request))
            .unwrap()
            .is_some()
    );
}

#[test]
fn shared_refinement_retains_thread_safe_selector_support() {
    let selector = SyncSelector::new().route(
        path("/{id}").unwrap().filter(|c| c.get("id") == Some("42")),
        42,
    );
    std::thread::spawn(move || {
        let request = Request::builder().uri("/42").body(()).unwrap();
        assert_eq!(
            *selector
                .select(&Head::from_request(&request))
                .unwrap()
                .unwrap()
                .target,
            42
        );
    })
    .join()
    .unwrap();
}

#[cfg(feature = "tower")]
#[tokio::test]
async fn refined_dispatch_keeps_the_body_and_never_retries_a_handler_error() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tower::{ServiceExt, service_fn};
    let fallback_calls = Arc::new(AtomicUsize::new(0));
    let later_calls = fallback_calls.clone();
    let router = warded::Router::new()
        .route(
            path("/users/{id}")
                .unwrap()
                .filter(|c| c.get("id") == Some("42")),
            service_fn(|request: Request<String>| async move {
                assert_eq!(
                    request.extensions().get::<Captures>().unwrap().get("id"),
                    Some("42")
                );
                assert_eq!(request.body(), "unread body");
                Err::<http::Response<String>, _>("selected failure")
            }),
        )
        .route(
            path("/users/{username}").unwrap(),
            service_fn(move |_: Request<String>| {
                later_calls.fetch_add(1, Ordering::SeqCst);
                async { Ok::<_, &str>(http::Response::new(String::new())) }
            }),
        );
    let request = Request::builder()
        .uri("/users/42")
        .body("unread body".into())
        .unwrap();
    assert!(matches!(
        router.oneshot(request).await,
        Err(warded::DispatchError::Service("selected failure"))
    ));
    assert_eq!(fallback_calls.load(Ordering::SeqCst), 0);
}
