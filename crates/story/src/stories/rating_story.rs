use gpui_kit::component::{ActiveTheme, Size, rating::Rating, v_flex};
use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Window, px,
};

use crate::{ChangeStorySize, section, story_toolbar};

pub struct RatingStory {
    focus_handle: gpui_kit::FocusHandle,
    size: Size,
    value: usize,
}

impl super::Story for RatingStory {
    fn title() -> &'static str {
        "Rating"
    }

    fn description() -> &'static str {
        "A simple interactive star rating component."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl RatingStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            size: Size::default(),
            value: 3,
        }
    }
}

impl Focusable for RatingStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

pub fn init(_cx: &mut App) {
    // No global init required for RatingStory
}

impl Render for RatingStory {
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
                    .description("Select a value directly from the rating.")
                    .w_128()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_3()
                            .justify_center()
                            .items_center()
                            .child(
                                Rating::new("rating-1")
                                    .with_size(self.size)
                                    .value(self.value)
                                    .max(5)
                                    .on_click(cx.listener(|this, value: &usize, _, cx| {
                                        this.value = *value;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                section("Disabled").w(px(480.)).child(
                    Rating::new("rating-2")
                        .with_size(self.size)
                        .value(2)
                        .color(cx.theme().green)
                        .max(5)
                        .disabled(true),
                ),
            )
            .child(
                section("Color").w(px(480.)).child(
                    Rating::new("rating-3")
                        .with_size(self.size)
                        .value(self.value)
                        .color(cx.theme().green)
                        .max(5),
                ),
            )
    }
}
