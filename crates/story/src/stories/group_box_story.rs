use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render,
    StyleRefinement, Styled, Window, relative,
};

use gpui_kit::component::{
    ActiveTheme as _, StyledExt,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex,
    radio::{Radio, RadioGroup},
    switch::Switch,
    text::markdown,
    v_flex,
};

use crate::section;

pub struct GroupBoxStory {
    focus_handle: gpui_kit::FocusHandle,
    email_options: [bool; 3],
    profile_private: bool,
    private_contributions: bool,
    compact_private: bool,
    theme: Option<usize>,
}

impl super::Story for GroupBoxStory {
    fn title() -> &'static str {
        "GroupBox"
    }

    fn description() -> &'static str {
        "A styled container element that with an optional title \
        to groups related content together."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl GroupBoxStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            email_options: [false; 3],
            profile_private: true,
            private_contributions: false,
            compact_private: true,
            theme: Some(2),
        }
    }
}

impl Focusable for GroupBoxStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GroupBoxStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .gap_6()
            .child(
                section("Default").w_128().child(
                    GroupBox::new()
                        .title("Email notifications")
                        .child(
                            Checkbox::new("all")
                                .label("All activity")
                                .checked(self.email_options[0])
                                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                    this.email_options[0] = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Checkbox::new("news-letter")
                                .label("Product updates")
                                .checked(self.email_options[1])
                                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                    this.email_options[1] = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Checkbox::new("account-activity")
                                .label("Account activity")
                                .checked(self.email_options[2])
                                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                    this.email_options[2] = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(Button::new("ok").primary().label("Save preferences")),
                ),
            )
            .child(
                section("Filled").w_128().child(
                    GroupBox::new()
                        .id("activity")
                        .fill()
                        .title("Contributions & activity")
                        .child(
                            h_flex()
                                .justify_between()
                                .child("Make profile private and hide activity")
                                .child(
                                    Switch::new("profile-private")
                                        .checked(self.profile_private)
                                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                            this.profile_private = *checked;
                                            cx.notify();
                                        })),
                                ),
                        )
                        .child(
                            h_flex()
                                .justify_between()
                                .child("Include private contributions on my profile")
                                .child(
                                    Switch::new("private-contributions")
                                        .checked(self.private_contributions)
                                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                            this.private_contributions = *checked;
                                            cx.notify();
                                        })),
                                ),
                        )
                        .child(Button::new("btn-1").primary().label("Save")),
                ),
            )
            .child(
                section("Outlined").w_128().child(
                    GroupBox::new()
                        .id("appearance")
                        .outline()
                        .title("Appearance")
                        .child(
                            RadioGroup::vertical("theme")
                                .child(Radio::new("light").label("Light"))
                                .child(Radio::new("dark").label("Dark"))
                                .child(Radio::new("system").label("System"))
                                .selected_index(self.theme)
                                .on_click(cx.listener(|this, selected: &usize, _, cx| {
                                    this.theme = Some(*selected);
                                    cx.notify();
                                })),
                        ),
                ),
            )
            .child(
                section("Without Title").w_128().child(
                    GroupBox::new().outline().child(
                        h_flex()
                            .justify_between()
                            .child("Make profile private and hide activity")
                            .child(
                                Switch::new("compact-private")
                                    .checked(self.compact_private)
                                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                        this.compact_private = *checked;
                                        cx.notify();
                                    })),
                            ),
                    ),
                ),
            )
            .child(
                section("Custom Style").w_128().child(
                    GroupBox::new()
                        .outline()
                        .bg(cx.theme().group_box)
                        .rounded_xl()
                        .p_5()
                        .title("This is a custom style")
                        .title_style(
                            StyleRefinement::default()
                                .font_semibold()
                                .line_height(relative(1.0))
                                .px_3(),
                        )
                        .content_style(
                            StyleRefinement::default()
                                .rounded_xl()
                                .py_3()
                                .px_4()
                                .border_2(),
                        )
                        .child(markdown(
                            "You can use `title_style` to customize the style \
                                of the title. \n \
                                And any style in `GroupBox` will apply to the content container.",
                        )),
                ),
            )
    }
}
