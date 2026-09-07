use gpui_kit::component::{button::*, input::*, select::*, separator::*, *};
use gpui_kit::*;
use itertools::Itertools as _;
use serde::{Deserialize, Serialize};

use crate::{ChangeStorySize, section, story_toolbar};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = select_story, no_json)]
struct ToggleDisabled;

pub fn init(_: &mut App) {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Country {
    name: SharedString,
    code: SharedString,
}

impl Country {
    pub fn letter_prefix(&self) -> char {
        self.name.chars().next().unwrap_or(' ')
    }
}

impl SelectItem for Country {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn display_title(&self) -> Option<gpui_kit::AnyElement> {
        Some(format!("{} ({})", self.name, self.code).into_any_element())
    }

    fn value(&self) -> &Self::Value {
        &self.code
    }
}

pub struct SelectStory {
    disabled: bool,
    size: gpui_kit::component::Size,
    country_select: Entity<SelectState<SearchableVec<SelectGroup<Country>>>>,
    fruit_select: Entity<SelectState<SearchableVec<&'static str>>>,
    simple_select1: Entity<SelectState<Vec<&'static str>>>,
    simple_select2: Entity<SelectState<SearchableVec<&'static str>>>,
    simple_select3: Entity<SelectState<Vec<SharedString>>>,
    menu_max_h_select: Entity<SelectState<Vec<&'static str>>>,
    disabled_select: Entity<SelectState<Vec<SharedString>>>,
    appearance_select: Entity<SelectState<Vec<SharedString>>>,
    input_state: Entity<InputState>,
}

impl super::Story for SelectStory {
    fn title() -> &'static str {
        "Select"
    }

    fn description() -> &'static str {
        "Displays a list of options for the user to pick from—triggered by a button."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for SelectStory {
    fn focus_handle(&self, cx: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.fruit_select.focus_handle(cx)
    }
}

impl SelectStory {
    fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let countries =
            serde_json::from_str::<Vec<Country>>(include_str!("../fixtures/countries.json"))
                .unwrap();
        let mut grouped_countries: SearchableVec<SelectGroup<Country>> = SearchableVec::new(vec![]);
        for (prefix, items) in countries.iter().chunk_by(|c| c.letter_prefix()).into_iter() {
            let items = items.cloned().collect::<Vec<Country>>();
            grouped_countries.push(SelectGroup::new(prefix.to_string()).items(items));
        }

        let country_select = cx.new(|cx| {
            SelectState::new(
                grouped_countries,
                Some(IndexPath::default().row(8).section(2)),
                window,
                cx,
            )
            .searchable(true)
        });
        let appearance_select = cx.new(|cx| {
            SelectState::new(
                vec![
                    "CN".into(),
                    "US".into(),
                    "HK".into(),
                    "JP".into(),
                    "KR".into(),
                ],
                Some(IndexPath::default()),
                window,
                cx,
            )
        });
        let input_state = cx.new(|cx| InputState::new(window, cx).placeholder("Your phone number"));

        let fruits = SearchableVec::new(vec![
            "Apple",
            "Orange",
            "Banana",
            "Grape",
            "Pineapple",
            "Watermelon & This is a long long long long long long long long long title",
            "Avocado",
        ]);
        let fruit_select = cx.new(|cx| SelectState::new(fruits, None, window, cx).searchable(true));

        cx.new(|cx| {
            cx.subscribe_in(&country_select, window, Self::on_select_event)
                .detach();

            Self {
                disabled: false,
                size: gpui_kit::component::Size::Medium,
                country_select,
                fruit_select,
                simple_select1: cx.new(|cx| {
                    SelectState::new(
                        vec![
                            "GPUI", "Iced", "egui", "Makepad", "Slint", "QT", "ImGui", "Cocoa",
                            "WinUI",
                        ],
                        Some(IndexPath::default()),
                        window,
                        cx,
                    )
                }),
                simple_select2: cx.new(|cx| {
                    let mut select = SelectState::new(SearchableVec::new(vec![]), None, window, cx)
                        .searchable(true);

                    select.set_items(
                        SearchableVec::new(vec!["Rust", "Go", "C++", "JavaScript"]),
                        window,
                        cx,
                    );

                    select
                }),
                simple_select3: cx
                    .new(|cx| SelectState::new(Vec::<SharedString>::new(), None, window, cx)),
                menu_max_h_select: cx.new(|cx| {
                    SelectState::new(
                        vec![
                            "GPUI", "Iced", "egui", "Makepad", "Slint", "QT", "ImGui", "Cocoa",
                            "WinUI",
                        ],
                        Some(IndexPath::default()),
                        window,
                        cx,
                    )
                }),
                disabled_select: cx
                    .new(|cx| SelectState::new(Vec::<SharedString>::new(), None, window, cx)),
                appearance_select,
                input_state,
            }
        })
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        Self::new(window, cx)
    }

    fn on_select_event(
        &mut self,
        _: &Entity<SelectState<SearchableVec<SelectGroup<Country>>>>,
        event: &SelectEvent<SearchableVec<SelectGroup<Country>>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        match event {
            SelectEvent::Confirm(value) => println!("Selected country: {:?}", value),
        }
    }
}

