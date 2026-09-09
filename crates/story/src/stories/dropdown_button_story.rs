use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde::Deserialize;

use crate::{ChangeStorySize, section, story_toolbar};
use gpui_kit::component::{
    ActiveTheme, Disableable, Selectable as _, Sizable as _, Size, Theme,
    button::{Button, ButtonVariants as _, DropdownButton},
    h_flex, v_flex,
};

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = dropdown_button_story, no_json)]
enum ButtonAction {
    Disabled,
    Loading,
    Selected,
    Compact,
    Shadow,
    ExportCsv,
    ExportPdf,
    SaveCopy,
    SaveTemplate,
    OpenQuarterlyReport,
    OpenWatchlistLayout,
}

pub struct DropdownButtonStory {
    focus_handle: gpui_kit::FocusHandle,
    disabled: bool,
    loading: bool,
    selected: bool,
    compact: bool,
    size: Size,
    last_action: SharedString,
}

impl DropdownButtonStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            disabled: false,
            loading: false,
            selected: false,
            compact: false,
            size: Size::Medium,
            last_action: "Nothing yet".into(),
        })
    }
}

impl super::Story for DropdownButtonStory {
    fn title() -> &'static str {
        "DropdownButton"
    }

    fn description() -> &'static str {
        "A button with an attached dropdown menu for additional options."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for DropdownButtonStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DropdownButtonStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let disabled = self.disabled;
        let loading = self.loading;
        let selected = self.selected;
        let compact = self.compact;
        let view = cx.entity();

        v_flex()
            .gap_6()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
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
                    ButtonAction::ExportCsv => this.last_action = "Exported as CSV".into(),
                    ButtonAction::ExportPdf => this.last_action = "Exported as PDF".into(),
                    ButtonAction::SaveCopy => this.last_action = "Saved as a new file".into(),
                    ButtonAction::SaveTemplate => this.last_action = "Saved as a template".into(),
                    ButtonAction::OpenQuarterlyReport => {
                        this.last_action = "Opened Quarterly Report.gpui".into()
                    }
                    ButtonAction::OpenWatchlistLayout => {
                        this.last_action = "Opened Watchlist Layout.gpui".into()
                    }
                }
                cx.notify();
            }))
            .child(story_toolbar(self.size).dropdown_child(
                Button::new("dropdown-button-options").label("Options"),
                {
                    let shadow = cx.theme().shadow;
                    move |menu, _, _| {
                        menu.menu_with_check("Disabled", disabled, Box::new(ButtonAction::Disabled))
                            .menu_with_check("Loading", loading, Box::new(ButtonAction::Loading))
                            .menu_with_check("Selected", selected, Box::new(ButtonAction::Selected))
                            .menu_with_check("Compact", compact, Box::new(ButtonAction::Compact))
                            .menu_with_check("Shadow", shadow, Box::new(ButtonAction::Shadow))
                    }
                },
            ))
            .child(
                h_flex()
                    .gap_1()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Last action:")
                    .child(self.last_action.clone()),
            )
            .child(
                section("Basic split").child(
                    DropdownButton::new("export")
                        .with_size(self.size)
                        .primary()
                        .button(
                            Button::new("export-default")
                                .label("Export")
                                .when(self.compact, |this| this.compact())
                                .on_click({
                                    let view = view.clone();
                                    move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.last_action = "Exported current view".into();
                                            cx.notify();
                                        });
                                    }
                                }),
                        )
                        .disabled(self.disabled)
                        .selected(selected)
                        .dropdown_menu_with_anchor(Anchor::TopRight, move |this, _, _| {
                            this.menu("Export all rows (.csv)", Box::new(ButtonAction::ExportCsv))
                                .menu("Download report (.pdf)", Box::new(ButtonAction::ExportPdf))
                        }),
                ),
            )
            .child(
                section("Inner button options").child(
                    DropdownButton::new("save")
                        .with_size(self.size)
                        .outline()
                        .button(
                            Button::new("save-default")
                                .label("Save")
                                .tooltip("Save the current document")
                                .when(compact, |this| this.compact())
                                .loading(loading)
                                .on_click({
                                    let view = view.clone();
                                    move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.last_action = "Saved document".into();
                                            cx.notify();
                                        });
                                    }
                                }),
                        )
                        .disabled(disabled)
                        .dropdown_menu(move |this, _, _| {
                            this.menu("Save as new file…", Box::new(ButtonAction::SaveCopy))
                                .menu("Save as template…", Box::new(ButtonAction::SaveTemplate))
                        }),
                ),
            )
            .child(
                section("Inherited styling").child(
                    DropdownButton::new("recent")
                        .button(
                            Button::new("recent-default")
                                .label("Open latest")
                                .ghost()
                                .small()
                                .on_click({
                                    let view = view.clone();
                                    move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.last_action = "Opened latest file".into();
                                            cx.notify();
                                        });
                                    }
                                }),
                        )
                        .selected(selected)
                        .disabled(disabled)
                        .dropdown_menu(move |this, _, _| {
                            this.menu(
                                "Quarterly Report.gpui",
                                Box::new(ButtonAction::OpenQuarterlyReport),
                            )
                            .menu(
                                "Watchlist Layout.gpui",
                                Box::new(ButtonAction::OpenWatchlistLayout),
                            )
                        }),
                ),
            )
    }
}
