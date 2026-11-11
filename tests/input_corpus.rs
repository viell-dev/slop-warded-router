//! Deterministic request-input corpus. Failure messages retain seed and case.
//! This complements protocol examples; it is not exhaustive fuzzing or a parser oracle.

use http::{Request, header::HeaderValue};
use std::panic::{AssertUnwindSafe, catch_unwind};
use warded::{Head, Matcher, matchers::*};

const SEED: u64 = 0x7761_7264_6564_0001;

struct Generator(u64);
impl Generator {
    fn next(&mut self) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        // Keep the sequence identical on 32- and 64-bit hosts.
        (self.0 & 0xffff_ffff) as usize
    }

    fn text(&mut self, alphabet: &[u8], max: usize) -> String {
        let len = self.next() % (max + 1);
        (0..len)
            .map(|_| char::from(alphabet[self.next() % alphabet.len()]))
            .collect()
    }

    fn field(&mut self) -> Vec<u8> {
        let len = self.next() % 97;
        (0..len)
            .map(|_| match self.next() % 5 {
                0 => 0x80 + (self.next() % 128) as u8,
                1 => {
                    let punctuation = b" ,;=\t\"\\*-/q.0129";
                    punctuation[self.next() % punctuation.len()]
                }
                _ => b'a' + (self.next() % 26) as u8,
            })
            .collect()
    }
}

#[test]
fn seeded_raw_headers_queries_and_compiled_paths_preserve_their_contracts() {
    let mut generator = Generator(SEED);
    let paths = ["/fixed", "/items/{id}", "/items/{*tail}", "/{left}/{right}"]
        .map(|pattern| path(pattern).unwrap());
    for case in 0..1024 {
        let raw = generator.field();
        let id = format!("v{}", generator.text(b"abZ09%2F+_-.@:", 48));
        let query_value = generator.text(b"az09%2B+=/?:", 48);
        let present = case % 3;
        let query = match present {
            0 => "other=needle&targeted=needle".to_owned(),
            1 => "other=needle&target".to_owned(),
            _ => format!("other=needle&target={query_value}&target=second"),
        };
        let uri = format!("/items/{id}?{query}");
        let mut request = Request::builder().uri(uri).body(()).unwrap();
        if case % 4 != 0 {
            request
                .headers_mut()
                .append("x-raw", HeaderValue::from_bytes(&raw).unwrap());
        }
        if case % 4 == 2 {
            request
                .headers_mut()
                .append("x-raw", HeaderValue::from_bytes(&raw).unwrap());
        }
        let arbitrary_path = format!("/{}", generator.text(b"a09%2F_-/+:.@", 128));
        let arbitrary = Request::builder().uri(arbitrary_path).body(()).unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let head = Head::from_request(&request);
            assert_eq!(
                header_present("X-RAW")
                    .unwrap()
                    .evaluate(&head)
                    .unwrap()
                    .is_some(),
                case % 4 != 0
            );
            assert_eq!(
                header_any_eq("x-raw", &raw)
                    .unwrap()
                    .evaluate(&head)
                    .unwrap()
                    .is_some(),
                case % 4 != 0
            );
            assert_eq!(
                header_eq("x-raw", &raw)
                    .unwrap()
                    .evaluate(&head)
                    .unwrap()
                    .is_some(),
                matches!(case % 4, 1 | 3)
            );
            assert_eq!(
                query_present("target").evaluate(&head).unwrap().is_some(),
                present != 0
            );
            assert_eq!(
                query_pair("target", &query_value)
                    .evaluate(&head)
                    .unwrap()
                    .is_some(),
                present == 2
            );
            let captures = paths[1].evaluate(&head).unwrap().unwrap();
            assert_eq!(captures.get("id"), Some(id.as_str()));
            assert_eq!(captures.iter().count(), 1);
            for matcher in &paths {
                let head = Head::from_request(&arbitrary);
                assert_eq!(matcher.evaluate(&head), matcher.evaluate(&head.clone()));
                assert!(matcher.evaluate(&head).is_ok());
            }
        }));
        assert!(
            result.is_ok(),
            "seed={SEED:#x}, case={case}, raw={raw:?}, request={request:?}, arbitrary={arbitrary:?}"
        );
    }
}

