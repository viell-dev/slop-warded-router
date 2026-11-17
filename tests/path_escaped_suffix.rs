//! Public matching regressions for escaped literals beside declared captures.

use http::Request;
use warded::{
    DefinitionError, Head, Matcher, Selector,
    matchers::{header_present, path},
};

fn assert_captures(pattern: &str, uri: &str, expected: &[(&str, &str)]) {
    let matcher = path(pattern).unwrap_or_else(|error| panic!("{pattern}: {error}"));
    let request = Request::builder().uri(uri).body(()).unwrap();
    let captures = matcher
        .evaluate(&Head::from_request(&request))
        .unwrap()
        .unwrap_or_else(|| panic!("{pattern} missed {uri}"));
    assert_eq!(captures.iter().collect::<Vec<_>>(), expected, "{pattern}");
}

#[test]
fn escaped_suffix_accepts_a_longer_name() {
    assert_captures("/{id}{{x}}", "/value{x}", &[("id", "value")]);
}

#[test]
fn escaped_suffix_accepts_a_unicode_name() {
    assert_captures("/{é}{{x}}", "/value{x}", &[("é", "value")]);
}

#[test]
fn escaped_literals_before_between_and_after_captures_keep_their_meaning() {
    assert_captures(
        "/{{pre}}{zebra}{{mid}}/{{next}}{alpha}{{post}}",
        "/{pre}raw%2Fvalue{mid}/{next}second{post}",
        &[("alpha", "second"), ("zebra", "raw%2Fvalue")],
    );
    assert_captures(
        "/{first}{{tag}}/{a}/{{files}}{*rest}",
        "/one{tag}/two/{files}raw%2Fpart/end",
        &[("a", "two"), ("first", "one"), ("rest", "raw%2Fpart/end")],
    );
}

#[test]
fn escaped_suffix_misses_and_rejected_candidates_discard_captures() {
    let matcher = path("/{id}{{x}}").unwrap();
    for uri in ["/value%7Bx%7D", "/{x}", "/value{other}", "/value{x}/"] {
        let request = Request::builder().uri(uri).body(()).unwrap();
        assert_eq!(matcher.evaluate(&Head::from_request(&request)), Ok(None));
    }

    let routes = Selector::new()
        .route(matcher.and(header_present("x-selected").unwrap()), "first")
        .route(path("/{fallback}").unwrap(), "fallback");
    let request = Request::builder().uri("/value{x}").body(()).unwrap();
    let selected = routes
        .select(&Head::from_request(&request))
        .unwrap()
        .unwrap();
    assert_eq!(*selected.target, "fallback");
    assert_eq!(
        selected.selection.captures.iter().collect::<Vec<_>>(),
        [("fallback", "value{x}")]
    );
}

#[test]
fn escaped_suffix_keeps_definition_errors() {
    for pattern in [
        "/{}{{x}}",
        "/{*}{{x}}",
        "/{bad*name}{{x}}",
        "/{bad/name}{{x}}",
        "/{}}name}{{x}}",
        "/{id}{{x}}{other}",
        "/{*rest}{{x}}",
        "/{*rest}/{{x}}",
        "/{id}{{x}}/{id}",
        "/{na{{me}{{x}}/{*na{{me}",
        "/{id}{{x}",
        "/{id}{{x}}/{missing",
    ] {
        let error: DefinitionError = path(pattern).expect_err(pattern);
        assert!(!error.to_string().is_empty(), "{pattern}");
    }
}
