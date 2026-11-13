use gpui::{
    AnyElement, App, Context, IntoElement, ParentElement as _, SharedString, Styled as _, Window,
    div,
};

use crate::{
    ActiveTheme, Disableable as _, Icon, IconName, IndexPath, Sizable as _, Size, StyleSized as _,
    list::{ListDelegate, ListState},
};

use super::{
    delegate::{SearchableListDelegate, SearchableListItem as _},
    item::SearchableListItemElement,
};

/// Bridges a [`SearchableListDelegate`] into the [`ListDelegate`] protocol.
///
/// Parent states (`SelectState`, `ComboBoxState`) create one of these, supplying closures that
/// encode parent-specific confirm / cancel / empty-render behaviour. All state mutation stays in
/// the parent; the adapter only handles item layout.
pub(crate) struct SearchableListAdapter<D: SearchableListDelegate + 'static> {
    pub(crate) delegate: D,
    /// Keyboard cursor row — updated by `ListDelegate::set_selected_index`.
    selected_index: Option<IndexPath>,
    /// Snapshot of the parent's committed selection, kept in sync by the parent state after every
    /// selection change. `render_item` reads this directly so it never touches the parent entity
    /// (which would panic — the `ListState` entity is already locked during render).
    pub(crate) selection_snapshot: Vec<(IndexPath, D::Item)>,
    /// The last search query, passed to rows when `highlight_matches` is on.
    query: SharedString,
    /// Render rows with [`SearchableListItem::render_highlighted`].
    pub(crate) highlight_matches: bool,
    /// Called when the user confirms an item (click or Enter).
    on_confirm:
        Box<dyn Fn(Option<IndexPath>, bool, &mut Window, &mut Context<ListState<Self>>) + 'static>,
    /// Called when the user cancels (Escape) or focus leaves the dropdown.
    on_cancel: Box<dyn Fn(Option<IndexPath>, &mut Window, &mut Context<ListState<Self>>) + 'static>,
    /// Renders the empty-state placeholder.
    on_render_empty: Box<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>,
    pub(crate) size: Size,
    /// Override the trailing check icon; defaults to `IconName::Check`.
    pub(crate) check_icon: Option<Icon>,
}

impl<D: SearchableListDelegate + 'static> SearchableListAdapter<D> {
    pub(crate) fn new(
        delegate: D,
        on_confirm: impl Fn(Option<IndexPath>, bool, &mut Window, &mut Context<ListState<Self>>)
        + 'static,
        on_cancel: impl Fn(Option<IndexPath>, &mut Window, &mut Context<ListState<Self>>) + 'static,
        on_render_empty: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            delegate,
            selected_index: None,
            selection_snapshot: Vec::new(),
            query: SharedString::default(),
            highlight_matches: false,
            on_confirm: Box::new(on_confirm),
            on_cancel: Box::new(on_cancel),
            on_render_empty: Box::new(on_render_empty),
            size: Size::default(),
            check_icon: None,
        }
    }

    /// Replace the selection snapshot. Call this after every selection mutation so that
    /// `render_item` sees up-to-date check state without touching any external entity.
    pub(crate) fn update_selection_snapshot(&mut self, snapshot: Vec<(IndexPath, D::Item)>) {
        self.selection_snapshot = snapshot;
    }
}