#[test]
fn generated_pattern_boundaries_return_a_matcher_or_definition_error() {
    let mut generator = Generator(SEED);
    for case in 0..1024 {
        let count = [0, 1, 24, 25, 26][generator.next() % 5];
        let prefix: String = (0..count)
            .map(|index| format!("/{{parameter{index}}}"))
            .collect();
        let suffix = generator.text(b"{}ab/*", 32);
        let pattern = format!("/{prefix}{suffix}");
        let outcome = catch_unwind(|| path(&pattern));
        assert!(
            outcome.is_ok(),
            "seed={SEED:#x}, case={case}, pattern={pattern:?}"
        );
        if let Ok(matcher) = outcome.unwrap() {
            let request = Request::builder().uri("/value/{a}/tail").body(()).unwrap();
            assert!(
                matcher.evaluate(&Head::from_request(&request)).is_ok(),
                "{pattern:?}"
            );
        }
    }
    // Verified configuration workaround for dependency issue #13. Do not turn
    // the valid longer-name pattern's rejection into an intended contract.
    let matcher = path("/{a}{{x}}").unwrap();
    let request = Request::builder().uri("/value{x}").body(()).unwrap();
    assert_eq!(
        matcher
            .evaluate(&Head::from_request(&request))
            .unwrap()
            .unwrap()
            .get("a"),
        Some("value")
    );
}

#[cfg(feature = "negotiation")]
mod negotiation {
    use super::*;
    use warded::{MatchError, negotiation::*};

    fn check(rules: &[Negotiated], request: &Request<()>, bound: usize, bytes: usize) {
        let head = Head::from_request(request);
        let limits = Limits::DEFAULT
            .max_field_bytes(bound)
            .max_list_members(8)
            .max_params(4);
        let results: Vec<_> = rules
            .iter()
            .map(|rule| {
                rule.clone()
                    .parser_limits(limits)
                    .invalid_is_error()
                    .evaluate(&head)
            })
            .collect();
        assert!(
            results
                .iter()
                .filter(|result| matches!(result, Ok(Some(_))))
                .count()
                <= 1
        );
        for (index, rule) in rules.iter().enumerate().rev() {
            let strict = rule.clone().parser_limits(limits).invalid_is_error();
            assert_eq!(strict.evaluate(&head), results[index]);
            assert_eq!(strict.evaluate(&head.clone()), results[index]);
            assert_eq!(
                strict.evaluate(&Head::from_request(request)),
                results[index]
            );
            let ordinary = rule.clone().parser_limits(limits).evaluate(&head);
            match &results[index] {
                Err(MatchError::InvalidHeader { .. }) => assert_eq!(ordinary, Ok(None)),
                Err(other) => panic!("unexpected negotiation error: {other}"),
                valid => assert_eq!(&ordinary, valid),
            }
            if bytes > bound {
                assert!(matches!(
                    results[index],
                    Err(MatchError::InvalidHeader { .. })
                ));
            }
        }
    }

