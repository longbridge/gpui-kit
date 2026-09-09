use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, Size, avatar::Avatar, badge::Badge,
    dock::PanelControl, v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window, px,
};

use crate::{ChangeStorySize, section, story_toolbar};

pub struct BadgeStory {
    focus_handle: gpui_kit::FocusHandle,
    size: Size,
}

impl BadgeStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            size: Size::Medium,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for BadgeStory {
    fn title() -> &'static str {
        "Badge"
    }

    fn description() -> &'static str {
        "A red dot that indicates the number of unread messages."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for BadgeStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BadgeStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(
                section("Icon")
                    .w_128()
                    .child(
                        Badge::new()
                            .with_size(self.size)
                            .count(3)
                            .child(Icon::new(IconName::Bell).with_size(self.size)),
                    )
                    .child(
                        Badge::new()
                            .with_size(self.size)
                            .count(103)
                            .child(Icon::new(IconName::Inbox).with_size(self.size)),
                    ),
            )
            .child(
                section("Count")
                    .w_128()
                    .child(
                        Badge::new().with_size(self.size).count(3).child(
                            Avatar::new()
                                .with_size(self.size)
                                .src("https://avatars.githubusercontent.com/u/5518?v=4"),
                        ),
                    )
                    .child(
                        Badge::new().with_size(self.size).count(103).child(
                            Avatar::new()
                                .with_size(self.size)
                                .src("https://avatars.githubusercontent.com/u/28998859?v=4"),
                        ),
                    ),
            )
            .child(
                section("Badge icon")
                    .w_128()
                    .child(
                        Badge::new()
                            .with_size(self.size)
                            .icon(IconName::Check)
                            .color(cx.theme().cyan)
                            .child(
                                Avatar::new()
                                    .with_size(self.size)
                                    .src("https://avatars.githubusercontent.com/u/5518?v=4"),
                            ),
                    )
                    .child(
                        Badge::new()
                            .with_size(self.size)
                            .icon(IconName::Star)
                            .color(cx.theme().yellow)
                            .child(
                                Avatar::new()
                                    .with_size(self.size)
                                    .src("https://avatars.githubusercontent.com/u/20092316?v=4"),
                            ),
                    ),
            )
            .child(
                section("Dot").w(px(480.)).child(
                    Badge::new().with_size(self.size).dot().count(1).child(
                        Avatar::new()
                            .with_size(self.size)
                            .src("https://avatars.githubusercontent.com/u/5518?v=4"),
                    ),
                ),
            )
            .child(
                section("Color")
                    .w_128()
                    .child(
                        Badge::new()
                            .with_size(self.size)
                            .count(3)
                            .color(cx.theme().blue)
                            .child(
                                Avatar::new()
                                    .with_size(self.size)
                                    .src("https://avatars.githubusercontent.com/u/5518?v=4"),
                            ),
                    )
                    .child(
                        Badge::new()
                            .with_size(self.size)
                            .dot()
                            .color(cx.theme().green)
                            .count(1)
                            .child(
                                Avatar::new()
                                    .with_size(self.size)
                                    .src("https://avatars.githubusercontent.com/u/5518?v=4"),
                            ),
                    ),
            )
            .child(
                section("Nested")
                    .w_128()
                    .child(
                        Badge::new().with_size(self.size).count(212).large().child(
                            Badge::new()
                                .with_size(self.size)
                                .icon(IconName::Check)
                                .color(cx.theme().cyan)
                                .child(
                                    Avatar::new()
                                        .with_size(self.size)
                                        .src("https://avatars.githubusercontent.com/u/5518?v=4"),
                                ),
                        ),
                    )
                    .child(
                        Badge::new()
                            .with_size(self.size)
                            .count(2)
                            .color(cx.theme().green)
                            .large()
                            .child(
                                Badge::new()
                                    .with_size(self.size)
                                    .icon(IconName::Star)
                                    .color(cx.theme().yellow)
                                    .child(Avatar::new().with_size(self.size).large().src(
                                        "https://avatars.githubusercontent.com/u/20092316?v=4",
                                    )),
                            ),
                    )
                    .child(
                        Badge::new()
                            .with_size(self.size)
                            .count(3)
                            .color(cx.theme().green)
                            .child(
                                Badge::new()
                                    .with_size(self.size)
                                    .icon(IconName::Asterisk)
                                    .color(cx.theme().green)
                                    .child(Avatar::new().with_size(self.size).src(
                                        "https://avatars.githubusercontent.com/u/22312482?v=4",
                                    )),
                            ),
                    )
                    .child(
                        Badge::new().with_size(self.size).dot().child(
                            Badge::new()
                                .with_size(self.size)
                                .icon(IconName::Sun)
                                .color(cx.theme().red)
                                .child(
                                    Avatar::new().with_size(self.size).small().src(
                                        "https://avatars.githubusercontent.com/u/150917089?v=4",
                                    ),
                                ),
                        ),
                    ),
            )
    }
}
