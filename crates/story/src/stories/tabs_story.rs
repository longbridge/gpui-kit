use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde::Deserialize;

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Selectable as _, Sizable, Size,
    button::{Button, ButtonGroup, ButtonVariants},
    h_flex,
    tab::{Tab, TabBar},
    v_flex,
};

use crate::{ChangeStorySize, section, story_toolbar};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = tabs_story, no_json)]
struct ToggleMoreMenu;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = tabs_story, no_json)]
struct SetMaxTabWidth(usize);

/// The max tab widths to choose from in the story, `None` leaves tabs uncapped.
const MAX_WIDTHS: [Option<f32>; 5] = [None, Some(60.), Some(90.), Some(120.), Some(160.)];

pub struct TabsStory {
    focus_handle: FocusHandle,
    active_tab_ix: usize,
    dynamic_active_tab_ix: usize,
    dynamic_tabs: Vec<usize>,
    dynamic_next_tab_id: usize,
    size: Size,
    menu: bool,
    max_width_ix: usize,
}

impl super::Story for TabsStory {
    fn title() -> &'static str {
        "Tabs"
    }

    fn description() -> &'static str {
        "A set of layered sections of content—known as tab panels—that are displayed one at a time."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl TabsStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            active_tab_ix: 0,
            dynamic_active_tab_ix: 0,
            dynamic_tabs: vec![0, 1, 2],
            dynamic_next_tab_id: 3,
            size: Size::default(),
            menu: false,
            max_width_ix: 0,
        }
    }

    fn set_active_tab(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) {
        self.active_tab_ix = ix;
        cx.notify();
    }

    fn set_dynamic_active_tab(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) {
        self.dynamic_active_tab_ix = ix;
        cx.notify();
    }

    fn add_dynamic_tab(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let id = self.dynamic_next_tab_id;
        self.dynamic_next_tab_id += 1;
        self.dynamic_tabs.push(id);
        self.dynamic_active_tab_ix = self.dynamic_tabs.len() - 1;
        cx.notify();
    }

    fn remove_last_dynamic_tab(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if self.dynamic_tabs.len() <= 1 {
            return;
        }

        self.dynamic_tabs.pop();
        if self.dynamic_active_tab_ix >= self.dynamic_tabs.len() {
            self.dynamic_active_tab_ix = self.dynamic_tabs.len() - 1;
        }
        cx.notify();
    }
}

