use gpui_base::{TextView, TextViewStyle};

use super::*;
use crate::showcase::palette::ExamplePalette;

pub const MARKDOWN: &str = include_str!("../../../../../examples/fixtures/test.md");

fn text_view_style(palette: ExamplePalette) -> TextViewStyle {
    let is_dark = palette.canvas == ExamplePalette::for_dark(true).canvas;
    TextViewStyle::default()
        .with_foreground(gpui::rgb(palette.foreground).into())
        .with_muted_foreground(gpui::rgb(palette.muted_foreground).into())
        .with_link(gpui::rgb(palette.resolve(0x007fff)).into())
        .with_code_background(gpui::rgb(palette.elevated).into())
        .with_border(gpui::rgb(palette.border).into())
        .with_inline_code(gpui::HighlightStyle {
            background_color: Some(gpui::rgb(palette.elevated).into()),
            ..Default::default()
        })
        .with_dark(is_dark)
}

impl BaseShowcase {
    pub(in super::super) fn text_view(&self, window: &Window) -> impl IntoElement {
        let palette = ExamplePalette::from_window(window);
        let style = text_view_style(palette);
        div()
            .id("text-view-example")
            .debug_selector(|| "text-view-example".into())
            .w_full()
            .h(px(560.))
            .max_h_full()
            .text_color(gpui::rgb(palette.foreground))
            .child(
                div()
                    .debug_selector(|| "text-view-markdown".into())
                    .size_full()
                    .min_h_0()
                    .overflow_hidden()
                    .child(
                        TextView::new(&self.text_view)
                            .size_full()
                            .px_4()
                            .scrollable(true)
                            .style(style),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gpui::{
        Modifiers, MouseButton, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext,
        point, px,
    };
    use gpui_base::{TextSelection, TextViewStyle};

    use super::text_view_style;
    use crate::showcase::BaseShowcase;
    use crate::showcase::palette::ExamplePalette;

    #[test]
    fn text_view_style_uses_dark_palette_colors() {
        let style = text_view_style(ExamplePalette::for_dark(true));

        assert_eq!(style.foreground(), gpui::rgb(0xffffff).into());
        assert_eq!(style.muted_foreground(), gpui::rgb(0xa3a3a3).into());
        assert_eq!(style.code_background(), gpui::rgb(0x262626).into());
        assert_eq!(style.border(), gpui::rgb(0x404040).into());
        assert_eq!(style.selection(), TextViewStyle::default().selection());
        assert!(style.is_dark());
    }

    #[gpui::test]
    fn text_view_showcase_renders_with_base_defaults(cx: &mut TestAppContext) {
        cx.update(gpui_base::init);
        let (view, cx) =
            cx.add_window_view(|window, cx| BaseShowcase::new("text-view", window, cx));
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        let example = cx
            .debug_bounds("text-view-example")
            .expect("example bounds");
        let markdown = cx
            .debug_bounds("text-view-markdown")
            .expect("Markdown bounds");
        let document = view.read_with(cx, |view, cx| view.text_view.read(cx).bounds());

        assert_eq!(markdown.left(), example.left());
        assert_eq!(markdown.right(), example.right());
        assert_eq!(document.left() - example.left(), px(16.));
        assert_eq!(example.right() - document.right(), px(16.));
    }

    #[gpui::test]
    fn text_view_showcase_drag_selection_settles(cx: &mut TestAppContext) {
        cx.update(gpui_base::init);
        let (_, cx) = cx.add_window_view(|window, cx| BaseShowcase::new("text-view", window, cx));
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        let bounds = cx
            .debug_bounds("text-view-example")
            .expect("example bounds");
        // Exercise selection inside the visible, virtualized Markdown blocks.
        let start = point(bounds.left() + px(36.), bounds.top() + px(36.));
        let end = point(bounds.right() - px(36.), bounds.top() + px(180.));
        cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());

        assert!(cx.update(|window, cx| TextSelection::has_selection(window, cx)));
    }

    #[gpui::test]
    fn text_view_showcase_scrolls_the_document_inside_a_fixed_viewport(cx: &mut TestAppContext) {
        cx.update(gpui_base::init);
        let (view, cx) =
            cx.add_window_view(|window, cx| BaseShowcase::new("text-view", window, cx));
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        let viewport = cx
            .debug_bounds("text-view-markdown")
            .expect("Markdown viewport bounds");
        let example = cx
            .debug_bounds("text-view-example")
            .expect("TextView example bounds");
        let scroll_before = view.read_with(cx, |view, cx| {
            let offset = view.text_view.read(cx).list_state().logical_scroll_top();
            (offset.item_ix, offset.offset_in_item)
        });
        cx.simulate_event(ScrollWheelEvent {
            position: example.center(),
            delta: ScrollDelta::Pixels(point(px(0.), px(-120.))),
            ..Default::default()
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let after = cx
            .debug_bounds("text-view-markdown")
            .expect("Markdown viewport bounds after scrolling");
        let scroll_after = view.read_with(cx, |view, cx| {
            let offset = view.text_view.read(cx).list_state().logical_scroll_top();
            (offset.item_ix, offset.offset_in_item)
        });

        assert_eq!(
            after, viewport,
            "the TextView viewport itself must stay fixed"
        );
        assert_ne!(
            scroll_after, scroll_before,
            "the TextView's virtual list must consume the wheel event"
        );
    }

    #[gpui::test]
    fn dragging_selection_scrolls_the_containing_region_without_text_view_parameters(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_base::init);
        let (view, cx) =
            cx.add_window_view(|window, cx| BaseShowcase::new("text-view", window, cx));
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        let markdown = cx
            .debug_bounds("text-view-markdown")
            .expect("Markdown section bounds");
        let scroll_before = view.read_with(cx, |view, cx| {
            let offset = view.text_view.read(cx).list_state().logical_scroll_top();
            (offset.item_ix, offset.offset_in_item)
        });
        let start = point(markdown.left() + px(24.), markdown.top() + px(24.));
        let edge = point(markdown.left() + px(120.), markdown.bottom() - px(2.));
        cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(edge, MouseButton::Left, Modifiers::default());
        cx.executor().advance_clock(Duration::from_millis(64));
        cx.run_until_parked();
        cx.simulate_mouse_up(edge, MouseButton::Left, Modifiers::default());
        let scroll_after = view.read_with(cx, |view, cx| {
            let offset = view.text_view.read(cx).list_state().logical_scroll_top();
            (offset.item_ix, offset.offset_in_item)
        });

        assert!(
            scroll_after != scroll_before,
            "dragging at the viewport edge must scroll the TextView document"
        );
        cx.executor().advance_clock(Duration::from_millis(64));
        cx.run_until_parked();
        let scroll_stopped = view.read_with(cx, |view, cx| {
            let offset = view.text_view.read(cx).list_state().logical_scroll_top();
            (offset.item_ix, offset.offset_in_item)
        });
        assert_eq!(
            scroll_stopped, scroll_after,
            "selection auto-scroll must stop on mouse-up"
        );
    }
}
