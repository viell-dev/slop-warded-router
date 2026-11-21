//! Route conditions on `Accept`, `Accept-Language` and `Accept-Encoding`.
//!
//! Clients list the representations they accept, with weights from `q=0` (not acceptable)
//! to `q=1` (the default). Negotiation compares that list with your offers. There are two
//! kinds of conditions:
//!
//! - **Preference within a set.** [`Offers`] holds every alternative you offer, and
//!   [`Offers::selected`] matches only when its offer is the client's preferred one. Give
//!   each alternative route one of these; at most one of them matches a request.
//! - **Acceptability of one offer.** [`accepts_language`], [`accepts_media_type`] and
//!   [`accepts_encoding`] match whenever the client would accept that offer, whatever else
//!   exists. Several such routes can match one request, and declaration order then decides.
//!
//! # Serve a page in several languages
//!
//! Every language route shares one offer set, and a final route without a language
//! condition serves the default when no offer is acceptable or the field is malformed:
//!
//! ```
//! use http::Request;
//! use warded::{
//!     Head, Matcher, Selector,
//!     matchers::{exact_path, get},
//!     negotiation::{LanguageMatching, Offers},
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let languages = Offers::languages(["en", "fr", "de"], LanguageMatching::Lookup)?;
//! let page = || get().and(exact_path("/welcome"));
//! let routes = Selector::new()
//!     .route(page().and(languages.selected("en")?), "English")
//!     .route(page().and(languages.selected("fr")?), "French")
//!     .route(page().and(languages.selected("de")?), "German")
//!     .route(page(), "English, as the default");
//!
//! let select = |accept_language: &str| -> Result<_, Box<dyn std::error::Error>> {
//!     let request = Request::builder()
//!         .uri("/welcome")
//!         .header("accept-language", accept_language)
//!         .body(())?;
//!     Ok(routes.select(&Head::from_request(&request))?.map(|selected| *selected.target))
//! };
//! // Lookup shortens fr-CA to reach the offer fr.
//! assert_eq!(select("fr-CA, fr;q=0.9, en;q=0.5")?, Some("French"));
//! // The highest weight wins.
//! assert_eq!(select("en;q=0.2, de;q=0.8")?, Some("German"));
//! // Nothing offered is acceptable, so every language route misses.
//! assert_eq!(select("es")?, Some("English, as the default"));
//! // A malformed field misses too, unless the condition uses `invalid_is_error`.
//! assert_eq!(select("fr;q=2")?, Some("English, as the default"));
//! # Ok(())
//! # }
//! ```
//!
//! Add `Vary: Accept-Language` to the responses of these routes, including the default,
//! so shared caches keep the variants apart. Warded does not change responses.
//!
//! # Language matching
//!
//! [`LanguageMatching::Lookup`] shortens the client's range until it reaches an offer, so
//! a request for `fr-CA` can select the offer `fr`; it suits a set of general languages
//! with a default. [`LanguageMatching::BasicFilter`] lets a range cover longer tags, so a
//! request for `fr` can select the offer `fr-CA`; it suits regional offers. Neither mode
//! goes the other way, which is why the mode is a required argument.
//!
//! # Absent, empty and malformed fields
//!
//! | Field                     | [`Offers::selected`]            | `accepts_*`  |
//! | ------------------------- | ------------------------------- | ------------ |
//! | Absent                    | The first offer in the set      | Matches      |
//! | Present but empty         | Misses                          | Misses       |
//! | Malformed or over [`Limits`] | Misses, or [`MatchError::InvalidHeader`] with [`Negotiated::invalid_is_error`] | The same |
//!
//! An empty `Accept-Encoding` is the exception: it accepts only `identity`. An empty field
//! is valid, so it misses even with `invalid_is_error`. Repeated fields are combined into
//! one list in order.
//!
//! # Rules and dependencies
//!
//! Ties between equal weights go to the earlier offer in the set, except that lookup keeps
//! the client's order of equal-weight ranges. Shared offer sets compute their choice once
//! per immutable [`Head`] and limits; independent conditions parse the field each time.
//!
//! Parsing, quality syntax, quoting, media-range matching, and encoding negotiation come
//! from `parse-net`; media precedence and language selection follow Warded's own rules.
//! The [content negotiation guide](https://viell-dev.github.io/slop-warded-router/negotiation.html)
//! works through wildcards, exclusions, media types, encodings and limits.

