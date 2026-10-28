use crate::common;
use std::ops::Range;

use gpui_kit::component::{
    IndexPath,
    list::{List, ListDelegate, ListItem, ListState},
    table::{Column, ColumnSort, DataTable, TableDelegate, TableEvent, TableSelection, TableState},
    tree::{Tree, TreeItem, TreeState},
};
use gpui_kit::test::{ClickOptions, TestSupportExt, TestWindowExt};
use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, Modifiers, TestAppContext, Window, div, point,
    prelude::*, px, size,
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
    let (handle, _) = common::open_window(cx, Some(size(px(480.), px(320.))), |window, cx| {
        cx.new(|cx| {
            let tree = cx.new(|cx| {
                TreeState::new(cx).items(vec![
                    TreeItem::new("src", "src").child(TreeItem::new("main", "main.rs")),
                    TreeItem::new("tests", "tests"),
                ])
            });
            tree.update(cx, |tree, cx| tree.focus(window, cx));
            Files { tree }
        })
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
fn table_selection_getters_follow_the_active_mode(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, handle_content) =
        common::open_window(cx, Some(size(px(640.), px(320.))), |window, cx| {
            cx.new(|cx| Records {
                table: cx.new(|cx| TableState::new(Rows, window, cx).cell_selectable(true)),
            })
        });
    cx.update_window(handle.into(), |_root, _, cx| {
        let table = handle_content.clone().read(cx).table.clone();
        table.update(cx, |table, cx| {
            let selection = |table: &TableState<Rows>| {
                (
                    table.selection(),
                    table.selected_row(),
                    table.selected_col(),
                    table.selected_cell(),
                )
            };
            assert_eq!(selection(table), (TableSelection::None, None, None, None));
            table.set_selected_cell(5, 1, cx);
            assert_eq!(
                selection(table),
                (TableSelection::Cell(5, 1), None, None, Some((5, 1)))
            );
            table.set_selected_row(3, cx);
            assert_eq!(
                selection(table),
                (TableSelection::Row(3), Some(3), None, None)
            );
            table.set_selected_cell(4, 0, cx);
            assert_eq!(
                selection(table),
                (TableSelection::Cell(4, 0), None, None, Some((4, 0)))
            );
            table.set_selected_col(1, cx);
            assert_eq!(
                selection(table),
                (TableSelection::Column(1), None, Some(1), None)
            );
            table.set_selected_row(2, cx);
            assert_eq!(
                selection(table),
                (TableSelection::Row(2), Some(2), None, None)
            );
            table.set_selected_col(0, cx);
            assert_eq!(
                selection(table),
                (TableSelection::Column(0), None, Some(0), None)
            );
            table.set_selected_cell(1, 1, cx);
            assert_eq!(
                selection(table),
                (TableSelection::Cell(1, 1), None, None, Some((1, 1)))
            );
            table.clear_selection(cx);
            assert_eq!(selection(table), (TableSelection::None, None, None, None));

            // `set_selection` round-trips through `selection()`.
            for value in [
                TableSelection::Row(7),
                TableSelection::Column(1),
                TableSelection::Cell(3, 0),
                TableSelection::None,
            ] {
                table.set_selection(value, cx);
                assert_eq!(table.selection(), value);
            }
            assert_eq!(selection(table), (TableSelection::None, None, None, None));
        });
    })
    .unwrap();
}

/// `Rows`, with each cell's text also given to assistive technology.
struct LabelledRows;
impl TableDelegate for LabelledRows {
    fn columns_count(&self, cx: &App) -> usize {
        Rows.columns_count(cx)
    }
    fn rows_count(&self, cx: &App) -> usize {
        Rows.rows_count(cx)
    }
    fn column(&self, ix: usize, cx: &App) -> Column {
        Rows.column(ix, cx)
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
    fn cell_label(&self, row: usize, col: usize, _: &App) -> Option<gpui_kit::SharedString> {
        Some(format!("{row}:{col}").into())
    }
}
struct LabelledRecords {
    table: Entity<TableState<LabelledRows>>,
}
impl Render for LabelledRecords {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(DataTable::new(&self.table))
    }
}

/// Plain text in a cell is invisible to a screen reader, so the table exposes
/// its own structure: a grid, column headers named after their columns, and
/// cells named by the delegate.
#[gpui_kit::test]
fn table_exposes_its_header_and_cells_to_assistive_technology(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, Some(size(px(640.), px(320.))), |window, cx| {
        cx.new(|cx| LabelledRecords {
            table: cx.new(|cx| TableState::new(LabelledRows, window, cx)),
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(
            window.find("table-inner").role(),
            Some(gpui_kit::Role::Grid)
        );
        for (ix, name) in ["Name", "Status"].into_iter().enumerate() {
            let header = window.find(("col-header", ix));
            assert_eq!(header.role(), Some(gpui_kit::Role::ColumnHeader));
            assert_eq!(header.label(), Some(name));
        }
        let cell = window.find("table-cell-3:1");
        assert_eq!(cell.role(), Some(gpui_kit::Role::Cell));
        assert_eq!(cell.label(), Some("3:1"));
    })
    .unwrap();
}

/// In cell selection mode each cell is a grid cell that reports whether it is
/// selected, which is how a screen reader learns the selection.
#[gpui_kit::test]
fn selectable_cells_report_their_selection(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, content) =
        common::open_window(cx, Some(size(px(640.), px(320.))), |window, cx| {
            cx.new(|cx| LabelledRecords {
                table: cx.new(|cx| TableState::new(LabelledRows, window, cx).cell_selectable(true)),
            })
        });
    cx.update_window(handle.into(), |_, window, cx| {
        let table = content.read(cx).table.clone();
        table.update(cx, |table, cx| table.set_selected_cell(2, 1, cx));
        window.render_frame(cx);
        let selected = window.find("table-cell-2:1");
        assert_eq!(selected.role(), Some(gpui_kit::Role::GridCell));
        assert_eq!(selected.selected(), Some(true));
        assert_eq!(window.find("table-cell-2:0").selected(), Some(false));
    })
    .unwrap();
}

struct RecordsBelowLabel {
    table: Entity<TableState<Rows>>,
}
impl Render for RecordsBelowLabel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(
                div()
                    .id("outside")
                    .test_support()
                    .h(px(40.))
                    .child("outside"),
            )
            .child(div().h(px(280.)).child(DataTable::new(&self.table)))
    }
}

