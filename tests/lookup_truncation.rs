//! Lookup truncation never ends in a one-character subtag (RFC 4647 section 3.4).
//! The first and third cases failed before the correction for issue #21.

#![cfg(feature = "negotiation")]

use http::Request;
use warded::{Head, Matcher, negotiation::*};

fn matches(rule: &impl Matcher, accept_language: &str) -> bool {
    let request = Request::builder()
        .header("accept-language", accept_language)
        .body(())
        .unwrap();
    rule.evaluate(&Head::from_request(&request))
        .unwrap()
        .is_some()
}

#[test]
fn truncation_removes_every_trailing_one_character_subtag() {
    let lone = Offers::languages(["x-a"], LanguageMatching::Lookup).unwrap();
    assert!(!matches(&lone.selected("x-a").unwrap(), "x-a-b-cc"));
    let acceptable = accepts_language("x-a", LanguageMatching::Lookup).unwrap();
    assert!(!matches(&acceptable, "x-a-b-cc"));

    let offers = Offers::languages(["en-x-a", "en"], LanguageMatching::Lookup).unwrap();
    assert!(matches(&offers.selected("en").unwrap(), "en-x-a-b-cc"));
    assert!(!matches(&offers.selected("en-x-a").unwrap(), "en-x-a-b-cc"));
}

#[test]
fn the_rfc_private_use_example_still_truncates_to_its_private_subtag() {
    let offers = Offers::languages(
        ["zh-Hant-CN-x-private1", "zh-Hant-CN"],
        LanguageMatching::Lookup,
    )
    .unwrap();
    let first = offers.selected("zh-Hant-CN-x-private1").unwrap();
    assert!(matches(&first, "zh-Hant-CN-x-private1-private2"));
    let second = offers.selected("zh-Hant-CN").unwrap();
    assert!(matches(&second, "zh-Hant-CN-x-private2"));
}

#[test]
fn zero_quality_exclusions_reach_only_valid_truncations() {
    // The wildcard is the final default, so only an exclusion can stop `x-a`.
    let offers = Offers::languages(["x-a"], LanguageMatching::Lookup).unwrap();
    let selected = offers.selected("x-a").unwrap();
    assert!(matches(&selected, "x-a-b-cc;q=0,*"));
    // An exactly equal excluded range still applies.
    assert!(!matches(&selected, "x-a;q=0,*"));
}
