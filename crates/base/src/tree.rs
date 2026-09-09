use crate::TestSupportExt as _;
use gpui::StatefulInteractiveElement as _;
use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, Context, ElementId, Entity, EventEmitter, FocusHandle, InteractiveElement,
    IntoElement, KeyBinding, MouseButton, ParentElement, Render, RenderOnce, SharedString,
    StyleRefinement, Styled, UniformListScrollHandle, Window, div, prelude::FluentBuilder as _,
    uniform_list,
};

use crate::{
    actions::{Confirm, SelectDown, SelectLeft, SelectRight, SelectUp},
    styled::StyledExt as _,
};

const CONTEXT: &str = "Tree";

#[doc(hidden)]
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
    ]);
}

#[doc(hidden)]
pub const fn key_context() -> &'static str {
    CONTEXT
}

struct TreeItemState {
    expanded: bool,
    disabled: bool,
}

/// A tree item with a stable id, display label, children, and shared state.
#[derive(Clone)]
pub struct TreeItem {
    pub id: SharedString,
    pub label: SharedString,
    pub children: Vec<TreeItem>,
    state: Rc<RefCell<TreeItemState>>,
}

/// A flat representation of a tree item with its depth.
#[derive(Clone)]
pub struct TreeEntry {
    item: TreeItem,
    depth: usize,
}

impl TreeEntry {
    pub fn new(item: TreeItem, depth: usize) -> Self {
        Self { item, depth }
    }

    #[inline]
    pub fn item(&self) -> &TreeItem {
        &self.item
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub fn is_root(&self) -> bool {
        self.depth == 0
    }

    #[inline]
    pub fn is_folder(&self) -> bool {
        self.item.is_folder()
    }

    #[inline]
    pub fn is_expanded(&self) -> bool {
        self.item.is_expanded()
    }

    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.item.is_disabled()
    }
}

/// Event emitted by a tree when user-visible expansion state changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeEvent {
    Expanded(SharedString),
    Collapsed(SharedString),
}

impl TreeItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
            state: Rc::new(RefCell::new(TreeItemState {
                expanded: false,
                disabled: false,
            })),
        }
    }

    pub fn child(mut self, child: TreeItem) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = TreeItem>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn expanded(self, expanded: bool) -> Self {
        self.state.borrow_mut().expanded = expanded;
        self
    }

    pub fn disabled(self, disabled: bool) -> Self {
        self.state.borrow_mut().disabled = disabled;
        self
    }

    #[inline]
    pub fn is_folder(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn is_disabled(&self) -> bool {
        self.state.borrow().disabled
    }

    #[inline]
    pub fn is_expanded(&self) -> bool {
        self.state.borrow().expanded
    }

    /// Returns the target's ancestors from nearest parent to root.
    pub fn ancestors(&self, target_id: &SharedString) -> Option<Vec<TreeItem>> {
        if self.id == *target_id {
            return Some(Vec::new());
        }

        for child in &self.children {
            if let Some(mut path) = child.ancestors(target_id) {
                path.push(self.clone());
                return Some(path);
            }
        }

        None
    }
}

/// The interaction state supplied while rendering a visible tree entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TreeEntryState {
    selected: bool,
    right_clicked: bool,
}

impl TreeEntryState {
    #[inline]
    pub fn is_selected(self) -> bool {
        self.selected
    }

    #[inline]
    pub fn is_right_clicked(self) -> bool {
        self.right_clicked
    }
}

type RenderItem = dyn Fn(usize, &TreeEntry, TreeEntryState, &mut Window, &mut App) -> AnyElement;

/// Behavior and interaction state for a virtualized tree.
pub struct TreeState {
    focus_handle: FocusHandle,
    entries: Vec<TreeEntry>,
    scroll_handle: UniformListScrollHandle,
    selected_ix: Option<usize>,
    right_clicked_ix: Option<usize>,
    render_item: Rc<RenderItem>,
    list_style: StyleRefinement,
}

impl EventEmitter<TreeEvent> for TreeState {}

