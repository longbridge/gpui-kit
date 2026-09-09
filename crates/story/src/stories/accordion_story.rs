use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable, Size, StyledExt as _,
    accordion::{Accordion, AccordionItem},
    button::Button,
    checkbox::Checkbox,
    h_flex,
    switch::Switch,
    tag::Tag,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde::Deserialize;

use crate::{ChangeStorySize, section, story_toolbar};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = accordion_story, no_json)]
enum ToggleOption {
    Multiple,
    Icon,
    Disabled,
    Bordered,
}

/// The theme-derived values a settings row draws with, read once so the row
/// builder itself needs no app context.
#[derive(Clone, Copy)]
struct SettingsItemStyle {
    icon_bg: Hsla,
    muted: Hsla,
    icon_radius: Pixels,
}

/// A settings row: the icon sits in a rounded square, and the content lines up
/// with the title rather than with the icon.
fn settings_item(
    item: AccordionItem,
    icon: IconName,
    title: &'static str,
    tag: Option<Tag>,
    body: &'static str,
    style: SettingsItemStyle,
) -> AccordionItem {
    let SettingsItemStyle {
        icon_bg,
        muted,
        icon_radius,
    } = style;

    item.title(
        h_flex()
            .gap_2()
            .items_center()
            .child(
                h_flex()
                    .flex_none()
                    .size(px(32.))
                    .items_center()
                    .justify_center()
                    .rounded(icon_radius)
                    .bg(icon_bg)
                    .child(Icon::new(icon).small().text_color(muted)),
            )
            .child(div().font_semibold().child(title))
            .children(tag.map(|tag| tag.small())),
    )
    .title_style({
        let mut style = StyleRefinement::default();
        style.padding.top = Some(px(8.).into());
        style.padding.bottom = Some(px(8.).into());
        style
    })
    .content_style({
        let mut style = StyleRefinement::default();
        style.text.color = Some(muted);
        // Past the icon square, so the text starts under the title.
        style.padding.left = Some(px(52.).into());
        style.padding.top = Some(px(0.).into());
        style.padding.bottom = Some(px(12.).into());
        style
    })
    .child(body)
}

pub struct AccordionStory {
    open_ixs: Vec<usize>,
    styled_open_ixs: Vec<usize>,
    size: Size,
    bordered: bool,
    disabled: bool,
    multiple: bool,
    show_icon: bool,
    focus_handle: FocusHandle,
}

impl super::Story for AccordionStory {
    fn title() -> &'static str {
        "Accordion"
    }

    fn description() -> &'static str {
        "The accordion uses collapse internally to make it collapsible."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl AccordionStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            bordered: false,
            open_ixs: vec![0],
            styled_open_ixs: vec![0],
            size: Size::default(),
            disabled: false,
            multiple: false,
            show_icon: false,
            focus_handle: cx.focus_handle(),
        }
    }

    fn toggle_accordion(&mut self, open_ixs: Vec<usize>, _: &mut Window, cx: &mut Context<Self>) {
        self.open_ixs = open_ixs;
        cx.notify();
    }
}