impl Render for SelectStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .gap_4()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleDisabled, _, cx| {
                this.disabled = !this.disabled;
                cx.notify();
            }))
            .child(story_toolbar(self.size).dropdown_child(
                Button::new("select-options").label("Options"),
                {
                    let disabled = self.disabled;
                    move |menu, _, _| {
                        menu.menu_with_check("Disabled", disabled, Box::new(ToggleDisabled))
                    }
                },
            ))
            .child(
                section("Search and clear")
                    .description("Search options and clear the value.")
                    .w(px(280.))
                    .items_center()
                    .child(
                        Select::new(&self.country_select)
                            .w(px(280.))
                            .with_size(self.size)
                            .search_placeholder("Search…")
                            .cleanable(true)
                            .disabled(self.disabled),
                    ),
            )
            .child(
                section("Menu width")
                    .description("Set trigger and menu widths independently.")
                    .w(px(280.))
                    .items_center()
                    .child(
                        Select::new(&self.fruit_select)
                            .with_size(self.size)
                            .disabled(self.disabled)
                            .icon(IconName::Search)
                            .w(px(280.))
                            .menu_width(px(400.)),
                    ),
            )
            .child(
                section("Disabled")
                    .description("Keep the selected value visible.")
                    .w(px(280.))
                    .items_center()
                    .child(
                        Select::new(&self.disabled_select)
                            .w(px(280.))
                            .with_size(self.size)
                            .disabled(true),
                    ),
            )
            .child(
                section("Title prefix")
                    .description("Prefix the selected value.")
                    .w(px(280.))
                    .items_center()
                    .child(
                        Select::new(&self.simple_select1)
                            .w(px(280.))
                            .with_size(self.size)
                            .disabled(self.disabled)
                            .placeholder("UI")
                            .title_prefix("UI: "),
                    ),
            )
            .child(
                section("Menu height")
                    .description("Limit the popup height.")
                    .w(px(280.))
                    .items_center()
                    .child(
                        Select::new(&self.menu_max_h_select)
                            .w(px(280.))
                            .with_size(self.size)
                            .disabled(self.disabled)
                            .placeholder("UI")
                            .title_prefix("UI: ")
                            .menu_max_h(rems(6.)),
                    ),
            )
            .child(
                section("Search")
                    .description("Filter options from the popup.")
                    .w(px(280.))
                    .items_center()
                    .child(
                        Select::new(&self.simple_select2)
                            .accessibility_label("Programming language")
                            .w(px(280.))
                            .with_size(self.size)
                            .disabled(self.disabled)
                            .placeholder("Language")
                            .title_prefix("Language: "),
                    ),
            )
            .child(
                section("Empty")
                    .description("Render a custom empty state.")
                    .w(px(280.))
                    .items_center()
                    .child(
                        Select::new(&self.simple_select3)
                            .w(px(280.))
                            .with_size(self.size)
                            .disabled(self.disabled)
                            .empty(|_, cx| {
                                h_flex()
                                    .h_24()
                                    .justify_center()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("No Data")
                            }),
                    ),
            )
            .child(
                section("Custom appearance")
                    .description("Compose an appearance-free select with another control.")
                    .w(px(280.))
                    .items_center()
                    .child(
                        h_flex()
                            .border_1()
                            .border_color(cx.theme().input)
                            .rounded(cx.theme().radius_lg)
                            .text_color(cx.theme().secondary_foreground)
                            .w(px(280.))
                            .gap_1()
                            .child(
                                div().w(px(140.)).child(
                                    Select::new(&self.appearance_select)
                                        .with_size(self.size)
                                        .appearance(false)
                                        .py_2()
                                        .pl_3(),
                                ),
                            )
                            .child(Separator::vertical())
                            .child(
                                div().flex_1().child(
                                    Input::new(&self.input_state)
                                        .appearance(false)
                                        .pr_3()
                                        .py_2(),
                                ),
                            )
                            .child(
                                div().p_2().child(
                                    Button::new("send")
                                        .with_size(self.size)
                                        .ghost()
                                        .label("Send"),
                                ),
                            ),
                    ),
            )
            .child(
                section("Values")
                    .description("Read selected values from state.")
                    .w_128()
                    .child(
                        v_flex()
                            .gap_3()
                            .child(format!(
                                "Country: {:?}",
                                self.country_select.read(cx).selected_value()
                            ))
                            .child(format!(
                                "fruit: {:?}",
                                self.fruit_select.read(cx).selected_value()
                            ))
                            .child(format!(
                                "UI: {:?}",
                                self.simple_select1.read(cx).selected_value()
                            ))
                            .child(format!(
                                "Language: {:?}",
                                self.simple_select2.read(cx).selected_value()
                            ))
                            .child("This is other text."),
                    ),
            )
    }
}
