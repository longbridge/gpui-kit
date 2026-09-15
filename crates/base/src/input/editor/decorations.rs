use crate::input::EditorMode;
use std::{collections::BTreeMap, ops::Range};

use gpui::{App, Context, HighlightStyle, Hsla, WeakEntity};
use ropey::Rope;
use sum_tree::Bias;

use super::{InputBaseState, RopeExt as _};

/// Geometric presentation for an editor range decoration.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RangeDecorationStyle {
    /// Fill the continuous visual range.
    Fill,
    /// Draw a continuous one-pixel frame around the visual range.
    #[default]
    Frame,
}

/// A geometric decoration over a UTF-8 byte range.
#[derive(Clone, Debug, PartialEq)]
pub struct RangeDecoration {
    range: Range<usize>,
    style: RangeDecorationStyle,
    color: Option<Hsla>,
}

impl RangeDecoration {
    /// Create a frame using the editor foreground color.
    pub fn new(range: Range<usize>) -> Self {
        Self {
            range,
            style: RangeDecorationStyle::default(),
            color: None,
        }
    }

    /// The half-open UTF-8 byte range supplied to this decoration.
    pub fn range(&self) -> &Range<usize> {
        &self.range
    }

    /// The geometric paint style.
    pub fn style(&self) -> RangeDecorationStyle {
        self.style
    }

    /// An application-owned color override, or `None` for the editor fallback.
    pub fn color(&self) -> Option<Hsla> {
        self.color
    }

    /// Choose a fill or frame without changing text layout.
    pub fn with_style(mut self, style: RangeDecorationStyle) -> Self {
        self.style = style;
        self
    }

