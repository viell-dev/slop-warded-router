//! Counterexamples reproduced against alpha.1 before implementing alpha.2.

#![cfg(feature = "negotiation")]

use http::Request;
use warded::{Head, Matcher, matchers::path, negotiation::*};

fn matches(rule: &impl Matcher, name: &str, value: &str) -> bool {
    let request = Request::builder().header(name, value).body(()).unwrap();
    rule.evaluate(&Head::from_request(&request))
        .unwrap()
        .is_some()
}

#[test]
fn parameter_specificity_preserves_exclusions_in_both_member_orders() {
    let rule = accepts_media_type("text/html;a=1;b=2").unwrap();
    for value in [
        "text/html;a=1;q=1,text/html;a=1;b=2;q=0",
        "text/html;a=1;b=2;q=0,text/html;a=1;q=1",
    ] {
        assert!(!matches(&rule, "accept", value), "{value}");
    }
}

#[test]
fn media_parameters_constrain_the_offer_on_both_sides_of_q() {
    let offers = Offers::media_types(["text/html;level=1", "text/html;level=2"]).unwrap();
    let second = offers.selected("text/html;level=2").unwrap();
    for value in ["text/html;level=2;q=1", "text/html;q=1;level=2"] {
        assert!(matches(&second, "accept", value), "{value}");
    }
}

#[test]
fn lookup_tries_high_priority_fallback_before_lower_priority_exact_match() {
    let offers = Offers::languages(["en", "fr"], LanguageMatching::Lookup).unwrap();
    assert!(matches(
        &offers.selected("en").unwrap(),
        "accept-language",
        "en-US;q=1,en;q=0.1,fr;q=0.5"
    ));
}

#[test]
fn lookup_skips_a_wildcard_when_an_explicit_range_can_match() {
    let offers = Offers::languages(["en", "fr"], LanguageMatching::Lookup).unwrap();
    assert!(matches(
        &offers.selected("fr").unwrap(),
        "accept-language",
        "*;q=1,fr;q=0.5"
    ));
}

#[test]
fn language_offers_are_concrete_tags_in_both_modes() {
    for mode in [LanguageMatching::Lookup, LanguageMatching::BasicFilter] {
        for value in ["*;q=1", "en,*", " *", "fr;q=0,*;q=1"] {
            assert!(
                Offers::languages([value], mode).is_err(),
                "{mode:?} {value}"
            );
        }
    }
}

#[test]
fn a_duplicate_capture_in_one_pattern_is_a_definition_error() {
    assert!(path("/{id}/{id}").is_err());
}

#[test]
fn weighted_lookup_and_its_zero_quality_exclusions_have_explicit_rules() {
    let offers = Offers::languages(["fr", "en"], LanguageMatching::Lookup).unwrap();
    for (value, expected) in [
        ("fr-CA,en", "fr"),
        ("en-US;q=1,en;q=0,fr;q=0.5", "fr"),
        ("en-US;q=1,*;q=0", "en"),
        ("*;q=1,fr;q=0", "en"),
        ("en-x-private;q=1,fr;q=0.5", "en"),
    ] {
        assert!(
            matches(
                &offers.selected(expected).unwrap(),
                "accept-language",
                value
            ),
            "{value}"
        );
    }
    for tag in ["x-private", "i-klingon", "zh-Hant-TW", "en-US-u-ca-gregory"] {
        assert!(
            Offers::languages([tag], LanguageMatching::Lookup).is_ok(),
            "{tag}"
        );
    }
    for tag in ["en--US", "en_US", "en;q=1", "en,fr", "en-*", "*"] {
        assert!(
            Offers::languages([tag], LanguageMatching::Lookup).is_err(),
            "{tag}"
        );
    }
}

#[test]
fn media_parameter_rules_cover_wildcards_quotes_case_and_parameters_around_q() {
    let offer = accepts_media_type("text/html;a=1;b=\"x,y\";charset=UTF-8").unwrap();
    for value in [
        "text/*;a=1;q=1,text/*;a=1;b=\"x,y\";q=0",
        "text/*;a=1;b=\"x,y\";q=0,text/*;a=1;q=1",
        "text/html;a=1;Q=0;b=\"x,y\"",
    ] {
        assert!(!matches(&offer, "accept", value), "{value}");
    }
    assert!(matches(
        &offer,
        "accept",
        "TEXT/HTML;a=1;Q=1;b=\"x,y\";charset=utf-8"
    ));
    let escaped = accepts_media_type("text/html;a=\"x\\\"y\"").unwrap();
    assert!(matches(&escaped, "accept", "text/html;q=1;a=\"x\\\"y\""));
}

#[test]
fn field_byte_limits_are_tested_with_valid_syntax_and_repeated_field_separators() {
    let rule = accepts_media_type("text/html").unwrap();
    let prefix = "text/html";
    for length in [65_535, 65_536, 65_537] {
        let value = format!("{prefix}{}", " ".repeat(length - prefix.len()));
        assert_eq!(matches(&rule, "accept", &value), length <= 65_536);
    }
    let strict = rule.clone().invalid_is_error();
    let request = Request::builder()
        .header(
            "accept",
            format!("{prefix}{}", " ".repeat(65_537 - prefix.len())),
        )
        .body(())
        .unwrap();
    assert!(strict.evaluate(&Head::from_request(&request)).is_err());
    // "text/html" + comma + "*/*" is exactly 13 bytes.
    for (limit, expected) in [(12, false), (13, true), (14, true)] {
        let request = Request::builder()
            .header("accept", "text/html")
            .header("accept", "*/*")
            .body(())
            .unwrap();
        let limited = rule
            .clone()
            .parser_limits(Limits::DEFAULT.max_field_bytes(limit));
        assert_eq!(
            limited
                .evaluate(&Head::from_request(&request))
                .unwrap()
                .is_some(),
            expected
        );
    }
}

