use gpui_kit::component::{
    list::ListItem,
    table::{Column, DataTable, TableDelegate, TableState},
    tree::{Tree, TreeItem, TreeState},
};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, TestAppContext, Window, div, prelude::*, px, size,
};

struct Files {
    tree: Entity<TreeState>,
}
impl Render for Files {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(Tree::new(&self.tree, |ix, entry, _, _, _| {
                ListItem::new(("file", ix)).child(entry.item().label.clone())
            }))
    }
}
#[gpui_kit::test]
fn tree_pointer_and_keyboard_expand_collapse_and_select_nodes(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(480.), px(320.)), |window, cx| {
        let tree = cx.new(|cx| {
            TreeState::new(cx).items(vec![
                TreeItem::new("src", "src").child(TreeItem::new("main", "main.rs")),
                TreeItem::new("tests", "tests"),
            ])
        });
        tree.update(cx, |tree, cx| tree.focus(window, cx));
        Files { tree }
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find(0usize).expanded(), Some(false));
        window.click(0usize, cx);
        assert_eq!(window.find(0usize).expanded(), Some(true));
        assert_eq!(window.find(1usize).label(), Some("main.rs"));
        assert!(window.find(1usize).bounds().top() >= window.find(0usize).bounds().bottom());
        window.press("left", cx);
        assert_eq!(window.find(0usize).expanded(), Some(false));
        assert_eq!(window.find(1usize).label(), Some("tests"));
        window.press("right", cx);
        window.press("down", cx);
        assert_eq!(window.find(1usize).selected(), Some(true));
        assert_eq!(window.find(0usize).selected(), Some(false));
    })
    .unwrap();
}

struct Rows;
impl TableDelegate for Rows {
    fn columns_count(&self, _: &App) -> usize {
        2
    }
    fn rows_count(&self, _: &App) -> usize {
        200
    }
    fn column(&self, ix: usize, _: &App) -> Column {
        Column::new(
            format!("column-{ix}"),
            if ix == 0 { "Name" } else { "Status" },
        )
        .width(px(180.))
    }
    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div().child(format!("{row}:{col}"))
    }
}
struct Records {
    table: Entity<TableState<Rows>>,
}
impl Render for Records {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(DataTable::new(&self.table))
    }
}
#[gpui_kit::test]
fn table_selects_rows_and_keyboard_scrolls_virtualized_content(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(640.), px(320.)), |window, cx| Records {
        table: cx.new(|cx| TableState::new(Rows, window, cx).row_selectable(true)),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find(("row", 150usize)).is_none());
        window.click(("row", 1usize), cx);
        assert_eq!(window.find(("row", 1usize)).selected(), Some(true));
        window.press("down", cx);
        assert_eq!(window.find(("row", 2usize)).selected(), Some(true));
        let viewport = window.find("table").bounds();
        for _ in 0..30 {
            window.press("down", cx);
        }
        let selected = window.find(("row", 32usize));
        assert_eq!(selected.selected(), Some(true));
        assert!(selected.bounds().top() >= viewport.top());
        assert!(selected.bounds().bottom() <= viewport.bottom());
        assert!(window.try_find(("row", 1usize)).is_none());
        window.scroll(
            "table",
            gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(2000.))),
            cx,
        );
        assert!(window.find(("row", 1usize)).visible());
        assert!(window.try_find(("row", 32usize)).is_none());
    })
    .unwrap();
}
#[gpui_kit::test]
fn table_keyboard_leaves_rows_unselected_when_rows_are_not_selectable(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.open_window(size(px(640.), px(320.)), |window, cx| Records {
        table: cx.new(|cx| TableState::new(Rows, window, cx).row_selectable(false)),
    });
    let table = cx
        .update_window(handle.into(), |root, _, cx| {
            root.downcast::<Records>().unwrap().read(cx).table.clone()
        })
        .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click(("row", 1usize), cx);
        assert!(table.focus_handle(cx).is_focused(window));
        assert_eq!(table.read(cx).selected_row(), None);
        for key in ["down", "down", "pagedown", "up", "pageup"] {
            window.press(key, cx);
            assert_eq!(
                table.read(cx).selected_row(),
                None,
                "`{key}` moved the row selection"
            );
            assert_eq!(window.find(("row", 0usize)).selected(), Some(false));
        }
        assert!(window.find(("row", 0usize)).visible());
    })
    .unwrap();
}
