//! Field-specific negotiation and malformed-input regression checks.

#![cfg(feature = "negotiation")]
use http::{Request, header::HeaderValue};
use warded::{Head, MatchError, Matcher, Selector, negotiation::*};

fn outcome(rule: &impl Matcher, name: &str, value: Option<&str>) -> Result<bool, MatchError> {
    let mut request = Request::new(());
    if let Some(value) = value {
        request.headers_mut().insert(
            http::header::HeaderName::try_from(name).unwrap(),
            HeaderValue::from_str(value).unwrap(),
        );
    }
    rule.evaluate(&Head::from_request(&request))
        .map(|value| value.is_some())
}

#[test]
fn media_acceptability_respects_specific_exclusions_parameters_and_quotes() {
    let html = accepts_media_type("text/html").unwrap();
    for (input, expected) in [
        (Some("text/*"), true),
        (Some("text/*;q=1,text/html;q=0"), false),
        (Some("text/html;q=0"), false),
        (Some("application/json"), false),
        (None, true),
    ] {
        assert_eq!(
            outcome(&html, "accept", input).unwrap(),
            expected,
            "{input:?}"
        );
    }
    let level = accepts_media_type("text/html;level=1").unwrap();
    assert!(outcome(&level, "accept", Some("text/html;level=1")).unwrap());
    assert!(!outcome(&level, "accept", Some("text/html;level=2")).unwrap());
    let quoted = accepts_media_type("text/html;profile=\"a,b\"").unwrap();
    assert!(outcome(&quoted, "accept", Some("text/html;profile=\"a,b\"")).unwrap());
}

#[test]
fn preferred_offers_are_distinct_from_independent_acceptable_routes() {
    let offers = Offers::media_types(["text/html", "application/json"]).unwrap();
    let selector = Selector::new()
        .route(offers.selected("text/html").unwrap(), "html")
        .route(offers.selected("application/json").unwrap(), "json");
    let request = Request::builder()
        .header("accept", "application/json;q=1,text/html;q=0.5")
        .body(())
        .unwrap();
    assert_eq!(
        *selector
            .select(&Head::from_request(&request))
            .unwrap()
            .unwrap()
            .target,
        "json"
    );
    let acceptable = Selector::new()
        .route(accepts_media_type("text/html").unwrap(), "html")
        .route(accepts_media_type("application/json").unwrap(), "json");
    assert_eq!(
        *acceptable
            .select(&Head::from_request(&request))
            .unwrap()
            .unwrap()
            .target,
        "html"
    );
    assert!(offers.selected("image/png").is_err());
    assert!(Offers::media_types(Vec::<String>::new()).is_err());
    assert!(accepts_media_type("*/*").is_err());
    assert!(accepts_media_type("text/html;q=0.5").is_err());
}

#[test]
fn malformed_repeated_and_non_utf8_fields_use_explicit_miss_or_error_policy() {
    let html = accepts_media_type("text/html").unwrap();
    for value in [
        "text/html;q=2",
        "text/html;q=.5",
        "text/html;q=0.1234",
        "text/html;q=garbage",
    ] {
        assert!(!outcome(&html, "accept", Some(value)).unwrap());
        assert!(matches!(
            outcome(&html.clone().invalid_is_error(), "accept", Some(value)),
            Err(MatchError::InvalidHeader { .. })
        ));
    }
    let mut request = Request::new(());
    request
        .headers_mut()
        .append("accept", HeaderValue::from_static("text/html;q=0.5"));
    request
        .headers_mut()
        .append("accept", HeaderValue::from_static("application/json"));
    let offers = Offers::media_types(["text/html", "application/json"]).unwrap();
    assert!(
        offers
            .selected("application/json")
            .unwrap()
            .evaluate(&Head::from_request(&request))
            .unwrap()
            .is_some()
    );
    request
        .headers_mut()
        .insert("accept", HeaderValue::from_bytes(&[0xff]).unwrap());
    assert!(
        html.evaluate(&Head::from_request(&request))
            .unwrap()
            .is_none()
    );
}

#[test]
fn language_scheme_is_explicit_and_supports_case_wildcards_and_exclusions() {
    let filter = accepts_language("fr-CA", LanguageMatching::BasicFilter).unwrap();
    assert!(outcome(&filter, "accept-language", Some("FR")).unwrap());
    assert!(outcome(&filter, "accept-language", Some("*")).unwrap());
    assert!(!outcome(&filter, "accept-language", Some("fr;q=0")).unwrap());
    assert!(!outcome(&filter, "accept-language", Some("en")).unwrap());
    let lookup = accepts_language("fr", LanguageMatching::Lookup).unwrap();
    assert!(outcome(&lookup, "accept-language", Some("fr-CA")).unwrap());
    assert!(accepts_language("not_a_language", LanguageMatching::Lookup).is_err());
    assert!(accepts_language("*", LanguageMatching::Lookup).is_err());
}

