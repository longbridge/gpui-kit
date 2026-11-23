use std::{collections::BTreeMap, ops::Range, rc::Rc};

use gpui::{App, Context, Hsla, WeakEntity};

use super::{EditorMode, InputBaseState};

/// A decoration of one whole buffer row: a background band across it.
#[derive(Clone, Debug, PartialEq)]
pub struct LineDecoration {
    row: usize,
    background: Option<Hsla>,
}

impl LineDecoration {
    /// Create an empty decoration for a zero-based buffer row.
    pub fn new(row: usize) -> Self {
        Self {
            row,
            background: None,
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

    /// Paint a band of `color` across the whole row.
    pub fn with_background(mut self, color: Hsla) -> Self {
        self.background = Some(color);
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

    pub(crate) fn has_any_provider(&self) -> bool {
        self.entries.values().any(Option::is_some)
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
    /// A background spans the whole row, gutter included, across its soft wraps,
    /// under the active line, indent guides, selections and text. Later
    /// collections paint over earlier ones. Backgrounds do not affect hit
    /// testing or focus. Collections live until explicitly disposed or the
    /// editor is dropped.
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

    struct EveryNth {
        every: usize,
        color: Hsla,
        asked: RefCell<Vec<Range<usize>>>,
    }

    impl EveryNth {
        fn new(every: usize, color: Hsla) -> Rc<Self> {
            Rc::new(Self {
                every,
                color,
                asked: RefCell::default(),
            })
        }
    }

    impl LineDecorationProvider for EveryNth {
        fn line_decorations(&self, rows: Range<usize>, _: &App) -> Vec<LineDecoration> {
            self.asked.borrow_mut().push(rows.clone());
            (rows.start.saturating_sub(1)..rows.end + 1)
                .filter(|row| row % self.every == 0)
                .map(|row| LineDecoration::new(row).with_background(self.color))
                .collect()
        }
    }

    fn editor(cx: &mut TestAppContext) -> gpui::Entity<EditorState> {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|window, cx| cx.new(|cx| EditorState::new(window, cx)))
    }

    fn red() -> Hsla {
        hsla(0., 0.5, 0.5, 0.2)
    }

    fn green() -> Hsla {
        hsla(0.3, 0.5, 0.5, 0.2)
    }

    #[test]
    fn a_decoration_is_built_from_its_parts() {
        let color = hsla(0.3, 0.5, 0.5, 0.2);
        let decoration = LineDecoration::new(7);
        assert_eq!(decoration.row(), 7);
        assert_eq!(decoration.background(), None);

        let decoration = decoration.with_background(color);
        assert_eq!(decoration.background(), Some(color));
    }

    #[gpui::test]
    fn line_decoration_collection_round_trips(cx: &mut TestAppContext) {
        let editor = editor(cx);
        let provider = EveryNth::new(1, green());

        let collection = editor.update(cx, |state, cx| {
            assert!(state.extras.line_decorations.entries.is_empty());
            state.create_line_decorations_collection(provider.clone(), cx)
        });
        cx.read(|cx| assert!(collection.has_provider(cx)));

        cx.update(|cx| collection.clear(cx));
        cx.read(|cx| {
            assert!(!collection.has_provider(cx));
            let state = editor.read(cx);
            assert!(state.extras.line_decorations.query(0..4, cx).is_empty());
        });

        cx.update(|cx| collection.clone().set_provider(provider.clone(), cx));
        cx.read(|cx| {
            assert!(collection.has_provider(cx));
            let state = editor.read(cx);
            assert_eq!(state.extras.line_decorations.query(0..2, cx).len(), 2);
        });

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
        let removed = EveryNth::new(2, red());
        let added = EveryNth::new(3, green());

        let (first, second) = editor.update(cx, |state, cx| {
            (
                state.create_line_decorations_collection(removed.clone(), cx),
                state.create_line_decorations_collection(added.clone(), cx),
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
                    .map(|decoration| (decoration.row(), decoration.background().unwrap()))
                    .collect::<Vec<_>>()
            })
        };

        assert_eq!(
            query(cx, 1..7),
            vec![
                (2, red()),
                (4, red()),
                (6, red()),
                (3, green()),
                (6, green()),
            ]
        );
        assert_eq!(*removed.asked.borrow(), vec![1..7]);
        assert_eq!(*added.asked.borrow(), vec![1..7]);

        assert!(query(cx, 3..3).is_empty());
        assert_eq!(removed.asked.borrow().len(), 1);

        cx.update(|cx| first.dispose(cx));
        assert_eq!(query(cx, 0..4), vec![(0, green()), (3, green())]);
        cx.read(|cx| assert!(second.has_provider(cx)));

        let third = editor.update(cx, |state, cx| {
            state.create_line_decorations_collection(removed.clone(), cx)
        });
        assert_ne!(third.id, first.id);
        cx.read(|cx| assert!(!first.has_provider(cx)));
    }

    #[gpui::test]
    fn a_dropped_editor_makes_its_collections_no_ops(cx: &mut TestAppContext) {
        let editor = editor(cx);
        let collection = editor.update(cx, |state, cx| {
            state.create_line_decorations_collection(EveryNth::new(1, red()), cx)
        });
        drop(editor);
        cx.run_until_parked();

        cx.update(|cx| {
            collection.set_provider(EveryNth::new(1, green()), cx);
            collection.clear(cx);
            collection.dispose(cx);
            assert!(!collection.has_provider(cx));
        });
    }
}
