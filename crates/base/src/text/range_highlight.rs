//! Application-supplied highlights over the text a [`TextViewState`] renders.
//!
//! Ranges address the rendered text, the string plain copy produces: an
//! application searches [`TextViewState::rendered_text`] and hands the ranges
//! it found back. Each range is split into the text leaves it covers (a
//! paragraph, a heading, a code block, a table cell), which paint it as a
//! background behind their glyphs, so a highlight never changes layout. Text
//! outside every leaf (the separators between blocks and cells, custom blocks,
//! HTML blocks, inline objects) is left unpainted.
//!
//! [`TextViewState`]: super::TextViewState
//! [`TextViewState::rendered_text`]: super::TextViewState::rendered_text

use std::{
    ops::Range,
    sync::{Arc, OnceLock},
};

use gpui::{EntityId, Hsla, SharedString};

use super::{
    document::ParsedDocument,
    node::{BlockNode, Paragraph, Table},
    stream_fade::{TextLeaf, TextLeafKey, text_leaves},
};

/// A snapshot of the text a [`TextViewState`](super::TextViewState) renders,
/// as of one parse of its content.
///
/// Offsets into it are UTF-8 byte offsets. It is the string plain copy
/// produces: `hello **world**` renders as `hello world`, escapes are
/// resolved, and heading markers and list markers are left out. Blocks end
/// with a newline and table cells are joined with a space; those separators
/// belong to no block, so no highlight paints them.
///
/// Two snapshots are equal when they come from the same view and the same
/// parse. Comparing the current [`rendered_text`] with the one last searched
/// tells an observer of the view whether its content changed, so setting
/// highlights, which notifies the view too, does not start another search.
/// The text itself is only built when it is first read, from the parsed
/// document the snapshot holds on to, so drop a snapshot that is no longer
/// needed rather than keeping it past many changes.
///
/// [`rendered_text`]: super::TextViewState::rendered_text
#[derive(Clone)]
pub struct RenderedText {
    owner: EntityId,
    revision: usize,
    document: ParsedDocument,
    index: Arc<OnceLock<RenderedIndex>>,
}

impl std::fmt::Debug for RenderedText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderedText")
            .field("owner", &self.owner)
            .field("revision", &self.revision)
            .finish_non_exhaustive()
    }
}

impl RenderedText {
    /// The text of `document`, whose index `index` holds once built.
    pub(super) fn new(
        owner: EntityId,
        revision: usize,
        document: ParsedDocument,
        index: Arc<OnceLock<RenderedIndex>>,
    ) -> Self {
        Self {
            owner,
            revision,
            document,
            index,
        }
    }

    /// The rendered text.
    pub fn as_str(&self) -> &str {
        &self.index().text
    }

    /// The length of the rendered text, in bytes.
    pub fn len(&self) -> usize {
        self.index().text.len()
    }

    /// Whether the view renders no text.
    pub fn is_empty(&self) -> bool {
        self.index().text.is_empty()
    }

    pub(super) fn index(&self) -> &RenderedIndex {
        self.index
            .get_or_init(|| RenderedIndex::new(&self.document))
    }
}

impl PartialEq for RenderedText {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner && self.revision == other.revision
    }
}

impl Eq for RenderedText {}

/// A background painted behind one range of a [`RenderedText`].
///
/// It is painted under the text and under the selection, and never changes
/// layout. Where highlights overlap, the later one paints over the earlier. A
#[derive(Clone, Debug, PartialEq)]
pub struct RangeHighlight {
    range: Range<usize>,
    background: Hsla,
}

impl RangeHighlight {
    /// A highlight over `range`, in byte offsets of a [`RenderedText`].
    pub fn new(range: Range<usize>, background: impl Into<Hsla>) -> Self {
        Self {
            range,
            background: background.into(),
        }
    }

    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    pub fn background(&self) -> Hsla {
        self.background
    }
}

/// Why setting range highlights was rejected. Existing highlights stay unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RangeHighlightError {
    /// The view renders HTML, which records no source positions to address
    /// its text by.
    Unsupported,
    /// The highlight at this index is reversed, out of bounds, or not on a
    /// character boundary.
    InvalidRange(usize),
}

impl std::fmt::Display for RangeHighlightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => f.write_str("HTML views do not support range highlights"),
            Self::InvalidRange(ix) => write!(f, "highlight {ix} is not a range of the text"),
        }
    }
}

impl std::error::Error for RangeHighlightError {}