use crate::{Captures, DefinitionError, Head, MatchError, MatchResult, Matcher};
use http::header::{ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, HeaderName};
use parse_net::negotiate::{MalformedPolicy, Negotiator, Outcome};
use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    sync::{Arc, Mutex},
};

mod scoring;

/// How a client's language range reaches a server's language offer (RFC 4647).
///
/// ```
/// use http::Request;
/// use warded::{Head, Matcher, negotiation::{LanguageMatching, accepts_language}};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let request = Request::builder().header("accept-language", "fr-CA").body(())?;
/// let head = Head::from_request(&request);
/// // Lookup shortens the requested fr-CA to the offered fr.
/// let lookup = accepts_language("fr", LanguageMatching::Lookup)?;
/// assert!(lookup.evaluate(&head)?.is_some());
/// // Basic filtering only lets a range cover longer tags, so fr-CA does not cover fr.
/// let filter = accepts_language("fr", LanguageMatching::BasicFilter)?;
/// assert!(filter.evaluate(&head)?.is_none());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum LanguageMatching {
    /// Try ranges in quality/client order, truncating each before the next range.
    ///
    /// A request for `fr-CA` tries the offer `fr-CA`, then `fr`, before the next range.
    /// A positive `*` selects the first offer that is not excluded, after every named range.
    Lookup,
    /// A range accepts itself and tags beginning with that range plus a hyphen.
    ///
    /// A request for `fr` accepts the offers `fr`, `fr-CA` and `fr-FR`. Each offer takes
    /// the weight of its longest covering range, and equal weights keep offer order.
    BasicFilter,
}

/// Bounds for one negotiation operation, independent of the parser dependency's API.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Limits {
    parser: parse_net::Limits,
    field_bytes: usize,
}

impl Limits {
    /// Default grammar bounds and a 64 KiB combined-field byte limit.
    pub const DEFAULT: Self = Self {
        parser: parse_net::Limits::DEFAULT,
        field_bytes: 65_536,
    };
    /// Bound the number of nonempty list members (default 1024).
    #[must_use]
    pub const fn max_list_members(mut self, count: u16) -> Self {
        self.parser = self.parser.max_list_members(count);
        self
    }
    /// Bound parameters per member, counting `q` (default 256).
    ///
    /// Duplicate parameters are always refused, and that check also refuses a member with more
    /// than 64 parameters, so a bound above 64 accepts no more.
    #[must_use]
    pub const fn max_params(mut self, count: u16) -> Self {
        self.parser = self.parser.max_params(count);
        self
    }
    /// Bound all combined field bytes, including commas between repeated fields.
    #[must_use]
    pub const fn max_field_bytes(mut self, count: usize) -> Self {
        self.field_bytes = count;
        self
    }
}
impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Clone, Copy, Debug)]
enum Field {
    Media,
    Language(LanguageMatching),
    Encoding,
}
impl Field {
    fn name(self) -> HeaderName {
        match self {
            Self::Media => ACCEPT,
            Self::Language(_) => ACCEPT_LANGUAGE,
            Self::Encoding => ACCEPT_ENCODING,
        }
    }
}

#[derive(Debug)]
struct OfferSet {
    field: Field,
    values: Vec<String>,
}

