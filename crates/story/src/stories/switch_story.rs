use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window, div, px,
};

use gpui_kit::component::{
    ActiveTheme, Disableable as _, Sizable, Size, StyledExt, h_flex, separator::Separator,
    switch::Switch, v_flex,
};

use crate::{ChangeStorySize, section, story_toolbar};

pub struct SwitchStory {
    focus_handle: FocusHandle,
    switch1: bool,
    switch2: bool,
    switch3: bool,
    switch4: bool,
    switch5: bool,
    size: Size,
}

impl super::Story for SwitchStory {
    fn title() -> &'static str {
        "Switch"
    }

    fn description() -> &'static str {
        "Turn a setting on or off."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SwitchStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            switch1: true,
            switch2: false,
            switch3: true,
            switch4: true,
            switch5: false,
            size: Size::default(),
        }
    }
}

impl Focusable for SwitchStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SwitchStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

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
                    .description("Switches work well in a compact settings list.")
                    .w_128()
                    .items_stretch()
                    .child(
                        v_flex()
                            .w_full()
                            .border_1()
                            .border_color(theme.border)
                            .rounded(theme.radius_lg)
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_6()
                                    .p_4()
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(div().font_medium().child("Product updates"))
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(theme.muted_foreground)
                                                    .child("New features and release notes."),
                                            ),
                                    )
                                    .child(
                                        Switch::new("switch1")
                                            .with_size(self.size)
                                            .checked(self.switch1)
                                            .on_click(cx.listener(|this, checked, _, cx| {
                                                this.switch1 = *checked;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(Separator::horizontal())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_6()
                                    .p_4()
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(div().font_medium().child("Security alerts"))
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(theme.muted_foreground)
                                                    .child("Important activity on your account."),
                                            ),
                                    )
                                    .child(
                                        Switch::new("switch2")
                                            .with_size(self.size)
                                            .checked(self.switch2)
                                            .on_click(cx.listener(|this, checked, _, cx| {
                                                this.switch2 = *checked;
                                                cx.notify();
                                            })),
                                    ),
                            ),
                    ),
            )
            .child(
                section("Disabled")
                    .description("Unavailable switches preserve their current value.")
                    .w_128()
                    .child(
                        Switch::new("switch3")
                            .with_size(self.size)
                            .checked(self.switch3)
                            .disabled(true),
                    )
                    .child(
                        Switch::new("switch3_1")
                            .with_size(self.size)
                            .w(px(200.))
                            .label("Airplane mode")
                            .checked(true)
                            .disabled(true),
                    ),
            )
            .child(
                section("Color")
                    .description("Semantic colors can reinforce the setting state.")
                    .child(
                        Switch::new("switch4")
                            .with_size(self.size)
                            .checked(self.switch4)
                            .label("Success")
                            .color(theme.success)
                            .on_click(cx.listener(|this, checked, _, cx| {
                                this.switch4 = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Switch::new("switch5")
                            .with_size(self.size)
                            .checked(self.switch5)
                            .label("Destructive")
                            .color(theme.danger)
                            .on_click(cx.listener(|this, checked, _, cx| {
                                this.switch5 = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Switch::new("switch4_disabled")
                            .with_size(self.size)
                            .checked(true)
                            .label("Disabled")
                            .color(theme.success)
                            .disabled(true),
                    ),
            )
    }
}
