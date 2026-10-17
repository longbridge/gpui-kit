use std::ops::Range;

use unicode_segmentation::GraphemeCursor;

use crate::text::{
    markdown_ext::MarkdownNode,
    node::{ImageNode, TextMark},
};

/// The placeholder an inline atom occupies in the flat text.
pub(crate) const ATOM_CHAR: char = '\u{FFFC}';
pub(crate) const ATOM_LEN: usize = ATOM_CHAR.len_utf8();

/// An inline object that is edited as a single character.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum InlineAtom {
    Image(ImageNode),
    Custom(MarkdownNode),
}

/// A run of text with one set of marks.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MarkRun {
    pub(crate) len: usize,
    pub(crate) marks: TextMark,
}

/// Flat inline content: text, contiguous mark runs and inline atoms.
///
/// Invariants (design §4.6), restored by [`InlineText::normalize`] after every
/// edit and checked by [`InlineText::is_normalized`]:
/// - N1: no empty runs, and no two adjacent runs with equal marks (except
///   around atoms, see N2).
/// - N2: every atom has a run of its own.
/// - N3: run lengths sum to the text length.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct InlineText {
    text: String,
    runs: Vec<MarkRun>,
    atoms: Vec<(usize, InlineAtom)>,
}

impl InlineText {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Plain text with one set of marks. Stray U+FFFC characters are dropped,
    /// because that character is reserved for atoms.
    pub(crate) fn plain(text: &str, marks: TextMark) -> Self {
        let text: String = text.chars().filter(|ch| *ch != ATOM_CHAR).collect();
        let mut out = Self {
            runs: vec![MarkRun {
                len: text.len(),
                marks,
            }],
            text,
            atoms: Vec::new(),
        };
        out.normalize();
        out
    }

