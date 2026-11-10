//! Primitive matchers and optional common compositions.

use crate::{Captures, Head, MatchError, MatchResult, Matcher, TransportSecurity, match_fn};
use http::{
    Method, Version,
    header::{HeaderName, HeaderValue},
};
use std::{collections::BTreeSet, fmt};

/// A malformed static route, header, or negotiation definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefinitionError(pub(crate) String);
impl fmt::Display for DefinitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for DefinitionError {}

/// Match the exact method, with no path or implicit HEAD condition.
pub fn method(method: Method) -> impl Matcher + Clone {
    match_fn(move |head| head.method() == method)
}
/// Match the HTTP version observed by the application server.
pub fn version(version: Version) -> impl Matcher + Clone {
    match_fn(move |head| head.version() == version)
}

macro_rules! method_helpers {
    ($($name:ident=>$method:ident),+ $(,)?)=> {$(
        #[doc=concat!("Match only the `",stringify!($method),"` method; no path is required.")]
        pub fn $name()->impl Matcher + Clone { method(Method::$method) }
    )+};
}
method_helpers!(get=>GET,head=>HEAD,post=>POST,put=>PUT,patch=>PATCH,delete=>DELETE,options=>OPTIONS,connect=>CONNECT,trace=>TRACE);

/// A condition on explicitly supplied transport security.
#[derive(Clone, Copy, Debug)]
pub struct SecurityMatcher {
    required: TransportSecurity,
    unknown_is_miss: bool,
}
impl SecurityMatcher {
    /// Treat missing security context as an ordinary mismatch instead of an error.
    #[must_use]
    pub fn unknown_as_miss(mut self) -> Self {
        self.unknown_is_miss = true;
        self
    }
}
impl Matcher for SecurityMatcher {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        if head.security() == TransportSecurity::Unknown {
            return if self.unknown_is_miss {
                Ok(None)
            } else {
                Err(MatchError::MissingSecurityContext)
            };
        }
        Ok((head.security() == self.required).then(Captures::default))
    }
}
/// Match known HTTPS. This is not an authentication or authorization check.
pub fn secured() -> SecurityMatcher {
    SecurityMatcher {
        required: TransportSecurity::Https,
        unknown_is_miss: false,
    }
}
/// Match known non-HTTPS. Unknown security is not interpreted as cleartext.
pub fn unsecure() -> SecurityMatcher {
    SecurityMatcher {
        required: TransportSecurity::Unsecure,
        unknown_is_miss: false,
    }
}
/// An optional shorthand for `secured().and(post())`.
pub fn secured_post() -> impl Matcher + Clone {
    secured().and(post())
}
/// An optional shorthand for `secured().and(get().or(head()))`.
///
/// This changes applicability only. HEAD response-body rules belong to the HTTP stack.
pub fn secured_get_and_head() -> impl Matcher + Clone {
    secured().and(get().or(head()))
}

/// Match the URI path exactly, without decoding or slash normalization.
pub fn exact_path(path: impl Into<String>) -> ExactPath {
    ExactPath(path.into())
}

/// An owned, exact raw path condition, including paths read from configuration.
#[derive(Clone, Debug)]
pub struct ExactPath(String);

impl Matcher for ExactPath {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        Ok((head.uri().path() == self.0).then(Captures::default))
    }
}

/// A compiled path pattern, evaluated independently of all other route patterns.
#[derive(Clone, Debug)]
pub struct PathMatcher {
    router: matchit::Router<()>,
}
impl Matcher for PathMatcher {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        let Ok(found) = self.router.at(head.uri().path()) else {
            return Ok(None);
        };
        let mut captures = Captures::default();
        for (name, value) in found.params.iter() {
            captures.insert(name, value)?;
        }
        Ok(Some(captures))
    }
}
/// Compile a matchit path pattern such as `/orders/{id}` or `/assets/{*rest}`.
///
/// Each route has its own pattern lookup, so static-path precedence cannot hide
/// a later overlapping candidate. Captures retain their raw percent encoding.
pub fn path(pattern: impl Into<String>) -> Result<PathMatcher, DefinitionError> {
    let pattern = pattern.into();
    if !pattern.starts_with('/') || pattern.contains(['?', '#']) {
        return Err(DefinitionError(
            "a path pattern must start with / and contain no query or fragment".into(),
        ));
    }
    let mut router = matchit::Router::new();
    router
        .insert(pattern.clone(), ())
        .map_err(|e| DefinitionError(e.to_string()))?;
    // matchit has validated the grammar. Inspect only capture names, respecting
    // its doubled opening-brace escape; this is not another path parser.
    let mut names = BTreeSet::new();
    let mut chars = pattern.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '{' {
            continue;
        }
        if chars.peek() == Some(&'{') {
            chars.next();
            continue;
        }
        let name: String = chars.by_ref().take_while(|&c| c != '}').collect();
        let name = name.strip_prefix('*').unwrap_or(&name);
        if !names.insert(name.to_owned()) {
            return Err(DefinitionError(format!("duplicate capture name: {name}")));
        }
    }
    Ok(PathMatcher { router })
}

