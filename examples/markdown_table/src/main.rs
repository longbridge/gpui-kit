//! A minimal markdown viewer for debugging table rendering.
//!
//! Cycle between the three table layouts:
//! - wrap (default): cells wrap, columns shrink to fit the frame.
//! - adaptive (`style.table` overflow-x: scroll): columns fit their content,
//!   wrap down to a floor as the frame narrows, then scroll horizontally.
//! - nowrap (adaptive + `style.table_cell` white-space: nowrap): cells stay
//!   on a single line, the table scrolls as soon as the content overflows.
//!
//! Edit `src/report.md` to change the markdown source.
//!
//! Run: `cargo run -p markdown_table`

use gpui_kit::assets::Assets;
use gpui_kit::component::{
    button::Button,
    text::{TextView, TextViewStyle},
    *,
};
use gpui_kit::*;

const SOURCE: &str = include_str!("report.md");

/// Markdown source: `MD_FILE=<path>` overrides the bundled `report.md`, so you
/// can iterate on a repro without recompiling.
fn source() -> SharedString {
    match std::env::var("MD_FILE") {
        Ok(path) => std::fs::read_to_string(&path)
            .unwrap_or_else(|err| format!("Failed to read `{path}`: {err}"))
            .into(),
        Err(_) => SOURCE.into(),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum TableMode {
    Wrap,
    Adaptive,
    Nowrap,
}

impl TableMode {
    /// `TABLE_MODE=wrap|adaptive|nowrap` picks the initial mode.
    fn initial() -> Self {
        match std::env::var("TABLE_MODE").as_deref() {
            Ok("wrap") => Self::Wrap,
            Ok("nowrap") => Self::Nowrap,
            _ => Self::Adaptive,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Wrap => Self::Adaptive,
            Self::Adaptive => Self::Nowrap,
            Self::Nowrap => Self::Wrap,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Wrap => "Table: wrap",
            Self::Adaptive => "Table: scroll (adaptive)",
            Self::Nowrap => "Table: scroll (nowrap)",
        }
    }

    fn style(self) -> TextViewStyle {
        if self == Self::Wrap {
            return TextViewStyle::default();
        }

        let mut table = StyleRefinement::default();
        table.overflow.x = Some(Overflow::Scroll);
        let style = TextViewStyle::default().table(table);

        if self == Self::Nowrap {
            let mut cell = StyleRefinement::default();
            cell.text.white_space = Some(WhiteSpace::Nowrap);
            style.table_cell(cell)
        } else {
            style
        }
    }
}

struct Example {
    mode: TableMode,
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .p_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("toggle")
                            .label(self.mode.label())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.mode = this.mode.next();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                TextView::markdown("report", source())
                    .style(self.mode.style())
                    .p_4()
                    .scrollable(true)
                    .selectable(true),
            )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_kit::init(cx);

        // `WIN_W=<px>` overrides the window width, to check how the table
        // adapts at different frame widths.
        let width = std::env::var("WIN_W")
            .ok()
            .and_then(|w| w.parse().ok())
            .unwrap_or(900.);
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(width), px(700.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|_| Example {
                    mode: TableMode::initial(),
                });
                // The first level view on the window should be a Root.
                cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
