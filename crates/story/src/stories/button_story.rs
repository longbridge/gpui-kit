use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use gpui_kit::component::{
    ActiveTheme, Disableable as _, Icon, IconName, Selectable as _, Sizable as _, Size, Theme,
    button::{Button, ButtonCustomVariant, ButtonGroup, ButtonVariants as _},
    h_flex,
    progress::ProgressCircle,
    v_flex,
};
use serde::Deserialize;

use crate::{ChangeStorySize, section, story_toolbar};

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = button_story, no_json)]
enum ButtonAction {
    Disabled,
    Loading,
    Selected,
    Compact,
    Shadow,
    Multiple,
}

pub struct ButtonStory {
    focus_handle: gpui_kit::FocusHandle,
    disabled: bool,
    loading: bool,
    selected: bool,
    compact: bool,
    toggle_multiple: bool,
    size: Size,
}

impl ButtonStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            disabled: false,
            loading: false,
            selected: false,
            compact: false,
            toggle_multiple: false,
            size: Size::Medium,
        })
    }

    fn on_click(ev: &ClickEvent, _: &mut Window, _: &mut App) {
        println!("Button clicked {:?}", ev);
    }

    fn on_hover(hovered: &bool, _: &mut Window, _: &mut App) {
        println!("Button hovered {:?}", hovered);
    }
}

