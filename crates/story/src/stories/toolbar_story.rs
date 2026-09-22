use gpui_kit::assets::IconName as AssetIconName;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IconName, IndexPath, Sizable as _, Size,
    button::{Button, Toggle},
    combobox::{Combobox, ComboboxState},
    dock::PanelControl,
    input::{Input, InputState},
    searchable_list::SearchableVec,
    select::{Select, SelectState},
    separator::Separator,
    toolbar::{Toolbar, ToolbarGroup},
    v_flex,
};
use gpui_kit::{
    Action, App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, Window, div, px,
};
use serde::Deserialize;

use crate::{ChangeStorySize, section, story_toolbar_group};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = toolbar_story, no_json)]
struct ToggleDisabled;

pub struct ToolbarStory {
    focus_handle: FocusHandle,
    size: Size,
    disabled: bool,
    formats: [bool; 3],
    font: Entity<SelectState<Vec<&'static str>>>,
    market: Entity<SelectState<Vec<&'static str>>>,
    status: Entity<ComboboxState<SearchableVec<&'static str>>>,
    query: Entity<InputState>,
}

impl ToolbarStory {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let font = cx.new(|cx| {
            SelectState::new(
                vec!["Inter", "SF Pro", "Helvetica", "Georgia"],
                Some(IndexPath::default()),
                window,
                cx,
            )
        });
        let market = cx.new(|cx| {
            SelectState::new(
                vec!["All markets", "US market", "Hong Kong", "Singapore"],
                Some(IndexPath::default()),
                window,
                cx,
            )
        });
        let status = cx.new(|cx| {
            ComboboxState::new(
                SearchableVec::new(vec!["Open", "In progress", "Filled", "Cancelled"]),
                vec![],
                window,
                cx,
            )
            .searchable(true)
        });
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));

        Self {
            focus_handle: cx.focus_handle(),
            size: Size::Medium,
            disabled: false,
            formats: [true, false, false],
            font,
            market,
            status,
            query,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

fn icon_button(id: &'static str, icon: IconName, tooltip: &'static str, disabled: bool) -> Button {
    Button::new(id)
        .icon(icon)
        .tooltip(tooltip)
        .disabled(disabled)
}

fn toolbar_options(size: Size, disabled: bool) -> impl IntoElement {
    let label = match size {
        Size::XSmall => "XSmall",
        Size::Small => "Small",
        _ => "Medium",
    };

    story_toolbar_group().dropdown_child(
        Button::new("toolbar-options").label(format!("Size: {label}")),
        move |menu, _, _| {
            menu.menu_with_check(
                "XSmall",
                size == Size::XSmall,
                Box::new(ChangeStorySize(Size::XSmall)),
            )
            .menu_with_check(
                "Small",
                size == Size::Small,
                Box::new(ChangeStorySize(Size::Small)),
            )
            .menu_with_check(
                "Medium",
                size == Size::Medium,
                Box::new(ChangeStorySize(Size::Medium)),
            )
            .separator()
            .menu_with_check("Disabled", disabled, Box::new(ToggleDisabled))
        },
    )
}

impl super::Story for ToolbarStory {
    fn title() -> &'static str {
        "Toolbar"
    }

    fn description() -> &'static str {
        "Groups commands and controls into one keyboard-navigable row."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for ToolbarStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ToolbarStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleDisabled, _, cx| {
                this.disabled = !this.disabled;
                cx.notify();
            }))
            .child(toolbar_options(self.size, self.disabled))
            .child(
                section("Default")
                    .description("Keep document, history, and formatting commands in one compact editor toolbar.")
                    .w(px(640.))
                    .child(
                        Toolbar::new("default-toolbar")
                            .w_full()
                            .with_size(self.size)
                            .disabled(self.disabled)
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .child(
                                ToolbarGroup::new("document-group")
                                    .label("Document")
                                    .gap_1()
                                    .child(
                                        Button::new("new-document")
                                            .icon(IconName::Plus)
                                            .label("New")
                                            .disabled(self.disabled),
                                    )
                                    .child(
                                        Button::new("save-document")
                                            .icon(AssetIconName::Save)
                                            .label("Save")
                                            .disabled(self.disabled),
                                    ),
                            )
                            .content(Separator::vertical().h_5())
                            .child(
                                ToolbarGroup::new("history-group")
                                    .label("History")
                                    .gap_1()
                                    .child(icon_button(
                                        "undo",
                                        IconName::Undo2,
                                        "Undo",
                                        self.disabled,
                                    ))
                                    .child(icon_button(
                                        "redo",
                                        IconName::Redo2,
                                        "Redo",
                                        self.disabled,
                                    )),
                            )
                            .content(Separator::vertical().h_5())
                            .child(
                                ToolbarGroup::new("formatting-group")
                                    .label("Formatting")
                                    .gap_1()
                                    .child(
                                        Toggle::new("bold")
                                            .label("B")
                                            .checked(self.formats[0])
                                            .disabled(self.disabled)
                                            .on_click(cx.listener(|this, checked, _, cx| {
                                                this.formats[0] = *checked;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Toggle::new("italic")
                                            .label("I")
                                            .checked(self.formats[1])
                                            .disabled(self.disabled)
                                            .on_click(cx.listener(|this, checked, _, cx| {
                                                this.formats[1] = *checked;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Toggle::new("underline")
                                            .label("U")
                                            .checked(self.formats[2])
                                            .disabled(self.disabled)
                                            .on_click(cx.listener(|this, checked, _, cx| {
                                                this.formats[2] = *checked;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .content(Separator::vertical().h_5())
                            .child(
                                Select::new(&self.font)
                                    .placeholder("Font")
                                    .disabled(self.disabled)
                                    .w_40(),
                            ),
                    ),
            )
            .child(
                section("Mixed controls")
                    .description("Select and Combobox inherit the same density while preserving their own popup behavior.")
                    .w(px(640.))
                    .child(
                        Toolbar::new("mixed-toolbar")
                            .w_full()
                            .with_size(self.size)
                            .disabled(self.disabled)
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .child(
                                Input::new(&self.query)
                                    .prefix(IconName::Search)
                                    .disabled(self.disabled)
                                    .w_40(),
                            )
                            .child(
                                Select::new(&self.market)
                                    .placeholder("Market")
                                    .disabled(self.disabled)
                                    .w_32(),
                            )
                            .child(
                                Combobox::new(&self.status)
                                    .placeholder("Order status")
                                    .disabled(self.disabled)
                                    .w_40(),
                            )
                            .content(div().flex_1())
                            .child(icon_button(
                                "refresh",
                                IconName::RotateCw,
                                "Refresh",
                                self.disabled,
                            ))
                            .child(icon_button(
                                "settings",
                                IconName::Settings2,
                                "Configure columns",
                                self.disabled,
                            )),
                    ),
            )
    }
}
