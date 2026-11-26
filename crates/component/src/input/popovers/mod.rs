mod caret_anchored;
mod code_action_menu;
mod completion_menu;
mod diagnostic_popover;
mod hover_popover;

pub(crate) use caret_anchored::*;
pub(crate) use code_action_menu::*;
pub(crate) use completion_menu::*;
pub(crate) use diagnostic_popover::*;
pub(crate) use hover_popover::*;

use gpui::{
    App, Div, ElementId, InteractiveElement as _, SharedString, Stateful, StyleRefinement,
    Styled as _, Window, div, rems,
};

use crate::{
    ActiveTheme, ThemeStyled as _,
    text::{TextView, TextViewStyle},
};

pub(super) fn render_markdown(
    id: impl Into<ElementId>,
    markdown: impl Into<SharedString>,
    _: &mut Window,
    cx: &mut App,
) -> TextView {
    TextView::markdown(id, markdown)
        .style(
            TextViewStyle::default()
                .paragraph_gap(rems(0.5))
                .heading_font_size(|level, rem_size| match level {
                    1..=3 => rem_size * 1,
                    4 => rem_size * 0.9,
                    _ => rem_size * 0.8,
                })
                .code_block(
                    StyleRefinement::default()
                        .bg(cx.theme().transparent)
                        .p_0()
                        .border_0()
                        .rounded_none()
                        .text_size(rems(0.75)),
                ),
        )
        // A hover is already one contextual card. Keep fenced signatures
        // highlighted, but omit the repeated per-block language/action header.
        .code_block_actions(|_, _, _| gpui::Empty)
        .on_link_click(|destination, event, _, cx| {
            // Empty/unresolved Markdown references are not URLs. In particular,
            // NSURL accepts an empty relative URL which Launch Services rejects.
            let activate = match event {
                gpui::ClickEvent::Mouse(click) => matches!(
                    click.up.button,
                    gpui::MouseButton::Left | gpui::MouseButton::Middle
                ),
                gpui::ClickEvent::Keyboard(_) => true,
                gpui::ClickEvent::Touch(click) => !click.long_press,
            };
            if activate
                && (destination.starts_with("https://") || destination.starts_with("http://"))
            {
                cx.open_url(destination);
            }
        })
        .selectable(true)
}

pub(super) fn editor_popover(id: impl Into<ElementId>, cx: &App) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .occlude()
        .popover_style(cx)
        .shadow_md()
        .text_xs()
        .p_1()
}
