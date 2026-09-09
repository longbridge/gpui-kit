use gpui_kit::component::{ActiveTheme as _, IconName, Sizable, Size, spinner::Spinner, v_flex};
use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Window, bounce, ease_in_out, ease_out_quint, linear, px,
};

use crate::{ChangeStorySize, section, story_toolbar};

pub struct SpinnerStory {
    focus_handle: gpui_kit::FocusHandle,
    value: f32,
    size: Size,
}

impl super::Story for SpinnerStory {
    fn title() -> &'static str {
        "Spinner"
    }

    fn description() -> &'static str {
        "Displays an spinner showing the completion progress of a task."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SpinnerStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            value: 50.,
            size: Size::Medium,
        }
    }

    pub fn set_value(&mut self, value: f32) {
        self.value = value;
    }
}

impl Focusable for SpinnerStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SpinnerStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(
                section("Default")
                    .description("An indeterminate loading indicator.")
                    .gap_x_2()
                    .child(Spinner::new().with_size(self.size)),
            )
            .child(
                section("Color")
                    .description("Use a color that suits the surrounding status.")
                    .gap_x_2()
                    .child(Spinner::new().with_size(self.size).color(cx.theme().blue))
                    .child(Spinner::new().with_size(self.size).color(cx.theme().green)),
            )
            .child(
                section("Custom size")
                    .description("A fixed pixel size is also supported.")
                    .gap_x_2()
                    .child(Spinner::new().with_size(px(64.))),
            )
            .child(
                section("Icon")
                    .description("Replace the default spinner glyph.")
                    .gap_x_2()
                    .child(
                        Spinner::new()
                            .with_size(self.size)
                            .icon(IconName::LoaderCircle),
                    )
                    .child(
                        Spinner::new()
                            .with_size(self.size)
                            .icon(IconName::LoaderCircle)
                            .color(cx.theme().cyan),
                    ),
            )
            .child(
                section("Easing")
                    .description("Customize the rotation timing curve.")
                    .gap_x_2()
                    .child(
                        Spinner::new()
                            .with_size(self.size)
                            .icon(IconName::Loader)
                            .ease(linear),
                    )
                    .child(
                        Spinner::new()
                            .with_size(self.size)
                            .icon(IconName::Loader)
                            .ease(bounce(ease_in_out)),
                    )
                    .child(
                        Spinner::new()
                            .with_size(self.size)
                            .icon(IconName::Loader)
                            .ease(ease_out_quint()),
                    ),
            )
    }
}
