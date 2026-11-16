use std::ops::Range;

/// A proposed user edit before it is committed to a single-line input.
pub struct InputEditProposal<'a> {
    pub(super) previous_text: &'a str,
    pub(super) text: String,
    pub(super) selection: Range<usize>,
}

impl InputEditProposal<'_> {
    /// The editor's last committed text.
    pub fn previous_text(&self) -> &str {
        self.previous_text
    }

    /// The value the edit would commit.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The proposed selection as UTF-8 byte offsets in the proposed value.
    pub fn selection(&self) -> Range<usize> {
        self.selection.clone()
    }

    /// Replace the proposed value and its selection. Selection endpoints
    /// must be ordered UTF-8 character boundaries in the replacement.
    pub fn replace(&mut self, text: impl Into<String>, selection: Range<usize>) {
        self.text = text.into();
        self.selection = selection;
    }

    pub(super) fn is_valid(&self) -> bool {
        self.selection.start <= self.selection.end
            && self.text.is_char_boundary(self.selection.start)
            && self.text.is_char_boundary(self.selection.end)
            && !self.text.contains(['\r', '\n'])
    }
}

pub(super) type InputTransform = Box<dyn Fn(&mut InputEditProposal<'_>) -> bool>;

/// The single UTF-8 splice that turns `before` into `after`.
pub(super) fn replacement<'a>(before: &str, after: &'a str) -> (Range<usize>, &'a str) {
    let start: usize = before
        .chars()
        .zip(after.chars())
        .take_while(|(old, new)| old == new)
        .map(|(ch, _)| ch.len_utf8())
        .sum();
    let suffix: usize = before[start..]
        .chars()
        .rev()
        .zip(after[start..].chars().rev())
        .take_while(|(old, new)| old == new)
        .map(|(ch, _)| ch.len_utf8())
        .sum();
    (
        start..before.len() - suffix,
        &after[start..after.len() - suffix],
    )
}