/// Right clicks the first cell of `row`, past the row header; the cells
/// themselves are not observed.
fn right_click_row(window: &mut Window, row: usize, cx: &mut App) {
    use gpui_kit::InputEvent as _;
    let position = window.find(("row", row)).bounds().origin + point(px(100.), px(5.));
    window.dispatch_event(
        gpui_kit::MouseMoveEvent {
            position,
            pressed_button: None,
            modifiers: Default::default(),
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
    window.dispatch_event(
        gpui_kit::MouseDownEvent {
            button: gpui_kit::MouseButton::Right,
            position,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        }
        .to_platform_input(),
        cx,
    );
    window.dispatch_event(
        gpui_kit::MouseUpEvent {
            button: gpui_kit::MouseButton::Right,
            position,
            modifiers: Default::default(),
            click_count: 1,
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
}

/// Rows whose second cell selects its row on mouse down.
struct RowsSelectedOnPress;
impl TableDelegate for RowsSelectedOnPress {
    fn columns_count(&self, _: &App) -> usize {
        2
    }
    fn rows_count(&self, _: &App) -> usize {
        3
    }
    fn column(&self, ix: usize, _: &App) -> Column {
        Column::new(format!("column-{ix}"), format!("Column {ix}")).width(px(180.))
    }
    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div().size_full().when(col == 1, |this| {
            this.on_mouse_down(
                gpui_kit::MouseButton::Left,
                cx.listener(move |table, _, _, cx| table.set_selected_row(row, cx)),
            )
        })
    }
}
struct PressRecords {
    table: Entity<TableState<RowsSelectedOnPress>>,
}
impl Render for PressRecords {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(DataTable::new(&self.table))
    }
}
fn open_press_records(
    cx: &mut TestAppContext,
    cell_selectable: bool,
) -> (
    gpui_kit::AnyWindowHandle,
    Entity<TableState<RowsSelectedOnPress>>,
) {
    cx.update(gpui_kit::init);
    let (handle, content) =
        common::open_window(cx, Some(size(px(640.), px(320.))), move |window, cx| {
            cx.new(|cx| PressRecords {
                table: cx.new(|cx| {
                    TableState::new(RowsSelectedOnPress, window, cx)
                        .cell_selectable(cell_selectable)
                }),
            })
        });
    let table = cx
        .update_window(handle.into(), |_, _, cx| content.read(cx).table.clone())
        .unwrap();
    (handle.into(), table)
}

#[gpui_kit::test]
fn table_keeps_focus_when_a_cell_selects_its_row_on_mouse_down(cx: &mut TestAppContext) {
    let (handle, table) = open_press_records(cx, false);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        // Inside the second cell, past the row header and the first column.
        window.click_at(("row", 1usize), point(px(260.), px(5.)), cx);
        assert_eq!(table.read(cx).selected_row(), Some(1));
        assert!(table.focus_handle(cx).is_focused(window));
    })
    .unwrap();
}

