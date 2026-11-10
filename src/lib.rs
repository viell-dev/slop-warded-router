//! Isolated API and cost probes; no production Warded API is changed.

use http::Request;
use std::str::FromStr;
use warded::{Captures, Head, MatchResult, Matcher, Selector, matchers::*};

/// Prototype extension: refine successful captures using an ordinary predicate.
pub trait Refine: Matcher + Sized {
    fn filter<F: Fn(&Captures) -> bool>(self, predicate: F) -> Filter<Self, F> {
        Filter {
            inner: self,
            predicate,
        }
    }
}
impl<M: Matcher> Refine for M {}

#[derive(Clone)]
pub struct Filter<M, F> {
    inner: M,
    predicate: F,
}
impl<M: Matcher, F: Fn(&Captures) -> bool> Matcher for Filter<M, F> {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        Ok(self
            .inner
            .evaluate(head)?
            .filter(|captures| (self.predicate)(captures)))
    }
}

/// Fixture policy, not a universal ID grammar: decimal u64 without a leading sign.
pub fn parse_id(raw: &str) -> Option<u64> {
    (!raw.is_empty() && raw.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| raw.parse().ok())
        .flatten()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserId(pub u64);

#[derive(Debug)]
pub struct InvalidId;
impl FromStr for UserId {
    type Err = InvalidId;
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        parse_id(raw).map(Self).ok_or(InvalidId)
    }
}

/// Fixture policy: ASCII letter first, then ASCII alphanumeric or underscore.
pub fn is_username(raw: &str) -> bool {
    let mut bytes = raw.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[derive(Clone, Copy, Debug)]
pub enum Approach {
    Predicate,
    NewtypePredicate,
    #[cfg(feature = "regex-candidate")]
    Regex,
}
impl Approach {
    pub fn name(self) -> &'static str {
        match self {
            Self::Predicate => "predicate",
            Self::NewtypePredicate => "newtype_predicate",
            #[cfg(feature = "regex-candidate")]
            Self::Regex => "regex_and_range_check",
        }
    }
    pub fn all() -> Vec<Self> {
        vec![
            Self::Predicate,
            Self::NewtypePredicate,
            #[cfg(feature = "regex-candidate")]
            Self::Regex,
        ]
    }
}

#[cfg(feature = "regex-candidate")]
struct RegexPath {
    expression: regex::Regex,
    name: &'static str,
}
#[cfg(feature = "regex-candidate")]
impl Matcher for RegexPath {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        let Some(found) = self.expression.captures(head.uri().path()) else {
            return Ok(None);
        };
        let raw = found
            .get(1)
            .expect("fixture regex has one capture")
            .as_str();
        // Digits alone are insufficient: preserve the same u64 overflow rejection.
        if self.name == "id" && parse_id(raw).is_none() {
            return Ok(None);
        }
        let mut captures = Captures::default();
        captures.insert(self.name, raw)?;
        Ok(Some(captures))
    }
}

pub struct Routes {
    selector: Selector<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Output {
    pub route: u8,
    pub captures: Captures,
    pub handler_value: u64,
}

impl Routes {
    pub fn new(approach: Approach) -> Self {
        let selector = Selector::new().route(
            get()
                .and(exact_path("/users/new"))
                .and(header_eq("x-special", "yes").unwrap()),
            0,
        );
        let selector = match approach {
            Approach::Predicate => selector
                .route(
                    get().and(path("/users/{id}").unwrap().filter(|captures| {
                        captures
                            .get("id")
                            .is_some_and(|raw| parse_id(raw).is_some())
                    })),
                    1,
                )
                .route(
                    get().and(
                        path("/users/{username}")
                            .unwrap()
                            .filter(|captures| captures.get("username").is_some_and(is_username)),
                    ),
                    2,
                ),
            Approach::NewtypePredicate => selector
                .route(
                    get().and(path("/users/{id}").unwrap().filter(|captures| {
                        captures
                            .get("id")
                            .is_some_and(|raw| raw.parse::<UserId>().is_ok())
                    })),
                    1,
                )
                .route(
                    get().and(
                        path("/users/{username}")
                            .unwrap()
                            .filter(|captures| captures.get("username").is_some_and(is_username)),
                    ),
                    2,
                ),
            #[cfg(feature = "regex-candidate")]
            Approach::Regex => selector
                .route(
                    get().and(RegexPath {
                        expression: regex::Regex::new(r"\A/users/([0-9]+)\z").unwrap(),
                        name: "id",
                    }),
                    1,
                )
                .route(
                    get().and(RegexPath {
                        expression: regex::Regex::new(r"\A/users/([A-Za-z][A-Za-z0-9_]*)\z")
                            .unwrap(),
                        name: "username",
                    }),
                    2,
                ),
        };
        Self { selector }
    }

