//! Independently written public-boundary regressions; see docs/upstream-tests.md for provenance.

use http::{Method, Request};
use std::{cell::RefCell, rc::Rc};
use warded::{Captures, Head, Matcher, Selector, match_fn, matchers::*, try_match_fn};

fn request(uri: &str) -> Request<()> {
    Request::builder().uri(uri).body(()).unwrap()
}

#[test]
fn empty_captures_and_partial_affixes_do_not_complete_a_path() {
    for (pattern, rejected, accepted, name, value) in [
        ("/people/{person}", "/people/", "/people/7", "person", "7"),
        (
            "/people/{person}/info",
            "/people//info",
            "/people/7/info",
            "person",
            "7",
        ),
        ("/files/{*tail}", "/files/", "/files/a/b", "tail", "a/b"),
        (
            "/media/p-{key}.json",
            "/media/p-.json",
            "/media/p-7.json",
            "key",
            "7",
        ),
    ] {
        let rule = path(pattern).unwrap();
        assert!(
            rule.evaluate(&Head::from_request(&request(rejected)))
                .unwrap()
                .is_none(),
            "{pattern}: {rejected}"
        );
        let captures = rule
            .evaluate(&Head::from_request(&request(accepted)))
            .unwrap()
            .unwrap();
        assert_eq!(captures.get(name), Some(value), "{pattern}: {accepted}");
    }
    let rule = path("/media/p-{key}.json").unwrap();
    for rejected in [
        "/media/p-7.jpg",
        "/media/p-7.json.extra",
        "/media/p-7.json/",
    ] {
        assert!(
            rule.evaluate(&Head::from_request(&request(rejected)))
                .unwrap()
                .is_none(),
            "{rejected}"
        );
    }
    let captures = rule
        .evaluate(&Head::from_request(&request(
            "/media/p-7.json?download=yes",
        )))
        .unwrap()
        .unwrap();
    assert_eq!(captures.get("key"), Some("7"));
}

#[test]
fn incomplete_longer_paths_and_failed_headers_leave_later_overlaps_reachable() {
    let selector = Selector::new()
        .route(
            path("/vault/{candidate}/inspect")
                .unwrap()
                .and(header_eq("x-view", "detailed").unwrap()),
            "detailed",
        )
        .route(path("/{section}/{*tail}").unwrap(), "general");
    for (uri, detailed) in [
        ("/vault/alice", false),
        ("/vault/alice/inspect", false),
        ("/vault/alice/inspect", true),
    ] {
        let mut request = request(uri);
        if detailed {
            request
                .headers_mut()
                .insert("x-view", "detailed".parse().unwrap());
        }
        let selected = selector
            .select(&Head::from_request(&request))
            .unwrap()
            .unwrap();
        if detailed {
            assert_eq!(*selected.target, "detailed");
            assert_eq!(
                selected.selection.captures.iter().collect::<Vec<_>>(),
                [("candidate", "alice")]
            );
        } else {
            assert_eq!(*selected.target, "general");
            assert_eq!(selected.selection.captures.get("section"), Some("vault"));
            assert_eq!(selected.selection.captures.get("tail"), Some(&uri[7..]));
            assert_eq!(selected.selection.captures.get("candidate"), None);
        }
    }
}

#[test]
fn rejected_nested_collisions_do_not_replace_successful_ancestor_captures() {
    let ancestor = path("/{id}/{action}").unwrap();
    let rejected_child = path("/{other}/{id}").unwrap().and(match_fn(|_| false));
    let rule = ancestor.and(rejected_child.or(exact_path("/alice/edit")));
    let captures = rule
        .evaluate(&Head::from_request(&request("/alice/edit")))
        .unwrap()
        .unwrap();
    assert_eq!(
        captures.iter().collect::<Vec<_>>(),
        [("action", "edit"), ("id", "alice")]
    );
}

