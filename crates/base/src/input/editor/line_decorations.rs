use std::{collections::BTreeMap, ops::Range, rc::Rc};

use gpui::{App, Context, Hsla, SharedString, WeakEntity};

use super::{EditorMode, InputBaseState};

/// The meaning of a gutter marker beside a row.
///
/// How a marker looks is chosen by
/// [`InputEditorStyle::gutter_marker_renderer`](super::InputEditorStyle::gutter_marker_renderer).
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum GutterMarker {
    /// A row a diff added.
    DiffAdded,
    /// A row a diff removed.
    DiffRemoved,
    /// A row a diff changed.
    DiffChanged,
    /// A merge conflict awaiting resolution.
    Conflict,
    /// A row the user bookmarked.
    Bookmark,
    /// A breakpoint.
    Breakpoint,
    /// An application-chosen icon, painted as given.
    Custom {
        /// The SVG asset path of the icon, for example `icons/star.svg`.
        icon: SharedString,
        /// The color the icon is painted in.
        color: Hsla,
    },
}

/// A decoration of one whole buffer row: a background band, a gutter marker, or both.
#[derive(Clone, Debug, PartialEq)]
pub struct LineDecoration {
    row: usize,
    background: Option<Hsla>,
    marker: Option<GutterMarker>,
}

impl LineDecoration {
    /// Create an empty decoration for a zero-based buffer row.
    pub fn new(row: usize) -> Self {
        Self {
            row,
            background: None,
            marker: None,
        }
    }

    /// The zero-based buffer row this decoration covers.
    pub fn row(&self) -> usize {
        self.row
    }

    /// The color of the background band, if any.
    pub fn background(&self) -> Option<Hsla> {
        self.background
    }

    /// The gutter marker, if any.
    pub fn marker(&self) -> Option<&GutterMarker> {
        self.marker.as_ref()
    }

    /// Paint a band of `color` across the whole row.
    pub fn with_background(mut self, color: Hsla) -> Self {
        self.background = Some(color);
        self
    }

    /// Paint `marker` in the row's gutter.
    pub fn with_marker(mut self, marker: GutterMarker) -> Self {
        self.marker = Some(marker);
        self
    }
}

/// Supplies the line decorations of the rows an editor paints.
///
/// The editor asks on every frame with the visible buffer rows and ignores
/// decorations outside them, so an implementation must be cheap. Colors may be
/// read from `cx`, so they follow theme changes.
pub trait LineDecorationProvider {
    /// Return the decorations for rows within `rows`.
    fn line_decorations(&self, rows: Range<usize>, cx: &App) -> Vec<LineDecoration>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct LineDecorationCollectionId(usize);

/// An independently managed collection of line decorations, supplied by a
/// [`LineDecorationProvider`].
///
/// Clones address the same collection. Dropping a handle does not clear it; use
/// [`Self::clear`] to stop painting it or [`Self::dispose`] to release it permanently.
/// Operations on a disposed collection or a dropped editor are harmless no-ops.
#[derive(Clone, Debug)]
pub struct LineDecorationCollection {
    state: WeakEntity<InputBaseState<EditorMode>>,
    id: LineDecorationCollectionId,
}

impl LineDecorationCollection {
    /// Replace this collection's provider. The next frame asks the new one.
    pub fn set_provider(&self, provider: Rc<dyn LineDecorationProvider>, cx: &mut App) {
        self.update(Some(provider), cx);
    }

    /// Stop painting this collection without invalidating its handles.
    pub fn clear(&self, cx: &mut App) {
        self.update(None, cx);
    }

    /// Release this collection, invalidating all of its cloned handles.
    pub fn dispose(&self, cx: &mut App) {
        let _ = self.state.update(cx, |state, cx| {
            if state.extras.line_decorations.remove(self.id) {
                cx.notify();
            }
        });
    }

    /// Whether this collection has a provider; `false` once cleared, disposed or dropped.
    pub fn has_provider(&self, cx: &App) -> bool {
        self.state
            .read_with(cx, |state, _| {
                state.extras.line_decorations.has_provider(self.id)
            })
            .unwrap_or(false)
    }