#[test]
fn encoding_distinguishes_absence_empty_identity_and_specific_exclusions() {
    let gzip = accepts_encoding("gzip").unwrap();
    let identity = accepts_encoding("identity").unwrap();
    for (input, g, i) in [
        (None, true, true),
        (Some(""), false, true),
        (Some("gzip"), true, true),
        (Some("*;q=0"), false, false),
        (Some("gzip;q=0,*;q=1"), false, true),
        (Some("identity;q=0"), false, false),
    ] {
        assert_eq!(
            outcome(&gzip, "accept-encoding", input).unwrap(),
            g,
            "{input:?}"
        );
        assert_eq!(
            outcome(&identity, "accept-encoding", input).unwrap(),
            i,
            "{input:?}"
        );
    }
    assert!(accepts_encoding("*").is_err());
    assert!(accepts_encoding("gzip,br").is_err());
}

#[test]
fn parser_and_combined_byte_limits_fail_without_choosing_an_offer() {
    let rule = accepts_media_type("text/html")
        .unwrap()
        .parser_limits(warded::negotiation::Limits::DEFAULT.max_list_members(1));
    assert!(!outcome(&rule, "accept", Some("text/html,application/json")).unwrap());
    let rule = accepts_media_type("text/html").unwrap();
    assert!(!outcome(&rule, "accept", Some(&"a".repeat(65_537))).unwrap());
}

#[test]
fn documented_rules_are_kept_where_the_parser_dependency_selects_differently() {
    // parse-net 0.10.0's own selection gives a different answer in each case.
    let lookup = LanguageMatching::Lookup;
    let basic = LanguageMatching::BasicFilter;
    for (mode, offers, value, expected) in [
        // A more specific zero-quality range excludes a truncated or wildcard candidate.
        (lookup, &["en"][..], "en-US,en-GB;q=0", None),
        (lookup, &["en", "fr"], "*,en-GB;q=0", Some("fr")),
        // A positive wildcard is a final default, even before lower-priority ranges.
        (lookup, &["en"], "*;q=0.5,de;q=0.1", Some("en")),
        (lookup, &["en"], "*,de", Some("en")),
        // Equal basic-filtering weights keep offer order, not the longer matching range.
        (basic, &["en", "fr-CA"], "*,fr", Some("en")),
        (basic, &["en", "fr-CA"], "en,fr-CA", Some("en")),
    ] {
        let set = Offers::languages(offers.iter().copied(), mode).unwrap();
        for offer in offers {
            assert_eq!(
                outcome(
                    &set.selected(offer).unwrap(),
                    "accept-language",
                    Some(value)
                )
                .unwrap(),
                expected == Some(*offer),
                "{mode:?} {value} {offer}"
            );
        }
    }
}

#[test]
fn media_precedence_counts_every_parameter_a_member_can_carry() {
    let parameters = |count: usize| -> String { (0..count).map(|i| format!(";p{i}=v")).collect() };
    let offer = format!("text/html{}", parameters(64));
    let rule = accepts_media_type(&offer).unwrap();
    // The dependency's own specificity stops counting at 63 parameters.
    for value in [
        format!(
            "text/html{};q=0,text/html{}",
            parameters(63),
            parameters(64)
        ),
        format!(
            "text/html{},text/html{};q=0",
            parameters(64),
            parameters(63)
        ),
    ] {
        assert!(outcome(&rule, "accept", Some(&value)).unwrap(), "{value}");
    }
    // Duplicate refusal limits a member to 64 parameters, counting q, whatever the bound.
    let strict = rule
        .parser_limits(warded::negotiation::Limits::DEFAULT.max_params(u16::MAX))
        .invalid_is_error();
    assert!(outcome(&strict, "accept", Some(&offer)).unwrap());
    assert!(matches!(
        outcome(&strict, "accept", Some(&format!("{offer};q=1"))),
        Err(MatchError::InvalidHeader { .. })
    ));
}

#[test]
fn a_positive_encoding_wildcard_also_weights_an_unlisted_identity() {
    // RFC 9110 §12.5.3 specifies only that `*;q=0` excludes an unlisted identity. Applying a
    // positive `*` weight to it as well is Warded's documented policy.
    let offers = Offers::encodings(["identity", "gzip"]).unwrap();
    for (value, expected) in [
        ("gzip;q=0.8,*;q=0.4", "gzip"),
        ("gzip;q=0.4,*;q=0.8", "identity"),
        ("gzip;q=0.4,*;q=0.8,identity;q=0.1", "gzip"),
    ] {
        for offer in ["identity", "gzip"] {
            assert_eq!(
                outcome(
                    &offers.selected(offer).unwrap(),
                    "accept-encoding",
                    Some(value)
                )
                .unwrap(),
                offer == expected,
                "{value} {offer}"
            );
        }
    }
}
