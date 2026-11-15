use gpui_base::{GridItem, GridLayout};

use super::*;

impl BaseShowcase {
    pub(in super::super) fn grid_layout(&self) -> impl IntoElement {
        let tile = |title: &'static str, value: &'static str| {
            div()
                .size_full()
                .p_2()
                .flex()
                .flex_col()
                .gap_1()
                .border_1()
                .border_color(super::example_rgb(0x171717))
                .bg(super::example_rgb(0xffffff))
                .child(
                    div()
                        .text_xs()
                        .text_color(super::example_rgb(0x737373))
                        .child(title),
                )
                .child(div().child(value))
        };
        div().w_96().text_sm().child(
            GridLayout::new("example-grid-layout", &self.grid_layout)
                .accessibility_label("Dashboard")
                .gap_2()
                .placeholder(|_, _| {
                    div()
                        .size_full()
                        .border_1()
                        .border_dashed()
                        .border_color(super::example_rgb(0x737373))
                        .into_any_element()
                })
                .resize_handle(|_, _| {
                    div()
                        .absolute()
                        .right(px(3.))
                        .bottom(px(3.))
                        .size(px(6.))
                        .border_r_2()
                        .border_b_2()
                        .border_color(super::example_rgb(0x737373))
                        .into_any_element()
                })
                .dragging_item(|style| style.opacity(0.85))
                .selected_item(|style| style.border_2().border_color(super::example_rgb(0x2563eb)))
                .item(GridItem::new("revenue").child(tile("Revenue", "$48,210")))
                .item(GridItem::new("orders").child(tile("Orders", "1,284")))
                .item(GridItem::new("visitors").child(tile("Visitors", "23,906")))
                .item(GridItem::new("notes").child(tile("Notes (locked)", "Pinned"))),
        )
    }
}
