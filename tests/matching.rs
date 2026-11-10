//! Request-head selection and primitive matcher regression checks.

use http::{Request, Version, header::HeaderValue};
use std::{cell::Cell, rc::Rc};
use warded::{
    Captures, Head, MatchError, Matcher, Selector, TransportSecurity, match_fn, matchers::*,
    try_match_fn,
};

fn matches(matcher: &impl Matcher, request: &Request<()>) -> bool {
    matcher
        .evaluate(&Head::from_request(request))
        .unwrap()
        .is_some()
}

#[test]
fn complete_conditions_and_owner_language_example() {
    let selector = Selector::new().route(
        exact_path("/orders").and(header_eq("Accept-Language", "fr").unwrap()),
        "fr",
    );
    for (uri, lang, expected) in [
        ("/orders", Some("fr"), Some("fr")),
        ("/orders", Some("en"), None),
        ("/orders", None, None),
        ("/orders", Some("fr-FR"), None),
        ("/other", Some("fr"), None),
    ] {
        let mut builder = Request::builder().uri(uri);
        if let Some(lang) = lang {
            builder = builder.header("accept-language", lang);
        }
        let request = builder.body(()).unwrap();
        assert_eq!(
            selector
                .select(&Head::from_request(&request))
                .unwrap()
                .map(|s| *s.target),
            expected
        );
    }
}

#[test]
fn same_path_variants_and_static_overlap_preserve_declaration_order() {
    let selector = Selector::new()
        .route(
            exact_path("/orders/new").and(header_eq("accept-language", "fr").unwrap()),
            "fr",
        )
        .route(path("/orders/{id}").unwrap(), "parameter")
        .route(exact_path("/orders/new"), "later static");
    let request = Request::builder()
        .uri("/orders/new")
        .header("accept-language", "en")
        .body(())
        .unwrap();
    let selected = selector
        .select(&Head::from_request(&request))
        .unwrap()
        .unwrap();
    assert_eq!(*selected.target, "parameter");
    assert_eq!(selected.selection.captures.get("id"), Some("new"));
    assert_eq!(selected.selection.index, 1);
}

#[test]
fn every_required_dimension_can_participate_without_a_path() {
    let rule = get()
        .and(version(Version::HTTP_2))
        .and(secured())
        .and(header_eq("x-mode", "preview").unwrap())
        .and(query_pair("revision", "2"));
    for missing in 0..6 {
        let mut request = Request::builder()
            .method(if missing == 1 { "POST" } else { "GET" })
            .version(if missing == 2 {
                Version::HTTP_11
            } else {
                Version::HTTP_2
            })
            .uri(if missing == 3 {
                "/any?revision=3"
            } else {
                "/any?revision=2"
            })
            .body(())
            .unwrap();
        if missing != 4 {
            request
                .headers_mut()
                .insert("x-mode", HeaderValue::from_static("preview"));
        }
        request.extensions_mut().insert(if missing == 5 {
            TransportSecurity::Unsecure
        } else {
            TransportSecurity::Https
        });
        assert_eq!(matches(&rule, &request), missing == 0);
    }
}

#[test]
fn pathless_version_rules_and_earlier_generic_routes_are_reachable() {
    let selector = Selector::new()
        .route(version(Version::HTTP_11), "legacy")
        .route(exact_path("/orders"), "path");
    for uri in ["/", "/orders", "/anything"] {
        let request = Request::builder().uri(uri).body(()).unwrap();
        assert_eq!(
            *selector
                .select(&Head::from_request(&request))
                .unwrap()
                .unwrap()
                .target,
            "legacy"
        );
    }
}

#[test]
fn failed_branches_and_negation_cannot_publish_captures() {
    let selector = Selector::new().route(
        path("/{discard}")
            .unwrap()
            .and(exact_path("/never"))
            .or(path("/{also_discard}").unwrap().not())
            .or(path("/{name}").unwrap()),
        "selected",
    );
    let request = Request::builder().uri("/alice").body(()).unwrap();
    let captures = selector
        .select(&Head::from_request(&request))
        .unwrap()
        .unwrap()
        .selection
        .captures;
    assert_eq!(captures.iter().collect::<Vec<_>>(), vec![("name", "alice")]);
}

#[test]
fn conjunction_reports_capture_collisions_and_errors_are_not_negated() {
    let request = Request::builder().uri("/value").body(()).unwrap();
    let duplicate = path("/{id}").unwrap().and(path("/{id}").unwrap());
    assert_eq!(
        duplicate.evaluate(&Head::from_request(&request)),
        Err(MatchError::DuplicateCapture("id".into()))
    );
    assert_eq!(
        secured().not().evaluate(&Head::from_request(&request)),
        Err(MatchError::MissingSecurityContext)
    );
    assert_eq!(
        secured()
            .or(match_fn(|_| true))
            .evaluate(&Head::from_request(&request)),
        Err(MatchError::MissingSecurityContext)
    );
}

