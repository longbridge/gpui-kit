use std::rc::Rc;

use gpui::{
    App, Context, ElementId, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, StyleRefinement, Styled, Window, div,
};

use crate::{
    Selectable as _, StyledExt,
    list::ListItem,
    menu::{ContextMenuExt as _, PopupMenu},
    scroll::ScrollableElement,
};

pub use gpui_base::{TreeEntry, TreeEvent, TreeItem, TreeState};

/// Create a [`Tree`].
pub fn tree<R>(state: &Entity<TreeState>, render_item: R) -> Tree
where
    R: Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem + 'static,
{
    Tree::new(state, render_item)
}

type RenderItem = dyn Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem;
type ContextMenuBuilder =
    dyn Fn(usize, &TreeEntry, PopupMenu, &mut Window, &mut Context<TreeState>) -> PopupMenu;

/// A styled tree view that preserves the legacy `gpui-component` API while
/// delegating tree behavior and interaction state to `gpui-base`.
#[derive(IntoElement)]
pub struct Tree {
    id: ElementId,
    state: Entity<TreeState>,
    style: StyleRefinement,
    render_item: Rc<RenderItem>,
    context_menu_builder: Option<Rc<ContextMenuBuilder>>,
}

impl Tree {
    pub fn new<R>(state: &Entity<TreeState>, render_item: R) -> Self
    where
        R: Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem + 'static,
    {
        Self {
            id: ElementId::Name(format!("tree-{}", state.entity_id()).into()),
            state: state.clone(),
            style: StyleRefinement::default(),
            render_item: Rc::new(render_item),
            context_menu_builder: None,
        }
    }

    /// Add a context menu to the tree.
    pub fn context_menu<F>(mut self, f: F) -> Self
    where
        F: Fn(usize, &TreeEntry, PopupMenu, &mut Window, &mut Context<TreeState>) -> PopupMenu
            + 'static,
    {
        self.context_menu_builder = Some(Rc::new(f));
        self
    }
}

impl Styled for Tree {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Tree {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.clone();
        let render_item = self.render_item;
        let context_menu_builder = self.context_menu_builder;
        let scroll_handle = self.state.read(cx).scroll_handle().clone();

        div()
            .id(self.id)
            .size_full()
            .child(
                gpui_base::Tree::new(&self.state)
                    .item(move |ix, entry, entry_state, window, cx| {
                        let entry = entry.clone();
                        let context_menu_builder = context_menu_builder.clone();
                        let context_menu_state = state.clone();
                        let item = render_item(ix, &entry, entry_state.is_selected(), window, cx)
                            .disabled(entry.is_disabled())
                            .selected(entry_state.is_selected())
                            .secondary_selected(entry_state.is_right_clicked());

                        div()
                            .child(item)
                            .context_menu(move |menu, window, cx| {
                                let Some(build) = context_menu_builder
                                    .as_ref()
                                    .filter(|_| !entry.is_disabled())
                                else {
                                    return menu;
                                };
                                context_menu_state
                                    .update(cx, |_, cx| build(ix, &entry, menu, window, cx))
                            })
                            .into_any_element()
                    })
                    .list_style(StyleRefinement::default().flex_grow_1().size_full())
                    .relative()
                    .size_full(),
            )
            .refine_style(&self.style)
            .vertical_scrollbar(&scroll_handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_tree_types_are_base_types() {
        fn accepts_base(_: gpui_base::TreeItem) {}
        accepts_base(TreeItem::new("id", "Label"));
    }
}