#[test]
fn raw_namespace_cases_are_preserved_instead_of_normalized_into_another_route() {
    let selector = Selector::new()
        .route(exact_path("/private"), "canonical")
        .route(path("/{*tail}").unwrap(), "raw");
    for raw in [
        "/./private",
        "/public/../private",
        "/%2e/private",
        "/%2E%2E/private",
        "/public//private",
        "/public%2fprivate",
        "/public%2Fprivate",
        "/public%5cprivate",
        "/public%5Cprivate",
        "/%252f",
        "/v1%2e2",
        "/hello%20world",
    ] {
        let request = request(raw);
        let selected = selector
            .select(&Head::from_request(&request))
            .unwrap()
            .unwrap();
        assert_eq!(*selected.target, "raw", "{raw}");
        assert_eq!(
            selected.selection.captures.get("tail"),
            Some(&raw[1..]),
            "{raw}"
        );
        assert_eq!(request.uri().path(), raw);
    }
    assert_eq!(
        *selector
            .select(&Head::from_request(&request("/private")))
            .unwrap()
            .unwrap()
            .target,
        "canonical"
    );
}

#[test]
fn absolute_form_uses_the_path_but_keeps_the_original_uri() {
    let uri = "https://example.invalid/people/a%2Fb?tag=first&tag=second";
    let request = request(uri);
    let rule = path("/people/{id}")
        .unwrap()
        .and(query_pair("tag", "second"));
    let captures = rule
        .evaluate(&Head::from_request(&request))
        .unwrap()
        .unwrap();
    assert_eq!(captures.get("id"), Some("a%2Fb"));
    assert_eq!(request.uri().to_string(), uri);
    assert_eq!(
        request.uri().authority().unwrap().as_str(),
        "example.invalid"
    );
}

#[test]
fn authority_form_connect_can_match_without_a_path_condition() {
    let request = Request::builder()
        .method(Method::CONNECT)
        .uri("gateway.invalid:9443")
        .body(())
        .unwrap();
    assert_eq!(request.uri().path(), "");
    assert_eq!(request.uri().path_and_query(), None);
    let routes = Selector::new()
        .route(exact_path("/"), "root")
        .route(method(Method::CONNECT), "tunnel");
    let head = Head::from_request(&request);
    assert_eq!(*routes.select(&head).unwrap().unwrap().target, "tunnel");
    assert_eq!(
        head.uri().authority().unwrap().as_str(),
        "gateway.invalid:9443"
    );
}

#[test]
fn query_and_extension_method_names_are_exact_and_case_sensitive() {
    let extension = Method::from_bytes(b"WARDEN-REPORT").unwrap();
    let routes = Selector::new()
        .route(method(Method::QUERY), "query")
        .route(method(extension), "extension");
    for (verb, expected) in [
        ("QUERY", Some("query")),
        ("query", None),
        ("WARDEN-REPORT", Some("extension")),
        ("warden-report", None),
        ("POST", None),
    ] {
        let request = Request::builder().method(verb).body(()).unwrap();
        assert_eq!(
            routes
                .select(&Head::from_request(&request))
                .unwrap()
                .map(|selected| *selected.target),
            expected,
            "{verb}"
        );
    }
}

#[test]
fn unreachable_boolean_branches_are_not_evaluated_and_order_is_left_to_right() {
    fn step(
        label: &'static str,
        matched: bool,
        log: Rc<RefCell<Vec<&'static str>>>,
    ) -> impl Matcher + Clone {
        match_fn(move |_| {
            log.borrow_mut().push(label);
            matched
        })
    }
    let log = Rc::new(RefCell::new(Vec::new()));
    let rule = step("first", true, log.clone())
        .and(step("miss", false, log.clone()).or(step("alternative", true, log.clone())))
        .or(step("unreachable", true, log.clone()));
    assert!(
        rule.evaluate(&Head::from_request(&request("/")))
            .unwrap()
            .is_some()
    );
    assert_eq!(*log.borrow(), ["first", "miss", "alternative"]);
    log.borrow_mut().clear();
    let rule = step("stop", false, log.clone()).and(step("unreachable", true, log.clone()));
    assert!(
        rule.evaluate(&Head::from_request(&request("/")))
            .unwrap()
            .is_none()
    );
    assert_eq!(*log.borrow(), ["stop"]);
}

#[test]
fn malformed_parameter_definitions_return_errors() {
    for pattern in [
        "/people/{}",
        "/files/{*}",
        "/files/{*rest}/suffix",
        "/{left}{right}",
        "/prefix-{left}-{right}",
    ] {
        assert!(path(pattern).is_err(), "{pattern}");
    }
}