#[test]
fn raw_path_and_query_rules_do_not_normalize_or_lose_repetitions() {
    let request = Request::builder()
        .uri("/a%2Fb/?x=1&x=2&space=a+b&encoded=%31&flag&empty=")
        .body(())
        .unwrap();
    assert!(matches(&exact_path("/a%2Fb/"), &request));
    assert!(!matches(&exact_path("/a/b/"), &request));
    assert!(!matches(&exact_path("/a%2Fb"), &request));
    let captures = path("/{value}/")
        .unwrap()
        .evaluate(&Head::from_request(&request))
        .unwrap()
        .unwrap();
    assert_eq!(captures.get("value"), Some("a%2Fb"));
    for matcher in [
        query_pair("x", "2"),
        query_pair("space", "a+b"),
        query_pair("encoded", "%31"),
        query_pair("empty", ""),
    ] {
        assert!(matches(&matcher, &request));
    }
    assert!(!matches(&query_pair("encoded", "1"), &request));
    assert!(!matches(&query_pair("flag", ""), &request));
    assert!(matches(&query_present("flag"), &request));
    assert!(
        path("/{*rest}")
            .unwrap()
            .evaluate(&Head::from_request(&request))
            .unwrap()
            .is_some()
    );
    assert!(path("/{unfinished").is_err());
}

#[test]
fn raw_headers_preserve_repetition_and_non_utf8() {
    let mut request = Request::new(());
    request
        .headers_mut()
        .append("x-language", HeaderValue::from_static("en"));
    request
        .headers_mut()
        .append("x-language", HeaderValue::from_static("fr"));
    request
        .headers_mut()
        .insert("x-bytes", HeaderValue::from_bytes(&[0xff]).unwrap());
    assert!(!matches(&header_eq("X-Language", "fr").unwrap(), &request));
    assert!(matches(
        &header_any_eq("x-language", "fr").unwrap(),
        &request
    ));
    assert!(matches(&header_eq("x-bytes", [0xff]).unwrap(), &request));
    assert!(matches(&header_present("x-language").unwrap(), &request));
    assert!(header_present("invalid name").is_err());
    assert!(header_eq("valid", b"line\nbreak").is_err());
}

#[test]
fn shortcuts_are_equivalent_and_unknown_security_is_explicit() {
    for method_name in ["GET", "HEAD", "POST", "PUT"] {
        for security in [
            TransportSecurity::Unknown,
            TransportSecurity::Https,
            TransportSecurity::Unsecure,
        ] {
            let mut request = Request::builder().method(method_name).body(()).unwrap();
            request.extensions_mut().insert(security);
            let view = Head::from_request(&request);
            assert_eq!(
                secured_post().evaluate(&view),
                secured().and(post()).evaluate(&view)
            );
            assert_eq!(
                secured_get_and_head().evaluate(&view),
                secured().and(get().or(head())).evaluate(&view)
            );
            assert_eq!(
                get().evaluate(&view).unwrap().is_some(),
                method_name == "GET"
            );
            assert_eq!(
                unsecure().evaluate(&view).unwrap_or(None).is_some(),
                security == TransportSecurity::Unsecure
            );
            if security == TransportSecurity::Unknown {
                assert_eq!(
                    unsecure().evaluate(&view),
                    Err(MatchError::MissingSecurityContext)
                );
                assert_eq!(unsecure().unknown_as_miss().evaluate(&view), Ok(None));
            }
        }
    }
    let request = Request::builder()
        .uri("https://example.test/")
        .header("forwarded", "proto=https")
        .header("x-forwarded-proto", "https")
        .body(())
        .unwrap();
    assert_eq!(
        secured().evaluate(&Head::from_request(&request)),
        Err(MatchError::MissingSecurityContext)
    );
}

#[test]
fn local_matchers_need_no_send_or_sync_and_short_circuit() {
    let count = Rc::new(Cell::new(0));
    let observed = count.clone();
    let local = match_fn(move |_| {
        observed.set(observed.get() + 1);
        true
    });
    let selector = Selector::new()
        .route(match_fn(|_| false).and(local.clone()), 1)
        .route(local, 2);
    let request = Request::new(());
    assert_eq!(
        *selector
            .select(&Head::from_request(&request))
            .unwrap()
            .unwrap()
            .target,
        2
    );
    assert_eq!(count.get(), 1);
    let custom = try_match_fn(|_| {
        let mut captures = Captures::default();
        captures.insert("id", "local")?;
        Ok(Some(captures))
    });
    assert_eq!(
        custom
            .evaluate(&Head::from_request(&request))
            .unwrap()
            .unwrap()
            .get("id"),
        Some("local")
    );
}