    pub fn execute(&self, request: &Request<()>) -> Option<Output> {
        let selected = self
            .selector
            .select(&Head::from_request(request))
            .unwrap()?;
        let route = *selected.target;
        let captures = selected.selection.captures;
        let handler_value = match route {
            // All approaches produce the same raw output. Include handler-side ID parsing.
            1 => captures.get("id").unwrap().parse::<UserId>().unwrap().0,
            2 => captures
                .get("username")
                .unwrap()
                .bytes()
                .fold(0_u64, |sum, byte| sum.wrapping_add(u64::from(byte))),
            _ => 0,
        };
        Some(Output {
            route,
            captures,
            handler_value,
        })
    }
}

pub fn request(uri: &str) -> Request<()> {
    Request::builder().uri(uri).body(()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use warded::{MatchError, try_match_fn};

    #[test]
    fn approaches_agree_on_boundaries_and_selected_values() {
        let reference = Routes::new(Approach::Predicate);
        for approach in Approach::all() {
            let routes = Routes::new(approach);
            for (uri, expected) in [
                ("/users/42", Some(1)),
                ("/users/00042?limit=2", Some(1)),
                ("/users/18446744073709551615", Some(1)),
                ("/users/18446744073709551616", None),
                ("/users/alice", Some(2)),
                ("/users/Alice_42", Some(2)),
                ("/users/new", Some(2)),
                ("/users/+42", None),
                ("/users/-1", None),
                ("/users/", None),
                ("/users/42/", None),
                ("/users/a-b", None),
                ("/users/%34%32", None),
                ("/users/a%2Fb", None),
                ("/users/%CE%B1", None),
                ("/orders/42", None),
            ] {
                let request = request(uri);
                let actual = routes.execute(&request);
                assert_eq!(
                    actual.as_ref().map(|out| out.route),
                    expected,
                    "{approach:?}: {uri}"
                );
                assert_eq!(actual, reference.execute(&request), "{approach:?}: {uri}");
            }
            let mut special = request("/users/new");
            special
                .headers_mut()
                .insert("x-special", "yes".parse().unwrap());
            assert_eq!(routes.execute(&special).unwrap().route, 0);
            let mut post = request("/users/42");
            *post.method_mut() = http::Method::POST;
            assert!(routes.execute(&post).is_none());
        }
    }

    #[test]
    fn seeded_inputs_preserve_reference_semantics() {
        let reference = Routes::new(Approach::Predicate);
        let alternatives = Approach::all()
            .into_iter()
            .map(Routes::new)
            .collect::<Vec<_>>();
        let alphabet = b"abcXYZ019_-+.%";
        let mut state = 0x4d595df4d0f33173_u64;
        for _ in 0..10_000 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let length = (state % 24 + 1) as usize;
            let mut value = String::new();
            for _ in 0..length {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                value.push(char::from(alphabet[(state as usize) % alphabet.len()]));
            }
            let request = request(&format!("/users/{value}"));
            let expected = reference.execute(&request);
            for routes in &alternatives {
                assert_eq!(routes.execute(&request), expected, "input {value}");
            }
        }
    }

    #[test]
    fn rejection_discards_captures_and_errors_still_stop() {
        let matcher = path("/{discard}")
            .unwrap()
            .filter(|_| false)
            .or(path("/{keep}").unwrap());
        let request = request("/alice");
        let captures = matcher
            .evaluate(&Head::from_request(&request))
            .unwrap()
            .unwrap();
        assert_eq!(captures.get("discard"), None);
        assert_eq!(captures.get("keep"), Some("alice"));
        let predicate_calls = std::cell::Cell::new(0);
        let error = try_match_fn(|_| Err(MatchError::Custom("probe".into()))).filter(|_| {
            predicate_calls.set(predicate_calls.get() + 1);
            true
        });
        assert_eq!(
            error.evaluate(&Head::from_request(&request)),
            Err(MatchError::Custom("probe".into()))
        );
        assert_eq!(predicate_calls.get(), 0);
    }

    #[test]
    fn ambiguous_values_follow_explicit_route_order() {
        fn broad_username(captures: &Captures) -> bool {
            captures.get("username").is_some_and(|raw| {
                !raw.is_empty() && raw.bytes().all(|b| b.is_ascii_alphanumeric())
            })
        }
        let id = || {
            path("/users/{id}").unwrap().filter(|captures: &Captures| {
                captures
                    .get("id")
                    .is_some_and(|raw| parse_id(raw).is_some())
            })
        };
        let name = || path("/users/{username}").unwrap().filter(broad_username);
        let id_first = Selector::new().route(id(), 1).route(name(), 2);
        let name_first = Selector::new().route(name(), 2).route(id(), 1);
        let request = request("/users/42");
        let head = Head::from_request(&request);
        assert_eq!(*id_first.select(&head).unwrap().unwrap().target, 1);
        assert_eq!(*name_first.select(&head).unwrap().unwrap().target, 2);
    }

    #[test]
    fn refinements_compose_with_suffix_and_catch_all_patterns() {
        let suffix = path("/images/img-{id}.png").unwrap().filter(|captures| {
            captures
                .get("id")
                .is_some_and(|raw| parse_id(raw).is_some())
        });
        assert!(
            suffix
                .evaluate(&Head::from_request(&request("/images/img-42.png")))
                .unwrap()
                .is_some()
        );
        assert!(
            suffix
                .evaluate(&Head::from_request(&request("/images/img-no.png")))
                .unwrap()
                .is_none()
        );
        let tail = path("/files/{*rest}").unwrap().filter(|captures| {
            captures
                .get("rest")
                .is_some_and(|raw| raw.ends_with(".txt"))
        });
        assert!(
            tail.evaluate(&Head::from_request(&request("/files/a/b.txt")))
                .unwrap()
                .is_some()
        );
        assert!(
            tail.evaluate(&Head::from_request(&request("/files/a/b.png")))
                .unwrap()
                .is_none()
        );
    }
}
