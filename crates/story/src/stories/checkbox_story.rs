use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Window, div, px,
};

use gpui_kit::component::{
    ActiveTheme, Disableable as _, Sizable, Size, StyledExt, checkbox::Checkbox, h_flex,
    text::markdown, v_flex,
};

use crate::{ChangeStorySize, section, story_toolbar};

pub struct CheckboxStory {
    focus_handle: gpui_kit::FocusHandle,
    check1: bool,
    check2: bool,
    check3: bool,
    check4: bool,
    check5: bool,
    check6: bool,
    size: Size,
}

impl super::Story for CheckboxStory {
    fn title() -> &'static str {
        "Checkbox"
    }

    fn description() -> &'static str {
        "Select one or more independent options."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl CheckboxStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            check1: false,
            check2: true,
            check3: false,
            check4: false,
            check5: false,
            check6: false,
            size: Size::default(),
        }
    }
}

impl Focusable for CheckboxStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CheckboxStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .justify_start()
            .gap_3()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(
                section("Default")
                    .description("Checked and unchecked options can be mixed freely.")
                    .child(
                        Checkbox::new("updates")
                            .with_size(self.size)
                            .checked(self.check1)
                            .label("Product updates")
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.check1 = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("remember")
                            .with_size(self.size)
                            .checked(self.check2)
                            .label("Remember this device")
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.check2 = *checked;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("Without label")
                    .description("The label can be supplied by surrounding content.")
                    .child(
                        Checkbox::new("unlabelled")
                            .with_size(self.size)
                            .checked(self.check3)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.check3 = *checked;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("Disabled")
                    .description("Both checked and unchecked values remain visible.")
                    .w_128()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_6()
                            .child(
                                Checkbox::new("disabled-checked")
                                    .with_size(self.size)
                                    .label("Checked")
                                    .checked(true)
                                    .disabled(true),
                            )
                            .child(
                                Checkbox::new("disabled-unchecked")
                                    .with_size(self.size)
                                    .label("Unchecked")
                                    .checked(false)
                                    .disabled(true),
                            ),
                    ),
            )
            .child(
                section("Labels")
                    .description("Labels can wrap and include supporting content.")
                    .w_128()
                    .v_flex()
                    .items_center()
                    .gap_5()
                    .child(
                        Checkbox::new("description")
                            .with_size(self.size)
                            .w(px(320.))
                            .checked(self.check4)
                            .label("Automatic updates")
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Download updates when the application is idle."),
                            )
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.check4 = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("wrapping")
                            .with_size(self.size)
                            .w(px(320.))
                            .checked(self.check6)
                            .label("Notify me when a new device signs in to my account")
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.check6 = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("markdown")
                            .with_size(self.size)
                            .w(px(320.))
                            .checked(self.check5)
                            .label("Accept the terms")
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(markdown(
                                        "Read the [terms of service](https://github.com) before continuing.",
                                    )),
                            )
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.check5 = *checked;
                                cx.notify();
                            })),
                    ),
            )
    }
}
