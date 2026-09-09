//! A `DataTable` (which has its own vertical scrollbar) nested inside a
//! scrollable page.
//!
//! Scrolling the mouse wheel over the table should only scroll the table
//! rows; once the table reaches its top/bottom edge (or when the cursor is
//! outside the table), the outer page scrolls instead.

use gpui_kit::component::{
    scroll::ScrollableElement as _,
    table::{Column, DataTable, TableDelegate, TableState},
    *,
};
use gpui_kit::*;

struct MyTable {
    columns: Vec<Column>,
}

impl MyTable {
    fn new(_: &mut App) -> Self {
        let columns = vec![
            Column::new("id", "ID").width(px(50.)),
            Column::new("name", "Name").width(px(150.)),
            Column::new("email", "Email").width(px(250.)),
            Column::new("role", "Role").width(px(150.)),
            Column::new("status", "Status").width(px(100.)),
        ];

        Self { columns }
    }
}

impl TableDelegate for MyTable {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        30
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
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let table = cx.new(|cx| TableState::new(MyTable::new(cx), window, cx));

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
    gpui_kit::application().run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_kit::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(700.), px(700.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                window.set_window_title("Table in Scrollable");
                let view = cx.new(|cx| Example::new(window, cx));
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
