use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window, px,
};

use gpui_kit::component::{
    ColorName, Sizable, Size, h_flex, indigo_50, indigo_500, tag::Tag, v_flex,
};

use crate::{ChangeStorySize, section, story_toolbar};

pub struct TagStory {
    focus_handle: FocusHandle,
    size: Size,
}

impl super::Story for TagStory {
    fn title() -> &'static str {
        "Tag"
    }

    fn description() -> &'static str {
        "A short item that can be used to categorize or label content."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl TagStory {
    pub(crate) fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            size: Size::Medium,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}
impl Focusable for TagStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for TagStory {
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
                section("Default").child(
                    h_flex()
                        .gap_2()
                        .child(Tag::primary().with_size(self.size).child("Tag"))
                        .child(Tag::secondary().with_size(self.size).child("Secondary"))
                        .child(Tag::danger().with_size(self.size).child("Danger"))
                        .child(Tag::success().with_size(self.size).child("Success"))
                        .child(Tag::warning().with_size(self.size).child("Warning"))
                        .child(Tag::info().with_size(self.size).child("Info"))
                        .child(
                            Tag::custom(indigo_500(), indigo_50(), indigo_500())
                                .with_size(self.size)
                                .child("Custom"),
                        ),
                ),
            )
            .child(
                section("Outline").child(
                    h_flex()
                        .gap_2()
                        .child(Tag::primary().with_size(self.size).outline().child("Tag"))
                        .child(
                            Tag::secondary()
                                .with_size(self.size)
                                .outline()
                                .child("Secondary"),
                        )
                        .child(Tag::danger().with_size(self.size).outline().child("Danger"))
                        .child(
                            Tag::success()
                                .with_size(self.size)
                                .outline()
                                .child("Success"),
                        )
                        .child(
                            Tag::warning()
                                .with_size(self.size)
                                .outline()
                                .child("Warning"),
                        )
                        .child(Tag::info().with_size(self.size).outline().child("Info"))
                        .child(
                            Tag::custom(indigo_500(), indigo_500(), indigo_500())
                                .with_size(self.size)
                                .outline()
                                .child("Custom"),
                        ),
                ),
            )
            .child(
                section("Rounded").child(
                    h_flex()
                        .gap_2()
                        .child(
                            Tag::primary()
                                .with_size(self.size)
                                .rounded_full()
                                .child("Tag"),
                        )
                        .child(
                            Tag::secondary()
                                .with_size(self.size)
                                .rounded_full()
                                .child("Secondary"),
                        )
                        .child(
                            Tag::danger()
                                .with_size(self.size)
                                .rounded_full()
                                .child("Danger"),
                        )
                        .child(
                            Tag::success()
                                .with_size(self.size)
                                .rounded_full()
                                .child("Success"),
                        )
                        .child(
                            Tag::warning()
                                .with_size(self.size)
                                .rounded_full()
                                .child("Warning"),
                        )
                        .child(
                            Tag::info()
                                .with_size(self.size)
                                .rounded_full()
                                .child("Info"),
                        ),
                ),
            )
            .child(
                section("Square").child(
                    h_flex()
                        .gap_2()
                        .child(
                            Tag::primary()
                                .with_size(self.size)
                                .rounded(px(0.))
                                .child("Tag"),
                        )
                        .child(
                            Tag::secondary()
                                .with_size(self.size)
                                .rounded(px(0.))
                                .child("Secondary"),
                        )
                        .child(
                            Tag::danger()
                                .with_size(self.size)
                                .rounded(px(0.))
                                .child("Danger"),
                        )
                        .child(
                            Tag::success()
                                .with_size(self.size)
                                .rounded(px(0.))
                                .child("Success"),
                        )
                        .child(
                            Tag::warning()
                                .with_size(self.size)
                                .rounded(px(0.))
                                .child("Warning"),
                        )
                        .child(
                            Tag::info()
                                .with_size(self.size)
                                .rounded(px(0.))
                                .child("Info"),
                        ),
                ),
            )
            .child(
                section("Colors").w(px(640.)).child(
                    v_flex().w_full().gap_4().child(
                        h_flex().w_full().gap_2().flex_wrap().children(
                            ColorName::all()
                                .into_iter()
                                .filter(|color| *color != ColorName::Gray)
                                .map(|color| {
                                    Tag::color(color)
                                        .with_size(self.size)
                                        .child(color.to_string())
                                }),
                        ),
                    ),
                ),
            )
    }
}
