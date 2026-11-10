use crate::Head;
use std::{collections::BTreeMap, fmt};

/// Raw path captures from a successful candidate.
///
/// Values retain percent encoding. This is a Warded extension type, not an
/// adapter for any framework's internal path extractor.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Captures(BTreeMap<String, String>);

impl Captures {
    /// Read a selected capture.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }
    /// Iterate over names and raw values in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
    /// Add a capture, reporting name collisions rather than overwriting values.
    pub fn insert(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), MatchError> {
        let name = name.into();
        if self.0.contains_key(&name) {
            return Err(MatchError::DuplicateCapture(name));
        }
        self.0.insert(name, value.into());
        Ok(())
    }
    fn merge(&mut self, other: Self) -> Result<(), MatchError> {
        for (name, value) in other.0 {
            self.insert(name, value)?;
        }
        Ok(())
    }
}

/// An operational evaluation error, distinct from an ordinary mismatch.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MatchError {
    /// A security predicate needed trusted context that was not supplied.
    MissingSecurityContext,
    /// Successful composed matchers produced the same capture name.
    DuplicateCapture(String),
    /// A strict semantic header helper rejected malformed input.
    InvalidHeader {
        /// The field name.
        name: String,
        /// The parser diagnostic.
        detail: String,
    },
    /// A consumer-defined matcher could not evaluate the request.
    Custom(String),
}

impl fmt::Display for MatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSecurityContext => f.write_str("trusted HTTPS context is missing"),
            Self::DuplicateCapture(name) => write!(f, "duplicate capture name: {name}"),
            Self::InvalidHeader { name, detail } => write!(f, "invalid {name}: {detail}"),
            Self::Custom(detail) => f.write_str(detail),
        }
    }
}
impl std::error::Error for MatchError {}

/// A match with tentative captures, an ordinary miss, or an operational error.
pub type MatchResult = Result<Option<Captures>, MatchError>;

/// A synchronous, body-free request predicate.
///
/// Implementations should be bounded and free of externally visible effects.
/// There are no `Send` or `Sync` bounds here; only the Tower adapter requires them.
pub trait Matcher {
    /// Evaluate applicability. Returned captures are private to this evaluation.
    fn evaluate(&self, head: &Head<'_>) -> MatchResult;
    /// Require both matchers; discard captures if either misses or fails.
    fn and<M: Matcher>(self, other: M) -> And<Self, M>
    where
        Self: Sized,
    {
        And(self, other)
    }
    /// Try the second matcher only if the first misses. Errors propagate.
    fn or<M: Matcher>(self, other: M) -> Or<Self, M>
    where
        Self: Sized,
    {
        Or(self, other)
    }
    /// Negate a successful/missed predicate, discarding all operand captures.
    fn not(self) -> Not<Self>
    where
        Self: Sized,
    {
        Not(self)
    }
    /// Refine a successful match using a predicate on its tentative raw captures.
    ///
    /// The predicate runs only after this matcher succeeds. Returning false is an
    /// ordinary miss and discards its captures; operational errors still propagate.
    /// Only captures produced inside this matcher are visible, so refine a composed
    /// matcher to inspect several parameters together. No decoding is implicit.
    ///
    /// ```
    /// use warded::{Matcher, matchers::path};
    /// let user = path("/users/{id}")?.filter(|captures| {
    ///     captures.get("id").is_some_and(|raw| raw.parse::<u64>().is_ok())
    /// });
    /// # Ok::<(), warded::DefinitionError>(())
    /// ```
    ///
    /// A newtype's `FromStr` implementation or a precompiled regex can supply the
    /// condition. Parsed values are not retained. Missing names need an explicit
    /// caller policy, such as `is_some_and` above. Predicates should be bounded and
    /// free of externally visible effects, like other matchers.
    fn filter<F: Fn(&Captures) -> bool>(self, predicate: F) -> Filter<Self, F>
    where
        Self: Sized,
    {
        Filter {
            inner: self,
            predicate,
        }
    }
}

impl<M: Matcher + ?Sized> Matcher for Box<M> {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        (**self).evaluate(head)
    }
}

/// Conjunction constructed with [`Matcher::and`].
#[derive(Clone, Debug)]
pub struct And<A, B>(A, B);
impl<A: Matcher, B: Matcher> Matcher for And<A, B> {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        let Some(mut left) = self.0.evaluate(head)? else {
            return Ok(None);
        };
        let Some(right) = self.1.evaluate(head)? else {
            return Ok(None);
        };
        left.merge(right)?;
        Ok(Some(left))
    }
}
/// Ordered alternative constructed with [`Matcher::or`].
#[derive(Clone, Debug)]
pub struct Or<A, B>(A, B);
impl<A: Matcher, B: Matcher> Matcher for Or<A, B> {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        match self.0.evaluate(head)? {
            Some(value) => Ok(Some(value)),
            None => self.1.evaluate(head),
        }
    }
}
/// Capture-free negation constructed with [`Matcher::not`].
#[derive(Clone, Debug)]
pub struct Not<M>(M);
impl<M: Matcher> Matcher for Not<M> {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        Ok(self.0.evaluate(head)?.is_none().then(Captures::default))
    }
}

/// Capture refinement constructed with [`Matcher::filter`].
///
/// `Clone`, `Send`, and `Sync` depend on the wrapped matcher and predicate. Local
/// selection does not require them; shared selection and dispatch retain their
/// ordinary bounds.
#[derive(Clone, Debug)]
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

#[derive(Clone)]
struct Function<F>(F);
impl<F: Fn(&Head<'_>) -> MatchResult> Matcher for Function<F> {
    fn evaluate(&self, head: &Head<'_>) -> MatchResult {
        (self.0)(head)
    }
}

/// Create an infallible, capture-free matcher from a request-head predicate.
pub fn match_fn<F: Fn(&Head<'_>) -> bool + Clone>(function: F) -> impl Matcher + Clone {
    Function(move |head: &Head<'_>| Ok(function(head).then(Captures::default)))
}
/// Create a fallible matcher with optional captures. No thread-safety bounds apply.
pub fn try_match_fn<F: Fn(&Head<'_>) -> MatchResult + Clone>(function: F) -> impl Matcher + Clone {
    Function(function)
}
