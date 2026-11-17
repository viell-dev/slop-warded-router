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
fn compiled_path_controls_hit_and_preserve_raw_captures() {
    let controls = [
        ("/fixed", "/fixed?ignored=1", &[][..]),
        (
            "/items/{*tail}",
            "/items/nested/raw%2Fpart",
            &[("tail", "nested/raw%2Fpart")][..],
        ),
        (
            "/{left}/{right}",
            "/left+value/right%2Fvalue",
            &[("left", "left+value"), ("right", "right%2Fvalue")][..],
        ),
    ];
    for (pattern, uri, expected) in controls {
        let request = Request::builder().uri(uri).body(()).unwrap();
        let captures = path(pattern)
            .unwrap()
            .evaluate(&Head::from_request(&request))
            .unwrap()
            .unwrap();
        assert_eq!(captures.iter().collect::<Vec<_>>(), expected, "{pattern}");
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
        let pattern = if prefix.is_empty() {
            format!("/{suffix}")
        } else {
            format!("{prefix}{suffix}")
        };
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
    // The one-byte name is the control for the escaped-suffix regressions.
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

#[test]
fn ordinary_parameter_boundaries_match_up_to_capacity() {
    for count in [24, 25, 26] {
        let pattern: String = (0..count)
            .map(|index| format!("/{{parameter{index}}}"))
            .collect();
        if count == 26 {
            let error: warded::DefinitionError = path(pattern).expect_err("26 parameters");
            assert!(error.to_string().contains("at most 25"));
            continue;
        }
        let uri: String = (0..count)
            .map(|index| format!("/value{index}%2Fraw"))
            .collect();
        let request = Request::builder().uri(uri).body(()).unwrap();
        let captures = path(pattern)
            .unwrap()
            .evaluate(&Head::from_request(&request))
            .unwrap()
            .unwrap();
        assert_eq!(captures.iter().count(), count);
        for index in 0..count {
            assert_eq!(
                captures.get(&format!("parameter{index}")),
                Some(format!("value{index}%2Fraw").as_str())
            );
        }
    }
}

#[cfg(feature = "negotiation")]
mod negotiation {
    use super::*;
    use std::{cmp::Reverse, ops::Range};
    use warded::{DefinitionError, MatchError, MatchResult, negotiation::*};

    /// Seed for grammar-guided valid fields, independent of the raw field stream.
    const GUIDED_SEED: u64 = 0x7761_7264_6564_0002;
    const GUIDED_CASES: usize = 256;
    /// Raw-field bounds at `check`'s widest byte bound, also used to classify their yield.
    const RAW_LIMITS: Limits = Limits::DEFAULT
        .max_field_bytes(256)
        .max_list_members(8)
        .max_params(4);

    fn check(rules: &[Negotiated], request: &Request<()>, bound: usize, bytes: usize) {
        let head = Head::from_request(request);
        let limits = RAW_LIMITS.max_field_bytes(bound);
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

    /// Strict outcomes for one request, classified as in the beta.1 yield measurement.
    #[derive(Clone, Copy, Debug, Default)]
    struct Yield {
        winner: usize,
        miss: usize,
        refusal: usize,
    }
    impl Yield {
        fn record(&mut self, outcomes: &[MatchResult]) {
            if outcomes.iter().any(Result::is_err) {
                self.refusal += 1;
            } else if outcomes
                .iter()
                .any(|outcome| matches!(outcome, Ok(Some(_))))
            {
                self.winner += 1;
            } else {
                self.miss += 1;
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
                "media",
                "accept",
                ["text/html", "application/json", "text/plain;level=1"]
                    .map(|value| media.selected(value).unwrap()),
            ),
            (
                "encoding",
                "accept-encoding",
                ["gzip", "br", "identity"].map(|value| encoding.selected(value).unwrap()),
            ),
            (
                "basic language",
                "accept-language",
                ["en", "fr-CA", "x-private"].map(|value| basic.selected(value).unwrap()),
            ),
            (
                "lookup language",
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
        // Absence is a separate control: every listed control is one present field line.
        for (_, _, rules) in &configurations {
            for bound in [0, 256] {
                check(rules, &Request::new(()), bound, 0);
            }
        }
        let mut yields = [[Yield::default(); 2]; 4];
        let mut generator = Generator(SEED);
        for case in 0..512 {
            let control = case < controls.len();
            let first = if control {
                controls[case].to_vec()
            } else {
                generator.field()
            };
            // Draw a second field for every case to retain the original raw sequence.
            let second = generator.field();
            // Only raw generated fields repeat, so no control depends on a random partner.
            let repeated = !control && case % 3 == 0;
            for (index, (_, name, rules)) in configurations.iter().enumerate() {
                let mut request = Request::new(());
                request
                    .headers_mut()
                    .append(*name, HeaderValue::from_bytes(&first).unwrap());
                if repeated {
                    request
                        .headers_mut()
                        .append(*name, HeaderValue::from_bytes(&second).unwrap());
                }
                let bytes = first.len() + if repeated { 1 + second.len() } else { 0 };
                let outcome = catch_unwind(AssertUnwindSafe(|| {
                    for bound in [0, bytes.saturating_sub(1), bytes, 256] {
                        check(rules, &request, bound, bytes);
                    }
                    let head = Head::from_request(&request);
                    rules
                        .iter()
                        .map(|rule| {
                            rule.clone()
                                .parser_limits(RAW_LIMITS)
                                .invalid_is_error()
                                .evaluate(&head)
                        })
                        .collect::<Vec<_>>()
                }));
                let Ok(outcomes) = outcome else {
                    panic!("seed={SEED:#x}, case={case}, request={request:?}");
                };
                yields[index][usize::from(!control)].record(&outcomes);
            }
        }
        for (index, (label, _, _)) in configurations.iter().enumerate() {
            for (group, name) in ["standalone controls", "generated raw"].iter().enumerate() {
                let Yield {
                    winner,
                    miss,
                    refusal,
                } = yields[index][group];
                println!(
                    "yield config={label}; group={name}; winner={winner}; miss={miss}; \
                     strict refusal={refusal}"
                );
            }
        }
    }

    fn assert_layout(
        name: &'static str,
        lines: &[&str],
        rules: &Rules,
        expected: Option<usize>,
        acceptable: &[bool],
    ) {
        let mut request = Request::new(());
        for line in lines {
            request
                .headers_mut()
                .append(name, HeaderValue::from_str(line).unwrap());
        }
        let head = Head::from_request(&request);
        for (index, rule) in rules.selected.iter().enumerate() {
            for rule in [rule.clone(), rule.clone().invalid_is_error()] {
                assert_eq!(
                    rule.evaluate(&head).map(|matched| matched.is_some()),
                    Ok(expected == Some(index)),
                    "{name}: {lines:?}, preferred offer {index}"
                );
            }
        }
        for (index, rule) in rules.acceptable.iter().enumerate() {
            for rule in [rule.clone(), rule.clone().invalid_is_error()] {
                assert_eq!(
                    rule.evaluate(&head).map(|matched| matched.is_some()),
                    Ok(acceptable[index]),
                    "{name}: {lines:?}, acceptable offer {index}"
                );
            }
        }
    }

    #[test]
    fn absent_empty_standalone_and_repeated_fields_have_explicit_outcomes() {
        let media = ["text/html", "application/json"];
        let languages = ["en", "fr-CA"];
        let encodings = ["gzip", "br", "identity"];
        let lookup = LanguageMatching::Lookup;
        let basic = LanguageMatching::BasicFilter;
        // (field, offers, empty-list choice and acceptability, value split into two lines,
        // the value's choice and acceptability)
        let configurations = [
            (
                "accept",
                Rules::media(&media),
                (None, vec![false, false]),
                ["text/html;q=0.5", "application/json"],
                (Some(1), vec![true, true]),
            ),
            (
                "accept-language",
                Rules::languages(&languages, lookup),
                (None, vec![false, false]),
                ["en;q=0.5", "fr-CA-x-private1"],
                (Some(1), vec![true, true]),
            ),
            (
                "accept-language",
                Rules::languages(&languages, basic),
                (None, vec![false, false]),
                ["en;q=0.5", "fr"],
                (Some(1), vec![true, true]),
            ),
            (
                // An empty Accept-Encoding field accepts only identity (RFC 9110 §12.5.3).
                "accept-encoding",
                Rules::encodings(&encodings),
                (Some(2), vec![false, false, true]),
                ["gzip;q=0.5", "br,identity;q=0.1"],
                (Some(1), vec![true, true, true]),
            ),
        ];
        for (name, rules, (empty, empty_acceptable), [left, right], (choice, acceptable)) in
            &configurations
        {
            let name = *name;
            let all = vec![true; rules.selected.len()];
            assert_layout(name, &[], rules, Some(0), &all);
            // Present without list members, including only empty elements or empty lines.
            for lines in [&[""][..], &[","], &[", ,"], &["", ""]] {
                assert_layout(name, lines, rules, *empty, empty_acceptable);
            }
            let joined = format!("{left},{right}");
            for lines in [
                &[joined.as_str()][..],
                &[*left, *right],
                &["", joined.as_str()],
                &[joined.as_str(), ""],
            ] {
                assert_layout(name, lines, rules, *choice, acceptable);
            }
        }
    }

    /// The preferred-offer predicates of one complete set and each offer's own acceptability.
    struct Rules {
        selected: Vec<Negotiated>,
        acceptable: Vec<Negotiated>,
    }
    impl Rules {
        fn new(
            set: Result<Offers, DefinitionError>,
            offers: &[&str],
            alone: impl Fn(&str) -> Result<Negotiated, DefinitionError>,
        ) -> Self {
            let set = set.unwrap();
            Self {
                selected: offers.iter().map(|o| set.selected(o).unwrap()).collect(),
                acceptable: offers.iter().map(|o| alone(o).unwrap()).collect(),
            }
        }
        fn media(offers: &[&str]) -> Self {
            let set = Offers::media_types(offers.iter().copied());
            Self::new(set, offers, accepts_media_type)
        }
        fn languages(offers: &[&str], mode: LanguageMatching) -> Self {
            let set = Offers::languages(offers.iter().copied(), mode);
            Self::new(set, offers, |offer| accepts_language(offer, mode))
        }
        fn encodings(offers: &[&str]) -> Self {
            let set = Offers::encodings(offers.iter().copied());
            Self::new(set, offers, accepts_encoding)
        }
    }

    // Grammar-guided valid fields. Each case is generated as structured members, rendered
    // with equivalent syntax (case, quoting, quoted-pairs, `q` position, optional whitespace,
    // empty list elements and line splits), and judged by a small oracle over the structure.
    // The oracle never parses the rendered field or calls Warded's or parse-net's selection.

    /// A generated case that neither RFC 9110 nor Warded's documentation decides.
    #[derive(Debug)]
    struct Undocumented;

    /// The index of the preferred offer, `None` when none is acceptable.
    type Verdict = Result<Option<usize>, Undocumented>;

    /// The highest positive weight wins; equal weights keep server offer order.
    fn preferred(weights: &[u16]) -> Option<usize> {
        let best = weights.iter().copied().max().filter(|&weight| weight > 0)?;
        weights.iter().position(|&weight| weight == best)
    }

    const COMMAS: &[&str] = &[",", ", ", " ,", ",\t"];
    const SEMICOLONS: &[&str] = &[";", " ;", "; ", "\t;\t"];

    fn pick<T: Copy>(generator: &mut Generator, items: &[T]) -> T {
        items[generator.next() % items.len()]
    }

    /// Render a case-insensitive token (type, parameter name, coding or language range).
    fn vary_case(generator: &mut Generator, value: &str) -> String {
        value
            .chars()
            .map(|character| {
                if generator.next().is_multiple_of(4) {
                    character.to_ascii_uppercase()
                } else {
                    character
                }
            })
            .collect()
    }

    /// A weight in thousandths and an optional `q` parameter in any valid qvalue form
    /// (RFC 9110 §12.4.2). An omitted weight is 1.
    fn weight(generator: &mut Generator) -> (u16, Option<String>) {
        let thousandths = pick(
            generator,
            &[1000, 1000, 1000, 900, 750, 500, 500, 100, 1, 0, 0],
        );
        if thousandths == 1000 && generator.next().is_multiple_of(2) {
            return (1000, None);
        }
        let value = match thousandths {
            1000 => pick(generator, &["1", "1.", "1.0", "1.000"]).to_owned(),
            0 => pick(generator, &["0", "0.", "0.0", "0.000"]).to_owned(),
            other => {
                let digits = format!("{other:03}");
                if generator.next().is_multiple_of(2) {
                    format!("0.{}", digits.trim_end_matches('0'))
                } else {
                    format!("0.{digits}")
                }
            }
        };
        let name = pick(generator, &["q", "Q"]);
        (thousandths, Some(format!("{name}={value}")))
    }

    /// Rendered members of one valid list, as one field line or split across two.
    struct Field {
        members: Vec<String>,
        separators: Vec<&'static str>,
        /// The most parameters, including `q`, on any member.
        parameters: usize,
    }
    impl Field {
        fn new(generator: &mut Generator, members: Vec<String>, parameters: usize) -> Self {
            let separators = (1..members.len())
                .map(|_| {
                    // Recipients ignore a reasonable number of empty elements (RFC 9110 §5.6.1).
                    if generator.next().is_multiple_of(8) {
                        ", ,"
                    } else {
                        pick(generator, COMMAS)
                    }
                })
                .collect();
            Self {
                members,
                separators,
                parameters,
            }
        }

        fn line(&self, members: Range<usize>) -> String {
            let mut line = String::new();
            for index in members.clone() {
                if index > members.start {
                    line.push_str(self.separators[index - 1]);
                }
                line.push_str(&self.members[index]);
            }
            line
        }
    }

    #[derive(Clone, Copy)]
    struct MediaOffer {
        registered: &'static str,
        type_: &'static str,
        subtype: &'static str,
        parameters: &'static [(&'static str, &'static str)],
    }
    const MEDIA_OFFERS: &[MediaOffer] = &[
        MediaOffer {
            registered: "text/html;level=1",
            type_: "text",
            subtype: "html",
            parameters: &[("level", "1")],
        },
        MediaOffer {
            registered: "text/html",
            type_: "text",
            subtype: "html",
            parameters: &[],
        },
        MediaOffer {
            registered: "application/json",
            type_: "application",
            subtype: "json",
            parameters: &[],
        },
        MediaOffer {
            registered: "text/plain;charset=utf-8;format=flowed",
            type_: "text",
            subtype: "plain",
            parameters: &[("charset", "utf-8"), ("format", "flowed")],
        },
        MediaOffer {
            registered: "application/ld+json;profile=\"a,b\"",
            type_: "application",
            subtype: "ld+json",
            parameters: &[("profile", "a,b")],
        },
    ];
    const MEDIA_ESSENCES: &[(&str, &str)] = &[
        ("*", "*"),
        ("text", "*"),
        ("application", "*"),
        ("text", "html"),
        ("text", "plain"),
        ("application", "json"),
        ("application", "ld+json"),
        ("image", "png"),
    ];
    const MEDIA_PARAMETERS: &[(&str, &[&str])] = &[
        ("level", &["1", "2"]),
        ("charset", &["utf-8", "iso-8859-1"]),
        ("format", &["flowed", "fixed"]),
        ("profile", &["a,b", "c"]),
    ];

    struct MediaRange {
        type_: &'static str,
        subtype: &'static str,
        parameters: MediaParameters,
        weight: u16,
    }

    /// A token or an equivalent quoted-string, sometimes with a quoted-pair (RFC 9110 §5.6.6).
    fn parameter_value(generator: &mut Generator, value: &str) -> String {
        if value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            && generator.next().is_multiple_of(2)
        {
            return value.to_owned();
        }
        let escaped = generator.next() % (value.len() + 1);
        let mut quoted = String::from('"');
        for (index, character) in value.chars().enumerate() {
            if index == escaped {
                quoted.push('\\');
            }
            quoted.push(character);
        }
        quoted.push('"');
        quoted
    }

    type MediaParameters = Vec<(&'static str, &'static str)>;

    /// Mostly ranges derived from an offer, so that wildcards, parameter subsets and
    /// contradicting or extra parameters compete for the same offers; otherwise any range.
    fn media_range(generator: &mut Generator) -> (&'static str, &'static str, MediaParameters) {
        let values = |name: &str| {
            MEDIA_PARAMETERS
                .iter()
                .find(|(known, _)| *known == name)
                .map_or(&[][..], |(_, values)| *values)
        };
        let mut chosen = Vec::new();
        if generator.next().is_multiple_of(4) {
            let (type_, subtype) = pick(generator, MEDIA_ESSENCES);
            let wanted = pick(generator, &[0, 0, 1, 1, 2, 3]);
            while chosen.len() < wanted {
                let (name, values) = pick(generator, MEDIA_PARAMETERS);
                if chosen.iter().all(|(existing, _)| *existing != name) {
                    chosen.push((name, pick(generator, values)));
                }
            }
            return (type_, subtype, chosen);
        }
        let offer = pick(generator, MEDIA_OFFERS);
        let (type_, subtype) = match generator.next() % 6 {
            0 => ("*", "*"),
            1 => (offer.type_, "*"),
            _ => (offer.type_, offer.subtype),
        };
        for &parameter in offer.parameters {
            if !generator.next().is_multiple_of(3) {
                chosen.push(parameter);
            }
        }
        match generator.next() % 6 {
            0 if !chosen.is_empty() => {
                let at = generator.next() % chosen.len();
                let (name, value) = chosen[at];
                let other = values(name).iter().find(|other| **other != value);
                chosen[at] = (name, other.copied().unwrap_or(value));
            }
            1 => {
                let (name, values) = pick(generator, MEDIA_PARAMETERS);
                if chosen.iter().all(|(existing, _)| *existing != name) {
                    chosen.push((name, pick(generator, values)));
                }
            }
            _ => {}
        }
        (type_, subtype, chosen)
    }

    fn media_field(generator: &mut Generator) -> (Vec<MediaRange>, Field) {
        let count = 1 + generator.next() % 6;
        let (mut ranges, mut members, mut parameters) = (Vec::<MediaRange>::new(), Vec::new(), 0);
        while ranges.len() < count {
            let (type_, subtype, chosen) = media_range(generator);
            let (weight, q) = weight(generator);
            // Duplicate ranges have no documented precedence; keep members distinct.
            let key = |parameters: &[(&'static str, &'static str)]| {
                let mut sorted = parameters.to_vec();
                sorted.sort_unstable();
                sorted
            };
            if ranges.iter().any(|range| {
                (range.type_, range.subtype) == (type_, subtype)
                    && key(&range.parameters) == key(&chosen)
            }) {
                continue;
            }
            let mut rendered: Vec<String> = chosen
                .iter()
                .map(|&(name, value)| {
                    // Charset values are case-insensitive (RFC 9110 §8.3.2); others stay exact.
                    let value = if name == "charset" {
                        vary_case(generator, value)
                    } else {
                        value.to_owned()
                    };
                    let name = vary_case(generator, name);
                    format!("{name}={}", parameter_value(generator, &value))
                })
                .collect();
            // Recipients read `q` as the weight wherever it appears (RFC 9110 §12.5.1).
            if let Some(q) = q {
                let at = generator.next() % (rendered.len() + 1);
                rendered.insert(at, q);
            }
            parameters = parameters.max(rendered.len());
            let mut member = format!(
                "{}/{}",
                vary_case(generator, type_),
                vary_case(generator, subtype)
            );
            for parameter in rendered {
                member.push_str(pick(generator, SEMICOLONS));
                member.push_str(&parameter);
            }
            members.push(member);
            ranges.push(MediaRange {
                type_,
                subtype,
                parameters: chosen,
                weight,
            });
        }
        (ranges, Field::new(generator, members, parameters))
    }

    /// RFC 9110 §12.5.1: the most specific applicable range supplies an offer's weight. A
    /// range applies when its type and subtype are equal or `*` and the offer has each of its
    /// parameters. Values are compared after the rendering's case and quoting variation has
    /// been removed. Specificity compares wildcards, then the parameter count, as Warded
    /// documents; equally specific ranges with different weights are not decided.
    fn media_oracle(ranges: &[MediaRange], offers: &[MediaOffer]) -> Verdict {
        let specificity = |range: &MediaRange| {
            (
                usize::from(range.type_ != "*") + usize::from(range.subtype != "*"),
                range.parameters.len(),
            )
        };
        let mut weights = Vec::new();
        for offer in offers {
            let applicable: Vec<&MediaRange> = ranges
                .iter()
                .filter(|range| {
                    (range.type_ == "*" || range.type_ == offer.type_)
                        && (range.subtype == "*" || range.subtype == offer.subtype)
                        && range
                            .parameters
                            .iter()
                            .all(|parameter| offer.parameters.contains(parameter))
                })
                .collect();
            let Some(best) = applicable.iter().map(|range| specificity(range)).max() else {
                weights.push(0);
                continue;
            };
            let mut precedent = applicable
                .iter()
                .filter(|range| specificity(range) == best)
                .map(|range| range.weight);
            let weight = precedent.next().unwrap_or(0);
            if precedent.any(|other| other != weight) {
                return Err(Undocumented);
            }
            weights.push(weight);
        }
        Ok(preferred(&weights))
    }

    /// One member of a weighted token list (Accept-Encoding, Accept-Language).
    struct Weighted {
        value: &'static str,
        weight: u16,
    }

    fn weighted_field(
        generator: &mut Generator,
        pool: &[&'static str],
        most: usize,
        same: fn(&str, &str) -> bool,
    ) -> (Vec<Weighted>, Field) {
        let count = 1 + generator.next() % most;
        let (mut list, mut members, mut parameters) = (Vec::<Weighted>::new(), Vec::new(), 0);
        while list.len() < count {
            let value = pick(generator, pool);
            let (weight, q) = weight(generator);
            if list.iter().any(|existing| same(existing.value, value)) {
                continue;
            }
            let mut member = vary_case(generator, value);
            if let Some(q) = q {
                member.push_str(pick(generator, SEMICOLONS));
                member.push_str(&q);
                parameters = 1;
            }
            members.push(member);
            list.push(Weighted { value, weight });
        }
        (list, Field::new(generator, members, parameters))
    }

    const CODINGS: &[&str] = &["gzip", "x-gzip", "br", "zstd", "deflate", "identity", "*"];
    const ENCODING_OFFERS: &[&str] = &["gzip", "br", "zstd", "identity"];

    /// Recipients consider `x-gzip` equivalent to `gzip` (RFC 9110 §8.4.1.3).
    fn coding(value: &str) -> &str {
        if value == "x-gzip" { "gzip" } else { value }
    }

    /// RFC 9110 §12.5.3: a listed coding has its weight and `*` matches codings not listed,
    /// including identity. Otherwise identity is acceptable by default; the RFC gives that
    /// default no weight, so it decides a choice only when no other offer is acceptable.
    fn encoding_oracle(codings: &[Weighted], offers: &[&str]) -> Verdict {
        let star = codings
            .iter()
            .find(|member| member.value == "*")
            .map(|member| member.weight);
        let (mut weights, mut default) = (Vec::new(), None);
        for (index, offer) in offers.iter().enumerate() {
            let listed = codings
                .iter()
                .find(|member| coding(member.value) == *offer)
                .map(|member| member.weight);
            weights.push(match listed.or(star) {
                Some(weight) => weight,
                None if *offer == "identity" => {
                    default = Some(index);
                    0
                }
                None => 0,
            });
        }
        match (default, preferred(&weights)) {
            (Some(identity), None) => Ok(Some(identity)),
            (Some(_), Some(_)) => Err(Undocumented),
            (None, choice) => Ok(choice),
        }
    }

    const LANGUAGE_RANGES: &[&str] = &[
        "*",
        "en",
        "en-gb",
        "en-us",
        "en-gb-oxendict",
        "en-gb-u-ca-gregory",
        "fr",
        "fr-ca",
        "fr-fr",
        "de",
        "de-ch",
        "de-ch-1996",
        "zh",
        "zh-hant",
        "zh-hant-cn",
        "zh-hant-cn-x-private1",
        "x-private",
        "es-419",
    ];
    const LANGUAGE_OFFERS: &[&str] =
        &["en", "en-GB", "fr-CA", "de-CH-1996", "zh-Hant", "x-private"];

    fn language_field(generator: &mut Generator) -> (Vec<Weighted>, Field) {
        weighted_field(generator, LANGUAGE_RANGES, 6, |a, b| a == b)
    }

    /// RFC 4647 §3.4: the range itself, then shorter prefixes; a single-character subtag is
    /// removed together with its closest trailing subtag.
    fn truncations(range: &str) -> Vec<String> {
        let subtags: Vec<&str> = range.split('-').collect();
        let mut candidates = vec![range.to_owned()];
        for end in (1..subtags.len()).rev() {
            if subtags[end - 1].len() > 1 {
                candidates.push(subtags[..end].join("-"));
            }
        }
        candidates
    }

    /// How specifically a lookup range reaches an offer.
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Reach {
        Wildcard,
        Truncated,
        Exact,
    }

    fn reach(range: &str, offer: &str) -> Option<Reach> {
        if range == "*" {
            Some(Reach::Wildcard)
        } else if range.eq_ignore_ascii_case(offer) {
            Some(Reach::Exact)
        } else {
            truncations(range)
                .iter()
                .skip(1)
                .any(|candidate| candidate.eq_ignore_ascii_case(offer))
                .then_some(Reach::Truncated)
        }
    }

    /// Lookup as Warded documents it: positive ranges by descending weight, then client order,
    /// each truncated completely (RFC 4647 §3.4) before the next; a positive `*` then selects
    /// the first remaining offer. A zero-weight range excludes an offer that it reaches at
    /// least as specifically as the candidate does (exact, then truncated, then wildcard).
    fn lookup_oracle(ranges: &[Weighted], offers: &[&str]) -> Verdict {
        let excluded = |offer: &str, candidate: Reach| {
            ranges.iter().any(|range| {
                range.weight == 0 && reach(range.value, offer).is_some_and(|zero| zero >= candidate)
            })
        };
        let mut positive: Vec<&Weighted> = ranges.iter().filter(|range| range.weight > 0).collect();
        positive.sort_by_key(|range| Reverse(range.weight));
        for range in positive.iter().filter(|range| range.value != "*") {
            for (step, candidate) in truncations(range.value).iter().enumerate() {
                let level = if step == 0 {
                    Reach::Exact
                } else {
                    Reach::Truncated
                };
                if let Some(index) = offers
                    .iter()
                    .position(|offer| offer.eq_ignore_ascii_case(candidate))
                    && !excluded(offers[index], level)
                {
                    return Ok(Some(index));
                }
            }
        }
        if positive.iter().any(|range| range.value == "*") {
            return Ok(offers
                .iter()
                .position(|offer| !excluded(offer, Reach::Wildcard)));
        }
        Ok(None)
    }

    /// RFC 4647 §3.3.1 matching, weighted as HTTP has done since RFC 2616 §14.4: the longest
    /// matching range supplies a tag's weight, and `*` covers only tags no range matches.
    fn basic_oracle(ranges: &[Weighted], offers: &[&str]) -> Verdict {
        let weights: Vec<u16> = offers
            .iter()
            .map(|offer| {
                let prefix = |range: &str| {
                    range.eq_ignore_ascii_case(offer)
                        || (offer.len() > range.len()
                            && offer.as_bytes()[range.len()] == b'-'
                            && offer[..range.len()].eq_ignore_ascii_case(range))
                };
                ranges
                    .iter()
                    .filter(|range| range.value != "*" && prefix(range.value))
                    .max_by_key(|range| range.value.len())
                    .or_else(|| ranges.iter().find(|range| range.value == "*"))
                    .map_or(0, |range| range.weight)
            })
            .collect();
        Ok(preferred(&weights))
    }

    /// One configuration of the guided corpus.
    struct Guided<X, T: 'static> {
        label: &'static str,
        name: &'static str,
        offers: &'static [T],
        generate: fn(&mut Generator) -> (Vec<X>, Field),
        oracle: fn(&[X], &[T]) -> Verdict,
        rules: Rules,
    }

    /// Totals for one configuration: outcomes, rejected draws and boundary refusals checked.
    struct GuidedYield {
        label: &'static str,
        outcomes: Yield,
        undocumented: usize,
        boundaries: usize,
    }

    fn run_guided<X, T: 'static>(generator: &mut Generator, config: Guided<X, T>) -> GuidedYield {
        let mut totals = GuidedYield {
            label: config.label,
            outcomes: Yield::default(),
            undocumented: 0,
            boundaries: 0,
        };
        for case in 0..GUIDED_CASES {
            let (field, expected, acceptable) = loop {
                let (members, field) = (config.generate)(generator);
                let choice = (config.oracle)(&members, config.offers);
                let alone: Result<Vec<bool>, Undocumented> = config
                    .offers
                    .iter()
                    .map(|offer| {
                        (config.oracle)(&members, std::slice::from_ref(offer))
                            .map(|chosen| chosen.is_some())
                    })
                    .collect();
                if let (Ok(choice), Ok(alone)) = (choice, alone) {
                    break (field, choice, alone);
                }
                totals.undocumented += 1;
                assert!(
                    totals.undocumented <= 2 * GUIDED_CASES,
                    "{}: the oracle rarely decides",
                    config.label
                );
            };
            let count = field.members.len();
            let mut layouts = vec![vec![field.line(0..count)]];
            if count > 1 {
                let at = 1 + generator.next() % (count - 1);
                layouts.push(vec![field.line(0..at), field.line(at..count)]);
            }
            for lines in layouts {
                let mut request = Request::new(());
                for line in &lines {
                    request
                        .headers_mut()
                        .append(config.name, HeaderValue::from_str(line).unwrap());
                }
                let bounds = Bounds {
                    bytes: lines.iter().map(String::len).sum::<usize>() + lines.len() - 1,
                    members: count,
                    parameters: field.parameters,
                };
                let outcome = catch_unwind(AssertUnwindSafe(|| {
                    assert_guided(&request, &config.rules, expected, &acceptable, &bounds)
                }));
                let Ok(boundaries) = outcome else {
                    panic!(
                        "seed={GUIDED_SEED:#x}, config={}, case={case}, lines={lines:?}, \
                         expected={expected:?}, acceptable={acceptable:?}",
                        config.label
                    );
                };
                totals.boundaries += boundaries;
            }
            if expected.is_some() {
                totals.outcomes.winner += 1;
            } else {
                totals.outcomes.miss += 1;
            }
        }
        totals
    }

    /// The exact size of a valid field: combined bytes, nonempty members and parameters.
    struct Bounds {
        bytes: usize,
        members: usize,
        parameters: usize,
    }

    /// Assert the oracle's choice under default and exactly sufficient limits, on cached and
    /// fresh heads, each offer's own acceptability, and a refusal one below each bound.
    fn assert_guided(
        request: &Request<()>,
        rules: &Rules,
        expected: Option<usize>,
        acceptable: &[bool],
        bounds: &Bounds,
    ) -> usize {
        let count = |value: usize| u16::try_from(value).unwrap();
        let head = Head::from_request(request);
        let exact = Limits::DEFAULT
            .max_field_bytes(bounds.bytes)
            .max_list_members(count(bounds.members))
            .max_params(count(bounds.parameters));
        for limits in [Limits::DEFAULT, exact] {
            for (index, rule) in rules.selected.iter().enumerate() {
                let rule = rule.clone().parser_limits(limits);
                let wanted = Ok(expected == Some(index));
                let strict = rule.clone().invalid_is_error();
                assert_eq!(
                    strict.evaluate(&head).map(|m| m.is_some()),
                    wanted,
                    "{index}"
                );
                let fresh = Head::from_request(request);
                assert_eq!(
                    rule.evaluate(&fresh).map(|m| m.is_some()),
                    wanted,
                    "{index}"
                );
            }
        }
        for (index, rule) in rules.acceptable.iter().enumerate() {
            let strict = rule.clone().invalid_is_error();
            assert_eq!(
                strict.evaluate(&head).map(|m| m.is_some()),
                Ok(acceptable[index]),
                "acceptable offer {index}"
            );
        }
        let below = [
            bounds
                .bytes
                .checked_sub(1)
                .map(|bound| Limits::DEFAULT.max_field_bytes(bound)),
            bounds
                .members
                .checked_sub(1)
                .map(|bound| Limits::DEFAULT.max_list_members(count(bound))),
            bounds
                .parameters
                .checked_sub(1)
                .map(|bound| Limits::DEFAULT.max_params(count(bound))),
        ];
        let mut refused = 0;
        for limits in below.into_iter().flatten() {
            let rule = rules.selected[expected.unwrap_or(0)]
                .clone()
                .parser_limits(limits);
            assert!(
                matches!(
                    rule.clone().invalid_is_error().evaluate(&head),
                    Err(MatchError::InvalidHeader { .. })
                ),
                "{limits:?}"
            );
            assert_eq!(rule.evaluate(&head), Ok(None), "{limits:?}");
            refused += 1;
        }
        refused
    }

    #[test]
    fn grammar_guided_valid_fields_select_independently_expected_offers() {
        let mut generator = Generator(GUIDED_SEED);
        let media: Vec<&str> = MEDIA_OFFERS.iter().map(|offer| offer.registered).collect();
        let totals = [
            run_guided(
                &mut generator,
                Guided {
                    label: "media",
                    name: "accept",
                    offers: MEDIA_OFFERS,
                    generate: media_field,
                    oracle: media_oracle,
                    rules: Rules::media(&media),
                },
            ),
            run_guided(
                &mut generator,
                Guided {
                    label: "encoding",
                    name: "accept-encoding",
                    offers: ENCODING_OFFERS,
                    generate: |generator| {
                        weighted_field(generator, CODINGS, 5, |a, b| coding(a) == coding(b))
                    },
                    oracle: encoding_oracle,
                    rules: Rules::encodings(ENCODING_OFFERS),
                },
            ),
            run_guided(
                &mut generator,
                Guided {
                    label: "basic language",
                    name: "accept-language",
                    offers: LANGUAGE_OFFERS,
                    generate: language_field,
                    oracle: basic_oracle,
                    rules: Rules::languages(LANGUAGE_OFFERS, LanguageMatching::BasicFilter),
                },
            ),
            run_guided(
                &mut generator,
                Guided {
                    label: "lookup language",
                    name: "accept-language",
                    offers: LANGUAGE_OFFERS,
                    generate: language_field,
                    oracle: lookup_oracle,
                    rules: Rules::languages(LANGUAGE_OFFERS, LanguageMatching::Lookup),
                },
            ),
        ];
        for GuidedYield {
            label,
            outcomes,
            undocumented,
            boundaries,
        } in totals
        {
            let Yield {
                winner,
                miss,
                refusal,
            } = outcomes;
            println!(
                "yield config={label}; group=grammar-guided; winner={winner}; miss={miss}; \
                 strict refusal={refusal}; undecided draws={undocumented}; \
                 boundary refusals={boundaries}"
            );
            // Keep both outcomes common so the generator continues to exercise selection.
            assert!(
                winner >= GUIDED_CASES / 2 && miss >= GUIDED_CASES / 64,
                "{label}: winner={winner}, miss={miss}"
            );
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