/// The rendered text of one parsed document, and where each text leaf sits
/// in it.
#[derive(Debug, Default)]
pub(super) struct RenderedIndex {
    text: SharedString,
    /// In document order, so by their position in `text`.
    leaves: Vec<LeafSpan>,
}

#[derive(Debug)]
struct LeafSpan {
    /// Where the leaf's text sits in the rendered text.
    range: Range<usize>,
    key: TextLeafKey,
    /// Inline objects in the leaf's text, in leaf offsets. They paint as
    /// objects rather than as text, so no highlight paints them.
    objects: Vec<Range<usize>>,
}

impl RenderedIndex {
    pub(super) fn new(document: &ParsedDocument) -> Self {
        let mut builder = IndexBuilder::default();
        for block in document.blocks.iter() {
            builder.push_block(block);
        }
        let index = Self {
            text: builder.text.into(),
            leaves: builder.leaves,
        };
        debug_assert_eq!(index.text.as_ref(), document.text());
        index
    }

    /// The leaf ranges `range` paints over, which are none when it covers no
    /// leaf text, or `None` when it is not a range of the text.
    fn resolve(&self, range: &Range<usize>) -> Option<Vec<(TextLeafKey, Range<usize>)>> {
        if range.start > range.end
            || range.end > self.text.len()
            || !self.text.is_char_boundary(range.start)
            || !self.text.is_char_boundary(range.end)
        {
            return None;
        }

        let first = self
            .leaves
            .partition_point(|leaf| leaf.range.end <= range.start);
        let mut pieces = Vec::new();
        for leaf in &self.leaves[first..] {
            if leaf.range.start >= range.end {
                break;
            }
            let end = range.end.min(leaf.range.end) - leaf.range.start;
            let mut cursor = range.start.max(leaf.range.start) - leaf.range.start;
            for object in &leaf.objects {
                if object.start >= end {
                    break;
                }
                if object.end <= cursor {
                    continue;
                }
                if object.start > cursor {
                    pieces.push((leaf.key, cursor..object.start));
                }
                cursor = object.end;
            }
            if cursor < end {
                pieces.push((leaf.key, cursor..end));
            }
        }
        Some(pieces)
    }
}

/// Builds the rendered text the way `BlockNode::text` does, recording each
/// leaf as it goes.
#[derive(Default)]
struct IndexBuilder {
    text: String,
    leaves: Vec<LeafSpan>,
}

impl IndexBuilder {
    fn push_block(&mut self, block: &BlockNode) {
        let start = self.text.len();
        match block {
            BlockNode::Root { children, .. } | BlockNode::Blockquote { children, .. } => {
                for child in children {
                    self.push_block(child);
                }
            }
            BlockNode::List { children, .. } | BlockNode::ListItem { children, .. } => {
                for child in children {
                    self.push_block(child);
                }
                return;
            }
            BlockNode::Paragraph(paragraph) => {
                self.push_paragraph(
                    paragraph,
                    paragraph.span.map(|span| TextLeafKey::block(span.start)),
                );
            }
            BlockNode::Heading { children, span, .. } => {
                self.push_paragraph(children, span.map(|span| TextLeafKey::block(span.start)));
            }
            BlockNode::Table(table) => {
                let mut ordinal = 0;
                for row in table.children.iter().filter(|row| !row.children.is_empty()) {
                    for (ix, cell) in row.children.iter().enumerate() {
                        if ix > 0 {
                            self.text.push(' ');
                        }
                        self.push_paragraph(
                            &cell.children,
                            table
                                .span
                                .map(|span| TextLeafKey::table_cell(span.start, ordinal)),
                        );
                        ordinal += 1;
                    }
                    self.text.push('\n');
                }
            }
            BlockNode::CodeBlock(code_block) => {
                self.push_leaf(
                    &code_block.code(),
                    code_block.span.map(|span| TextLeafKey::block(span.start)),
                    Vec::new(),
                );
            }
            BlockNode::Custom(node) => self.text.push_str(node.as_text()),
            BlockNode::Definition { .. }
            | BlockNode::Break { .. }
            | BlockNode::HorizontalRule { .. }
            | BlockNode::Unknown => {}
        }
        if self.text.len() > start {
            self.text.push('\n');
        }
    }

    fn push_paragraph(&mut self, paragraph: &Paragraph, key: Option<TextLeafKey>) {
        let mut text = String::new();
        let mut objects = Vec::new();
        for child in &paragraph.children {
            if child.custom.is_some() {
                objects.push(text.len()..text.len() + child.text.len());
            }
            text.push_str(&child.text);
        }
        self.push_leaf(&text, key, objects);
    }