impl super::Story for ButtonStory {
    fn title() -> &'static str {
        "Button"
    }

    fn description() -> &'static str {
        "Displays a button or a component that looks like a button."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ButtonStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ButtonStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let disabled = self.disabled;
        let loading = self.loading;
        let selected = self.selected;
        let compact = self.compact;
        let toggle_multiple = self.toggle_multiple;
        let button = |id: &'static str| Button::new(id).with_size(self.size);

        let custom_variant = ButtonCustomVariant::new(cx)
            .color(cx.theme().magenta)
            .foreground(cx.theme().magenta)
            .hover(cx.theme().magenta.opacity(0.1))
            .active(cx.theme().magenta.opacity(0.2));

        v_flex()
            .on_action(cx.listener(|this, action: &ButtonAction, window, cx| {
                match action {
                    ButtonAction::Disabled => this.disabled = !this.disabled,
                    ButtonAction::Loading => this.loading = !this.loading,
                    ButtonAction::Selected => this.selected = !this.selected,
                    ButtonAction::Compact => this.compact = !this.compact,
                    ButtonAction::Shadow => {
                        let mut theme = cx.theme().clone();
                        theme.shadow = !theme.shadow;
                        cx.set_global::<Theme>(theme);
                        window.refresh();
                    }
                    ButtonAction::Multiple => this.toggle_multiple = !this.toggle_multiple,
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .gap_6()
            .child(story_toolbar(self.size).dropdown_child(
                button("button-options").label("Options"),
                {
                    let disabled = self.disabled;
                    let loading = self.loading;
                    let selected = self.selected;
                    let compact = self.compact;
                    let shadow = cx.theme().shadow;
                    let multiple = self.toggle_multiple;
                    move |menu, _, _| {
                        menu.menu_with_check("Disabled", disabled, Box::new(ButtonAction::Disabled))
                            .menu_with_check("Loading", loading, Box::new(ButtonAction::Loading))
                            .menu_with_check("Selected", selected, Box::new(ButtonAction::Selected))
                            .menu_with_check("Compact", compact, Box::new(ButtonAction::Compact))
                            .menu_with_check("Shadow", shadow, Box::new(ButtonAction::Shadow))
                            .separator()
                            .menu_with_check(
                                "Multiple selection",
                                multiple,
                                Box::new(ButtonAction::Multiple),
                            )
                    }
                },
            ))
            .child(
                section("Variants")
                    .description("Visual treatments communicate action priority.")
                    .w_128()
                    .child(
                        button("button-0")
                            .label("Default")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        button("button-1")
                            .primary()
                            .label("Primary")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        button("button-2")
                            .secondary()
                            .label("Secondary")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        button("button-4")
                            .danger()
                            .label("Danger")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        button("button-4-warning")
                            .warning()
                            .label("Warning")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        button("button-4-success")
                            .success()
                            .label("Success")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        button("button-5-info")
                            .info()
                            .label("Info")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        button("button-5-ghost")
                            .ghost()
                            .label("Ghost")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        button("button-5-link")
                            .link()
                            .label("Link")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        button("button-5-text")
                            .text()
                            .label("Text")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    ),
            )
            .child(
                section("Icons")
                    .description("Icons can lead labels or appear in custom content.")
                    .child(
                        button("button-icon-1")
                            .outline()
                            .label("Confirm")
                            .icon(IconName::Check)
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-icon-2")
                            .outline()
                            .label("Abort")
                            .icon(IconName::Close)
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-icon-3")
                            .outline()
                            .label("Maximize")
                            .icon(Icon::new(IconName::Maximize))
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-icon-4")
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child("Custom Child")
                                    .child(IconName::ChevronDown)
                                    .child(IconName::Eye),
                            )
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-icon-5-ghost")
                            .ghost()
                            .icon(IconName::Check)
                            .label("Confirm")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-icon-6-link")
                            .link()
                            .icon(IconName::Check)
                            .label("Link")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-icon-6-text")
                            .text()
                            .icon(IconName::Check)
                            .label("Text Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    ),
            )
            .child(
                section("Progress")
                    .description("Buttons can show determinate progress.")
                    .child(
                        h_flex()
                            .gap_4()
                            .child(
                                button("progress-button-1")
                                    .primary()
                                    .icon(
                                        ProgressCircle::new("circle-progress-1")
                                            .color(cx.theme().primary_foreground)
                                            .value(25.),
                                    )
                                    .label("Installing..."),
                            )
                            .child(
                                button("progress-button-2")
                                    .icon(ProgressCircle::new("circle-progress-2").value(35.))
                                    .label("Installing..."),
                            )
                            .child(
                                button("progress-button-3")
                                    .icon(ProgressCircle::new("circle-progress-3").value(68.))
                                    .label("Installing..."),
                            )
                            .child(
                                button("progress-button-4")
                                    .icon(ProgressCircle::new("circle-progress-4").value(85.))
                                    .label("Installing..."),
                            ),
                    ),
            )
            .child(
                section("Outline")
                    .description("Outlined treatments keep actions visually quiet.")
                    .w_128()
                    .child(
                        button("button-outline-1")
                            .primary()
                            .outline()
                            .label("Primary Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-outline-2")
                            .outline()
                            .label("Normal Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-outline-4-danger")
                            .danger()
                            .outline()
                            .label("Danger Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-outline-4-warning")
                            .warning()
                            .outline()
                            .label("Warning Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-outline-4-success")
                            .success()
                            .outline()
                            .label("Success Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-outline-5-info")
                            .info()
                            .outline()
                            .label("Info Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-outline-5-ghost")
                            .ghost()
                            .outline()
                            .label("Ghost Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-outline-5-link")
                            .link()
                            .outline()
                            .label("Link Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-outline-5-text")
                            .text()
                            .outline()
                            .label("Text Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    ),
            )
            .child(
                section("Dropdown")
                    .description("A caret indicates an attached menu.")
                    .w_128()
                    .child(
                        button("button-dropdown-caret-primary")
                            .primary()
                            .dropdown_caret(true)
                            .label("Primary Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-dropdown-caret-default")
                            .label("Default Button")
                            .dropdown_caret(true)
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-outline-3")
                            .secondary()
                            .label("Secondary Button")
                            .dropdown_caret(true)
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-dropdown-caret-ghost")
                            .ghost()
                            .dropdown_caret(true)
                            .label("Ghost Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-dropdown-caret-link")
                            .link()
                            .dropdown_caret(true)
                            .label("Link Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-dropdown-caret-small")
                            .outline()
                            .dropdown_caret(true)
                            .label("Small Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    ),
            )
            .child(
                section("Horizontal group").child(
                    ButtonGroup::new("button-group")
                        .outline()
                        .disabled(disabled)
                        .child(
                            button("button-one")
                                .label("One")
                                .disabled(disabled)
                                .selected(selected)
                                .when(compact, |this| this.compact())
                                .on_click(Self::on_click),
                        )
                        .child(
                            button("button-two")
                                .label("Two")
                                .disabled(disabled)
                                .selected(selected)
                                .when(compact, |this| this.compact())
                                .on_click(Self::on_click),
                        )
                        .child(
                            button("button-three")
                                .label("Three")
                                .disabled(disabled)
                                .selected(selected)
                                .when(compact, |this| this.compact())
                                .on_click(Self::on_click),
                        ),
                ),
            )
            .child(
                section("Vertical group").child(
                    ButtonGroup::new("button-group-vertical")
                        .outline()
                        .layout(Axis::Vertical)
                        .disabled(disabled)
                        .child(
                            button("button-vertical-one")
                                .label("One")
                                .disabled(disabled)
                                .selected(selected)
                                .when(compact, |this| this.compact())
                                .on_click(Self::on_click),
                        )
                        .child(
                            button("button-vertical-two")
                                .label("Two")
                                .disabled(disabled)
                                .selected(selected)
                                .when(compact, |this| this.compact())
                                .on_click(Self::on_click),
                        )
                        .child(
                            button("button-vertical-three")
                                .label("Three")
                                .disabled(disabled)
                                .selected(selected)
                                .when(compact, |this| this.compact())
                                .on_click(Self::on_click),
                        ),
                ),
            )
            .child(
                section("Selection group")
                    .description("Groups support single or multiple selection.")
                    .child(
                        ButtonGroup::new("toggle-button-group")
                            .outline()
                            .compact()
                            .multiple(toggle_multiple)
                            .child(
                                button("disabled-toggle-button")
                                    .label("Disabled")
                                    .selected(disabled),
                            )
                            .child(
                                button("loading-toggle-button")
                                    .label("Loading")
                                    .selected(loading),
                            )
                            .child(
                                button("selected-toggle-button")
                                    .label("Selected")
                                    .selected(selected),
                            )
                            .child(
                                button("compact-toggle-button")
                                    .label("Compact")
                                    .selected(compact),
                            )
                            .on_click(cx.listener(|view, selected: &Vec<usize>, _, cx| {
                                view.disabled = selected.contains(&0);
                                view.loading = selected.contains(&1);
                                view.selected = selected.contains(&2);
                                view.compact = selected.contains(&3);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("Icon-only")
                    .description("Compact actions can omit visible labels.")
                    .child(
                        button("icon-button-primary")
                            .icon(IconName::Search)
                            .loading_icon(IconName::LoaderCircle)
                            .primary()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        button("icon-button-secondary")
                            .icon(IconName::Info)
                            .loading(true)
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        button("icon-button-danger")
                            .icon(IconName::Close)
                            .danger()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        button("icon-button-small-primary")
                            .icon(IconName::Search)
                            .primary()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        button("icon-button-outline")
                            .icon(IconName::Search)
                            .outline()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        button("icon-button-ghost")
                            .icon(IconName::ArrowLeft)
                            .loading_icon(IconName::LoaderCircle)
                            .ghost()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    ),
            )
            .child(
                section("Custom size")
                    .description("A fixed pixel size is available for compact icon actions.")
                    .child(
                        button("icon-button-9")
                            .icon(IconName::Heart)
                            .size(px(24.))
                            .ghost()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    ),
            )
            .child(
                section("Custom color")
                    .child(
                        button("button-6-custom")
                            .custom(custom_variant)
                            .label("Custom Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-outline-6-custom")
                            .outline()
                            .custom(custom_variant)
                            .label("Outline Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        button("button-outline-6-custom-1")
                            .outline()
                            .icon(IconName::Bell)
                            .custom(custom_variant)
                            .label("Icon Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    ),
            )
    }
}
