use std::fmt::Debug;

use crate::input::Selection;

/// One text replacement, in the coordinates of the document as it stood
/// immediately before the replacement was applied.
#[derive(Debug, PartialEq, Clone)]
pub(super) struct Change {
    pub(crate) old_range: Selection,
    pub(crate) old_text: String,
    pub(crate) new_range: Selection,
    pub(crate) new_text: String,
}

impl Change {
    pub(super) fn new(
        old_range: impl Into<Selection>,
        old_text: &str,
        new_range: impl Into<Selection>,
        new_text: &str,
    ) -> Self {
        Self {
            old_range: old_range.into(),
            old_text: old_text.to_string(),
            new_range: new_range.into(),
            new_text: new_text.to_string(),
        }
    }

    /// The same change as it would read after `delta` bytes were inserted
    /// (positive) or removed (negative) ahead of it.
    pub(super) fn shifted(&self, delta: isize) -> Self {
        let shift = |offset: usize| (offset as isize + delta).max(0) as usize;
        Self {
            old_range: (shift(self.old_range.start)..shift(self.old_range.end)).into(),
            old_text: self.old_text.clone(),
            new_range: (shift(self.new_range.start)..shift(self.new_range.end)).into(),
            new_text: self.new_text.clone(),
        }
    }
}