    fn push_leaf(&mut self, text: &str, key: Option<TextLeafKey>, objects: Vec<Range<usize>>) {
        let start = self.text.len();
        self.text.push_str(text);
        if let Some(key) = key
            && !text.is_empty()
        {
            self.leaves.push(LeafSpan {
                range: start..self.text.len(),
                key,
                objects,
            });
        }
    }
}

/// Where the source of the table row holding cell `cell_ix` of the table
/// starting at `table_start` ends, when the parser recorded it.
fn row_source_end(blocks: &[BlockNode], table_start: usize, cell_ix: usize) -> Option<usize> {
    fn find_table(blocks: &[BlockNode], start: usize) -> Option<&Table> {
        blocks.iter().find_map(|block| match block {
            BlockNode::Table(table) if table.span.map(|span| span.start) == Some(start) => {
                Some(table)
            }
            BlockNode::Root { children, .. }
            | BlockNode::Blockquote { children, .. }
            | BlockNode::List { children, .. }
            | BlockNode::ListItem { children, .. } => find_table(children, start),
            _ => None,
        })
    }

    let mut first_cell = 0;
    for row in &find_table(blocks, table_start)?.children {
        if cell_ix < first_cell + row.children.len() {
            return row
                .children
                .iter()
                .filter_map(|cell| paragraph_source_end(&cell.children))
                .max();
        }
        first_cell += row.children.len();
    }
    None
}

/// Where the source of `paragraph`'s text ends, when the parser recorded it.
fn paragraph_source_end(paragraph: &Paragraph) -> Option<usize> {
    paragraph
        .children
        .iter()
        .flat_map(|node| {
            node.source_segments
                .iter()
                .map(|segment| segment.source.end)
                .chain(
                    node.custom
                        .as_ref()
                        .and_then(|custom| custom.source_range())
                        .map(|range| range.end),
                )
        })
        .max()
}

/// The highlights each leaf paints, resolved once when they change so
/// rendering only looks up its leaf.
#[derive(Debug, Default)]
pub(crate) struct RangeHighlightFrame {
    /// Sorted by key. A leaf's backgrounds keep the order the application
    /// gave them in, so a later one paints over an earlier one.
    leaves: Vec<(TextLeafKey, Vec<(Range<usize>, Hsla)>)>,
}

impl RangeHighlightFrame {
    /// Validates `highlights` against `text` and resolves them to leaves.
    pub(super) fn new(
        text: &RenderedText,
        highlights: impl IntoIterator<Item = RangeHighlight>,
    ) -> Result<Option<Self>, RangeHighlightError> {
        let mut pieces = Vec::new();
        for (ix, highlight) in highlights.into_iter().enumerate() {
            let leaf_ranges = text
                .index()
                .resolve(&highlight.range)
                .ok_or(RangeHighlightError::InvalidRange(ix))?;
            pieces.extend(
                leaf_ranges
                    .into_iter()
                    .map(|(key, range)| (key, range, highlight.background)),
            );
        }

        // Stable, so each leaf keeps the application's order.
        pieces.sort_by_key(|(key, _, _)| *key);
        let mut leaves: Vec<(TextLeafKey, Vec<(Range<usize>, Hsla)>)> = Vec::new();
        for (key, range, background) in pieces {
            match leaves.last_mut() {
                Some((last, backgrounds)) if *last == key => backgrounds.push((range, background)),
                _ => leaves.push((key, vec![(range, background)])),
            }
        }
        Ok((!leaves.is_empty()).then_some(Self { leaves }))
    }

    /// The backgrounds of leaf `key`, in its rendered byte space.
    pub(crate) fn backgrounds(&self, key: TextLeafKey) -> &[(Range<usize>, Hsla)] {
        self.leaves
            .binary_search_by_key(&key, |(leaf, _)| *leaf)
            .map_or(&[], |ix| self.leaves[ix].1.as_slice())
    }