    pub(crate) fn atom(atom: InlineAtom, marks: TextMark) -> Self {
        Self {
            text: ATOM_CHAR.to_string(),
            runs: vec![MarkRun {
                len: ATOM_LEN,
                marks,
            }],
            atoms: vec![(0, atom)],
        }
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn len(&self) -> usize {
        self.text.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn runs(&self) -> &[MarkRun] {
        &self.runs
    }

    #[cfg(test)]
    pub(crate) fn atoms(&self) -> &[(usize, InlineAtom)] {
        &self.atoms
    }

    /// Runs with their byte ranges and the atom they hold, if any.
    pub(crate) fn pieces(
        &self,
    ) -> impl Iterator<Item = (Range<usize>, &TextMark, Option<&InlineAtom>)> {
        let mut start = 0;
        self.runs.iter().map(move |run| {
            let range = start..start + run.len;
            start += run.len;
            let atom = self
                .atoms
                .iter()
                .find(|(offset, _)| *offset == range.start && run.len == ATOM_LEN)
                .map(|(_, atom)| atom);
            (range, &run.marks, atom)
        })
    }

    /// Append `other`, shifting its atoms.
    pub(crate) fn push(&mut self, other: InlineText) {
        let shift = self.text.len();
        self.text.push_str(&other.text);
        self.runs.extend(other.runs);
        self.atoms.extend(
            other
                .atoms
                .into_iter()
                .map(|(offset, atom)| (offset + shift, atom)),
        );
        self.normalize();
    }

    /// A copy of `range`. The range must lie on char boundaries.
    pub(crate) fn slice(&self, range: Range<usize>) -> InlineText {
        let mut out = InlineText {
            text: self.text[range.clone()].to_string(),
            runs: Vec::new(),
            atoms: Vec::new(),
        };
        for (run_range, marks, _) in self.pieces() {
            let lo = run_range.start.max(range.start);
            let hi = run_range.end.min(range.end);
            if lo < hi {
                out.runs.push(MarkRun {
                    len: hi - lo,
                    marks: marks.clone(),
                });
            }
        }
        out.atoms = self
            .atoms
            .iter()
            .filter(|(offset, _)| *offset >= range.start && *offset + ATOM_LEN <= range.end)
            .map(|(offset, atom)| (offset - range.start, atom.clone()))
            .collect();
        out.normalize();
        out
    }

    /// Replace `range` with `insert` and return what was removed.
    pub(crate) fn replace(&mut self, range: Range<usize>, insert: InlineText) -> InlineText {
        let removed = self.slice(range.clone());
        let mut out = self.slice(0..range.start);
        out.push(insert);
        out.push(self.slice(range.end..self.len()));
        *self = out;
        removed
    }

    /// Rewrite the marks over `range`.
    pub(crate) fn map_marks(&mut self, range: Range<usize>, f: impl Fn(&TextMark) -> TextMark) {
        let mut runs = Vec::new();
        for (run_range, marks, _) in self.pieces() {
            for (lo, hi, inside) in [
                (run_range.start, run_range.end.min(range.start), false),
                (
                    run_range.start.max(range.start),
                    run_range.end.min(range.end),
                    true,
                ),
                (run_range.start.max(range.end), run_range.end, false),
            ] {
                if lo < hi {
                    runs.push(MarkRun {
                        len: hi - lo,
                        marks: if inside { f(marks) } else { marks.clone() },
                    });
                }
            }
        }
        self.runs = runs;
        self.normalize();
    }

    /// The marks of the character before `offset`, if any.
    pub(crate) fn mark_before(&self, offset: usize) -> Option<&TextMark> {
        self.pieces()
            .find(|(range, _, _)| range.start < offset && offset <= range.end)
            .map(|(_, marks, _)| marks)
    }

    /// The marks of the character after `offset`, if any.
    pub(crate) fn mark_after(&self, offset: usize) -> Option<&TextMark> {
        self.pieces()
            .find(|(range, _, _)| range.start <= offset && offset < range.end)
            .map(|(_, marks, _)| marks)
    }

    pub(crate) fn is_grapheme_boundary(&self, offset: usize) -> bool {
        if offset > self.text.len() || !self.text.is_char_boundary(offset) {
            return false;
        }
        GraphemeCursor::new(offset, self.text.len(), true)
            .is_boundary(&self.text, 0)
            .unwrap_or(false)
    }

    pub(crate) fn prev_grapheme(&self, offset: usize) -> Option<usize> {
        GraphemeCursor::new(offset, self.text.len(), true)
            .prev_boundary(&self.text, 0)
            .ok()
            .flatten()
    }

    pub(crate) fn next_grapheme(&self, offset: usize) -> Option<usize> {
        GraphemeCursor::new(offset, self.text.len(), true)
            .next_boundary(&self.text, 0)
            .ok()
            .flatten()
    }

    /// Restore N1–N3.
    pub(crate) fn normalize(&mut self) {
        let mut runs: Vec<MarkRun> = Vec::with_capacity(self.runs.len());
        let mut last_is_atom = false;
        let mut start = 0;
        for run in std::mem::take(&mut self.runs) {
            // Split runs at atom edges so every atom gets a run of its own.
            let mut cuts = vec![start, start + run.len];
            for (offset, _) in &self.atoms {
                for cut in [*offset, *offset + ATOM_LEN] {
                    if cut > start && cut < start + run.len {
                        cuts.push(cut);
                    }
                }
            }
            cuts.sort_unstable();
            cuts.dedup();
            for pair in cuts.windows(2) {
                let (lo, hi) = (pair[0], pair[1]);
                if lo == hi {
                    continue;
                }
                let is_atom = self.atoms.iter().any(|(offset, _)| *offset == lo);
                match runs.last_mut() {
                    Some(prev) if !is_atom && !last_is_atom && prev.marks == run.marks => {
                        prev.len += hi - lo
                    }
                    _ => runs.push(MarkRun {
                        len: hi - lo,
                        marks: run.marks.clone(),
                    }),
                }
                last_is_atom = is_atom;
            }
            start += run.len;
        }
        // N3: cover the whole text even if the runs fell short.
        if start < self.text.len() {
            runs.push(MarkRun {
                len: self.text.len() - start,
                marks: TextMark::default(),
            });
        }
        self.runs = runs;
        self.atoms.sort_by_key(|(offset, _)| *offset);
        debug_assert!(self.is_normalized(), "inline text not normalized: {self:?}");
    }

    pub(crate) fn is_normalized(&self) -> bool {
        let total: usize = self.runs.iter().map(|run| run.len).sum();
        if total != self.text.len() || self.runs.iter().any(|run| run.len == 0) {
            return false;
        }
        let mut start = 0;
        let mut starts = Vec::with_capacity(self.runs.len());
        for run in &self.runs {
            starts.push(start);
            start += run.len;
        }
        let is_atom_run = |ix: usize| {
            self.atoms
                .iter()
                .any(|(offset, _)| *offset == starts[ix] && self.runs[ix].len == ATOM_LEN)
        };
        for (offset, _) in &self.atoms {
            match starts.iter().position(|s| s == offset) {
                Some(ix) if self.runs[ix].len == ATOM_LEN => {}
                _ => return false,
            }
            if self.text.get(*offset..*offset + ATOM_LEN) != Some("\u{FFFC}") {
                return false;
            }
        }
        if self.text.matches(ATOM_CHAR).count() != self.atoms.len() {
            return false;
        }
        for ix in 1..self.runs.len() {
            if self.runs[ix].marks == self.runs[ix - 1].marks
                && !is_atom_run(ix)
                && !is_atom_run(ix - 1)
            {
                return false;
            }
        }
        true
    }
}
