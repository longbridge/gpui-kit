pub(super) use super::example_rgb;
use super::*;

mod accordion;
mod alert_dialog;
mod avatar;
mod button;
mod calendar;
mod checkbox;
mod collapsible;
mod color_picker;
mod combobox;
mod date_picker;
mod dialog;
mod dock;
pub(super) use dock::build_dock;
mod editor;
mod hover_card;
mod input;
mod link;
mod nav_stack;
pub(super) use nav_stack::{ShowcasePage, slide};
mod number_input;
mod otp_input;
mod pagination;
mod popover;
mod popup;
mod progress;
mod radio;
mod radio_group;
mod resizable;
mod scrollbar;
mod select;
mod sheet;
mod slider;
mod switch;
mod table;
mod tabs;
mod text_selection;
mod text_view;
pub(super) use text_view::MARKDOWN as TEXT_VIEW_MARKDOWN;
mod textarea;
mod time_field;
mod toast;
mod toggle;
mod toggle_group;
mod toolbar;
mod tooltip;
mod tree;
mod virtual_list;

/// A disclosure chevron drawn as a path in the current text color. The `⌄` and
/// `⌃` glyphs sit at the baseline and cap height, so a centered row still shows
/// them off center; a path is centered in its own box.
fn chevron(up: bool) -> impl IntoElement {
    gpui::canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let center = bounds.center();
            let (half_width, half_height) = (px(3.5), px(1.75));
            let tip = if up { -half_height } else { half_height };
            let mut path = gpui::PathBuilder::stroke(px(1.25));
            path.move_to(gpui::point(center.x - half_width, center.y - tip));
            path.line_to(gpui::point(center.x, center.y + tip));
            path.line_to(gpui::point(center.x + half_width, center.y - tip));
            if let Ok(path) = path.build() {
                window.paint_path(path, window.text_style().color);
            }
        },
    )
    .size(px(12.))
    .flex_none()
}
