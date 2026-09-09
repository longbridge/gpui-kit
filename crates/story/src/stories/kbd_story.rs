use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, IntoElement, Keystroke, ParentElement, Render,
    Styled, Window, px,
};

use gpui_kit::component::{h_flex, kbd::Kbd, v_flex};

use crate::section;

pub struct KbdStory {
    focus_handle: gpui_kit::FocusHandle,
}

impl super::Story for KbdStory {
    fn title() -> &'static str {
        "Kbd"
    }

    fn description() -> &'static str {
        "A tag style to display keyboard shortcuts"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl KbdStory {
    pub(crate) fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}
impl Focusable for KbdStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for KbdStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .child(
                section("Default")
                    .description("Displays single keys and multi-key shortcuts.")
                    .w(px(560.))
                    .child(
                        h_flex()
                            .w_full()
                            .justify_center()
                            .gap_2()
                            .flex_wrap()
                            .child(Kbd::new(Keystroke::parse("cmd-shift-p").unwrap()))
                            .child(Kbd::new(Keystroke::parse("cmd-ctrl-t").unwrap()))
                            .child(Kbd::new(Keystroke::parse("cmd--").unwrap()))
                            .child(Kbd::new(Keystroke::parse("cmd-+").unwrap()))
                            .child(Kbd::new(Keystroke::parse("escape").unwrap()))
                            .child(Kbd::new(Keystroke::parse("backspace").unwrap()))
                            .child(Kbd::new(Keystroke::parse("/").unwrap()))
                            .child(Kbd::new(Keystroke::parse("enter").unwrap())),
                    ),
            )
            .child(
                section("Outlined")
                    .description("An outlined treatment adds emphasis on dense surfaces.")
                    .w(px(560.))
                    .child(
                        h_flex()
                            .w_full()
                            .justify_center()
                            .gap_2()
                            .flex_wrap()
                            .child(Kbd::new(Keystroke::parse("cmd-shift-p").unwrap()).outline())
                            .child(Kbd::new(Keystroke::parse("cmd-ctrl-t").unwrap()).outline())
                            .child(Kbd::new(Keystroke::parse("enter").unwrap()).outline()),
                    ),
            )
    }
}
