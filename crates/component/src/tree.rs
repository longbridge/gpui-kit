use std::rc::Rc;

use gpui::{
    App, Context, ElementId, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use crate::{
    Selectable as _, StyledExt,
    list::ListItem,
    menu::{ContextMenuExt as _, KeyboardTrigger, PopupMenu, ShowContextMenu, open_from_keyboard},
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
    scroll_bounce: Option<bool>,
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
            scroll_bounce: None,
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

    /// Bounce at the vertical boundaries. On by default on iOS, Android and
    /// macOS, off elsewhere; reduced motion disables displacement.
    pub fn scroll_bounce(mut self, enabled: bool) -> Self {
        self.scroll_bounce = Some(enabled);
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
        // The selected row's menu, for Shift+F10 while the tree has focus:
        // the rows are drawn inside the focused tree, not around it, so the
        // tree opens it for them.
        let selected_menu: Rc<std::cell::RefCell<Option<KeyboardTrigger>>> = Rc::default();
        let row_menu = selected_menu.clone();

        div()
            .id(self.id)
            .size_full()
            .on_action(move |_: &ShowContextMenu, window, cx| {
                let opened = selected_menu
                    .borrow()
                    .as_ref()
                    .is_some_and(|trigger| open_from_keyboard(trigger, window, cx));
                if !opened {
                    cx.propagate();
                }
            })
            .child(
                gpui_base::Tree::new(&self.state)
                    .item(move |ix, entry, entry_state, window, cx| {
                        // Capture only the id: cloning the entry would deep-clone
                        // its whole subtree for every visible row on every frame.
                        let id = entry.item().id.clone();
                        let context_menu_builder = context_menu_builder.clone();
                        let context_menu_state = state.clone();
                        let item = render_item(ix, entry, entry_state.is_selected(), window, cx)
                            .disabled(entry.is_disabled())
                            .selected(entry_state.is_selected())
                            .secondary_selected(entry_state.is_right_clicked());

                        let row = div().child(item).context_menu(move |menu, window, cx| {
                            let Some(build) = context_menu_builder.as_ref() else {
                                return menu;
                            };
                            context_menu_state.update(cx, |state, cx| {
                                let Some(entry) = state
                                    .entry(ix)
                                    .filter(|entry| entry.item().id == id)
                                    .filter(|entry| !entry.is_disabled())
                                    .cloned()
                                else {
                                    return menu;
                                };
                                build(ix, &entry, menu, window, cx)
                            })
                        });
                        if entry_state.is_selected() {
                            *row_menu.borrow_mut() = Some(row.keyboard_trigger());
                        }
                        row.into_any_element()
                    })
                    .list_style(StyleRefinement::default().flex_grow_1().size_full())
                    .relative()
                    .size_full()
                    .map(|tree| {
                        gpui_base::ScrollBounce::new("tree-scroll-bounce", &scroll_handle, tree)
                            .when_some(self.scroll_bounce, |bounce, on| bounce.enabled(on))
                    }),
            )
            .refine_style(&self.style)
            .vertical_scrollbar(&scroll_handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::PopupMenuItem;
    use gpui::{AppContext as _, Render, TestAppContext, px};
    use std::cell::RefCell;

    /// A tree with a menu on each row, noting each row whose menu ran.
    struct TreeRoot {
        tree: Entity<TreeState>,
        ran: Rc<RefCell<Vec<gpui::SharedString>>>,
    }

    impl Render for TreeRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let ran = self.ran.clone();
            div().size_full().child(
                tree(&self.tree, |ix, entry, _, _, _| {
                    ListItem::new(ix)
                        .h(px(24.))
                        .child(entry.item().label.clone())
                })
                .context_menu(move |_, entry, menu, _, _| {
                    let (ran, id) = (ran.clone(), entry.item().id.clone());
                    menu.item(
                        PopupMenuItem::new("Open")
                            .on_click(move |_, _, _| ran.borrow_mut().push(id.clone())),
                    )
                }),
            )
        }
    }

    #[gpui::test]
    fn shift_f10_opens_the_selected_rows_menu(cx: &mut TestAppContext) {
        cx.update(|cx| crate::init(cx));
        let ran = Rc::new(RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let ran = ran.clone();
            move |window, cx| {
                let items = vec![TreeItem::new("a", "a"), TreeItem::new("b", "b")];
                let tree = cx.new(|cx| TreeState::new(cx).items(items));
                tree.update(cx, |tree, cx| {
                    tree.set_selected_index(Some(1), cx);
                    tree.focus(window, cx);
                });
                TreeRoot { tree, ran }
            }
        });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });

        cx.simulate_keystrokes("shift-f10");
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("down enter");
        cx.run_until_parked();

        assert_eq!(ran.borrow().as_slice(), ["b"]);
    }

    #[test]
    fn legacy_tree_types_are_base_types() {
        fn accepts_base(_: gpui_base::TreeItem) {}
        accepts_base(TreeItem::new("id", "Label"));
    }
}