/// A reusable complete offer set with identical non-negotiation applicability.
///
/// Media, encoding, and basic filtering ties use server offer order. Lookup
/// uses client range order among equal weights and offer order for a default.
///
/// Share one set among routes that are alternatives with otherwise identical conditions.
/// If a route's other conditions can fail while the set still prefers its offer, no
/// alternative matches; give routes with different availability their own sets.
///
/// ```
/// use http::Request;
/// use warded::{Head, Selector, negotiation::Offers};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // With equal weights, or with `*/*`, the first offer wins.
/// let formats = Offers::media_types(["text/html", "application/json"])?;
/// let routes = Selector::new()
///     .route(formats.selected("text/html")?, "html")
///     .route(formats.selected("application/json")?, "json");
///
/// let request = Request::builder()
///     .header("accept", "application/json, text/html;q=0.5")
///     .body(())?;
/// let selected = routes.select(&Head::from_request(&request))?.expect("json is preferred");
/// assert_eq!(*selected.target, "json");
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct Offers {
    shared: Arc<OfferSet>,
}
impl Offers {
    fn new(
        field: Field,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, DefinitionError> {
        let values: Vec<String> = values.into_iter().map(Into::into).collect();
        if values.is_empty() {
            return Err(DefinitionError("an offer set must not be empty".into()));
        }
        let mut seen = BTreeSet::new();
        for value in &values {
            let valid = !value.is_empty()
                && value.trim() == value
                && match field {
                    Field::Media => {
                        parse_net::media::parse(value.as_bytes(), &parse_net::Limits::DEFAULT)
                            .is_ok_and(|media| media.param("q").is_none())
                    }
                    Field::Language(_) => oxilangtag::LanguageTag::parse(value.as_str()).is_ok(),
                    Field::Encoding => value != "*" && parse_net::text::is_token(value.as_bytes()),
                };
            let key = match field {
                Field::Media => value.clone(),
                _ => value.to_ascii_lowercase(),
            };
            if !valid || !seen.insert(key) {
                return Err(DefinitionError(format!(
                    "invalid or duplicate server offer for {}: {value:?}",
                    field.name()
                )));
            }
        }
        Ok(Self {
            shared: Arc::new(OfferSet { field, values }),
        })
    }
    /// Concrete media types, including representation parameters but no weight.
    pub fn media_types(
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, DefinitionError> {
        Self::new(Field::Media, values)
    }
    /// Well-formed concrete BCP 47 tags, including private-use/grandfathered tags.
    ///
    /// Syntax is checked by `oxilangtag`; IANA registration is not required.
    pub fn languages(
        values: impl IntoIterator<Item = impl Into<String>>,
        mode: LanguageMatching,
    ) -> Result<Self, DefinitionError> {
        Self::new(Field::Language(mode), values)
    }
    /// Supported content codings; this does not compress a response.
    pub fn encodings(
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, DefinitionError> {
        Self::new(Field::Encoding, values)
    }
    /// Match the preferred offer, identified by its exact registered string.
    pub fn selected(&self, offer: &str) -> Result<Negotiated, DefinitionError> {
        let expected = self
            .shared
            .values
            .iter()
            .position(|value| value == offer)
            .ok_or_else(|| DefinitionError(format!("offer is not registered: {offer:?}")))?;
        Ok(Negotiated {
            offers: self.clone(),
            expected,
            invalid_is_error: false,
            limits: Limits::DEFAULT,
        })
    }
}

/// A predicate for one choice within a complete shared offer set.
#[derive(Clone, Debug)]
pub struct Negotiated {
    offers: Offers,
    expected: usize,
    invalid_is_error: bool,
    limits: Limits,
}
impl Negotiated {
    /// Report malformed input as an error. The default is an ordinary miss.
    ///
    /// Negating the default malformed-as-miss predicate matches malformed input.
    /// Use this policy when that is inappropriate; errors propagate through NOT.
    #[must_use]
    pub fn invalid_is_error(mut self) -> Self {
        self.invalid_is_error = true;
        self
    }
    /// Configure grammar and combined-field bounds.
    #[must_use]
    pub fn parser_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }
}

impl Matcher for Negotiated {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        let result = head
            .negotiation
            .choice(head, &self.offers.shared, self.limits);
        match result {
            Ok(index) => Ok((index == Some(self.expected)).then(Captures::default)),
            Err(detail) if self.invalid_is_error => Err(MatchError::InvalidHeader {
                name: self.offers.shared.field.name().to_string(),
                detail,
            }),
            Err(_) => Ok(None),
        }
    }
}

type Choice = Result<Option<usize>, String>;

#[derive(Debug)]
struct CachedChoice {
    // Retaining the Arc prevents an allocator reusing an offer-set address as
    // a different cache key while this head still exists.
    _offers: Arc<OfferSet>,
    result: Choice,
}

/// Internal memoization belongs to one immutable head, never the request body.
#[derive(Debug, Default)]
pub(crate) struct Cache {
    entries: Mutex<HashMap<(usize, Limits), CachedChoice>>,
}
impl Clone for Cache {
    fn clone(&self) -> Self {
        Self::default()
    }
}
impl Cache {
    fn choice(&self, head: &Head<'_>, offers: &Arc<OfferSet>, limits: Limits) -> Choice {
        let key = (Arc::as_ptr(offers) as usize, limits);
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        entries
            .entry(key)
            .or_insert_with(|| CachedChoice {
                _offers: offers.clone(),
                result: choose(head, offers, limits),
            })
            .result
            .clone()
    }
}

fn choose(head: &Head<'_>, offers: &OfferSet, limits: Limits) -> Choice {
    let value = combined_field(head, &offers.field.name(), limits.field_bytes)?;
    let values: Vec<&str> = offers.values.iter().map(String::as_str).collect();
    match offers.field {
        Field::Media => scoring::media(value.as_deref(), &values, limits.parser),
        Field::Language(mode) => scoring::language(value.as_deref(), &values, mode, limits.parser),
        Field::Encoding => match Negotiator::new()
            .limits(limits.parser)
            .on_malformed(MalformedPolicy::Refuse)
            .encoding(value.as_deref(), &values)
        {
            Outcome::Chosen { index, .. } => Ok(Some(index)),
            Outcome::NoneAcceptable => Ok(None),
            Outcome::Refused(error) => Err(error.to_string()),
            _ => Err("unexpected encoding negotiation result under refusal policy".into()),
        },
    }
}

fn combined_field<'a>(
    head: &'a Head<'_>,
    name: &HeaderName,
    limit: usize,
) -> Result<Option<Cow<'a, [u8]>>, String> {
    let mut values = head.headers().get_all(name).iter();
    let Some(first) = values.next() else {
        return Ok(None);
    };
    if first.as_bytes().len() > limit {
        return Err("combined field exceeds configured byte limit".into());
    }
    let Some(second) = values.next() else {
        return Ok(Some(Cow::Borrowed(first.as_bytes())));
    };
    let mut joined = first.as_bytes().to_vec();
    for value in std::iter::once(second).chain(values) {
        if joined
            .len()
            .checked_add(1)
            .and_then(|n| n.checked_add(value.as_bytes().len()))
            .is_none_or(|length| length > limit)
        {
            return Err("combined field exceeds configured byte limit".into());
        }
        joined.push(b',');
        joined.extend_from_slice(value.as_bytes());
    }
    Ok(Some(Cow::Owned(joined)))
}

