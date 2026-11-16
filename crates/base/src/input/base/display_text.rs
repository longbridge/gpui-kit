use super::{Rope, RopeExt};

/// The text to shape for a single-line input and its caret positions.
pub struct InputDisplayText {
    text: String,
    offsets: Vec<usize>,
}

impl InputDisplayText {
    /// Supply one display byte offset for every Unicode scalar boundary in the
    /// source, including the start and end. Offsets must be nondecreasing and
    /// land on character boundaries in the displayed text. Input falls back to
    /// the source if the map is invalid or the result contains line breaks.
    pub fn new(text: impl Into<String>, offsets: Vec<usize>) -> Self {
        Self {
            text: text.into(),
            offsets,
        }
    }

    pub(super) fn resolve(self, source: &Rope) -> Option<(Rope, InputDisplayOffsets)> {
        if self.text.contains(['\n', '\r'])
            || self.offsets.len() != source.chars().count() + 1
            || self.offsets.windows(2).any(|pair| pair[0] > pair[1])
            || self
                .offsets
                .iter()
                .any(|&offset| !self.text.is_char_boundary(offset))
        {
            return None;
        }
        Some((
            Rope::from(self.text.as_str()),
            InputDisplayOffsets {
                source: source.clone(),
                offsets: self.offsets,
            },
        ))
    }
}

pub(super) type InputDisplayTransform = Box<dyn Fn(&str) -> InputDisplayText>;

pub(super) struct InputDisplayOffsets {
    source: Rope,
    offsets: Vec<usize>,
}

impl InputDisplayOffsets {
    pub(super) fn matches_source(&self, source: &Rope) -> bool {
        &self.source == source
    }

    pub(super) fn display_offset(&self, offset: usize) -> usize {
        self.offsets[self
            .source
            .offset_to_char_index(offset.min(self.source.len()))]
    }

    pub(super) fn source_offset(&self, offset: usize) -> usize {
        let right = self
            .offsets
            .partition_point(|&candidate| candidate <= offset);
        let index = match (right.checked_sub(1), self.offsets.get(right)) {
            (Some(left), Some(&next)) if offset - self.offsets[left] > next - offset => right,
            (Some(left), _) => left,
            (None, _) => 0,
        };
        self.source.char_index_to_offset(index)
    }
}