impl TreeState {
    pub fn new(cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            entries: Vec::new(),
            scroll_handle: UniformListScrollHandle::default(),
            selected_ix: None,
            right_clicked_ix: None,
            render_item: Rc::new(|_, _, _, _, _| div().into_any_element()),
            list_style: StyleRefinement::default(),
        }
    }

    pub fn items(mut self, items: impl Into<Vec<TreeItem>>) -> Self {
        self.replace_items(items.into());
        self
    }

    pub fn set_items(&mut self, items: impl Into<Vec<TreeItem>>, cx: &mut Context<Self>) {
        self.replace_items(items.into());
        self.selected_ix = None;
        self.right_clicked_ix = None;
        cx.notify();
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_ix
    }

    pub fn set_selected_index(&mut self, ix: Option<usize>, cx: &mut Context<Self>) {
        self.selected_ix = ix;
        cx.notify();
    }

    pub fn set_selected_item(&mut self, item: Option<&TreeItem>, cx: &mut Context<Self>) {
        if let Some(item) = item {
            self.selected_ix = self.index_of(&item.id);
            if self.selected_ix.is_none() {
                self.expand_ancestors(item.id.clone(), cx);
                self.selected_ix = self.index_of(&item.id);
            }
        } else {
            self.selected_ix = None;
        }
        cx.notify();
    }

    pub fn selected_item(&self) -> Option<&TreeItem> {
        self.selected_ix
            .and_then(|ix| self.entries.get(ix).map(TreeEntry::item))
    }

    pub fn selected_entry(&self) -> Option<&TreeEntry> {
        self.selected_ix.and_then(|ix| self.entries.get(ix))
    }

    pub fn entry(&self, ix: usize) -> Option<&TreeEntry> {
        self.entries.get(ix)
    }

    pub fn scroll_handle(&self) -> &UniformListScrollHandle {
        &self.scroll_handle
    }

    pub fn scroll_to_item(&mut self, ix: usize, strategy: gpui::ScrollStrategy) {
        self.scroll_handle.scroll_to_item(ix, strategy);
    }

    pub fn index_of(&self, id: &SharedString) -> Option<usize> {
        self.entries.iter().position(|entry| &entry.item.id == id)
    }

    pub fn reveal_item(
        &mut self,
        id: &SharedString,
        strategy: gpui::ScrollStrategy,
        cx: &mut Context<Self>,
    ) {
        self.expand_ancestors(id.clone(), cx);
        if let Some(ix) = self.index_of(id) {
            self.scroll_to_item(ix, strategy);
        }
    }

    pub fn focus(&mut self, window: &mut Window, cx: &mut App) {
        self.focus_handle.focus(window, cx);
    }

    fn replace_items(&mut self, items: Vec<TreeItem>) {
        self.entries.clear();
        for item in items {
            self.add_entry(item, 0);
        }
    }

    fn expand_ancestors(&mut self, target_id: SharedString, cx: &mut Context<Self>) {
        let ancestors = self
            .entries
            .iter()
            .find_map(|entry| entry.item.ancestors(&target_id))
            .unwrap_or_default();

        if ancestors.is_empty() {
            return;
        }

        for ancestor in ancestors.into_iter().rev() {
            if !ancestor.is_expanded() {
                ancestor.state.borrow_mut().expanded = true;
                cx.emit(TreeEvent::Expanded(ancestor.id.clone()));
            }
        }
        self.rebuild_entries();
    }

    fn add_entry(&mut self, item: TreeItem, depth: usize) {
        self.entries.push(TreeEntry::new(item.clone(), depth));
        if item.is_expanded() {
            for child in &item.children {
                self.add_entry(child.clone(), depth + 1);
            }
        }
    }

    fn toggle_expand(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(entry) = self.entries.get(ix) else {
            return;
        };
        if !entry.is_folder() {
            return;
        }

        let expanded = !entry.is_expanded();
        let id = entry.item.id.clone();
        entry.item.state.borrow_mut().expanded = expanded;
        cx.emit(if expanded {
            TreeEvent::Expanded(id)
        } else {
            TreeEvent::Collapsed(id)
        });
        self.right_clicked_ix = None;
        self.rebuild_entries();
    }

    fn rebuild_entries(&mut self) {
        let roots = self
            .entries
            .iter()
            .filter(|entry| entry.is_root())
            .map(|entry| entry.item.clone())
            .collect::<Vec<_>>();
        self.replace_items(roots);
    }

    fn on_action_confirm(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        if self
            .selected_ix
            .and_then(|ix| self.entries.get(ix).map(|entry| (ix, entry.is_folder())))
            .is_some_and(|(ix, is_folder)| {
                if is_folder {
                    self.toggle_expand(ix, cx);
                }
                is_folder
            })
        {
            cx.notify();
        }
    }

    fn on_action_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.selected_ix
            && self
                .entries
                .get(ix)
                .is_some_and(|entry| entry.is_folder() && entry.is_expanded())
        {
            self.toggle_expand(ix, cx);
            cx.notify();
        }
    }

    fn on_action_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.selected_ix
            && self
                .entries
                .get(ix)
                .is_some_and(|entry| entry.is_folder() && !entry.is_expanded())
        {
            self.toggle_expand(ix, cx);
            cx.notify();
        }
    }

    fn on_action_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        let mut ix = self.selected_ix.unwrap_or(0);
        ix = ix
            .checked_sub(1)
            .unwrap_or_else(|| self.entries.len().saturating_sub(1));
        self.selected_ix = Some(ix);
        self.scroll_handle
            .scroll_to_item(ix, gpui::ScrollStrategy::Top);
        cx.notify();
    }

    fn on_action_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        let mut ix = self.selected_ix.unwrap_or(0);
        ix = if ix + 1 < self.entries.len() {
            ix + 1
        } else {
            0
        };
        self.selected_ix = Some(ix);
        self.scroll_handle
            .scroll_to_item(ix, gpui::ScrollStrategy::Bottom);
        cx.notify();
    }

    fn on_entry_click(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.selected_ix = Some(ix);
        self.toggle_expand(ix, cx);
        cx.notify();
    }
}

