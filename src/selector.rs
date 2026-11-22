use crate::{Captures, Head, MatchError, Matcher};

/// Which route was selected, by declaration index, and that route's captures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selection {
    /// Zero-based declaration index.
    pub index: usize,
    /// Only the chosen candidate's captures.
    pub captures: Captures,
}

pub(crate) fn select<'a>(
    matchers: impl Iterator<Item = &'a dyn Matcher>,
    head: &Head<'_>,
) -> Result<Option<Selection>, MatchError> {
    for (index, matcher) in matchers.enumerate() {
        if let Some(captures) = matcher.evaluate(head)? {
            return Ok(Some(Selection { index, captures }));
        }
    }
    Ok(None)
}

/// The selected route's target, with its index and captures.
#[derive(Debug)]
pub struct Selected<'a, T> {
    /// The consumer's target, which has not been executed.
    pub target: &'a T,
    /// Committed identity and captures.
    pub selection: Selection,
}

/// A route table that returns the first matching route's target without running it.
///
/// Local-only custom matchers are supported. No fallback response is fabricated.
pub struct Selector<T> {
    entries: Vec<(Box<dyn Matcher>, T)>,
}

/// A [`Selector`] whose matchers are `Send` and `Sync`, for a table shared between threads.
///
/// Targets retain their own `Send`/`Sync` requirements. Use [`Selector`] when
/// custom matchers capture local state such as `Rc` or `Cell`.
pub struct SyncSelector<T> {
    entries: Vec<(Box<dyn Matcher + Send + Sync>, T)>,
}

impl<T> Default for SyncSelector<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T> SyncSelector<T> {
    /// Start with an empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a thread-safe candidate; neither path nor method is required.
    #[must_use]
    pub fn route<M: Matcher + Send + Sync + 'static>(mut self, matcher: M, target: T) -> Self {
        self.entries.push((Box::new(matcher), target));
        self
    }

    /// Choose the first complete match without executing any target.
    pub fn select(&self, head: &Head<'_>) -> Result<Option<Selected<'_, T>>, MatchError> {
        let selection = select(
            self.entries.iter().map(|(m, _)| m.as_ref() as &dyn Matcher),
            head,
        )?;
        Ok(selection.map(|selection| Selected {
            target: &self.entries[selection.index].1,
            selection,
        }))
    }
}
impl<T> Default for Selector<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}
impl<T> Selector<T> {
    /// Start with an empty table.
    pub fn new() -> Self {
        Self::default()
    }
    /// Append a candidate; neither path nor method is required.
    #[must_use]
    pub fn route<M: Matcher + 'static>(mut self, matcher: M, target: T) -> Self {
        self.entries.push((Box::new(matcher), target));
        self
    }
    /// Choose the first complete match without executing any target.
    pub fn select(&self, head: &Head<'_>) -> Result<Option<Selected<'_, T>>, MatchError> {
        let selection = select(self.entries.iter().map(|(m, _)| m.as_ref()), head)?;
        Ok(selection.map(|selection| Selected {
            target: &self.entries[selection.index].1,
            selection,
        }))
    }
}
