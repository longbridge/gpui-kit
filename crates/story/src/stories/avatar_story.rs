use gpui_kit::component::{
    ActiveTheme, IconName, Sizable as _, Size, StyledExt,
    avatar::{Avatar, AvatarGroup},
    dock::PanelControl,
    v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window, px,
};

use crate::{ChangeStorySize, section, story_toolbar};

const AVATARS: [&str; 11] = [
    "https://avatars.githubusercontent.com/u/5518?v=4",
    "https://avatars.githubusercontent.com/u/28998859?v=4",
    "https://avatars.githubusercontent.com/u/20092316?v=4",
    "https://avatars.githubusercontent.com/u/22312482?v=4",
    "https://avatars.githubusercontent.com/u/150917089?v=4",
    "https://avatars.githubusercontent.com/u/20337280?v=4",
    "https://avatars.githubusercontent.com/u/629429?v=4",
    "https://avatars.githubusercontent.com/u/583231?v=4",
    "https://avatars.githubusercontent.com/u/1264109?v=4",
    "https://avatars.githubusercontent.com/u/2936367?v=4",
    "https://avatars.githubusercontent.com/u/1253486?v=4",
];

pub struct AvatarStory {
    focus_handle: gpui_kit::FocusHandle,
    size: Size,
}

impl AvatarStory {
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

impl super::Story for AvatarStory {
    fn title() -> &'static str {
        "Avatar"
    }

    fn description() -> &'static str {
        "Represent a person or organization with an image or fallback."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for AvatarStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AvatarStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(
                section("Image")
                    .description("Use an image when one is available.")
                    .w_128()
                    .child(
                        Avatar::new()
                            .name("Jason Lee")
                            .src(AVATARS[0])
                            .with_size(self.size),
                    )
                    .child(Avatar::new().src(AVATARS[1]).with_size(self.size)),
            )
            .child(
                section("Fallback")
                    .description("Show initials or an icon when no image is available.")
                    .w_128()
                    .child(Avatar::new().name("Jason Lee").with_size(self.size))
                    .child(Avatar::new().with_size(self.size))
                    .child(
                        Avatar::new()
                            .placeholder(IconName::Building2)
                            .with_size(self.size),
                    ),
            )
            .child(
                section("Group")
                    .description("Groups can limit visible avatars and show overflow.")
                    .v_flex()
                    .w_128()
                    .items_center()
                    .gap_5()
                    .child(
                        AvatarGroup::new()
                            .with_size(self.size)
                            .children(AVATARS[..6].iter().map(|src| Avatar::new().src(*src))),
                    )
                    .child(
                        AvatarGroup::new()
                            .with_size(self.size)
                            .limit(5)
                            .ellipsis()
                            .children(AVATARS.iter().map(|src| Avatar::new().src(*src))),
                    ),
            )
            .child(
                section("Custom shape")
                    .description("Set an explicit size and corner radius.")
                    .child(
                        Avatar::new()
                            .src(AVATARS[0])
                            .with_size(px(100.))
                            .rounded(px(20.)),
                    ),
            )
            .child(
                section("Custom style")
                    .description("Add borders and shadows to the image.")
                    .child(
                        Avatar::new()
                            .src(AVATARS[2])
                            .with_size(px(100.))
                            .border_3()
                            .border_color(cx.theme().foreground)
                            .shadow_sm(),
                    ),
            )
    }
}