impl Render for TreeState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let render_item = self.render_item.clone();
        uniform_list("entries", self.entries.len(), {
            cx.processor(move |state, visible_range: Range<usize>, window, cx| {
                visible_range
                    .map(|ix| {
                        let entry = &state.entries[ix];
                        let entry_state = TreeEntryState {
                            selected: Some(ix) == state.selected_ix,
                            right_clicked: Some(ix) == state.right_clicked_ix,
                        };
                        div()
                            .id(ix)
                            .test_support()
                            .role(gpui::Role::TreeItem)
                            .aria_label(entry.item().label.clone())
                            .aria_selected(entry_state.selected)
                            .when(entry.is_folder(), |this| {
                                this.aria_expanded(entry.is_expanded())
                            })
                            .child((render_item)(ix, entry, entry_state, window, cx))
                            .when(!entry.is_disabled(), |this| {
                                this.on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |state, _, _, cx| {
                                        state.on_entry_click(ix, cx);
                                    }),
                                )
                                .on_mouse_down(
                                    MouseButton::Right,
                                    cx.listener(move |state, _, _, cx| {
                                        state.right_clicked_ix = Some(ix);
                                        cx.notify();
                                    }),
                                )
                            })
                    })
                    .collect()
            })
        })
        .track_scroll(&self.scroll_handle)
        .refine_style(&self.list_style)
    }
}

/// An unstyled, virtualized tree element.
#[derive(IntoElement)]
pub struct Tree {
    id: ElementId,
    state: Entity<TreeState>,
    style: StyleRefinement,
    list_style: StyleRefinement,
    render_item: Rc<RenderItem>,
}

impl Tree {
    pub fn new(state: &Entity<TreeState>) -> Self {
        Self {
            id: ElementId::Name(format!("tree-{}", state.entity_id()).into()),
            state: state.clone(),
            style: StyleRefinement::default(),
            list_style: StyleRefinement::default(),
            render_item: Rc::new(|_, _, _, _, _| div().into_any_element()),
        }
    }

    /// Supplies the application-owned content for each visible entry.
    pub fn item<R>(mut self, render_item: R) -> Self
    where
        R: Fn(usize, &TreeEntry, TreeEntryState, &mut Window, &mut App) -> AnyElement + 'static,
    {
        self.render_item = Rc::new(render_item);
        self
    }

    /// Applies caller-owned presentation to the internal virtual list.
    pub fn list_style(mut self, style: StyleRefinement) -> Self {
        self.list_style = style;
        self
    }
}

impl Styled for Tree {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Tree {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = self.state.read(cx).focus_handle.clone();
        self.state.update(cx, |state, _| {
            state.render_item = self.render_item;
            state.list_style = self.list_style;
        });

        div()
            .id(self.id)
            .test_support()
            .role(gpui::Role::Tree)
            .key_context(CONTEXT)
            .track_focus(&focus_handle)
            .on_action(window.listener_for(&self.state, TreeState::on_action_confirm))
            .on_action(window.listener_for(&self.state, TreeState::on_action_left))
            .on_action(window.listener_for(&self.state, TreeState::on_action_right))
            .on_action(window.listener_for(&self.state, TreeState::on_action_up))
            .on_action(window.listener_for(&self.state, TreeState::on_action_down))
            .child(self.state)
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, Subscription};

