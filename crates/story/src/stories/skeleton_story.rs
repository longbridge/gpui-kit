use gpui_kit::component::{ActiveTheme as _, ThemeStyled as _, skeleton::Skeleton, v_flex};
use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Window, px,
};

use crate::section;

pub struct SkeletonStory {
    focus_handle: gpui_kit::FocusHandle,
    value: f32,
}

impl super::Story for SkeletonStory {
    fn title() -> &'static str {
        "Skeleton"
    }

    fn description() -> &'static str {
        "Use to show a placeholder while content is loading."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SkeletonStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            value: 50.,
        }
    }

    pub fn set_value(&mut self, value: f32) {
        self.value = value;
    }
}

impl Focusable for SkeletonStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SkeletonStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .child(
                section("Text")
                    .description("Represents an avatar and text while profile content loads.")
                    .w(px(360.))
                    .child(
                        gpui_kit::component::h_flex()
                            .w_full()
                            .gap_3()
                            .child(Skeleton::new().size_12().rounded_full_style(cx))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_2()
                                    .child(
                                        Skeleton::new().w_full().h_4().rounded(cx.theme().radius),
                                    )
                                    .child(
                                        Skeleton::new().w_2_3().h_4().rounded(cx.theme().radius),
                                    ),
                            ),
                    ),
            )
            .child(
                section("Card")
                    .description("Combines media and text placeholders in a content card.")
                    .w(px(360.))
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                Skeleton::new()
                                    .w_full()
                                    .h(px(180.))
                                    .rounded(cx.theme().radius),
                            )
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        Skeleton::new().w_full().h_4().rounded(cx.theme().radius),
                                    )
                                    .child(
                                        Skeleton::new()
                                            .w(px(200.))
                                            .h_4()
                                            .rounded(cx.theme().radius),
                                    ),
                            ),
                    ),
            )
    }
}