    fn update(&self, provider: Option<Rc<dyn LineDecorationProvider>>, cx: &mut App) {
        let _ = self.state.update(cx, |state, cx| {
            if state.extras.line_decorations.set(self.id, provider) {
                cx.notify();
            }
        });
    }
}

/// The line decoration providers of one editor, in creation order.
#[derive(Default)]
pub(crate) struct LineDecorationProviders {
    entries: BTreeMap<LineDecorationCollectionId, Option<Rc<dyn LineDecorationProvider>>>,
    next_id: usize,
}

impl LineDecorationProviders {
    fn create(&mut self, provider: Rc<dyn LineDecorationProvider>) -> LineDecorationCollectionId {
        let id = LineDecorationCollectionId(self.next_id);
        self.next_id += 1;
        self.entries.insert(id, Some(provider));
        id
    }

    fn set(
        &mut self,
        id: LineDecorationCollectionId,
        provider: Option<Rc<dyn LineDecorationProvider>>,
    ) -> bool {
        let Some(entry) = self.entries.get_mut(&id) else {
            return false;
        };
        *entry = provider;
        true
    }

    fn remove(&mut self, id: LineDecorationCollectionId) -> bool {
        self.entries.remove(&id).is_some()
    }

    fn has_provider(&self, id: LineDecorationCollectionId) -> bool {
        self.entries.get(&id).is_some_and(Option::is_some)
    }