fn header_definition(
    name: &str,
    value: &[u8],
) -> Result<(HeaderName, HeaderValue), DefinitionError> {
    Ok((
        HeaderName::try_from(name).map_err(|e| DefinitionError(e.to_string()))?,
        HeaderValue::from_bytes(value).map_err(|e| DefinitionError(e.to_string()))?,
    ))
}
/// Require exactly one field value, equal to the requested bytes.
///
/// Header names are case insensitive. Repeated fields miss, even when equal.
pub fn header_eq(name: &str, value: impl AsRef<[u8]>) -> Result<HeaderMatcher, DefinitionError> {
    let (name, value) = header_definition(name, value.as_ref())?;
    Ok(HeaderMatcher {
        name,
        condition: HeaderCondition::ExactlyOne(value),
    })
}
/// Require at least one exact raw value among possibly repeated fields.
pub fn header_any_eq(
    name: &str,
    value: impl AsRef<[u8]>,
) -> Result<HeaderMatcher, DefinitionError> {
    let (name, value) = header_definition(name, value.as_ref())?;
    Ok(HeaderMatcher {
        name,
        condition: HeaderCondition::Any(value),
    })
}
/// Match field presence without interpreting its values.
pub fn header_present(name: &str) -> Result<HeaderMatcher, DefinitionError> {
    let name = HeaderName::try_from(name).map_err(|e| DefinitionError(e.to_string()))?;
    Ok(HeaderMatcher {
        name,
        condition: HeaderCondition::Present,
    })
}

/// An owned condition on one raw header field.
#[derive(Clone, Debug)]
pub struct HeaderMatcher {
    name: HeaderName,
    condition: HeaderCondition,
}

#[derive(Clone, Debug)]
enum HeaderCondition {
    ExactlyOne(HeaderValue),
    Any(HeaderValue),
    Present,
}

impl Matcher for HeaderMatcher {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        let matched = match &self.condition {
            HeaderCondition::ExactlyOne(value) => {
                let mut values = head.headers().get_all(&self.name).iter();
                values.next() == Some(value) && values.next().is_none()
            }
            HeaderCondition::Any(value) => head
                .headers()
                .get_all(&self.name)
                .iter()
                .any(|v| v == value),
            HeaderCondition::Present => head.headers().contains_key(&self.name),
        };
        Ok(matched.then(Captures::default))
    }
}

/// Match any exact raw `key=value` query pair, preserving repetitions and encoding.
///
/// Split only on `&` and the first `=`. `+` stays a plus and `%31` stays encoded.
/// A bare flag does not equal a pair with an empty value.
pub fn query_pair(key: impl Into<String>, value: impl Into<String>) -> QueryMatcher {
    QueryMatcher {
        key: key.into(),
        value: Some(value.into()),
    }
}
/// Match a raw query key, whether a bare flag or a `key=value` pair.
pub fn query_present(key: impl Into<String>) -> QueryMatcher {
    QueryMatcher {
        key: key.into(),
        value: None,
    }
}

/// An owned raw query-key or key/value condition.
#[derive(Clone, Debug)]
pub struct QueryMatcher {
    key: String,
    value: Option<String>,
}

impl Matcher for QueryMatcher {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        let matched = head.uri().query().is_some_and(|query| {
            query
                .split('&')
                .any(|pair| match (&self.value, pair.split_once('=')) {
                    (Some(expected), Some((key, value))) => key == self.key && value == expected,
                    (Some(_), None) => false,
                    (None, pair_parts) => pair_parts.map_or(pair, |(key, _)| key) == self.key,
                })
        });
        Ok(matched.then(Captures::default))
    }
}