    /// The highlights that still describe `new`, the document parsed after
    /// `old`.
    ///
    /// A block that starts before the first change of the source is found at
    /// the same offset in `new`, and one after the last change at an offset
    /// moved by the change in length; a block that starts between them is
    /// gone. A highlight follows its block, as far as the text of its leaf is
    /// unchanged, and is dropped with a leaf that is gone.
    ///
    /// With `tail_only`, `new` was parsed by appending to `old`, which parses
    /// only the last block of `old` again and keeps the others as they were.
    pub(super) fn remap(
        &self,
        old: &ParsedDocument,
        new: &ParsedDocument,
        tail_only: bool,
    ) -> Option<Self> {
        let (old_len, new_len) = (old.source.len(), new.source.len());
        // An append starts after the old source, and parses its last block
        // again, or only the new text when that block has no span.
        let tail_start = tail_only.then(|| {
            old.blocks
                .last()
                .and_then(BlockNode::span)
                .map_or(old_len, |span| span.start)
        });
        let (unchanged_prefix, unchanged_suffix) = if tail_only {
            (old_len, 0)
        } else {
            let prefix = old
                .source
                .bytes()
                .zip(new.source.bytes())
                .take_while(|(old, new)| old == new)
                .count();
            let shorter = old_len.min(new_len);
            if prefix == shorter {
                // One source extends the other, as when text is appended.
                (prefix, 0)
            } else {
                // Where the two overlap, as when a deleted block starts like
                // the block after it, the end wins: the blocks after a change
                // keep following their text rather than their offset.
                let suffix = old
                    .source
                    .bytes()
                    .rev()
                    .zip(new.source.bytes().rev())
                    .take_while(|(old, new)| old == new)
                    .count()
                    .min(shorter);
                (prefix.min(shorter - suffix), suffix)
            }
        };
        // Where the block starting at `start` in `old` starts in `new`.
        let moved = |start: usize| {
            if start < unchanged_prefix {
                Some(start)
            } else if start >= old_len - unchanged_suffix {
                Some(start + new_len - old_len)
            } else {
                None
            }
        };

        fn leaves_from(
            document: &ParsedDocument,
            tail_start: Option<usize>,
        ) -> Vec<(TextLeafKey, TextLeaf<'_>)> {
            let mut leaves = Vec::new();
            for block in document.blocks.iter().rev() {
                if let Some(tail_start) = tail_start
                    && block.span().is_none_or(|span| span.start < tail_start)
                {
                    break;
                }
                text_leaves(block, &mut leaves);
            }
            leaves.sort_by_key(|(key, _)| *key);
            leaves
        }
        fn find<'a>(
            leaves: &'a [(TextLeafKey, TextLeaf<'a>)],
            key: TextLeafKey,
        ) -> Option<&'a TextLeaf<'a>> {
            let ix = leaves.binary_search_by_key(&key, |(leaf, _)| *leaf).ok()?;
            Some(&leaves[ix].1)
        }
        let old_leaves = leaves_from(old, tail_start);
        let new_leaves = leaves_from(new, tail_start);

        let mut leaves = self
            .leaves
            .iter()
            .filter_map(|(key, backgrounds)| {
                if tail_start.is_some_and(|tail_start| key.block_start() < tail_start) {
                    return Some((*key, backgrounds.clone()));
                }
                let new_key = key.moved_to(moved(key.block_start())?);
                let old_leaf = find(&old_leaves, *key)?;
                // A table's cells are only known by their place in it, so
                // after a change inside the table a cell is the same one only
                // when the source of its whole row ends before that change.
                if let Some(cell_ix) = key.cell_ix()
                    && key.block_start() < unchanged_prefix
                    && tail_start.is_none()
                    && row_source_end(&old.blocks, key.block_start(), cell_ix)
                        .is_none_or(|end| end > unchanged_prefix)
                {
                    return None;
                }
                let new_leaf = find(&new_leaves, new_key)?;
                let prefix = new_leaf.common_prefix_len(old_leaf);
                let clipped = backgrounds
                    .iter()
                    .filter(|(range, _)| range.start < prefix)
                    .map(|(range, background)| (range.start..range.end.min(prefix), *background))
                    .collect::<Vec<_>>();
                (!clipped.is_empty()).then_some((new_key, clipped))
            })
            .collect::<Vec<_>>();
        // Moving keys keeps their order, but stay safe for the binary search.
        leaves.sort_by_key(|(key, _)| *key);
        (!leaves.is_empty()).then_some(Self { leaves })
    }
}

#[cfg(test)]
mod tests {
    use gpui::hsla;

    use super::RangeHighlight;

    #[test]
    fn range_highlight_requires_a_background() {
        let color = hsla(0.15, 1., 0.5, 0.4);
        let highlight = RangeHighlight::new(2..5, color);
        assert_eq!(highlight.range(), 2..5);
        assert_eq!(highlight.background(), color);
    }
}
