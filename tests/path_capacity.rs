//! Public construction boundaries for matchit's ordinary-parameter capacity.

use http::Request;
use warded::{Head, Matcher, matchers::path};

fn parameters(count: usize) -> String {
    (0..count).map(|n| format!("/{{p{n}}}")).collect()
}

#[test]
fn capacity_returns_an_error_instead_of_panicking() {
    for count in [26, 27, 100] {
        assert!(path(parameters(count)).is_err(), "{count} parameters");
    }
}

#[test]
fn escaped_suffix_does_not_bypass_capacity_guard() {
    let mut pattern = (0..25).map(|n| format!("/{{p{n}}}")).collect::<String>();
    pattern.push_str("{{extra}");
    assert!(warded::matchers::path(pattern).is_err());
}

#[test]
fn maximum_ordinary_parameters_and_a_tail_still_match() {
    let request_path = "/value".repeat(25);
    for (suffix, requested, tail) in [("", "", None), ("/{*tail}", "/a/b", Some("a/b"))] {
        let matcher = path(format!("{}{suffix}", parameters(25))).unwrap();
        let request = Request::builder()
            .uri(format!("{request_path}{requested}"))
            .body(())
            .unwrap();
        let captures = matcher
            .evaluate(&Head::from_request(&request))
            .unwrap()
            .unwrap();
        assert_eq!(captures.iter().count(), 25 + usize::from(tail.is_some()));
        assert_eq!(captures.get("p24"), Some("value"));
        assert_eq!(captures.get("tail"), tail);
    }
    assert!(path(format!("{}/{{*tail}}", parameters(26))).is_err());
}

#[test]
fn escaped_literals_names_and_affixes_do_not_change_capacity() {
    let literals = "/{{literal}}".repeat(30);
    assert!(path(format!("{literals}{}", parameters(25))).is_ok());
    assert!(path(format!("{literals}{}", parameters(26))).is_err());
    assert!(path(format!("{}/pre-{{last}}.json", parameters(24))).is_ok());
    assert!(path(format!("{}/pre-{{last}}.json", parameters(25))).is_err());
    for name in ["last}}}", "last{{name}", "用户}"] {
        assert!(
            path(format!("{}/{{{name}", parameters(24))).is_ok(),
            "{name}"
        );
        assert!(
            path(format!("{}/{{{name}", parameters(25))).is_err(),
            "{name}"
        );
    }
}
