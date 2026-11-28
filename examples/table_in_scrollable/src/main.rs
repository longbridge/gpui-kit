//! A `DataTable` (which has its own vertical scrollbar) nested inside a
//! scrollable page.
//!
//! Scrolling the mouse wheel over the table should only scroll the table
//! rows; once the table reaches its top/bottom edge (or when the cursor is
//! outside the table), the outer page scrolls instead.
//! Run with `--mixed-rows` to try compact section labels and taller records.

use std::rc::Rc;

use gpui_kit::component::{
    scroll::ScrollableElement as _,
    table::{Column, DataTable, TableDelegate, TableState},
    *,
};
use gpui_kit::*;

struct MyTable {
    columns: Vec<Column>,
    heights: Option<Rc<Vec<Pixels>>>,
}

impl MyTable {
    fn new(mixed_rows: bool) -> Self {
        let columns = vec![
            Column::new("id", "ID").width(px(50.)),
            Column::new("name", "Name").width(px(150.)),
            Column::new("email", "Email").width(px(250.)),
            Column::new("role", "Role").width(px(150.)),
            Column::new("status", "Status").width(px(100.)),
        ];

        let heights = mixed_rows.then(|| {
            Rc::new(
                (0..30)
                    .map(|row| px(if row % 5 == 0 { 24. } else { 48. }))
                    .collect(),
            )
        });
        Self { columns, heights }
    }
}

impl TableDelegate for MyTable {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        30
    }

    fn row_heights(&self, _: &App) -> Option<Rc<Vec<Pixels>>> {
        self.heights.clone()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        if self.heights.is_some() && row_ix % 5 == 0 {
            return if col_ix == 1 {
                div()
                    .font_semibold()
                    .child(format!("Team {}", row_ix / 5 + 1))
                    .into_any_element()
            } else {
                div().into_any_element()
            };
        }
        match col_ix {
            0 => format!("{}", row_ix).into_any_element(),
            1 => format!("User {}", row_ix).into_any_element(),
            2 => format!("user-{}@mail.com", row_ix).into_any_element(),
            3 => "User".into_any_element(),
            4 => "Active".into_any_element(),
            _ => panic!("Invalid column index"),
        }
    }
}

pub struct Example {
    table: Entity<TableState<MyTable>>,
}

impl Example {
    fn new(mixed_rows: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let table = cx.new(|cx| TableState::new(MyTable::new(mixed_rows), window, cx));

        Self { table }
    }

    fn filler(&self, label: &str, height: f32, cx: &App) -> impl IntoElement {
        div()
            .h(px(height))
            .w_full()
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_dashed()
            .border_color(cx.theme().border)
            .text_color(cx.theme().muted_foreground)
            .child(SharedString::from(label.to_string()))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().id("page").size_full().overflow_y_scrollbar().child(
            v_flex()
                .p_4()
                .gap_4()
                .child(self.filler("Content above the table", 400., cx))
                .child(
                    div()
                        .h(px(300.))
                        .w_full()
                        .child(DataTable::new(&self.table).stripe(true).bordered(true)),
                )
                .child(self.filler("Content below the table", 800., cx)),
        )
    }
}

fn main() {
    let mixed_rows = std::env::args().any(|arg| arg == "--mixed-rows");
    gpui_kit::application().run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_kit::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(None, size(px(700.), px(700.)), cx)),
            ..Default::default()
        };

        gpui_kit::open_window(window_options, cx, |window, cx| {
            window.set_window_title("Table in Scrollable");
            cx.new(|cx| Example::new(mixed_rows, window, cx))
        })
        .expect("Failed to open window");
    });
}