impl<D: SearchableListDelegate + 'static> ListDelegate for SearchableListAdapter<D> {
    type Item = SearchableListItemElement;

    fn sections_count(&self, cx: &App) -> usize {
        self.delegate.sections_count(cx)
    }

    fn items_count(&self, section: usize, _: &App) -> usize {
        self.delegate.items_count(section)
    }

    fn render_section_header(
        &mut self,
        section: usize,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        if let Some(el) = self.delegate.render_section_header(section, window, cx) {
            return Some(el.into_any_element());
        }

        #[allow(deprecated)]
        let item = self.delegate.section(section)?;

        Some(
            div()
                .py_0p5()
                .px_2()
                .list_size(self.size)
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(item)
                .into_any_element(),
        )
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        use gpui::IntoElement as _;

        let item = self.delegate.item(ix)?;
        // Read check state from the snapshot — never from an external entity, which would panic
        // because the ListState entity is already locked for this render pass.
        let is_checked = self
            .delegate
            .is_item_checked(ix, item, &self.selection_snapshot, cx);
        let disabled = !self.delegate.is_item_enabled(ix, item, cx);
        let size = self.size;

        if let Some(el) = self.delegate.render_item(ix, item, is_checked, window, cx) {
            return Some(
                SearchableListItemElement::new(ix.row)
                    .disabled(disabled)
                    .with_size(size)
                    .child(el),
            );
        }

        let check_icon = self
            .check_icon
            .clone()
            .unwrap_or_else(|| Icon::new(IconName::Check));

        let content = div().min_w_0().truncate().child(if self.highlight_matches {
            item.render_highlighted(self.query.trim(), window, cx)
                .into_any_element()
        } else {
            item.render(window, cx).into_any_element()
        });

        Some(
            SearchableListItemElement::new(ix.row)
                .checked(is_checked)
                .check_icon(check_icon)
                .disabled(disabled)
                .with_size(size)
                .child(content.into_any_element()),
        )
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        let saved = self.selected_index;
        (self.on_cancel)(saved, window, cx);
    }

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        let Some(ix) = self.selected_index else {
            return;
        };
        let Some(item) = self.delegate.item(ix) else {
            return;
        };
        if !self.delegate.is_item_enabled(ix, item, cx) {
            return;
        }
        (self.on_confirm)(self.selected_index, secondary, window, cx);
    }

    fn perform_search(
        &mut self,
        query: &str,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> gpui::Task<()> {
        self.query = SharedString::from(query.to_string());
        self.delegate.perform_search(query, window, cx)
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
    }

    fn render_empty(
        &mut self,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        (self.on_render_empty)(window, cx)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext as _, IntoElement as _, SharedString, TestAppContext, Window, div};

    use super::SearchableListAdapter;
    use crate::{
        IndexPath,
        list::{ListDelegate as _, ListState},
        searchable_list::SearchableListItem,
    };

    #[derive(Clone)]
    struct RecordingItem {
        title: SharedString,
        rendered: Rc<RefCell<Vec<Option<String>>>>,
    }

    impl SearchableListItem for RecordingItem {
        type Value = SharedString;

        fn title(&self) -> SharedString {
            self.title.clone()
        }

        fn render(&self, _: &mut Window, _: &mut gpui::App) -> impl gpui::IntoElement {
            self.rendered.borrow_mut().push(None);
            self.title.clone()
        }

        fn render_highlighted(
            &self,
            query: &str,
            _: &mut Window,
            _: &mut gpui::App,
        ) -> impl gpui::IntoElement {
            self.rendered.borrow_mut().push(Some(query.to_string()));
            self.title.clone()
        }

        fn value(&self) -> &SharedString {
            &self.title
        }
    }

    #[gpui::test]
    fn rows_highlight_the_query_only_when_enabled(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let cx = cx.add_empty_window();
        let rendered = Rc::new(RefCell::new(Vec::new()));

        cx.update(|window, cx| {
            let items = vec![RecordingItem {
                title: "Rust".into(),
                rendered: rendered.clone(),
            }];
            let list = cx.new(|cx| {
                let adapter = SearchableListAdapter::new(
                    items,
                    |_, _, _, _| {},
                    |_, _, _| {},
                    |_, _| div().into_any_element(),
                );
                ListState::new(adapter, window, cx).searchable(true)
            });
            let render_row = |window: &mut Window, cx: &mut gpui::App| {
                list.update(cx, |list, cx| {
                    _ = list
                        .delegate_mut()
                        .render_item(IndexPath::new(0), window, cx);
                });
            };

            list.update(cx, |list, cx| list.set_query(" ru ", window, cx));
            render_row(window, cx);
            list.update(cx, |list, _| list.delegate_mut().highlight_matches = true);
            render_row(window, cx);
            list.update(cx, |list, cx| list.set_query("", window, cx));
            render_row(window, cx);
        });

        assert_eq!(
            *rendered.borrow(),
            [None, Some("ru".to_string()), Some(String::new())]
        );
    }
}