    /// Ask every provider about `rows`, in creation order, keeping only the
    /// decorations that fall within `rows`.
    pub(crate) fn query(&self, rows: Range<usize>, cx: &App) -> Vec<LineDecoration> {
        if rows.is_empty() {
            return Vec::new();
        }
        let mut decorations = Vec::new();
        for provider in self.entries.values().flatten() {
            decorations.extend(
                provider
                    .line_decorations(rows.clone(), cx)
                    .into_iter()
                    .filter(|decoration| rows.contains(&decoration.row)),
            );
        }
        decorations
    }
}

impl InputBaseState<EditorMode> {
    /// Create an independently owned collection of line decorations, supplied by
    /// `provider`.
    ///
    /// The provider is asked for the visible rows on every frame. Unlike text and
    /// range decorations, rows are not tracked across edits: the provider answers
    /// for the text as it is when asked.
    ///
    /// A background spans the row from the gutter to the right edge, across its
    /// soft wraps, under the active line, indent guides, selections and text. A
    /// marker is painted at the left of the line number area, only while line
    /// numbers are shown. Later collections paint over earlier ones. Neither
    /// affects text layout, hit testing or focus. Collections live until
    /// explicitly disposed or the editor is dropped.
    pub fn create_line_decorations_collection(
        &mut self,
        provider: Rc<dyn LineDecorationProvider>,
        cx: &mut Context<Self>,
    ) -> LineDecorationCollection {
        let id = self.extras.line_decorations.create(provider);
        cx.notify();
        LineDecorationCollection {
            state: cx.entity().downgrade(),
            id,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use gpui::{AppContext as _, TestAppContext, hsla};

    use super::*;
    use crate::input::EditorState;

    /// Marks every row it is asked about whose index is a multiple of `every`,
    /// and records each range it was asked for.
    struct EveryNth {
        every: usize,
        marker: GutterMarker,
        asked: RefCell<Vec<Range<usize>>>,
    }

    impl EveryNth {
        fn new(every: usize, marker: GutterMarker) -> Rc<Self> {
            Rc::new(Self {
                every,
                marker,
                asked: RefCell::default(),
            })
        }
    }

    impl LineDecorationProvider for EveryNth {
        fn line_decorations(&self, rows: Range<usize>, _: &App) -> Vec<LineDecoration> {
            self.asked.borrow_mut().push(rows.clone());
            // One past each end too: the editor must drop what it did not ask for.
            (rows.start.saturating_sub(1)..rows.end + 1)
                .filter(|row| row % self.every == 0)
                .map(|row| LineDecoration::new(row).with_marker(self.marker.clone()))
                .collect()
        }
    }

    fn editor(cx: &mut TestAppContext) -> gpui::Entity<EditorState> {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|window, cx| cx.new(|cx| EditorState::new(window, cx)))
    }

    #[test]
    fn a_decoration_is_built_from_its_parts() {
        let color = hsla(0.3, 0.5, 0.5, 0.2);
        let decoration = LineDecoration::new(7);
        assert_eq!(decoration.row(), 7);
        assert_eq!(decoration.background(), None);
        assert_eq!(decoration.marker(), None);

        let decoration = decoration
            .with_background(color)
            .with_marker(GutterMarker::Bookmark);
        assert_eq!(decoration.background(), Some(color));
        assert_eq!(decoration.marker(), Some(&GutterMarker::Bookmark));
    }

    #[gpui::test]
    fn line_decoration_collection_round_trips(cx: &mut TestAppContext) {
        let editor = editor(cx);
        let provider = EveryNth::new(1, GutterMarker::DiffAdded);

        let collection = editor.update(cx, |state, cx| {
            assert!(state.extras.line_decorations.entries.is_empty());
            state.create_line_decorations_collection(provider.clone(), cx)
        });
        cx.read(|cx| assert!(collection.has_provider(cx)));

        // Cleared: the handle stays usable, and nothing is asked.
        cx.update(|cx| collection.clear(cx));
        cx.read(|cx| {
            assert!(!collection.has_provider(cx));
            let state = editor.read(cx);
            assert!(state.extras.line_decorations.query(0..4, cx).is_empty());
        });

        // A clone addresses the same collection.
        cx.update(|cx| collection.clone().set_provider(provider.clone(), cx));
        cx.read(|cx| {
            assert!(collection.has_provider(cx));
            let state = editor.read(cx);
            assert_eq!(state.extras.line_decorations.query(0..2, cx).len(), 2);
        });

        // Disposed: every handle is spent, and setting a provider is a no-op.
        cx.update(|cx| {
            collection.dispose(cx);
            collection.set_provider(provider.clone(), cx);
        });
        cx.read(|cx| {
            assert!(!collection.has_provider(cx));
            let state = editor.read(cx);
            assert!(state.extras.line_decorations.query(0..4, cx).is_empty());
        });
    }

    #[gpui::test]
    fn collections_are_independent_and_asked_in_creation_order(cx: &mut TestAppContext) {
        let editor = editor(cx);
        let bookmarks = EveryNth::new(2, GutterMarker::Bookmark);
        let breakpoints = EveryNth::new(3, GutterMarker::Breakpoint);

        let (first, second) = editor.update(cx, |state, cx| {
            (
                state.create_line_decorations_collection(bookmarks.clone(), cx),
                state.create_line_decorations_collection(breakpoints.clone(), cx),
            )
        });

        let query = |cx: &mut TestAppContext, rows: Range<usize>| {
            cx.read(|cx| {
                editor
                    .read(cx)
                    .extras
                    .line_decorations
                    .query(rows, cx)
                    .into_iter()
                    .map(|decoration| (decoration.row(), decoration.marker().cloned().unwrap()))
                    .collect::<Vec<_>>()
            })
        };

        // Out-of-range answers are dropped; the first collection comes first.
        assert_eq!(
            query(cx, 1..7),
            vec![
                (2, GutterMarker::Bookmark),
                (4, GutterMarker::Bookmark),
                (6, GutterMarker::Bookmark),
                (3, GutterMarker::Breakpoint),
                (6, GutterMarker::Breakpoint),
            ]
        );
        assert_eq!(*bookmarks.asked.borrow(), vec![1..7]);
        assert_eq!(*breakpoints.asked.borrow(), vec![1..7]);

        // An empty range asks nobody.
        assert!(query(cx, 3..3).is_empty());
        assert_eq!(bookmarks.asked.borrow().len(), 1);

        // Disposing one owner's collection leaves the other's alone.
        cx.update(|cx| first.dispose(cx));
        assert_eq!(
            query(cx, 0..4),
            vec![(0, GutterMarker::Breakpoint), (3, GutterMarker::Breakpoint)]
        );
        cx.read(|cx| assert!(second.has_provider(cx)));

        // A collection created after a disposal gets a fresh identity.
        let third = editor.update(cx, |state, cx| {
            state.create_line_decorations_collection(bookmarks.clone(), cx)
        });
        assert_ne!(third.id, first.id);
        cx.read(|cx| assert!(!first.has_provider(cx)));
    }

    #[gpui::test]
    fn a_dropped_editor_makes_its_collections_no_ops(cx: &mut TestAppContext) {
        let editor = editor(cx);
        let collection = editor.update(cx, |state, cx| {
            state.create_line_decorations_collection(EveryNth::new(1, GutterMarker::Conflict), cx)
        });
        drop(editor);
        cx.run_until_parked();

        cx.update(|cx| {
            collection.set_provider(EveryNth::new(1, GutterMarker::DiffChanged), cx);
            collection.clear(cx);
            collection.dispose(cx);
            assert!(!collection.has_provider(cx));
        });
    }
}