    struct EventCollector {
        events: Rc<RefCell<Vec<TreeEvent>>>,
        _subscription: Subscription,
    }

    impl EventCollector {
        fn new(state: &Entity<TreeState>, cx: &mut Context<Self>) -> Self {
            let events = Rc::new(RefCell::new(Vec::new()));
            let captured = events.clone();
            let subscription = cx.subscribe(state, move |_, _, event, _| {
                captured.borrow_mut().push(event.clone());
            });
            Self {
                events,
                _subscription: subscription,
            }
        }
    }

    impl Render for EventCollector {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    #[test]
    fn clones_share_state_and_ancestors_keep_nearest_first_order() {
        let leaf = TreeItem::new("leaf", "Leaf");
        let branch = TreeItem::new("branch", "Branch").child(leaf.clone());
        let root = TreeItem::new("root", "Root").child(branch.clone());

        leaf.clone().disabled(true).expanded(true);
        assert!(leaf.is_disabled());
        assert!(leaf.is_expanded());

        let ancestors = root.ancestors(&"leaf".into()).unwrap();
        assert_eq!(
            ancestors
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["branch", "root"]
        );
    }

    #[gpui::test]
    fn state_flattens_expanded_items_and_resets_selection(cx: &mut gpui::TestAppContext) {
        let items = vec![
            TreeItem::new("src", "src")
                .expanded(true)
                .child(TreeItem::new("src/lib.rs", "lib.rs")),
            TreeItem::new("README.md", "README.md"),
        ];
        let state = cx.new(|cx| TreeState::new(cx).items(items));

        state.update(cx, |state, cx| {
            assert_eq!(state.entries.len(), 3);
            assert_eq!(state.entries[1].depth(), 1);
            state.set_selected_index(Some(1), cx);
            state.set_items(vec![TreeItem::new("Cargo.toml", "Cargo.toml")], cx);
            assert_eq!(state.selected_index(), None);
            assert_eq!(state.entries.len(), 1);
        });
    }

    #[gpui::test]
    fn selecting_hidden_item_expands_its_ancestors(cx: &mut gpui::TestAppContext) {
        let target = TreeItem::new("src/ui/tree.rs", "tree.rs");
        let root =
            TreeItem::new("src", "src").child(TreeItem::new("src/ui", "ui").child(target.clone()));
        let state = cx.new(|cx| TreeState::new(cx).items(vec![root]));

        state.update(cx, |state, cx| {
            state.set_selected_item(Some(&target), cx);
            assert_eq!(state.entries.len(), 3);
            assert_eq!(
                state.selected_item().map(|item| item.id.as_str()),
                Some("src/ui/tree.rs")
            );
        });
    }

    #[gpui::test]
    fn toggling_folder_rebuilds_visible_entries(cx: &mut gpui::TestAppContext) {
        let root = TreeItem::new("src", "src").child(TreeItem::new("src/lib.rs", "lib.rs"));
        let state = cx.new(|cx| TreeState::new(cx).items(vec![root]));

        state.update(cx, |state, cx| {
            state.toggle_expand(0, cx);
            assert_eq!(state.entries.len(), 2);
            state.toggle_expand(0, cx);
            assert_eq!(state.entries.len(), 1);
        });
    }

    #[gpui::test]
    fn expansion_events_preserve_ids_and_set_items_stays_silent(cx: &mut gpui::TestAppContext) {
        let root = TreeItem::new("src", "src").child(TreeItem::new("src/lib.rs", "lib.rs"));
        let state = cx.new(|cx| TreeState::new(cx).items(vec![root]));
        let collector = cx.new(|cx| EventCollector::new(&state, cx));

        state.update(cx, |state, cx| {
            state.toggle_expand(0, cx);
            state.toggle_expand(0, cx);
            state.set_items(vec![TreeItem::new("README.md", "README.md")], cx);
        });

        let events = collector.read_with(cx, |collector, _| collector.events.borrow().clone());
        assert_eq!(
            events,
            vec![
                TreeEvent::Expanded("src".into()),
                TreeEvent::Collapsed("src".into()),
            ]
        );
    }
}