/// Test one media type's acceptability, without comparing other routes.
pub fn accepts_media_type(offer: &str) -> Result<Negotiated, DefinitionError> {
    Offers::media_types([offer])?.selected(offer)
}
/// Test one language under an explicit lookup or basic-filtering scheme.
pub fn accepts_language(
    offer: &str,
    mode: LanguageMatching,
) -> Result<Negotiated, DefinitionError> {
    Offers::languages([offer], mode)?.selected(offer)
}
/// Test one content coding, including identity and absence/empty-field rules.
pub fn accepts_encoding(offer: &str) -> Result<Negotiated, DefinitionError> {
    Offers::encodings([offer])?.selected(offer)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_choices_are_cached_by_set_and_all_limits_but_not_error_mapping() {
        let request = http::Request::builder()
            .header("accept", "text/html,application/json")
            .body(())
            .unwrap();
        let head = Head::from_request(&request);
        let offers = Offers::media_types(["text/html", "application/json"]).unwrap();
        for _ in 0..3 {
            offers
                .selected("text/html")
                .unwrap()
                .evaluate(&head)
                .unwrap();
            offers
                .selected("application/json")
                .unwrap()
                .evaluate(&head)
                .unwrap();
        }
        assert_eq!(head.negotiation.entries.lock().unwrap().len(), 1);
        let limited = offers
            .selected("text/html")
            .unwrap()
            .parser_limits(Limits::DEFAULT.max_list_members(1));
        assert!(limited.evaluate(&head).unwrap().is_none());
        assert!(limited.invalid_is_error().evaluate(&head).is_err());
        assert_eq!(head.negotiation.entries.lock().unwrap().len(), 2);
        assert!(head.clone().negotiation.entries.lock().unwrap().is_empty());
    }
}