#[test]
fn discarded_branches_can_reuse_a_capture_name_without_publishing_it() {
    let rule = path("/{id}")
        .unwrap()
        .and(match_fn(|_| false))
        .or(try_match_fn(|_| {
            let mut captures = Captures::default();
            captures.insert("id", "fallback value")?;
            Ok(Some(captures))
        }));
    let captures = rule
        .evaluate(&Head::from_request(&request("/rejected")))
        .unwrap()
        .unwrap();
    assert_eq!(
        captures.iter().collect::<Vec<_>>(),
        [("id", "fallback value")]
    );
}

#[cfg(feature = "tower")]
mod dispatch {
    use super::*;
    use axum::{body::Body, error_handling::HandleError};
    use http::{Response, StatusCode};
    use std::{convert::Infallible, time::Duration};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        sync::oneshot,
    };
    use tower::{ServiceExt, service_fn};
    use warded::{DispatchError, Router};

    fn reply(status: StatusCode) -> Response<Body> {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = status;
        response
    }

    #[tokio::test]
    async fn deriving_routes_and_fallbacks_keeps_the_original_snapshot() {
        let base = Router::new()
            .route(
                exact_path("/known"),
                service_fn(|_: Request<()>| async { Ok::<_, Infallible>(reply(StatusCode::OK)) }),
            )
            .fallback(service_fn(|_: Request<()>| async {
                Ok::<_, Infallible>(reply(StatusCode::NOT_FOUND))
            }));
        let derived = base
            .clone()
            .route(
                exact_path("/added"),
                service_fn(|_: Request<()>| async {
                    Ok::<_, Infallible>(reply(StatusCode::CREATED))
                }),
            )
            .fallback(service_fn(|_: Request<()>| async {
                Ok::<_, Infallible>(reply(StatusCode::GONE))
            }));
        for (uri, original, changed) in [
            ("/known", StatusCode::OK, StatusCode::OK),
            ("/added", StatusCode::NOT_FOUND, StatusCode::CREATED),
            ("/missing", StatusCode::NOT_FOUND, StatusCode::GONE),
        ] {
            assert_eq!(
                base.clone().oneshot(request(uri)).await.unwrap().status(),
                original
            );
            assert_eq!(
                derived
                    .clone()
                    .oneshot(request(uri))
                    .await
                    .unwrap()
                    .status(),
                changed
            );
        }
    }

    #[tokio::test]
    async fn query_and_extension_methods_keep_their_body_over_http1() {
        tokio::time::timeout(Duration::from_secs(10), async {
            let router = Router::new().route(
                method(Method::QUERY).or(method(Method::from_bytes(b"WARDEN-REPORT").unwrap()))
                    .and(exact_path("/inspect")),
                service_fn(|request: Request<Body>| async move {
                    assert_eq!(request.uri().query(), Some("tag=first&tag=second"));
                    assert_eq!(request.headers()["content-type"], "application/x-www-form-urlencoded");
                    let verb = request.method().clone();
                    let bytes = axum::body::to_bytes(request.into_body(), 1024).await.unwrap();
                    let body = String::from_utf8(bytes.to_vec()).unwrap();
                    Ok::<_, Infallible>(Response::new(Body::from(format!("{verb}:{body}"))))
                }),
            );
            let host = axum::Router::new().fallback_service(HandleError::new(router, |_error: DispatchError<Infallible>| async { StatusCode::INTERNAL_SERVER_ERROR }));
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let (shutdown, stopped) = oneshot::channel();
            let server = tokio::spawn(async move {
                axum::serve(listener, host).with_graceful_shutdown(async { let _ = stopped.await; }).await.unwrap();
            });
            for (verb, expected) in [("QUERY", 200), ("WARDEN-REPORT", 200), ("query", 404), ("warden-report", 404)] {
                let body = "find=a%2Fb+z";
                let mut socket = TcpStream::connect(address).await.unwrap();
                let message = format!("{verb} /inspect?tag=first&tag=second HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                socket.write_all(message.as_bytes()).await.unwrap();
                let mut wire = String::new();
                socket.read_to_string(&mut wire).await.unwrap();
                let (head, received) = wire.split_once("\r\n\r\n").unwrap();
                assert_eq!(head.split_whitespace().nth(1).unwrap().parse::<u16>().unwrap(), expected, "{verb}: {wire}");
                if expected == 200 {
                    assert_eq!(received, format!("{verb}:{body}"));
                } else {
                    assert!(received.is_empty());
                }
            }
            shutdown.send(()).unwrap();
            server.await.unwrap();
        }).await.expect("bounded HTTP method probe timed out");
    }
}
