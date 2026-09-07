use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use gpui_kit::component::{
    IconName, Placement,
    button::{Button, ButtonVariant, ButtonVariants, Toggle},
    checkbox::Checkbox,
    clipboard::Clipboard,
    dock::PanelControl,
    h_flex,
    radio::Radio,
    switch::Switch,
    tooltip::Tooltip,
    v_flex,
};

use crate::{Story, section};

actions!(tooltip_story, [Info]);

pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-shift-delete", Info, Some("Tooltip"))]);
}

pub struct TooltipStory {
    focus_handle: gpui_kit::FocusHandle,
    removable_button_visible: bool,
}

impl TooltipStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            removable_button_visible: true,
        }
    }
}

impl Story for TooltipStory {
    fn title() -> &'static str {
        "Tooltip"
    }

    fn description() -> &'static str {
        "Describe a control on hover."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for TooltipStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TooltipStory {
    fn render(
        &mut self,
        _: &mut gpui_kit::Window,
        cx: &mut gpui_kit::Context<Self>,
    ) -> impl gpui_kit::IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .child(
                section("Button")
                    .description("Prefer the left, bottom, or right side, with an optional keyboard shortcut hint.")
                    .child(
                        Button::new("btn0")
                            .label("Search")
                            .with_variant(ButtonVariant::Primary)
                            .tooltip("This is a search Button.")
                            .tooltip_placement(Placement::Left),
                    )
                    .child(
                        Button::new("btn1")
                            .label("Info")
                            .tooltip_with_action(
                                "This is a tooltip with Action for display keybinding.",
                                &Info,
                                Some("Tooltip"),
                            )
                            .tooltip_placement(Placement::Bottom),
                    )
                    .child(
                        Button::new("btn2")
                            .label("Hover me")
                            .tooltip("This tooltip prefers the right side.")
                            .tooltip_placement(Placement::Right),
                    ),
            )
            .child(
                section("Checkbox")
                    .description("Tooltips work on selection controls.")
                    .child(
                        Checkbox::new("check")
                            .label("Remember me")
                            .checked(true)
                            .tooltip("This is a tooltip"),
                    ),
            )
            .child(
                section("Radio")
                    .description("Explain an individual radio option.")
                    .child(
                        Radio::new("radio")
                            .label("Radio with tooltip")
                            .checked(true)
                            .tooltip("This is a radio button"),
                    ),
            )
            .child(
                section("Switch")
                    .description("Add context without extending the visible label.")
                    .child(
                        Switch::new("switch")
                            .checked(true)
                            .tooltip("This is a switch"),
                    ),
            )
            .child(
                section("Toggle")
                    .description("Describe text and icon-only toggles.")
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Toggle::new("toggle1").label("Bold").tooltip("Toggle bold"))
                            .child(
                                Toggle::new("toggle2")
                                    .icon(IconName::Heart)
                                    .tooltip("Toggle favorite"),
                            ),
                    ),
            )
            .child(
                section("Clipboard")
                    .description("Clarify the copy action.")
                    .child(
                        Clipboard::new("clip1")
                            .value("Hello, World!")
                            .tooltip("Copy to clipboard"),
                    ),
            )
            .child(
                section("Custom content")
                    .description("Build tooltip content with an action hint.")
                    .child(
                        div()
                            .child("Hover me")
                            .id("tooltip-2")
                            .tooltip(|window, cx| {
                                Tooltip::new("This is a default tooltip style by GPUI.")
                                    .action(&Info, Some("Tooltip"))
                                    .build(window, cx)
                            }),
                    ),
            )
            .child(
                section("Removed trigger")
                    .description("Dismiss cleanly when the trigger leaves the view.")
                    .child(
                        h_flex()
                            .gap_2()
                            .when(self.removable_button_visible, |this| {
                                this.child(
                                    Button::new("remove-tooltip-trigger")
                                        .danger()
                                        .label("Remove me")
                                        .tooltip("Clicking this button removes the trigger.")
                                        .on_click(cx.listener(|story, _, _, cx| {
                                            story.removable_button_visible = false;
                                            cx.notify();
                                        })),
                                )
                            })
                            .when(!self.removable_button_visible, |this| {
                                this.child(
                                    Button::new("restore-tooltip-trigger")
                                        .label("Restore button")
                                        .on_click(cx.listener(|story, _, _, cx| {
                                            story.removable_button_visible = true;
                                            cx.notify();
                                        })),
                                )
                            }),
                    ),
            )
    }
}
