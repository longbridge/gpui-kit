use std::collections::HashSet;

use gpui_kit::component::{
    ActiveTheme, Icon, IconName, Sizable, StyledExt,
    avatar::Avatar,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    collapsible::Collapsible,
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex,
    progress::Progress,
    tag::Tag,
    v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Div, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Stateful, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::section;

/// Keys of the panels, each one tracks its own open state.
const ORDER: &str = "order";
const FAQ: &str = "faq";
const USAGE: &str = "usage";
const SETTINGS: &str = "settings";
const API_KEYS: &str = "api-keys";
const COMPONENTS_DIR: &str = "components-dir";
const UI_DIR: &str = "ui-dir";
const PROFILE: &str = "profile";

/// Order details, as (title, value) pairs.
const ORDER_DETAILS: [(&str, &str); 2] = [
    ("Shipping address", "100 Market St, San Francisco"),
    ("Items", "2x Studio Headphones"),
];

/// The usage breakdown of the billing example, as (label, value) pairs.
const USAGE_ITEMS: [(&str, &str); 4] = [
    ("Requests", "$210.84"),
    ("Active CPU", "$21.95"),
    ("Events", "$21.20"),
    ("Storage", "$20.45"),
];

/// The settings rows, as (key, label) pairs.
const NOTIFICATIONS: [(&str, &str); 3] = [
    ("push", "Push notifications"),
    ("email", "Email notifications"),
    ("sms", "SMS notifications"),
];

/// The keys listed by the row actions example, as (name, key) pairs.
const API_KEY_ROWS: [(&str, &str); 3] = [
    ("Production", "PRDK230454*242SDIFPPL"),
    ("Development", "DUILO30454*242SDIFUIP"),
    ("Staging", "IPPODAS230454*242SDI"),
];

/// The profile fields, as (icon, label, value) triples.
const PROFILE_FIELDS: [(IconName, &str, &str); 3] = [
    (IconName::Inbox, "Last activity", "2 hours ago"),
    (IconName::Calendar, "Online since", "Today, 9:00 AM"),
    (IconName::Globe, "Location", "Hong Kong"),
];

/// A bordered row that frames a piece of summary content.
fn panel_row(cx: &App) -> Div {
    h_flex()
        .px_3()
        .py_2()
        .gap_2()
        .items_center()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .text_sm()
}

/// The chevron of a row trigger: pointing right when collapsed, down when open.
fn chevron(open: bool, cx: &App) -> Icon {
    Icon::new(if open {
        IconName::ChevronDown
    } else {
        IconName::ChevronRight
    })
    .xsmall()
    .text_color(cx.theme().muted_foreground)
}

/// A leaf row of the tree example: a file icon and its name.
fn file_row(name: &'static str, cx: &App) -> impl IntoElement {
    let accent = cx.theme().accent;

    h_flex()
        .h_7()
        .px_2()
        .gap_2()
        .items_center()
        .rounded(cx.theme().radius)
        .hover(|this| this.bg(accent))
        .text_sm()
        // Aligns the name with the folder names, which are preceded by a chevron.
        .child(div().w_3())
        .child(
            Icon::new(IconName::File)
                .xsmall()
                .text_color(cx.theme().muted_foreground),
        )
        .child(name)
}

pub struct CollapsibleStory {
    focus_handle: FocusHandle,
    /// Keys of the panels that are expanded.
    open: HashSet<&'static str>,
    /// Keys of the settings rows that are checked.
    checked: HashSet<&'static str>,
}

impl super::Story for CollapsibleStory {
    fn title() -> &'static str {
        "Collapsible"
    }

    fn description() -> &'static str {
        "An interactive element that expands/collapses."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl CollapsibleStory {
    pub(crate) fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            open: HashSet::from([SETTINGS, API_KEYS, COMPONENTS_DIR, PROFILE]),
            checked: HashSet::from(["push"]),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn is_open(&self, key: &'static str) -> bool {
        self.open.contains(key)
    }

    fn toggle(&mut self, key: &'static str, cx: &mut Context<Self>) {
        if !self.open.remove(key) {
            self.open.insert(key);
        }
        cx.notify();
    }

    /// A row that toggles `key` when clicked anywhere along it.
    fn trigger_row(&self, key: &'static str, cx: &mut Context<Self>) -> Stateful<Div> {
        h_flex()
            .id(key)
            .w_full()
            .items_center()
            .on_click(cx.listener(move |this, _, _, cx| this.toggle(key, cx)))
    }

    /// A folder row of the tree example, the whole row toggles the folder.
    fn folder_row(
        &self,
        key: &'static str,
        name: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open = self.is_open(key);
        let accent = cx.theme().accent;
        let muted_foreground = cx.theme().muted_foreground;
        let radius = cx.theme().radius;

        self.trigger_row(key, cx)
            .h_7()
            .px_2()
            .gap_2()
            .rounded(radius)
            .hover(|this| this.bg(accent))
            .text_sm()
            .child(chevron(open, cx))
            .child(
                Icon::new(if open {
                    IconName::FolderOpen
                } else {
                    IconName::Folder
                })
                .xsmall()
                .text_color(muted_foreground),
            )
            .child(name)
    }

    /// A trigger beside the title, with a summary row that stays visible.
    fn render_basic(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted_foreground = cx.theme().muted_foreground;

        section("Basic")
            .description("A trigger beside the title, with a summary that stays visible.")
            .w(px(360.))
            .v_flex()
            .child(
                Collapsible::new()
                    .w_full()
                    .gap_2()
                    .open(self.is_open(ORDER))
                    .child(
                        h_flex()
                            .px_1()
                            .justify_between()
                            .items_center()
                            .gap_4()
                            .child(div().text_sm().font_semibold().child("Order #4189"))
                            .child(
                                Button::new(ORDER)
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::ChevronsUpDown)
                                    .tooltip("Toggle details")
                                    .on_click(cx.listener(|this, _, _, cx| this.toggle(ORDER, cx))),
                            ),
                    )
                    .child(
                        panel_row(cx)
                            .justify_between()
                            .bg(cx.theme().muted.opacity(0.3))
                            .child(div().text_color(muted_foreground).child("Status"))
                            .child(Tag::success().small().child("Shipped")),
                    )
                    .content(
                        v_flex()
                            .gap_2()
                            .children(ORDER_DETAILS.map(|(title, value)| {
                                panel_row(cx)
                                    .v_flex()
                                    .items_start()
                                    .gap_0()
                                    .child(div().font_medium().child(title))
                                    .child(div().text_color(muted_foreground).child(value))
                            })),
                    ),
            )
    }

    /// The whole row acts as the trigger.
    fn render_row_trigger(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted_foreground = cx.theme().muted_foreground;

        section("Row trigger")
            .description("The whole row is the trigger, as used by FAQ entries.")
            .w(px(360.))
            .v_flex()
            .child(
                GroupBox::new().outline().w_full().child(
                    Collapsible::new()
                        .w_full()
                        .open(self.is_open(FAQ))
                        .child(
                            self.trigger_row(FAQ, cx)
                                .justify_between()
                                .gap_2()
                                .child(div().text_sm().child("How do I reset my password?"))
                                .child(chevron(self.is_open(FAQ), cx)),
                        )
                        .content(div().pt_3().text_sm().text_color(muted_foreground).child(
                            "Click the Forgot Password link on the sign in page, \
                                     and we will send you an email with instructions to \
                                     create a new one.",
                        )),
                ),
            )
    }

    /// A round trigger sitting on the bottom edge of a card.
    fn render_bottom_trigger(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.is_open(USAGE);
        let muted_foreground = cx.theme().muted_foreground;

        section("Bottom trigger")
            .description("The trigger sits on the bottom edge of the card it opens.")
            .w(px(360.))
            .v_flex()
            .child(
                div()
                    .relative()
                    .w_full()
                    .child(
                        GroupBox::new()
                            .outline()
                            .w_full()
                            .title(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .items_center()
                                    .child(div().text_sm().child("3 days remaining in cycle"))
                                    .child(
                                        Button::new("billing").outline().xsmall().label("Billing"),
                                    ),
                            )
                            .child(
                                Collapsible::new()
                                    .w_full()
                                    .gap_3()
                                    .open(open)
                                    .child(
                                        panel_row(cx)
                                            .v_flex()
                                            .items_stretch()
                                            .gap_2()
                                            .bg(cx.theme().muted.opacity(0.6))
                                            .child(
                                                h_flex()
                                                    .justify_between()
                                                    .font_medium()
                                                    .child("$18.08 / $20")
                                                    .child("$200"),
                                            )
                                            .child(Progress::new(USAGE).value(90.)),
                                    )
                                    .content(v_flex().gap_2().children(USAGE_ITEMS.map(
                                        |(label, value)| {
                                            h_flex()
                                                .justify_between()
                                                .text_xs()
                                                .font_medium()
                                                .child(
                                                    div().text_color(muted_foreground).child(label),
                                                )
                                                .child(value)
                                        },
                                    ))),
                            ),
                    )
                    .child(
                        h_flex()
                            .absolute()
                            .bottom(px(-12.))
                            .left_0()
                            .right_0()
                            .justify_center()
                            .child(
                                Button::new("toggle-usage")
                                    .outline()
                                    .xsmall()
                                    .rounded(cx.theme().radius_full())
                                    .bg(cx.theme().background)
                                    .icon(if open {
                                        IconName::ChevronUp
                                    } else {
                                        IconName::ChevronDown
                                    })
                                    .tooltip("Toggle details")
                                    .on_click(cx.listener(|this, _, _, cx| this.toggle(USAGE, cx))),
                            ),
                    ),
            )
    }

    /// Optional settings held behind a full width trigger.
    fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let border = cx.theme().border;

        section("Settings")
            .description("Holds optional controls, keeping the default view short.")
            .w(px(360.))
            .v_flex()
            .child(
                Collapsible::new()
                    .w_full()
                    .gap_2()
                    .open(self.is_open(SETTINGS))
                    .child(
                        Button::new(SETTINGS)
                            .outline()
                            .w_full()
                            .justify_start()
                            .icon(chevron(self.is_open(SETTINGS), cx))
                            .label("Notification settings")
                            .on_click(cx.listener(|this, _, _, cx| this.toggle(SETTINGS, cx))),
                    )
                    .content(
                        v_flex()
                            .w_full()
                            .rounded(cx.theme().radius_lg)
                            .border_1()
                            .border_color(border)
                            .children(NOTIFICATIONS.into_iter().enumerate().map(
                                |(ix, (key, label))| {
                                    h_flex()
                                        .px_3()
                                        .py_2()
                                        .when(ix > 0, |this| this.border_t_1().border_color(border))
                                        .child(
                                            Checkbox::new(key)
                                                .label(label)
                                                .checked(self.checked.contains(key))
                                                .on_click(cx.listener(
                                                    move |this, checked: &bool, _, cx| {
                                                        if *checked {
                                                            this.checked.insert(key);
                                                        } else {
                                                            this.checked.remove(key);
                                                        }
                                                        cx.notify();
                                                    },
                                                )),
                                        )
                                },
                            )),
                    ),
            )
    }

    /// A list where the header and every row carry their own action.
    fn render_row_actions(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted;
        let radius = cx.theme().radius;

        section("Row actions")
            .description("Actions live beside the trigger, in the header and in every row.")
            .w(px(360.))
            .v_flex()
            .child(
                GroupBox::new().outline().w_full().child(
                    Collapsible::new()
                        .w_full()
                        .gap_3()
                        .open(self.is_open(API_KEYS))
                        .child(
                            h_flex()
                                .w_full()
                                .gap_2()
                                .child(
                                    self.trigger_row(API_KEYS, cx)
                                        .flex_1()
                                        .gap_2()
                                        .child(chevron(self.is_open(API_KEYS), cx))
                                        .child(div().text_sm().font_medium().child("API Keys")),
                                )
                                .child(
                                    Button::new("add-key")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Plus)
                                        .tooltip("Add key"),
                                ),
                        )
                        .content(v_flex().gap_2().children(API_KEY_ROWS.map(|(name, key)| {
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    h_flex()
                                        .flex_none()
                                        .size(px(20.))
                                        .items_center()
                                        .justify_center()
                                        .rounded(radius)
                                        .bg(muted)
                                        .child(
                                            Icon::new(IconName::Asterisk)
                                                .xsmall()
                                                .text_color(cx.theme().green),
                                        ),
                                )
                                .child(div().w_20().flex_none().text_xs().child(name))
                                .child(
                                    div()
                                        .flex_1()
                                        .overflow_hidden()
                                        .px_2()
                                        .py(px(2.))
                                        .rounded(radius)
                                        .bg(muted)
                                        .text_xs()
                                        .child(key),
                                )
                                .child(
                                    Button::new(name)
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Ellipsis)
                                        .tooltip("More"),
                                )
                        }))),
                ),
            )
    }

    /// Panels nested inside panels.
    fn render_tree(&self, cx: &mut Context<Self>) -> impl IntoElement {
        section("Nested")
            .description("Panels nest to any depth, here as a file tree.")
            .w(px(360.))
            .v_flex()
            .child(
                GroupBox::new().outline().w_full().child(
                    v_flex()
                        .w_full()
                        .child(
                            Collapsible::new()
                                .w_full()
                                .open(self.is_open(COMPONENTS_DIR))
                                .child(self.folder_row(COMPONENTS_DIR, "components", cx))
                                .content(
                                    v_flex()
                                        .pl_3()
                                        .child(
                                            Collapsible::new()
                                                .w_full()
                                                .open(self.is_open(UI_DIR))
                                                .child(self.folder_row(UI_DIR, "ui", cx))
                                                .content(
                                                    v_flex().pl_3().children(
                                                        ["button.rs", "card.rs", "dialog.rs"]
                                                            .map(|name| file_row(name, cx)),
                                                    ),
                                                ),
                                        )
                                        .child(file_row("login_form.rs", cx)),
                                ),
                        )
                        .child(file_row("main.rs", cx)),
                ),
            )
    }

    /// An identity row that stays visible, with the details folded behind it.
    fn render_profile(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted_foreground = cx.theme().muted_foreground;

        section("Profile")
            .description("Shows who someone is, and their details only on request.")
            .w(px(360.))
            .v_flex()
            .child(
                GroupBox::new().outline().w_full().child(
                    Collapsible::new()
                        .w_full()
                        .gap_3()
                        .open(self.is_open(PROFILE))
                        .child(
                            self.trigger_row(PROFILE, cx)
                                .justify_between()
                                .gap_2()
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(Avatar::new().name("Jason Lee").xsmall())
                                        .child(div().text_sm().font_medium().child("@huacnlee")),
                                )
                                .child(chevron(self.is_open(PROFILE), cx)),
                        )
                        .content(v_flex().gap_2().children(PROFILE_FIELDS.map(
                            |(icon, label, value)| {
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .text_xs()
                                    .child(Icon::new(icon).xsmall().text_color(muted_foreground))
                                    .child(div().text_color(muted_foreground).child(label))
                                    .child(div().font_medium().child(value))
                            },
                        ))),
                ),
            )
    }
}

impl Focusable for CollapsibleStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CollapsibleStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .child(self.render_basic(cx))
            .child(self.render_row_trigger(cx))
            .child(self.render_bottom_trigger(cx))
            .child(self.render_settings(cx))
            .child(self.render_row_actions(cx))
            .child(self.render_tree(cx))
            .child(self.render_profile(cx))
    }
}