#[test]
fn cached_choices_do_not_cross_heads_sets_limits_or_invalid_input_policies() {
    let offers = Offers::media_types(["text/html", "application/json"]).unwrap();
    let html = offers.selected("text/html").unwrap();
    let json = offers.selected("application/json").unwrap();
    for (header, expected_html) in [("text/html", true), ("application/json", false)] {
        let request = Request::builder()
            .header("accept", header)
            .body(())
            .unwrap();
        let head = Head::from_request(&request);
        assert_eq!(html.evaluate(&head).unwrap().is_some(), expected_html);
        assert_eq!(json.evaluate(&head).unwrap().is_some(), !expected_html);
        assert!(
            html.clone()
                .parser_limits(Limits::DEFAULT.max_field_bytes(1))
                .evaluate(&head)
                .unwrap()
                .is_none()
        );
        assert!(
            html.clone()
                .parser_limits(Limits::DEFAULT.max_field_bytes(1))
                .invalid_is_error()
                .evaluate(&head)
                .is_err()
        );
    }
    let request = Request::builder()
        .header("accept", "text/html")
        .body(())
        .unwrap();
    let head = Head::from_request(&request);
    assert!(
        accepts_media_type("text/html")
            .unwrap()
            .evaluate(&head)
            .unwrap()
            .is_some()
    );
    assert!(
        accepts_media_type("application/json")
            .unwrap()
            .evaluate(&head)
            .unwrap()
            .is_none()
    );
}

#[test]
fn private_use_server_tags_obey_subtag_grammar_in_both_modes() {
    for mode in [LanguageMatching::Lookup, LanguageMatching::BasicFilter] {
        for tag in ["x--", "x-private-", "x-123456789", "x-", "en-x--"] {
            assert!(Offers::languages([tag], mode).is_err(), "{mode:?}: {tag}");
        }
        for tag in [
            "x-a-b",
            "x-private",
            "x-12345678",
            "i-klingon",
            "sgn-BE-FR",
            "abcdefgh",
            "en-US-x-private",
        ] {
            assert!(Offers::languages([tag], mode).is_ok(), "{mode:?}: {tag}");
        }
    }
}

#[test]
fn supported_byte_bounds_cover_all_negotiation_fields_and_parameter_tokens() {
    for (field, value, rule) in [
        (
            "accept",
            "text/html",
            accepts_media_type("text/html").unwrap(),
        ),
        (
            "accept-language",
            "en",
            accepts_language("en", LanguageMatching::Lookup).unwrap(),
        ),
        ("accept-encoding", "gzip", accepts_encoding("gzip").unwrap()),
        (
            "accept",
            "t/h;longname=longvalue",
            accepts_media_type("t/h;longname=longvalue").unwrap(),
        ),
    ] {
        let request = Request::builder().header(field, value).body(()).unwrap();
        let head = Head::from_request(&request);
        for bound in [value.len() - 1, value.len(), value.len() + 1] {
            let limited = rule
                .clone()
                .parser_limits(Limits::DEFAULT.max_field_bytes(bound));
            let expected = bound >= value.len();
            assert_eq!(
                limited.evaluate(&head).unwrap().is_some(),
                expected,
                "{field} {bound}"
            );
            assert_eq!(
                limited.invalid_is_error().evaluate(&head).is_ok(),
                expected,
                "{field} {bound}"
            );
        }
    }
}

#[test]
fn supported_member_and_parameter_bounds_have_field_specific_boundaries() {
    for (field, value, rule) in [
        (
            "accept",
            "text/html,application/json",
            accepts_media_type("text/html").unwrap(),
        ),
        (
            "accept-language",
            "en,fr",
            accepts_language("en", LanguageMatching::Lookup).unwrap(),
        ),
        (
            "accept-encoding",
            "gzip,br",
            accepts_encoding("gzip").unwrap(),
        ),
    ] {
        let request = Request::builder().header(field, value).body(()).unwrap();
        let head = Head::from_request(&request);
        for bound in [1, 2, 3] {
            let limited = rule
                .clone()
                .parser_limits(Limits::DEFAULT.max_list_members(bound));
            assert_eq!(
                limited.evaluate(&head).unwrap().is_some(),
                bound >= 2,
                "{field} {bound}"
            );
            assert_eq!(
                limited.invalid_is_error().evaluate(&head).is_ok(),
                bound >= 2,
                "{field} {bound}"
            );
        }
    }
    for (field, value, rule) in [
        (
            "accept",
            "text/html;q=0.5",
            accepts_media_type("text/html").unwrap(),
        ),
        (
            "accept-language",
            "en;q=0.5",
            accepts_language("en", LanguageMatching::Lookup).unwrap(),
        ),
        (
            "accept-encoding",
            "gzip;q=0.5",
            accepts_encoding("gzip").unwrap(),
        ),
    ] {
        let request = Request::builder().header(field, value).body(()).unwrap();
        let head = Head::from_request(&request);
        for bound in [0, 1, 2] {
            let limited = rule
                .clone()
                .parser_limits(Limits::DEFAULT.max_params(bound));
            assert_eq!(
                limited.evaluate(&head).unwrap().is_some(),
                bound >= 1,
                "{field} {bound}"
            );
            assert_eq!(
                limited.invalid_is_error().evaluate(&head).is_ok(),
                bound >= 1,
                "{field} {bound}"
            );
        }
    }
}