impl Focusable for TabsStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TabsStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let max_width = MAX_WIDTHS[self.max_width_ix].map(px);
        let max_width_label = |width: Option<f32>| match width {
            Some(width) => format!("{width:.0}px"),
            None => "Unlimited".into(),
        };

        v_flex()
            .w_full()
            .gap_3()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleMoreMenu, _, cx| {
                this.menu = !this.menu;
                cx.notify();
            }))
            .on_action(cx.listener(|this, action: &SetMaxTabWidth, _, cx| {
                this.max_width_ix = action.0;
                cx.notify();
            }))
            .child(
                story_toolbar(self.size)
                    .dropdown_child(
                        Button::new("tabs-max-width").label(format!(
                            "Max width: {}",
                            max_width_label(MAX_WIDTHS[self.max_width_ix])
                        )),
                        {
                            let max_width_ix = self.max_width_ix;
                            move |menu, _, _| {
                                MAX_WIDTHS
                                    .iter()
                                    .enumerate()
                                    .fold(menu, |menu, (ix, width)| {
                                        menu.menu_with_check(
                                            max_width_label(*width),
                                            ix == max_width_ix,
                                            Box::new(SetMaxTabWidth(ix)),
                                        )
                                    })
                            }
                        },
                    )
                    .dropdown_child(Button::new("tabs-options").label("Options"), {
                        let menu = self.menu;
                        move |popup, _, _| {
                            popup.menu_with_check("More menu", menu, Box::new(ToggleMoreMenu))
                        }
                    }),
            )
            .child(
                section("Tabs").w_full().child(
                    TabBar::new("tabs")
                        .w_full()
                        .with_size(self.size)
                        .menu(self.menu)
                        .when_some(max_width, |this, max_width| this.max_width(max_width))
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .prefix(
                            h_flex()
                                .mx_1()
                                .child(
                                    Button::new("back")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::ArrowLeft),
                                )
                                .child(
                                    Button::new("forward")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::ArrowRight),
                                ),
                        )
                        .child(Tab::new().label("Account"))
                        .child(Tab::new().label("Profile").disabled(true))
                        .child(Tab::new().label("Documents"))
                        .child(Tab::new().label("Mail"))
                        .child(Tab::new().label("Appearance"))
                        .child(Tab::new().label("Settings"))
                        .child(Tab::new().label("About"))
                        .child(Tab::new().label("License"))
                        .suffix(
                            h_flex()
                                .mx_1()
                                .child(Button::new("inbox").ghost().xsmall().icon(IconName::Inbox))
                                .child(
                                    Button::new("more")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Ellipsis),
                                ),
                        ),
                ),
            )
            .child(
                section("Underline Tabs").w_full().child(
                    TabBar::new("underline")
                        .w_full()
                        .underline()
                        .with_size(self.size)
                        .menu(self.menu)
                        .when_some(max_width, |this, max_width| this.max_width(max_width))
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child("Account")
                        .child("Profile")
                        .child("Documents")
                        .child("Mail")
                        .child("Appearance")
                        .child("Settings")
                        .child("About")
                        .child("License"),
                ),
            )
            .child(
                section("Pill Tabs").w_full().child(
                    TabBar::new("pill")
                        .w_full()
                        .pill()
                        .with_size(self.size)
                        .menu(self.menu)
                        .when_some(max_width, |this, max_width| this.max_width(max_width))
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child(Tab::new().label("Account"))
                        .child(Tab::new().label("Profile").disabled(true))
                        .child(Tab::new().label("Documents & Files"))
                        .child(Tab::new().label("Mail"))
                        .child(Tab::new().label("Appearance"))
                        .child(Tab::new().label("Settings"))
                        .child(Tab::new().label("About"))
                        .child(Tab::new().label("License")),
                ),
            )
            .child(
                section("Outline Tabs").w_full().child(
                    TabBar::new("outline")
                        .w_full()
                        .outline()
                        .with_size(self.size)
                        .menu(self.menu)
                        .when_some(max_width, |this, max_width| this.max_width(max_width))
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child(Tab::new().label("Account"))
                        .child(Tab::new().label("Profile").disabled(true))
                        .child(Tab::new().label("Documents & Files"))
                        .child(Tab::new().label("Mail"))
                        .child(Tab::new().label("Appearance"))
                        .child(Tab::new().label("Settings"))
                        .child(Tab::new().label("About"))
                        .child(Tab::new().label("License")),
                ),
            )
            .child(
                section("Segmented Tabs").w_full().child(
                    TabBar::new("segmented")
                        .w_full()
                        .segmented()
                        .with_size(self.size)
                        .menu(self.menu)
                        .when_some(max_width, |this, max_width| this.max_width(max_width))
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child(IconName::Bot)
                        .child(IconName::Calendar)
                        .child(IconName::Map)
                        .children(vec!["Appearance", "Settings", "About", "License"]),
                ),
            )
            .child(
                section("Dynamic Tabs")
                    .description(
                        "Tabs can be added, removed, and composed with prefix and suffix content.",
                    )
                    .w_full()
                    .child(
                        ButtonGroup::new("dynamic-tab-actions")
                            .outline()
                            .compact()
                            .child(
                                Button::new("add-tab")
                                    .label("Add Tab")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.add_dynamic_tab(window, cx);
                                    })),
                            )
                            .child(Button::new("remove-tab").label("Remove Last").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.remove_last_dynamic_tab(window, cx);
                                }),
                            )),
                    )
                    .child(
                        TabBar::new("segmented-dynamic")
                            .w_full()
                            .segmented()
                            .with_size(self.size)
                            .when_some(max_width, |this, max_width| this.max_width(max_width))
                            .selected_index(self.dynamic_active_tab_ix)
                            .on_click(cx.listener(|this, ix: &usize, window, cx| {
                                this.set_dynamic_active_tab(*ix, window, cx);
                            }))
                            .children(self.dynamic_tabs.iter().enumerate().map(|(ix, id)| {
                                let label = format!("Tab {id}");
                                Tab::new()
                                    .px_2()
                                    .prefix(Icon::new(IconName::BookOpen))
                                    .label(label)
                                    .suffix(
                                        Button::new(format!("dynamic-tab-close-{id}"))
                                            .ghost()
                                            .xsmall()
                                            .icon(IconName::Close),
                                    )
                                    .selected(self.dynamic_active_tab_ix == ix)
                            })),
                    ),
            )
            .child(
                section("Filling Space")
                    .description("Segmented tabs can share the available width equally.")
                    .w_full()
                    .child(
                        TabBar::new("flex tabs")
                            .w_full()
                            .segmented()
                            .with_size(self.size)
                            .when_some(max_width, |this, max_width| this.max_width(max_width))
                            .selected_index(self.active_tab_ix)
                            .on_click(cx.listener(|this, ix: &usize, window, cx| {
                                this.set_active_tab(*ix, window, cx);
                            }))
                            .child(Tab::new().flex_1().label("About"))
                            .child(Tab::new().flex_1().label("Profile")),
                    ),
            )
    }
}