#[gpui_kit::test]
fn table_row_header_click_selects_its_row_once(cx: &mut TestAppContext) {
    let (handle, table) = open_press_records(cx, true);
    let selected = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    cx.update(|cx| {
        let selected = selected.clone();
        cx.subscribe(&table, move |_, event: &TableEvent, _| {
            if let TableEvent::SelectRow(row) = event {
                selected.borrow_mut().push(*row);
            }
        })
        .detach();
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        // The row header is the narrow column before the cells.
        window.click_at(("row", 2usize), point(px(6.), px(5.)), cx);
        assert_eq!(table.read(cx).selected_row(), Some(2));
    })
    .unwrap();
    assert_eq!(*selected.borrow(), [2]);
}

#[gpui_kit::test]
fn table_forgets_a_right_clicked_cell_on_the_next_click(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, handle_content) =
        common::open_window(cx, Some(size(px(640.), px(320.))), |window, cx| {
            cx.new(|cx| RecordsBelowLabel {
                table: cx.new(|cx| TableState::new(Rows, window, cx).cell_selectable(true)),
            })
        });
    cx.update_window(handle.into(), |_, window, cx| {
        let table = handle_content.clone().read(cx).table.clone();
        window.render_frame(cx);

        right_click_row(window, 1, cx);
        assert_eq!(table.read(cx).right_clicked_cell(), Some((1, 0)));
        window.click_at(("row", 2usize), point(px(100.), px(5.)), cx);
        assert_eq!(
            table.read(cx).right_clicked_cell(),
            None,
            "selecting another cell must clear the right-click outline"
        );

        right_click_row(window, 1, cx);
        assert_eq!(table.read(cx).right_clicked_cell(), Some((1, 0)));
        window.click("outside", cx);
        assert_eq!(
            table.read(cx).right_clicked_cell(),
            None,
            "a click outside the table must clear the right-click outline"
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn table_retains_navigation_positions_when_selection_mode_changes(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, handle_content) =
        common::open_window(cx, Some(size(px(640.), px(320.))), |window, cx| {
            cx.new(|cx| Records {
                table: cx.new(|cx| TableState::new(Rows, window, cx)),
            })
        });
    cx.update_window(handle.into(), |_, window, cx| {
        let table = handle_content.clone().read(cx).table.clone();
        table.focus_handle(cx).focus(window, cx);
        table.update(cx, |table, cx| {
            table.set_selected_row(5, cx);
            table.set_selected_col(0, cx);
        });
        window.render_frame(cx);
        window.press("down", cx);
        assert_eq!(table.read(cx).selected_row(), Some(6));
        assert_eq!(table.read(cx).selected_col(), None);
        window.press("right", cx);
        assert_eq!(table.read(cx).selected_col(), Some(1));
        assert_eq!(table.read(cx).selected_row(), None);
    })
    .unwrap();
}

#[gpui_kit::test]
fn table_home_and_end_select_the_first_and_last_row(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, handle_content) =
        common::open_window(cx, Some(size(px(640.), px(320.))), |window, cx| {
            cx.new(|cx| Records {
                table: cx.new(|cx| TableState::new(Rows, window, cx)),
            })
        });
    cx.update_window(handle.into(), |_, window, cx| {
        let table = handle_content.clone().read(cx).table.clone();
        window.render_frame(cx);
        window.click(("row", 1usize), cx);
        window.press("end", cx);
        assert_eq!(table.read(cx).selected_row(), Some(199));
        window.press("home", cx);
        assert_eq!(table.read(cx).selected_row(), Some(0));

        // A selected column still moves to the first or last column.
        table.update(cx, |table, cx| table.set_selected_col(0, cx));
        window.press("end", cx);
        assert_eq!(table.read(cx).selected_col(), Some(1));
    })
    .unwrap();

    // Without selectable rows, Home and End move the column as before.
    let (handle, handle_content) =
        common::open_window(cx, Some(size(px(640.), px(320.))), |window, cx| {
            cx.new(|cx| Records {
                table: cx.new(|cx| TableState::new(Rows, window, cx).row_selectable(false)),
            })
        });
    cx.update_window(handle.into(), |_, window, cx| {
        let table = handle_content.clone().read(cx).table.clone();
        table.focus_handle(cx).focus(window, cx);
        window.render_frame(cx);
        window.press("end", cx);
        assert_eq!(table.read(cx).selected_col(), Some(1));
        assert_eq!(table.read(cx).selected_row(), None);
    })
    .unwrap();
}

#[gpui_kit::test]
fn table_selects_rows_and_keyboard_scrolls_virtualized_content(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, Some(size(px(640.), px(320.))), |window, cx| {
        cx.new(|cx| Records {
            table: cx.new(|cx| TableState::new(Rows, window, cx).row_selectable(true)),
        })
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
    let (handle, handle_content) =
        common::open_window(cx, Some(size(px(640.), px(320.))), |window, cx| {
            cx.new(|cx| Records {
                table: cx.new(|cx| TableState::new(Rows, window, cx).row_selectable(false)),
            })
        });
    let table = cx
        .update_window(handle.into(), |_root, _, cx| {
            handle_content.clone().read(cx).table.clone()
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

/// A table recording the visible row and column ranges it reports to its delegate.
struct TableProbe {
    rows_count: usize,
    columns_count: usize,
    left_columns_count: usize,
    visible_row_ranges: Vec<Range<usize>>,
    visible_column_ranges: Vec<Range<usize>>,
}
impl TableProbe {
    fn new(rows_count: usize) -> Self {
        Self {
            rows_count,
            columns_count: 2,
            left_columns_count: 0,
            visible_row_ranges: Vec::new(),
            visible_column_ranges: Vec::new(),
        }
    }
}
impl TableDelegate for TableProbe {
    fn columns_count(&self, _: &App) -> usize {
        self.columns_count
    }
    fn rows_count(&self, _: &App) -> usize {
        self.rows_count
    }
    fn column(&self, col_ix: usize, _: &App) -> Column {
        Column::new(format!("column-{col_ix}"), format!("Column {col_ix}"))
            .width(px(180.))
            .when(col_ix < self.left_columns_count, |column| {
                column.fixed_left()
            })
    }
    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div().child(format!("{row_ix}:{col_ix}"))
    }
    fn visible_rows_changed(
        &mut self,
        visible_range: Range<usize>,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        self.visible_row_ranges.push(visible_range);
    }

    fn visible_columns_changed(
        &mut self,
        visible_range: Range<usize>,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        self.visible_column_ranges.push(visible_range);
    }
}
struct TableProbeView {
    table: Entity<TableState<TableProbe>>,
    stripe: bool,
}
impl Render for TableProbeView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(DataTable::new(&self.table).stripe(self.stripe))
    }
}
fn open_table_probe(
    cx: &mut TestAppContext,
    rows_count: usize,
    stripe: bool,
) -> (gpui_kit::AnyWindowHandle, Entity<TableState<TableProbe>>) {
    open_table_probe_with_delegate(cx, TableProbe::new(rows_count), stripe)
}

fn open_table_probe_with_delegate(
    cx: &mut TestAppContext,
    delegate: TableProbe,
    stripe: bool,
) -> (gpui_kit::AnyWindowHandle, Entity<TableState<TableProbe>>) {
    cx.update(gpui_kit::init);
    let (handle, content) =
        common::open_window(cx, Some(size(px(640.), px(320.))), move |window, cx| {
            cx.new(|cx| TableProbeView {
                table: cx.new(|cx| TableState::new(delegate, window, cx)),
                stripe,
            })
        });
    let table = cx
        .update_window(handle.into(), |_, _, cx| content.read(cx).table.clone())
        .unwrap();
    (handle.into(), table)
}

#[gpui_kit::test]
fn table_reports_one_row_and_empty_visible_ranges(cx: &mut TestAppContext) {
    let (handle, table) = open_table_probe(cx, 1, false);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(
            table.read(cx).delegate().visible_row_ranges.last(),
            Some(&(0..1))
        );

        table.update(cx, |table, cx| {
            table.delegate_mut().rows_count = 0;
            cx.notify();
        });
        window.render_frame(cx);
        assert_eq!(
            table.read(cx).delegate().visible_row_ranges.last(),
            Some(&(0..0))
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn table_visible_range_stops_at_the_last_row_under_stripe_filler(cx: &mut TestAppContext) {
    let (handle, table) = open_table_probe(cx, 3, true);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        let reported = table.read(cx).delegate().visible_row_ranges.clone();
        assert!(!reported.is_empty());
        assert!(
            reported.iter().all(|range| range.end <= 3),
            "reported past the last row: {reported:?}"
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn table_reports_one_scrollable_column_with_and_without_fixed_columns(cx: &mut TestAppContext) {
    for left_columns_count in [0, 1] {
        let delegate = TableProbe {
            columns_count: left_columns_count + 1,
            left_columns_count,
            ..TableProbe::new(3)
        };
        let (handle, table) = open_table_probe_with_delegate(cx, delegate, false);
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            let state = table.read(cx);
            // Column ranges are relative to the scrollable columns, excluding
            // the fixed prefix. Repeated row rendering reports a change once.
            assert_eq!(state.delegate().visible_column_ranges, vec![0..1]);
            assert_eq!(state.visible_range().cols(), &(0..1));

            window.render_frame(cx);
            assert_eq!(table.read(cx).delegate().visible_column_ranges, vec![0..1]);
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn table_modifier_clicks_select_rows_unless_turned_off(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let secondary = cx.update(|cx| Modifiers::secondary_key(cx));
    let shift = Modifiers::shift();
    // The row header, shown in cell selection, is the narrow column before
    // the cells.
    let row_header = ClickOptions::new().with_offset(point(px(6.), px(5.)));
    for (select_on_modifier_click, expected) in [(true, Some(3)), (false, Some(1))] {
        for cell_selectable in [false, true] {
            let (handle, handle_content) =
                common::open_window(cx, Some(size(px(640.), px(320.))), |window, cx| {
                    cx.new(|cx| Records {
                        table: cx.new(|cx| {
                            TableState::new(Rows, window, cx)
                                .cell_selectable(cell_selectable)
                                .select_on_modifier_click(select_on_modifier_click)
                        }),
                    })
                });
            cx.update_window(handle.into(), |_, window, cx| {
                let table = handle_content.read(cx).table.clone();
                if cell_selectable {
                    window.click_with_options(("row", 1usize), row_header, cx);
                    window.click_with_options(
                        ("row", 3usize),
                        row_header.with_modifiers(secondary),
                        cx,
                    );
                    assert_eq!(table.read(cx).selected_row(), expected, "row header");
                    window.click_with_options(("row", 3usize), row_header, cx);
                } else {
                    window.click(("row", 1usize), cx);
                    window.click_with_modifiers(("row", 3usize), secondary, cx);
                    assert_eq!(table.read(cx).selected_row(), expected);
                    window.click_with_modifiers(("row", 3usize), shift, cx);
                    assert_eq!(table.read(cx).selected_row(), expected);
                    window.click(("row", 3usize), cx);
                }
                assert_eq!(table.read(cx).selected_row(), Some(3));
            })
            .unwrap();
        }
    }
}

#[gpui_kit::test]
fn table_modifier_clicks_on_the_selected_cell_escalate_unless_turned_off(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    // Without a row header, the first cell starts at the row's leading edge.
    let first_cell = ClickOptions::new().with_offset(point(px(6.), px(5.)));
    for (select_on_modifier_click, expected) in [
        (true, TableSelection::Row(1)),
        (false, TableSelection::Cell(1, 0)),
    ] {
        let (handle, handle_content) =
            common::open_window(cx, Some(size(px(640.), px(320.))), |window, cx| {
                cx.new(|cx| Records {
                    table: cx.new(|cx| {
                        TableState::new(Rows, window, cx)
                            .cell_selectable(true)
                            .row_header(false)
                            .select_on_modifier_click(select_on_modifier_click)
                    }),
                })
            });
        cx.update_window(handle.into(), |_, window, cx| {
            let table = handle_content.read(cx).table.clone();
            window.click_with_options(("row", 1usize), first_cell, cx);
            assert_eq!(table.read(cx).selection(), TableSelection::Cell(1, 0));
            window.click_with_options(
                ("row", 1usize),
                first_cell.with_modifiers(Modifiers::secondary_key(cx)),
                cx,
            );
            assert_eq!(table.read(cx).selection(), expected);
        })
        .unwrap();
    }
}

struct Choices {
    confirmed: Vec<bool>,
}
impl ListDelegate for Choices {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &App) -> usize {
        2
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        Some(ListItem::new(("choice", ix.row)).child(format!("Choice {}", ix.row)))
    }

    fn set_selected_index(
        &mut self,
        _: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
    }

    fn confirm(&mut self, secondary: bool, _: &mut Window, _: &mut Context<ListState<Self>>) {
        self.confirmed.push(secondary);
    }
}
struct Picker {
    list: Entity<ListState<Choices>>,
}
impl Render for Picker {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(List::new(&self.list))
    }
}
#[gpui_kit::test]
fn list_click_confirms_as_secondary_with_the_secondary_modifier(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, picker) = common::open_window(cx, Some(size(px(320.), px(240.))), |window, cx| {
        cx.new(|cx| Picker {
            list: cx.new(|cx| ListState::new(Choices { confirmed: vec![] }, window, cx)),
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click(("choice", 0usize), cx);
        window.click_with_modifiers(("choice", 1usize), Modifiers::secondary_key(cx), cx);
        let list = picker.read(cx).list.clone();
        assert_eq!(list.read(cx).delegate().confirmed, [false, true]);
    })
    .unwrap();
}

#[derive(Default)]
struct SortableRows {
    sorts: Vec<(usize, ColumnSort)>,
}
impl TableDelegate for SortableRows {
    fn columns_count(&self, _: &App) -> usize {
        2
    }
    fn rows_count(&self, _: &App) -> usize {
        3
    }
    fn column(&self, ix: usize, _: &App) -> Column {
        let column = Column::new(
            format!("column-{ix}"),
            if ix == 0 { "Name" } else { "Status" },
        )
        .width(px(180.));
        if ix == 0 { column.sortable() } else { column }
    }
    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        self.sorts.push((col_ix, sort));
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
struct SortableRecords {
    table: Entity<TableState<SortableRows>>,
}
impl Render for SortableRecords {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(DataTable::new(&self.table))
    }
}

#[gpui_kit::test]
fn table_sort_icon_is_a_labelled_button_that_cycles_the_sort(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, handle_content) =
        common::open_window(cx, Some(size(px(640.), px(320.))), |window, cx| {
            cx.new(|cx| SortableRecords {
                table: cx.new(|cx| TableState::new(SortableRows::default(), window, cx)),
            })
        });
    cx.update_window(handle.into(), |_, window, cx| {
        let table = handle_content.clone().read(cx).table.clone();
        window.render_frame(cx);

        let icon = window.find(("icon-sort", 0usize));
        assert_eq!(icon.role(), Some(gpui_kit::Role::Button));
        assert_eq!(icon.label(), Some("Sort by Name"));
        assert!(window.try_find(("icon-sort", 1usize)).is_none());
        assert_eq!(table.read(cx).selected_col(), None);

        for _ in 0..3 {
            window.click(("icon-sort", 0usize), cx);
        }
        assert_eq!(
            table.read(cx).delegate().sorts,
            vec![
                (0, ColumnSort::Descending),
                (0, ColumnSort::Ascending),
                (0, ColumnSort::Default),
            ]
        );
        // The click also reaches the column header, which selects the column.
        assert_eq!(table.read(cx).selected_col(), Some(0));
    })
    .unwrap();
}
