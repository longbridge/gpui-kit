//! Application-supplied highlights over the text a [`TextViewState`] renders,
//! and scrolling one of its ranges into view.
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

#[cfg(not(target_family = "wasm"))]
use std::time::Instant;
use std::{
    ops::Range,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
#[cfg(target_family = "wasm")]
use web_time::Instant;

use gpui::{Bounds, EntityId, Hsla, Pixels, SharedString};

use super::{
    document::ParsedDocument,
    node::{BlockNode, Paragraph},
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
/// layout. Where highlights overlap, the later one paints over the earlier.
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

/// Why setting range highlights or revealing a range was rejected. Existing
/// highlights and reveals stay unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RangeHighlightError {
    /// The view renders HTML, which records no source positions to address
    /// its text by.
    Unsupported,
    /// The range at this index, the highlight's or the one revealed, is
    /// reversed, out of bounds, or not on a character boundary.
    InvalidRange(usize),
}

impl std::fmt::Display for RangeHighlightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => f.write_str("HTML views do not support ranges of their text"),
            Self::InvalidRange(ix) => write!(f, "range {ix} is not a range of the text"),
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
    /// Where the text of each top-level block sits, in document order.
    blocks: Vec<Range<usize>>,
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

impl LeafSpan {
    /// `offset` in the leaf's text, moved out of an inline object onto the
    /// text after it, or before it at the end of the leaf. `None` when the
    /// leaf has no text outside its objects.
    fn text_offset_near(&self, offset: usize) -> Option<usize> {
        let object_at = |offset: usize| self.objects.iter().find(|object| object.contains(&offset));
        let mut after = offset;
        while let Some(object) = object_at(after) {
            after = object.end;
        }
        if after < self.range.len() {
            return Some(after);
        }
        let mut before = offset;
        while let Some(object) = object_at(before) {
            before = object.start.checked_sub(1)?;
        }
        Some(before)
    }
}

impl RenderedIndex {
    pub(super) fn new(document: &ParsedDocument) -> Self {
        let mut builder = IndexBuilder::default();
        let mut blocks = Vec::with_capacity(document.blocks.len());
        for block in document.blocks.iter() {
            let start = builder.text.len();
            builder.push_block(block);
            blocks.push(start..builder.text.len());
        }
        let index = Self {
            text: builder.text.into(),
            leaves: builder.leaves,
            blocks,
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

    /// Where `range` starts: the line of the first leaf text it covers, or,
    /// when it covers none, as an empty range does, of the leaf text at its
    /// start or last before it in its top-level block, or else that whole
    /// block. `None` when it is not a range of the text, or there is none.
    fn locate(&self, range: &Range<usize>) -> Option<RevealTarget> {
        if let Some((key, leaf_range)) = self.resolve(range)?.into_iter().next() {
            return Some(RevealTarget::Line {
                key,
                offset: leaf_range.start,
            });
        }
        let block_ix = self
            .blocks
            .partition_point(|block| block.end <= range.start)
            .min(self.blocks.len().checked_sub(1)?);
        let block_start = self.blocks[block_ix].start;
        let ix = self
            .leaves
            .partition_point(|leaf| leaf.range.end <= range.start);
        let leaf_offset = match self.leaves.get(ix) {
            Some(leaf) if leaf.range.contains(&range.start) => {
                Some((leaf, range.start - leaf.range.start))
            }
            // A position after the text of a leaf, on the separators after
            // it or at the end of the text, is on the line of the last
            // character before it in its block.
            _ => ix
                .checked_sub(1)
                .and_then(|ix| self.leaves.get(ix))
                .filter(|leaf| leaf.range.start >= block_start)
                .and_then(|leaf| {
                    let (last, _) = self.text[leaf.range.clone()].char_indices().last()?;
                    Some((leaf, last))
                }),
        };
        if let Some((leaf, offset)) = leaf_offset
            && let Some(offset) = leaf.text_offset_near(offset)
        {
            return Some(RevealTarget::Line {
                key: leaf.key,
                offset,
            });
        }
        Some(RevealTarget::Block { ix: block_ix })
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

/// The last cell and source end of each table row. Collect them once so
/// remapping cells neither rescans a table nor recomputes a row's end.
fn table_row_source_ends(blocks: &[BlockNode], rows: &mut Vec<(TextLeafKey, Option<usize>)>) {
    for block in blocks {
        match block {
            BlockNode::Table(table) => {
                let Some(span) = table.span else {
                    continue;
                };
                let mut cell_count = 0;
                for row in &table.children {
                    if row.children.is_empty() {
                        continue;
                    }
                    cell_count += row.children.len();
                    let end = row
                        .children
                        .iter()
                        .filter_map(|cell| paragraph_source_end(&cell.children))
                        .max();
                    rows.push((TextLeafKey::table_cell(span.start, cell_count - 1), end));
                }
            }
            BlockNode::Root { children, .. }
            | BlockNode::Blockquote { children, .. }
            | BlockNode::List { children, .. }
            | BlockNode::ListItem { children, .. } => table_row_source_ends(children, rows),
            _ => {}
        }
    }
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

    /// The highlights that still describe `new`, the document `remap` maps
    /// the old one to: each follows its leaf as far as the leaf's text is
    /// unchanged, and is dropped with a leaf that is gone.
    pub(super) fn remap(&self, remap: &LeafRemap) -> Option<Self> {
        let mut leaves = self
            .leaves
            .iter()
            .filter_map(|(key, backgrounds)| {
                let (new_key, unchanged) = remap.leaf(*key)?;
                let clipped = backgrounds
                    .iter()
                    .filter(|(range, _)| range.start < unchanged)
                    .map(|(range, background)| (range.start..range.end.min(unchanged), *background))
                    .collect::<Vec<_>>();
                (!clipped.is_empty()).then_some((new_key, clipped))
            })
            .collect::<Vec<_>>();
        // Moving keys keeps their order, but stay safe for the binary search.
        leaves.sort_by_key(|(key, _)| *key);
        (!leaves.is_empty()).then_some(Self { leaves })
    }
}

/// Where the text leaves of one parsed document are found in the document
/// parsed after it.
///
/// A block that starts before the first change of the source is found at the
/// same offset, and one after the last change at an offset moved by the
/// change in length; a block that starts between them is gone. A leaf keeps
/// its text up to where it first differs from before.
pub(super) struct LeafRemap<'a> {
    old_len: usize,
    new_len: usize,
    /// With an append, where the block it parsed again starts: every leaf
    /// before it is unchanged.
    tail_start: Option<usize>,
    unchanged_prefix: usize,
    unchanged_suffix: usize,
    old_leaves: Vec<(TextLeafKey, TextLeaf<'a>)>,
    new_leaves: Vec<(TextLeafKey, TextLeaf<'a>)>,
    /// Sorted by each row's last cell. Unneeded for append-only remapping.
    table_rows: Vec<(TextLeafKey, Option<usize>)>,
}

impl<'a> LeafRemap<'a> {
    /// With `tail_only`, `new` was parsed by appending to `old`, which parses
    /// only the last block of `old` again and keeps the others as they were.
    pub(super) fn new(old: &'a ParsedDocument, new: &'a ParsedDocument, tail_only: bool) -> Self {
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

        let mut table_rows = Vec::new();
        if !tail_only {
            table_row_source_ends(&old.blocks, &mut table_rows);
            table_rows.sort_by_key(|(key, _)| *key);
        }

        Self {
            old_len,
            new_len,
            tail_start,
            unchanged_prefix,
            unchanged_suffix,
            old_leaves: leaves_from(old, tail_start),
            new_leaves: leaves_from(new, tail_start),
            table_rows,
        }
    }

    /// Where leaf `key` is in the new document, and how much of its text is
    /// unchanged, or `None` when it is gone.
    pub(super) fn leaf(&self, key: TextLeafKey) -> Option<(TextLeafKey, usize)> {
        if self
            .tail_start
            .is_some_and(|tail_start| key.block_start() < tail_start)
        {
            return Some((key, usize::MAX));
        }
        let new_key = key.moved_to(self.moved(key.block_start())?);
        let old_leaf = Self::find(&self.old_leaves, key)?;
        // A table's cells are only known by their place in it, so after a
        // change inside the table a cell is the same one only when the source
        // of its whole row ends before that change.
        if key.cell_ix().is_some()
            && key.block_start() < self.unchanged_prefix
            && self.tail_start.is_none()
            && self
                .row_source_end(key)
                .is_none_or(|end| end > self.unchanged_prefix)
        {
            return None;
        }
        let new_leaf = Self::find(&self.new_leaves, new_key)?;
        Some((new_key, new_leaf.common_prefix_len(old_leaf)))
    }

    fn row_source_end(&self, key: TextLeafKey) -> Option<usize> {
        let ix = self.table_rows.partition_point(|(last, _)| *last < key);
        let (last, end) = self.table_rows.get(ix)?;
        if last.block_start() != key.block_start() {
            return None;
        }
        *end
    }

    /// Where the block starting at `start` in the old document starts in the
    /// new one.
    fn moved(&self, start: usize) -> Option<usize> {
        if start < self.unchanged_prefix {
            Some(start)
        } else if start >= self.old_len - self.unchanged_suffix {
            Some(start + self.new_len - self.old_len)
        } else {
            None
        }
    }

    fn find<'b>(
        leaves: &'b [(TextLeafKey, TextLeaf<'a>)],
        key: TextLeafKey,
    ) -> Option<&'b TextLeaf<'a>> {
        let ix = leaves.binary_search_by_key(&key, |(leaf, _)| *leaf).ok()?;
        Some(&leaves[ix].1)
    }
}

#[cfg(test)]
mod tests {
    use gpui::hsla;

    use super::{LeafRemap, RangeHighlight, RangeHighlightFrame, TextLeafKey};
    use crate::text::{document::ParsedDocument, format::markdown, node::NodeContext};

    fn parse(source: &str) -> ParsedDocument {
        markdown::parse(source, &mut NodeContext::default()).unwrap()
    }

    #[test]
    fn table_row_index_keeps_empty_cells_in_their_row() {
        let source = "| a | b |\n|---|---|\n| é |   |\n|   |   |\n| c | d |\n";
        let document = parse(source);
        let remap = LeafRemap::new(&document, &document, false);
        assert_eq!(remap.table_rows.len(), 4);
        let ends = [Some("b"), Some("é"), None, Some("d")];
        for (row, text) in ends.into_iter().enumerate() {
            let expected = text.map(|text| source.find(text).unwrap() + text.len());
            for column in 0..2 {
                let key = TextLeafKey::table_cell(0, row * 2 + column);
                assert_eq!(remap.row_source_end(key), expected, "{key:?}");
            }
        }
        assert_eq!(remap.row_source_end(TextLeafKey::table_cell(0, 8)), None);
    }

    #[test]
    fn table_row_index_finds_nested_tables_without_crossing_between_them() {
        let source = concat!(
            "| a |\n|---|\n| b |\n\n",
            "> | c |\n> |---|\n> | d |\n\n",
            "- | e |\n  |---|\n  | f |\n",
        );
        let document = parse(source);
        let remap = LeafRemap::new(&document, &document, false);
        assert_eq!(remap.table_rows.len(), 6);
        for (header, body) in [("a", "b"), ("c", "d"), ("e", "f")] {
            let start = source.find(&format!("| {header} |")).unwrap();
            for (cell, text) in [header, body].into_iter().enumerate() {
                let key = TextLeafKey::table_cell(start, cell);
                assert_eq!(
                    remap.row_source_end(key),
                    Some(source.find(text).unwrap() + text.len()),
                );
            }
            assert_eq!(
                remap.row_source_end(TextLeafKey::table_cell(start, 2)),
                None
            );
            assert_eq!(
                remap.row_source_end(TextLeafKey::table_cell(start + 1, 0)),
                None
            );
        }
    }

    #[test]
    fn long_and_wide_tables_remap_highlights_by_whole_rows() {
        for (rows, columns) in [(4096, 1), (2, 1024), (64, 16)] {
            let row = format!("|{}\n", " x |".repeat(columns));
            let separator = format!("|{}\n", "---|".repeat(columns));
            let source = format!("{row}{separator}{}", row.repeat(rows));
            let old = parse(&source);
            let mut changed = source.clone();
            // Even unchanged cells earlier in the edited row must lose their
            // highlights, as must the unchanged rows that follow it.
            let edit = row.len() + separator.len() + row.rfind('x').unwrap();
            changed.replace_range(edit..edit + 1, "y");
            let new = parse(&changed);
            let remap = LeafRemap::new(&old, &new, false);
            assert_eq!(remap.table_rows.len(), rows + 1);
            let frame = RangeHighlightFrame {
                leaves: remap
                    .old_leaves
                    .iter()
                    .map(|(key, _)| (*key, vec![(0..1, hsla(0.15, 1., 0.5, 0.4))]))
                    .collect(),
            };
            assert_eq!(frame.leaves.len(), (rows + 1) * columns);
            let kept = frame.remap(&remap).unwrap();
            assert_eq!(kept.leaves.len(), columns);
            for column in 0..columns {
                assert_eq!(
                    kept.backgrounds(TextLeafKey::table_cell(0, column)),
                    &[(0..1, hsla(0.15, 1., 0.5, 0.4))],
                );
            }
        }
    }

    #[test]
    fn append_remapping_does_not_build_a_table_row_index() {
        let source = "| a |\n|---|\n| b |\n\n| c |\n|---|\n| d |\n";
        let old = parse(source);
        let new = parse(&format!("{source}| e |\n"));
        let remap = LeafRemap::new(&old, &new, true);
        assert!(remap.table_rows.is_empty());
        let first = TextLeafKey::table_cell(0, 0);
        assert_eq!(remap.leaf(first), Some((first, usize::MAX)));
        let last_start = source.find("| c |").unwrap();
        for cell in 0..2 {
            let key = TextLeafKey::table_cell(last_start, cell);
            assert_eq!(remap.leaf(key), Some((key, 1)));
        }
    }

    #[test]
    fn a_position_in_an_inline_object_moves_onto_text() {
        use super::{LeafSpan, TextLeafKey};
        // "ab" then two objects of 2 bytes each, then "cd".
        let leaf = |len: usize| LeafSpan {
            range: 10..10 + len,
            key: TextLeafKey::block(0),
            objects: vec![2..4, 4..6],
        };
        assert_eq!(leaf(8).text_offset_near(1), Some(1));
        // Onto the text after the objects.
        assert_eq!(leaf(8).text_offset_near(3), Some(6));
        assert_eq!(leaf(8).text_offset_near(5), Some(6));
        // At the end of the leaf, onto the text before them.
        assert_eq!(leaf(6).text_offset_near(5), Some(1));
    }

    #[test]
    fn range_highlight_requires_a_background() {
        let color = hsla(0.15, 1., 0.5, 0.4);
        let highlight = RangeHighlight::new(2..5, color);
        assert_eq!(highlight.range(), 2..5);
        assert_eq!(highlight.background(), color);
    }
}

/// Where a range to reveal starts.
#[derive(Clone, Copy, Debug, PartialEq)]
enum RevealTarget {
    /// A line of a text leaf: the leaf, and the offset in its text.
    Line { key: TextLeafKey, offset: usize },
    /// A whole top-level block, for text that belongs to no leaf.
    Block { ix: usize },
}

/// How long a reveal keeps trying. One that has not been carried out by
/// then, e.g. because its view was not painted, is dropped rather than
/// scrolling long after it was asked for.
const REVEAL_TIMEOUT: Duration = Duration::from_secs(1);

/// How many frames a reveal whose line was laid out but not visible keeps
/// trying, e.g. while an enclosing container scrolls to it.
const REVEAL_ATTEMPTS: usize = 8;

/// Where the line a reveal starts on was laid out in one frame, in window
/// coordinates, and whether it was inside the visible area.
#[derive(Clone, Copy, Debug)]
struct RevealReport {
    line: Bounds<Pixels>,
    visible: bool,
}

/// A range [`TextViewState::reveal_range`](super::TextViewState::reveal_range)
/// is scrolling into view.
///
/// The `Inline` that lays out the start of the range asks the enclosing list
/// to scroll its line into view during prepaint, and reports where the line
/// ended up. The view reads the report once painted, after any list has
/// scrolled, and is done once the line is visible.
#[derive(Debug)]
pub(super) struct PendingReveal {
    target: RevealTarget,
    requested_at: Instant,
    /// What the target's `Inline` reported this frame; `None` when it was not
    /// laid out.
    report: Arc<Mutex<Option<RevealReport>>>,
    attempts: usize,
}

/// How a pending reveal went in one frame.
pub(super) enum RevealProgress {
    /// The line is visible, so the reveal is done.
    Shown,
    /// The line was laid out at these window bounds without being visible.
    Hidden(Bounds<Pixels>),
    /// The line was not laid out.
    NotLaidOut,
}

impl PendingReveal {
    /// The start of `range` in `text`, asked for at `now`, or `None` when
    /// the range is not a range of it.
    pub(super) fn new(text: &RenderedText, range: &Range<usize>, now: Instant) -> Option<Self> {
        Some(Self {
            target: text.index().locate(range)?,
            requested_at: now,
            report: Arc::default(),
            attempts: 0,
        })
    }

    pub(super) fn is_expired(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.requested_at) > REVEAL_TIMEOUT
            || self.attempts >= REVEAL_ATTEMPTS
    }

    /// Whether the reveal is of a whole block rather than a line.
    pub(super) fn is_block(&self) -> bool {
        matches!(self.target, RevealTarget::Block { .. })
    }

    /// The index of the top-level block of `document` the reveal starts in.
    pub(super) fn block_ix(&self, document: &ParsedDocument) -> Option<usize> {
        match self.target {
            RevealTarget::Line { key, .. } => document.blocks.iter().rposition(|block| {
                block
                    .span()
                    .is_some_and(|span| span.start <= key.block_start())
            }),
            RevealTarget::Block { ix } => (ix < document.blocks.len()).then_some(ix),
        }
    }

    /// Whether the line was laid out in the previous frame.
    pub(super) fn was_laid_out(&self) -> bool {
        self.report.lock().is_ok_and(|report| report.is_some())
    }

    /// Starts a frame: forgets the previous report and hands the line to
    /// rendering. A block has no line.
    pub(super) fn request(&self) -> Option<RevealRequest> {
        if let Ok(mut report) = self.report.lock() {
            *report = None;
        }
        let RevealTarget::Line { key, offset } = self.target else {
            return None;
        };
        Some(RevealRequest {
            key,
            offset,
            report: self.report.clone(),
        })
    }

    /// Ends a frame with what the line reported, counting a frame in which
    /// it was laid out but hidden as an attempt.
    pub(super) fn progress(&mut self) -> RevealProgress {
        let report = self.report.lock().ok().and_then(|report| *report);
        match report {
            Some(report) if report.visible => RevealProgress::Shown,
            Some(report) => {
                self.attempts += 1;
                RevealProgress::Hidden(report.line)
            }
            None => RevealProgress::NotLaidOut,
        }
    }

    /// The reveal in the document `remap` maps the old one to, as long as
    /// the text it starts at is unchanged.
    pub(super) fn remap(mut self, remap: &LeafRemap) -> Option<Self> {
        let RevealTarget::Line { key, offset } = self.target else {
            return None;
        };
        let (key, unchanged) = remap.leaf(key)?;
        (offset < unchanged).then_some(())?;
        self.target = RevealTarget::Line { key, offset };
        Some(self)
    }
}

/// The start of a pending reveal, as rendering hands it to the `Inline`
/// that lays that text out.
#[derive(Clone, Debug)]
pub(crate) struct RevealRequest {
    key: TextLeafKey,
    offset: usize,
    report: Arc<Mutex<Option<RevealReport>>>,
}

impl RevealRequest {
    /// The start of the reveal, when it is in `key`'s text between `start`
    /// and `end`, rebased to `start`.
    pub(crate) fn at(
        &self,
        key: Option<TextLeafKey>,
        start: usize,
        end: usize,
    ) -> Option<RevealAt> {
        if key != Some(self.key) {
            return None;
        }
        RevealAt {
            offset: self.offset,
            report: self.report.clone(),
        }
        .rebase(start, end)
    }
}

/// The start of a pending reveal, in the byte space of one run of text.
#[derive(Clone, Debug)]
pub(crate) struct RevealAt {
    offset: usize,
    report: Arc<Mutex<Option<RevealReport>>>,
}

impl RevealAt {
    pub(crate) fn offset(&self) -> usize {
        self.offset
    }

    /// The reveal in the text between `start` and `end`, rebased to `start`,
    /// or `None` when it starts outside it.
    pub(crate) fn rebase(&self, start: usize, end: usize) -> Option<Self> {
        (start..end).contains(&self.offset).then(|| Self {
            offset: self.offset - start,
            report: self.report.clone(),
        })
    }

    /// The reveal moved into the text between `start` and `end` and rebased
    /// to `start`: one before it moves to its first character, one after it
    /// to its end.
    pub(crate) fn clamp(&self, start: usize, end: usize) -> Self {
        Self {
            offset: self.offset.clamp(start, end) - start,
            report: self.report.clone(),
        }
    }

    /// Report where the line the reveal starts on was laid out, in window
    /// coordinates, and whether it was inside the visible area.
    pub(crate) fn report(&self, line: Bounds<Pixels>, visible: bool) {
        if let Ok(mut report) = self.report.lock() {
            *report = Some(RevealReport { line, visible });
        }
    }
}
