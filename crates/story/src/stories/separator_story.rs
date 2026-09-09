use crate::section;
use gpui_kit::component::{ActiveTheme, h_flex, label::Label, separator::Separator, v_flex};
use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Window, px,
};

const DESCRIPTION: &str = "GPUI Component is a Rust GUI components for building fantastic cross-platform desktop application by using GPUI.";

pub struct SeparatorStory {
    focus_handle: gpui_kit::FocusHandle,
}

impl super::Story for SeparatorStory {
    fn title() -> &'static str {
        "Separator"
    }

    fn description() -> &'static str {
        "A separator that can be either vertical or horizontal."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SeparatorStory {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
        })
    }
}

impl Focusable for SeparatorStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SeparatorStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Horizontal")
                    .description(
                        "Separates stacked content, with optional labels and dashed rules.",
                    )
                    .w(px(520.))
                    .child(
                        v_flex()
                            .gap_4()
                            .w_full()
                            .mt_4()
                            .child(Separator::horizontal())
                            .child(Separator::horizontal().label("With Label"))
                            .child(Separator::horizontal_dashed())
                            .child(Separator::horizontal_dashed().label("Dashed With Label")),
                    ),
            )
            .child(
                section("Vertical")
                    .description("Separates actions or values arranged in a row.")
                    .w(px(520.))
                    .child(
                        h_flex()
                            .gap_4()
                            .h(px(100.))
                            .child(Separator::vertical())
                            .child(Separator::vertical().label("Solid"))
                            .child(Separator::vertical_dashed())
                            .child(Separator::vertical_dashed().label("Dashed")),
                    ),
            )
            .child(
                section("In Context")
                    .description("Horizontal and vertical rules can structure compact content.")
                    .w(px(520.))
                    .child(
                        v_flex()
                            .gap_y_4()
                            .child(
                                v_flex().gap_y_2().child("Hello GPUI Component").child(
                                    Label::new(DESCRIPTION)
                                        .text_color(cx.theme().muted_foreground)
                                        .text_sm(),
                                ),
                            )
                            .child(Separator::horizontal())
                            .child(
                                h_flex()
                                    .gap_x_4()
                                    .child("Docs")
                                    .child(Separator::vertical().dashed())
                                    .child("GitHub")
                                    .child(Separator::vertical().dashed())
                                    .child("Source"),
                            ),
                    ),
            )
    }
}