    #[test]
    fn seeded_accept_fields_are_bounded_policy_explicit_and_cache_consistent() {
        let media =
            Offers::media_types(["text/html", "application/json", "text/plain;level=1"]).unwrap();
        let encoding = Offers::encodings(["gzip", "br", "identity"]).unwrap();
        let basic =
            Offers::languages(["en", "fr-CA", "x-private"], LanguageMatching::BasicFilter).unwrap();
        let lookup =
            Offers::languages(["en", "fr-CA", "x-private"], LanguageMatching::Lookup).unwrap();
        let configurations = [
            (
                "accept",
                ["text/html", "application/json", "text/plain;level=1"]
                    .map(|value| media.selected(value).unwrap()),
            ),
            (
                "accept-encoding",
                ["gzip", "br", "identity"].map(|value| encoding.selected(value).unwrap()),
            ),
            (
                "accept-language",
                ["en", "fr-CA", "x-private"].map(|value| basic.selected(value).unwrap()),
            ),
            (
                "accept-language",
                ["en", "fr-CA", "x-private"].map(|value| lookup.selected(value).unwrap()),
            ),
        ];
        let controls: &[&[u8]] = &[
            b"",
            b"*",
            b"*/*",
            b"text/html",
            b"text/html;q=0,*/*;q=1",
            b"text/plain;level=1;q=0.5",
            b"text/html;profile=\"a,b\"",
            b"en",
            b"fr-CA,en;q=0.5",
            b"en;q=0,*;q=1",
            b"gzip",
            b"gzip;q=0,*;q=1",
            b"text/html;q=2",
            b"en;q=.5",
            b"gzip;q=0.1234",
            b"\xff",
            b"\t\"\\",
        ];
        let mut generator = Generator(SEED);
        for case in 0..512 {
            let first = if case < controls.len() {
                controls[case].to_vec()
            } else {
                generator.field()
            };
            let second = generator.field();
            for (name, rules) in &configurations {
                let mut request = Request::new(());
                if case != 0 {
                    request
                        .headers_mut()
                        .append(*name, HeaderValue::from_bytes(&first).unwrap());
                }
                let repeated = case != 0 && case % 3 == 0;
                if repeated {
                    request
                        .headers_mut()
                        .append(*name, HeaderValue::from_bytes(&second).unwrap());
                }
                let bytes = if case == 0 {
                    0
                } else {
                    first.len() + if repeated { 1 + second.len() } else { 0 }
                };
                let outcome = catch_unwind(AssertUnwindSafe(|| {
                    for bound in [0, bytes.saturating_sub(1), bytes, 256] {
                        check(rules, &request, bound, bytes);
                    }
                }));
                assert!(
                    outcome.is_ok(),
                    "seed={SEED:#x}, case={case}, request={request:?}"
                );
            }
        }
    }

    #[test]
    fn generated_valid_member_and_parameter_counts_respect_boundaries() {
        for count in 1..=12 {
            for (name, rule, member) in [
                (
                    "accept",
                    accepts_media_type("text/html").unwrap(),
                    "text/html",
                ),
                (
                    "accept-language",
                    accepts_language("en", LanguageMatching::BasicFilter).unwrap(),
                    "en",
                ),
                ("accept-encoding", accepts_encoding("gzip").unwrap(), "gzip"),
            ] {
                let value = vec![member; usize::from(count)].join(",");
                let request = Request::builder().header(name, value).body(()).unwrap();
                for limit in [count - 1, count, count + 1] {
                    let strict = rule
                        .clone()
                        .parser_limits(Limits::DEFAULT.max_list_members(limit))
                        .invalid_is_error();
                    let outcome = strict.evaluate(&Head::from_request(&request));
                    if limit < count {
                        assert!(
                            matches!(outcome, Err(MatchError::InvalidHeader { .. })),
                            "{name}, {count}, {limit}"
                        );
                    } else {
                        assert!(outcome.unwrap().is_some(), "{name}, {count}, {limit}");
                    }
                }
            }
            let parameters: String = (0..count).map(|index| format!(";p{index}=v")).collect();
            let request = Request::builder()
                .header("accept", format!("text/html{parameters}"))
                .body(())
                .unwrap();
            for limit in [count - 1, count, count + 1] {
                let rule = accepts_media_type("text/html")
                    .unwrap()
                    .parser_limits(Limits::DEFAULT.max_params(limit))
                    .invalid_is_error();
                let outcome = rule.evaluate(&Head::from_request(&request));
                assert_eq!(
                    outcome.is_err(),
                    limit < count,
                    "params={count}, limit={limit}"
                );
            }
        }
    }
}
