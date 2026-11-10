//! Accept-family helpers backed by `parse-net`, not a Warded protocol parser.
//!
//! Single-offer helpers test acceptability. [`Offers`] compares a shared offer set
//! before a route tests the selected offer. Neither operation reorders routes or
//! creates a 400/406 response. Malformed fields miss unless explicitly made errors.

use crate::{Captures, DefinitionError, Head, MatchError, MatchResult, Matcher};
use http::header::{ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, HeaderName};
pub use parse_net::negotiate::LanguageMatching;
use parse_net::{
    Limits,
    negotiate::{MalformedPolicy, Negotiator, Outcome},
};
use std::{borrow::Cow, sync::Arc};

const MAX_FIELD_BYTES: usize = 65_536;

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
    fn negotiate(self, value: Option<&[u8]>, offers: &[&str], limits: Limits) -> Outcome {
        let n = Negotiator::new()
            .limits(limits)
            .on_malformed(MalformedPolicy::Refuse);
        match self {
            Self::Media => n.media(value, offers),
            Self::Language(mode) => n.language_matching(mode).language(value, offers),
            Self::Encoding => n.encoding(value, offers),
        }
    }
}

/// A reusable set of representations sharing all non-negotiation conditions.
///
/// Equal preferences use the declared offer order, as defined by `parse-net`.
/// Construct separate sets when representations have different applicability.
#[derive(Clone, Debug)]
pub struct Offers {
    field: Field,
    values: Arc<[String]>,
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
        for value in &values {
            let valid = match field {
                Field::Media => parse_net::media::parse(value.as_bytes(), &Limits::DEFAULT)
                    .is_ok_and(|media| media.param("q").is_none()),
                Field::Language(_) => {
                    value != "*"
                        && matches!(
                            field.negotiate(
                                Some(value.as_bytes()),
                                &[value.as_str()],
                                Limits::DEFAULT
                            ),
                            Outcome::Chosen { .. }
                        )
                }
                Field::Encoding => value != "*" && parse_net::text::is_token(value.as_bytes()),
            };
            if !valid {
                return Err(DefinitionError(format!(
                    "invalid server offer for {}: {value:?}",
                    field.name()
                )));
            }
        }
        Ok(Self {
            field,
            values: values.into(),
        })
    }
    /// Concrete media types, including any representation parameters.
    pub fn media_types(
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, DefinitionError> {
        Self::new(Field::Media, values)
    }
    /// Concrete language tags with an explicitly chosen language matching scheme.
    pub fn languages(
        values: impl IntoIterator<Item = impl Into<String>>,
        mode: LanguageMatching,
    ) -> Result<Self, DefinitionError> {
        Self::new(Field::Language(mode), values)
    }
    /// Supported content codings such as `gzip`, `br`, or `identity`.
    ///
    /// This helper chooses applicability; it does not compress response bodies.
    pub fn encodings(
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, DefinitionError> {
        Self::new(Field::Encoding, values)
    }
    /// Match only when this offer is preferred within the complete shared set.
    pub fn selected(&self, offer: &str) -> Result<Negotiated, DefinitionError> {
        let expected = self
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

/// A field-specific predicate comparing one selected offer.
#[derive(Clone, Debug)]
pub struct Negotiated {
    offers: Offers,
    expected: usize,
    invalid_is_error: bool,
    limits: Limits,
}
impl Negotiated {
    /// Surface malformed input as [`MatchError::InvalidHeader`] rather than a miss.
    pub fn invalid_is_error(mut self) -> Self {
        self.invalid_is_error = true;
        self
    }
    /// Configure the existing parser's bounds and duplicate-parameter policy.
    ///
    /// Combined field bytes are independently limited to 64 KiB by this alpha.
    pub fn parser_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }
    fn invalid(&self, detail: impl Into<String>) -> MatchResult {
        if self.invalid_is_error {
            Err(MatchError::InvalidHeader {
                name: self.offers.field.name().to_string(),
                detail: detail.into(),
            })
        } else {
            Ok(None)
        }
    }
}
impl Matcher for Negotiated {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        let name = self.offers.field.name();
        let value = match combined_field(head, &name) {
            Ok(value) => value,
            Err(()) => return self.invalid("combined field exceeds 64 KiB"),
        };
        let offers: Vec<&str> = self.offers.values.iter().map(String::as_str).collect();
        match self
            .offers
            .field
            .negotiate(value.as_deref(), &offers, self.limits)
        {
            Outcome::Chosen { index, .. } => Ok((index == self.expected).then(Captures::default)),
            Outcome::NoneAcceptable => Ok(None),
            Outcome::Refused(error) => self.invalid(error.to_string()),
            _ => Err(MatchError::Custom(
                "unexpected negotiation result under refusal policy".into(),
            )),
        }
    }
}

fn combined_field<'a>(head: &'a Head<'_>, name: &HeaderName) -> Result<Option<Cow<'a, [u8]>>, ()> {
    let mut values = head.headers().get_all(name).iter();
    let Some(first) = values.next() else {
        return Ok(None);
    };
    if first.as_bytes().len() > MAX_FIELD_BYTES {
        return Err(());
    }
    let Some(second) = values.next() else {
        return Ok(Some(Cow::Borrowed(first.as_bytes())));
    };
    let mut joined = first.as_bytes().to_vec();
    for value in std::iter::once(second).chain(values) {
        if value.as_bytes().len() >= MAX_FIELD_BYTES - joined.len() {
            return Err(());
        }
        joined.push(b',');
        joined.extend_from_slice(value.as_bytes());
    }
    Ok(Some(Cow::Owned(joined)))
}

/// Test whether one concrete media type is acceptable, without comparing other routes.
pub fn accepts_media_type(offer: &str) -> Result<Negotiated, DefinitionError> {
    Offers::media_types([offer])?.selected(offer)
}
/// Test one language offer under an explicit lookup or basic-filtering scheme.
pub fn accepts_language(
    offer: &str,
    mode: LanguageMatching,
) -> Result<Negotiated, DefinitionError> {
    Offers::languages([offer], mode)?.selected(offer)
}
/// Test whether one content coding is acceptable, including identity rules.
pub fn accepts_encoding(offer: &str) -> Result<Negotiated, DefinitionError> {
    Offers::encodings([offer])?.selected(offer)
}