impl Focusable for AccordionStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AccordionStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_item_style = SettingsItemStyle {
            icon_bg: cx.theme().secondary.opacity(0.5),
            muted: cx.theme().muted_foreground,
            icon_radius: cx.theme().radius,
        };

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .on_action(cx.listener(|this, action: &ToggleOption, _, cx| {
                match action {
                    ToggleOption::Multiple => this.multiple = !this.multiple,
                    ToggleOption::Icon => this.show_icon = !this.show_icon,
                    ToggleOption::Disabled => this.disabled = !this.disabled,
                    ToggleOption::Bordered => this.bordered = !this.bordered,
                }
                cx.notify();
            }))
            .child(
                story_toolbar(self.size)
                    .items_center()
                    .flex_wrap()
                    .dropdown_child(Button::new("accordion-options").label("Options"), {
                        let multiple = self.multiple;
                        let show_icon = self.show_icon;
                        let disabled = self.disabled;
                        let bordered = self.bordered;
                        move |menu, _, _| {
                            menu.menu_with_check(
                                "Multiple",
                                multiple,
                                Box::new(ToggleOption::Multiple),
                            )
                            .menu_with_check("Icons", show_icon, Box::new(ToggleOption::Icon))
                            .menu_with_check("Disabled", disabled, Box::new(ToggleOption::Disabled))
                            .menu_with_check(
                                "Bordered",
                                bordered,
                                Box::new(ToggleOption::Bordered),
                            )
                        }
                    }),
            )
            .child(
                section("Default")
                    .description("Expand one item at a time by default.")
                    .child(
                        div().w(px(480.)).child(
                            Accordion::new("test")
                                .bordered(self.bordered)
                                .with_size(self.size)
                                .disabled(self.disabled)
                                .multiple(self.multiple)
                                .item(|this| {
                                    this.open(self.open_ixs.contains(&0))
                                    .when(self.show_icon, |this| this.icon(IconName::Info))
                                    .title("Is it accessible?")
                                    .child(
                                        "Yes. Each item is a button with an aria-expanded state, \
                                    so screen readers announce whether the section is open, \
                                    and the whole group can be reached with the keyboard.",
                                    )
                                })
                                .item(|this| {
                                    this.open(self.open_ixs.contains(&1))
                                        .when(self.show_icon, |this| this.icon(IconName::Inbox))
                                        .title("Can it hold any content?")
                                        .child(
                                            v_flex()
                                                .gap_3()
                                                .child(
                                                    "An item takes any element as its content, \
                                            not just text. The height animation measures \
                                            whatever you put in it.",
                                                )
                                                .child(
                                                    h_flex()
                                                        .gap_4()
                                                        .child(
                                                            Switch::new("switch1").label("Switch"),
                                                        )
                                                        .child(
                                                            Checkbox::new("checkbox1")
                                                                .label("Or a Checkbox"),
                                                        ),
                                                ),
                                        )
                                })
                                .item(|this| {
                                    this.open(self.open_ixs.contains(&2))
                                        .when(self.show_icon, |this| this.icon(IconName::Moon))
                                        .title("Is it animated?")
                                        .child(
                                            "Yes. Expanding and collapsing animates the height of \
                                    the content, and the chevron rotates to follow. \
                                    Items below move along with it.",
                                        )
                                })
                                .on_toggle_click(cx.listener(
                                    |this, open_ixs: &[usize], window, cx| {
                                        this.toggle_accordion(open_ixs.to_vec(), window, cx);
                                    },
                                )),
                        ),
                    ),
            )
            .child(
                section("Custom style").child(
                    // A tinted frame around the card.
                    div()
                        .w(px(480.))
                        .p(px(4.))
                        .rounded(cx.theme().radius_lg * 2.)
                        .bg(cx.theme().secondary.opacity(0.5))
                        .border_1()
                        .border_color(cx.theme().border.opacity(0.5))
                        .child(
                            Accordion::new("custom-style")
                                .multiple(self.multiple)
                                .disabled(self.disabled)
                                .item(|this| {
                                    settings_item(
                                        this,
                                        IconName::Settings,
                                        "Account Settings",
                                        Some(Tag::success().outline().child("New")),
                                        "Manage your account preferences, security settings, \
                                    and personal information. You can also configure \
                                    two-factor authentication here.",
                                        settings_item_style,
                                    )
                                    .open(self.styled_open_ixs.contains(&0))
                                })
                                .item(|this| {
                                    settings_item(
                                        this,
                                        IconName::Eye,
                                        "Privacy & Security",
                                        None,
                                        "Control who can see your profile and how your data \
                                    is used.",
                                        settings_item_style,
                                    )
                                    .open(self.styled_open_ixs.contains(&1))
                                })
                                .item(|this| {
                                    settings_item(
                                        this,
                                        IconName::Info,
                                        "Help & Support",
                                        None,
                                        "Browse the documentation, or get in touch with the \
                                    support team.",
                                        settings_item_style,
                                    )
                                    .open(self.styled_open_ixs.contains(&2))
                                })
                                .on_toggle_click(cx.listener(|this, open_ixs: &[usize], _, cx| {
                                    this.styled_open_ixs = open_ixs.to_vec();
                                    cx.notify();
                                })),
                        ),
                ),
            )
    }
}
