use crate::{Captures, Head, MatchError, Matcher};

/// The identity and captures of one committed selection.
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

/// A selected target and its capture metadata.
#[derive(Debug)]
pub struct Selected<'a, T> {
    /// The consumer's target, which has not been executed.
    pub target: &'a T,
    /// Committed identity and captures.
    pub selection: Selection,
}

/// An ordered, synchronous route table, independent of Tower and body types.
///
/// Local-only custom matchers are supported. No fallback response is fabricated.
pub struct Selector<T> {
    entries: Vec<(Box<dyn Matcher>, T)>,
}

/// An ordered selector whose matchers may move between and be shared by threads.
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