    /// Override the editor foreground fallback with an application-owned color.
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

/// A presentation style applied to a UTF-8 byte range in an input.
///
/// This is the GPUI [`HighlightStyle`] counterpart of Monaco's
/// [`IModelDeltaDecoration`](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.IModelDeltaDecoration.html).
#[derive(Clone, Debug, PartialEq)]
pub struct TextDecoration {
    pub range: Range<usize>,
    pub style: HighlightStyle,
}

impl TextDecoration {
    /// Create a text decoration from a UTF-8 byte range and a GPUI style.
    pub fn new(range: Range<usize>, style: HighlightStyle) -> Self {
        Self { range, style }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DecorationCollectionId(usize);

/// An independently managed collection of [`TextDecoration`]s.
///
/// This is the GPUI Component counterpart of Monaco's
/// [`IEditorDecorationsCollection`](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.IEditorDecorationsCollection.html).
#[derive(Clone, Debug)]
pub struct TextDecorationCollection {
    state: WeakEntity<InputBaseState<EditorMode>>,
    id: DecorationCollectionId,
}

impl TextDecorationCollection {
    /// Replace all decorations in this collection.
    ///
    /// This corresponds to Monaco's
    /// [`IEditorDecorationsCollection.set`](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.IEditorDecorationsCollection.html#set).
    pub fn set(&self, decorations: Vec<TextDecoration>, cx: &mut App) {
        let _ = self.state.update(cx, |state, cx| {
            let decorations = normalize(&state.text, decorations);
            if state.extras.decorations.set(self.id, decorations) {
                cx.notify();
            }
        });
    }

    /// Add decorations to this collection.
    ///
    /// This corresponds to Monaco's
    /// [`IEditorDecorationsCollection.append`](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.IEditorDecorationsCollection.html#append).
    pub fn append(&self, decorations: Vec<TextDecoration>, cx: &mut App) {
        let _ = self.state.update(cx, |state, cx| {
            let decorations = normalize(&state.text, decorations);
            if state.extras.decorations.append(self.id, decorations) {
                cx.notify();
            }
        });
    }

    /// Remove all decorations from this collection.
    ///
    /// This corresponds to Monaco's
    /// [`IEditorDecorationsCollection.clear`](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.IEditorDecorationsCollection.html#clear).
    pub fn clear(&self, cx: &mut App) {
        self.set(Vec::new(), cx);
    }

    /// Return the UTF-8 byte ranges in this collection.
    ///
    /// This corresponds to Monaco's
    /// [`IEditorDecorationsCollection.getRanges`](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.IEditorDecorationsCollection.html#getRanges).
    pub fn get_ranges(&self, cx: &App) -> Vec<Range<usize>> {
        self.state
            .read_with(cx, |state, _| {
                state
                    .extras
                    .decorations
                    .get(self.id)
                    .unwrap_or_default()
                    .iter()
                    .map(|decoration| decoration.range.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// An independently managed collection of geometric range decorations.
///
/// Clones address the same collection. Dropping a handle does not clear it; use
/// [`Self::clear`] to empty it or [`Self::dispose`] to release it permanently.
/// Operations on a disposed collection or a dropped editor are harmless no-ops.
#[derive(Clone, Debug)]
pub struct RangeDecorationCollection {
    state: WeakEntity<InputBaseState<EditorMode>>,
    id: DecorationCollectionId,
}

impl RangeDecorationCollection {
    /// Replace only this owner's decorations, clipping ranges to UTF-8 boundaries.
    pub fn set(&self, decorations: Vec<RangeDecoration>, cx: &mut App) {
        let _ = self.state.update(cx, |state, cx| {
            let decorations = normalize(&state.text, decorations);
            if state.extras.range_decorations.set(self.id, decorations) {
                cx.notify();
            }
        });
    }

    /// Append decorations, preserving their paint order.
    pub fn append(&self, decorations: Vec<RangeDecoration>, cx: &mut App) {
        let _ = self.state.update(cx, |state, cx| {
            let decorations = normalize(&state.text, decorations);
            if state.extras.range_decorations.append(self.id, decorations) {
                cx.notify();
            }
        });
    }

    /// Empty this collection without invalidating its handles.
    pub fn clear(&self, cx: &mut App) {
        self.set(Vec::new(), cx);
    }

    /// Release this collection, invalidating all of its cloned handles.
    pub fn dispose(&self, cx: &mut App) {
        let _ = self.state.update(cx, |state, cx| {
            if state
                .extras
                .range_decorations
                .entries
                .remove(&self.id)
                .is_some()
            {
                cx.notify();
            }
        });
    }

    /// Read tracked UTF-8 byte ranges in insertion order.
    pub fn get_ranges(&self, cx: &App) -> Vec<Range<usize>> {
        self.state
            .read_with(cx, |state, _| {
                state
                    .extras
                    .range_decorations
                    .get(self.id)
                    .unwrap_or_default()
                    .iter()
                    .map(|decoration| decoration.range.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Both text styles and geometric decorations share normalization and edit affinity.
pub(crate) trait TrackedDecoration {
    fn range(&self) -> &Range<usize>;
    fn range_mut(&mut self) -> &mut Range<usize>;
}

impl TrackedDecoration for TextDecoration {
    fn range(&self) -> &Range<usize> {
        &self.range
    }
    fn range_mut(&mut self) -> &mut Range<usize> {
        &mut self.range
    }
}

impl TrackedDecoration for RangeDecoration {
    fn range(&self) -> &Range<usize> {
        &self.range
    }
    fn range_mut(&mut self) -> &mut Range<usize> {
        &mut self.range
    }
}

/// A balanced interval index over stable insertion-order entries. Each midpoint
/// stores the maximum end of its subtree, so one document-spanning decoration
/// does not force a scan of every preceding decoration on each frame.
struct DecorationIndex {
    indices: Vec<usize>,
    max_ends: Vec<usize>,
}

impl DecorationIndex {
    fn new<T: TrackedDecoration>(decorations: &[T]) -> Self {
        let mut indices: Vec<_> = (0..decorations.len()).collect();
        indices.sort_unstable_by_key(|&ix| (decorations[ix].range().start, ix));
        let mut index = Self {
            max_ends: vec![0; indices.len()],
            indices,
        };
        index.build(decorations, 0..decorations.len());
        index
    }

    fn build<T: TrackedDecoration>(&mut self, decorations: &[T], span: Range<usize>) -> usize {
        if span.is_empty() {
            return 0;
        }
        let mid = span.start + span.len() / 2;
        let end = decorations[self.indices[mid]]
            .range()
            .end
            .max(self.build(decorations, span.start..mid))
            .max(self.build(decorations, mid + 1..span.end));
        self.max_ends[mid] = end;
        end
    }

    // Returns the number of visited nodes, allowing deterministic complexity tests.
    fn query<T: TrackedDecoration>(
        &self,
        decorations: &[T],
        span: Range<usize>,
        range: &Range<usize>,
        matches: &mut Vec<usize>,
    ) -> usize {
        if span.is_empty() || range.is_empty() {
            return 0;
        }
        let mid = span.start + span.len() / 2;
        if self.max_ends[mid] <= range.start {
            return 1;
        }
        let mut visited = 1 + self.query(decorations, span.start..mid, range, matches);
        let ix = self.indices[mid];
        let candidate = decorations[ix].range();
        if candidate.start < range.end {
            if candidate.end > range.start {
                matches.push(ix);
            }
            visited += self.query(decorations, mid + 1..span.end, range, matches);
        }
        visited
    }
}

struct DecorationEntries<T> {
    decorations: Vec<T>,
    index: DecorationIndex,
}

impl<T: TrackedDecoration> DecorationEntries<T> {
    fn new(decorations: Vec<T>) -> Self {
        let index = DecorationIndex::new(&decorations);
        Self { decorations, index }
    }

    fn reindex(&mut self) {
        self.index = DecorationIndex::new(&self.decorations);
    }
}

pub(crate) struct DecorationCollections<T = TextDecoration> {
    entries: BTreeMap<DecorationCollectionId, DecorationEntries<T>>,
    next_id: usize,
}

impl<T> Default for DecorationCollections<T> {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            next_id: 0,
        }
    }
}

impl<T: TrackedDecoration> DecorationCollections<T> {
    fn create(&mut self, decorations: Vec<T>) -> DecorationCollectionId {
        let id = DecorationCollectionId(self.next_id);
        self.next_id += 1;
        self.entries.insert(id, DecorationEntries::new(decorations));
        id
    }

    fn set(&mut self, id: DecorationCollectionId, decorations: Vec<T>) -> bool {
        let Some(current) = self.entries.get_mut(&id) else {
            return false;
        };
        *current = DecorationEntries::new(decorations);
        true
    }

    fn append(&mut self, id: DecorationCollectionId, decorations: Vec<T>) -> bool {
        let Some(current) = self.entries.get_mut(&id) else {
            return false;
        };
        current.decorations.extend(decorations);
        current.reindex();
        true
    }

    fn get(&self, id: DecorationCollectionId) -> Option<&[T]> {
        self.entries
            .get(&id)
            .map(|entry| entry.decorations.as_slice())
    }

    pub(super) fn adjust_for_edit(&mut self, edited_range: &Range<usize>, inserted_len: usize) {
        for entry in self.entries.values_mut() {
            let len = entry.decorations.len();
            if len == 0 || entry.index.max_ends[len / 2] <= edited_range.start {
                continue;
            }
            let mut remap = Vec::with_capacity(len);
            let mut retained = 0;
            entry.decorations.retain_mut(|decoration| {
                *decoration.range_mut() =
                    adjust_range_for_edit(decoration.range(), edited_range, inserted_len);
                let keep = !decoration.range().is_empty();
                remap.push(if keep { retained } else { usize::MAX });
                retained += usize::from(keep);
                keep
            });
            // Anchor transforms are monotone. Preserve start ordering and remap
            // removed entries rather than sorting on every keystroke: O(n).
            entry.index.indices.retain_mut(|ix| {
                *ix = remap[*ix];
                *ix != usize::MAX
            });
            entry.index.max_ends.resize(retained, 0);
            entry.index.build(&entry.decorations, 0..retained);
        }
    }

    pub(super) fn clear(&mut self) {
        for entry in self.entries.values_mut() {
            *entry = DecorationEntries::new(Vec::new());
        }
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &[T]> {
        self.entries
            .values()
            .map(|entry| entry.decorations.as_slice())
    }

    /// Query visible buffer spans (not the intervening folded-away text).
    /// Rebuilds happen on mutations, never in layout/paint. Preserve owner/item
    /// order after deduplicating ranges crossing multiple visible lines.
    pub(super) fn intersecting(&self, ranges: &[Range<usize>]) -> Vec<&T> {
        let mut result = Vec::new();
        for entry in self.entries.values() {
            let mut matches = Vec::new();
            for range in ranges {
                entry.index.query(
                    &entry.decorations,
                    0..entry.decorations.len(),
                    range,
                    &mut matches,
                );
            }
            matches.sort_unstable();
            matches.dedup();
            result.extend(matches.into_iter().map(|ix| &entry.decorations[ix]));
        }
        result
    }
}

fn adjust_range_for_edit(
    range: &Range<usize>,
    edited_range: &Range<usize>,
    inserted_len: usize,
) -> Range<usize> {
    let removed_len = edited_range.end.saturating_sub(edited_range.start);
    let shift = |offset: usize| {
        if inserted_len >= removed_len {
            offset.saturating_add(inserted_len - removed_len)
        } else {
            offset.saturating_sub(removed_len - inserted_len)
        }
    };

    if edited_range.is_empty() {
        let start = if range.start < edited_range.start {
            range.start
        } else {
            shift(range.start)
        };
        let end = if range.end <= edited_range.start {
            range.end
        } else {
            shift(range.end)
        };
        return start..end;
    }

    let inserted_end = edited_range.start + inserted_len;
    let start = if range.start <= edited_range.start {
        range.start
    } else if range.start >= edited_range.end {
        shift(range.start)
    } else {
        edited_range.start
    };
    let end = if range.end <= edited_range.start {
        range.end
    } else if range.end >= edited_range.end {
        shift(range.end)
    } else {
        inserted_end
    };
    start..end
}

fn normalize<T: TrackedDecoration>(text: &Rope, decorations: Vec<T>) -> Vec<T> {
    decorations
        .into_iter()
        .filter_map(|mut decoration| {
            // Reject reversed ranges before clipping, which could otherwise turn a
            // reversed pair within a multibyte character into a nonempty range.
            if decoration.range().is_empty() {
                return None;
            }
            let range = text.clip_offset(decoration.range().start, Bias::Left)
                ..text.clip_offset(decoration.range().end, Bias::Right);
            if range.is_empty() {
                return None;
            }
            *decoration.range_mut() = range;
            Some(decoration)
        })
        .collect()
}

impl InputBaseState<EditorMode> {
    /// Create an independently owned collection of geometric range decorations.
    ///
    /// Ranges use UTF-8 byte offsets and the same tracking as text decorations:
    /// insertion at either edge does not expand the range, insertion inside does,
    /// replacement clips overlapping anchors, and deletion removes empty ranges.
    /// Undo, redo, whole-document replacement and formatting apply these same edit
    /// transforms; decorations themselves are not undo history, so deleted ranges
    /// are not resurrected by undo. Folding changes projection, not stored ranges.
    ///
    /// Fills paint behind frames; within each style, later collections/items paint
    /// over earlier ones. Neither affects text layout, hit testing or focus. The
    /// default color is the editor foreground (12% opacity for fills).
    /// Collections live until explicitly disposed or the editor is dropped.
    pub fn create_range_decorations_collection(
        &mut self,
        decorations: Vec<RangeDecoration>,
        cx: &mut Context<Self>,
    ) -> RangeDecorationCollection {
        let id = self
            .extras
            .range_decorations
            .create(normalize(&self.text, decorations));
        cx.notify();
        RangeDecorationCollection {
            state: cx.entity().downgrade(),
            id,
        }
    }

    /// Create an independently managed collection of text decorations.
    ///
    /// This follows Monaco's
    /// [`createDecorationsCollection`](https://microsoft.github.io/monaco-editor/typedoc/interfaces/editor_editor_api.editor.ICodeEditor.html#createDecorationsCollection)
    /// ownership model. Ranges use UTF-8 byte offsets into [`Self::value`].
    ///
    /// Decoration ranges follow text edits and do not need to be set again
    /// after each change. Insertions at a range boundary do not expand the
    /// range, matching Monaco's
    /// [`NeverGrowsWhenTypingAtEdges`](https://microsoft.github.io/monaco-editor/typedoc/enums/editor_editor_api.editor.TrackedRangeStickiness.html#NeverGrowsWhenTypingAtEdges)
    /// behavior. Decorations are not rendered while the input is masked.
    /// Collections live until their [`InputBaseState`] is dropped.
    ///
    /// Collections are layered in insertion order; the first collection wins
    /// when overlapping decorations set the same [`HighlightStyle`] property.
    /// Callers should avoid conflicting overlaps within one collection.
    pub fn create_decorations_collection(
        &mut self,
        decorations: Vec<TextDecoration>,
        cx: &mut Context<Self>,
    ) -> TextDecorationCollection {
        let decorations = normalize(&self.text, decorations);
        let id = self.extras.decorations.create(decorations);
        cx.notify();
        TextDecorationCollection {
            state: cx.entity().downgrade(),
            id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometric_collections_share_utf8_normalization_and_edit_affinity() {
        let mut collections = DecorationCollections::<RangeDecoration>::default();
        let text = Rope::from("héllo world");
        let first = collections.create(normalize(
            &text,
            vec![
                RangeDecoration::new(2..4),
                RangeDecoration::new(2..1),
                RangeDecoration::new(100..200),
            ],
        ));
        let second = collections.create(normalize(&text, vec![RangeDecoration::new(7..12)]));
        assert_eq!(collections.get(first).unwrap()[0].range(), &(1..4));
        assert_eq!(collections.get(first).unwrap().len(), 1);
        collections.adjust_for_edit(&(1..1), 2);
        assert_eq!(collections.get(first).unwrap()[0].range(), &(3..6));
        collections.adjust_for_edit(&(6..6), 1);
        assert_eq!(collections.get(first).unwrap()[0].range(), &(3..6));
        collections.adjust_for_edit(&(4..4), 2);
        assert_eq!(collections.get(first).unwrap()[0].range(), &(3..8));
        collections.adjust_for_edit(&(3..8), 0);
        assert!(collections.get(first).unwrap().is_empty());
        assert!(!collections.get(second).unwrap().is_empty());
        collections.entries.remove(&first);
        let third = collections.create(vec![]);
        assert_ne!(third, first);
        assert!(!collections.set(first, vec![RangeDecoration::new(0..1)]));
        assert!(collections.get(second).is_some());
    }

    #[test]
    fn visible_query_preserves_layers_and_skips_folded_spans() {
        let mut collections = DecorationCollections::<RangeDecoration>::default();
        let first = collections.create(vec![
            RangeDecoration::new(90..100),
            RangeDecoration::new(0..100),
            RangeDecoration::new(40..50), // hidden in a fold
            RangeDecoration::new(0..5),
        ]);
        collections.create(vec![RangeDecoration::new(2..4)]);
        let ranges = |collections: &DecorationCollections<RangeDecoration>| {
            collections
                .intersecting(&[0..5, 90..100])
                .iter()
                .map(|d| d.range().clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(ranges(&collections), vec![90..100, 0..100, 0..5, 2..4]);
        collections.adjust_for_edit(&(0..0), 1);
        assert_eq!(ranges(&collections), vec![91..101, 1..101, 1..6, 3..5]);
        collections.set(first, vec![RangeDecoration::new(50..60)]);
        assert_eq!(ranges(&collections), vec![3..5]);
    }

    #[test]
    fn interval_index_culls_large_collections_even_with_a_spanning_range() {
        let mut decorations: Vec<_> = (0..100_000)
            .map(|ix| RangeDecoration::new(ix * 10..ix * 10 + 5))
            .collect();
        decorations.push(RangeDecoration::new(0..1_000_000));
        let index = DecorationIndex::new(&decorations);
        for query in [
            0..1,
            500_000..500_020,
            999_990..1_000_001,
            1_000_000..1_000_010,
        ] {
            let mut matches = Vec::new();
            let visited = index.query(&decorations, 0..decorations.len(), &query, &mut matches);
            matches.sort_unstable();
            let expected: Vec<_> = decorations
                .iter()
                .enumerate()
                .filter_map(|(ix, d)| {
                    (d.range.start < query.end && d.range.end > query.start).then_some(ix)
                })
                .collect();
            assert_eq!(matches, expected);
            assert!(visited < 100, "visited {visited} nodes for {query:?}");
        }
    }

    #[test]
    fn interval_index_matches_linear_reference_for_overlaps_and_mutations() {
        let mut collections = DecorationCollections::<RangeDecoration>::default();
        let id = collections.create(
            (0..512)
                .map(|ix| {
                    let start = (ix * 37) % 997;
                    RangeDecoration::new(start..start + ix % 61 + 1)
                })
                .collect(),
        );
        for edit in [0..0, 300..450, 900..1100] {
            collections.adjust_for_edit(&edit, 3);
            for start in (0..1100).step_by(13) {
                let query = start..start + 17;
                let expected: Vec<_> = collections
                    .get(id)
                    .unwrap()
                    .iter()
                    .filter(|d| d.range.start < query.end && d.range.end > query.start)
                    .map(|d| d.range.clone())
                    .collect();
                let actual: Vec<_> = collections
                    .intersecting(&[query])
                    .iter()
                    .map(|d| d.range.clone())
                    .collect();
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn collections_are_independent_and_ranges_are_clipped() {
        let text = Rope::from("héllo");
        let first_style = HighlightStyle {
            font_weight: Some(gpui::FontWeight::BOLD),
            ..Default::default()
        };
        let second_style = HighlightStyle {
            background_color: Some(gpui::red()),
            ..Default::default()
        };
        let mut collections = DecorationCollections::default();

        let first = collections.create(normalize(
            &text,
            vec![TextDecoration::new(2..4, first_style)],
        ));
        let second = collections.create(normalize(
            &text,
            vec![TextDecoration::new(5..100, second_style)],
        ));

        assert_ne!(first, second);
        assert_eq!(
            collections.get(first),
            Some(&[TextDecoration::new(1..4, first_style)][..])
        );
        assert_eq!(
            collections.get(second),
            Some(&[TextDecoration::new(5..6, second_style)][..])
        );

        assert!(collections.append(first, vec![TextDecoration::new(4..5, second_style)]));
        assert_eq!(
            collections.get(first),
            Some(
                &[
                    TextDecoration::new(1..4, first_style),
                    TextDecoration::new(4..5, second_style),
                ][..]
            )
        );

        assert!(collections.set(first, Vec::new()));
        assert_eq!(collections.get(first), Some(&[][..]));
        assert_eq!(
            collections.get(second),
            Some(&[TextDecoration::new(5..6, second_style)][..])
        );
    }

    #[test]
    fn decoration_ranges_follow_text_edits() {
        let style = HighlightStyle::default();
        let mut collections = DecorationCollections::default();
        let collection = collections.create(vec![TextDecoration::new(2..6, style)]);

        collections.adjust_for_edit(&(0..0), 2);
        assert_eq!(
            collections.get(collection),
            Some(&[TextDecoration::new(4..8, style)][..])
        );

        collections.adjust_for_edit(&(6..6), 2);
        assert_eq!(
            collections.get(collection),
            Some(&[TextDecoration::new(4..10, style)][..])
        );

        collections.adjust_for_edit(&(4..10), 3);
        assert_eq!(
            collections.get(collection),
            Some(&[TextDecoration::new(4..7, style)][..])
        );

        assert_eq!(adjust_range_for_edit(&(2..6), &(2..2), 2), 4..8);
        assert_eq!(adjust_range_for_edit(&(2..6), &(6..6), 2), 2..6);
        assert_eq!(adjust_range_for_edit(&(2..6), &(2..6), 3), 2..5);
    }
}
