use gpui_kit::component::{
    AxisExt, IndexPath, Sizable, Size,
    button::Button,
    checkbox::Checkbox,
    color_picker::{ColorPicker, ColorPickerState},
    date_picker::{DatePicker, DatePickerState},
    form::{field, v_form},
    input::{Input, InputState, Textarea, TextareaState},
    select::{Select, SelectState},
    separator::Separator,
    switch::Switch,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde::Deserialize;

use crate::{ChangeStorySize, story_toolbar};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = form_story, no_json)]
enum ToggleOption {
    Horizontal,
    MultipleColumns,
}

pub struct FormStory {
    focus_handle: FocusHandle,
    name_prefix_state: Entity<SelectState<Vec<String>>>,
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
    bio_input: Entity<TextareaState>,
    color_state: Entity<ColorPickerState>,
    subscribe_email: bool,
    date: Entity<DatePickerState>,
    layout: Axis,
    size: Size,
    columns: usize,
}

impl super::Story for FormStory {
    fn title() -> &'static str {
        "Form"
    }

    fn description() -> &'static str {
        "Form to collect multiple inputs."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl FormStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_prefix_state = cx.new(|cx| {
            SelectState::new(
                vec![
                    "Mr.".to_string(),
                    "Mrs.".to_string(),
                    "Ms.".to_string(),
                    "Dr.".to_string(),
                ],
                Some(IndexPath::default()),
                window,
                cx,
            )
        });

        let name_input = cx.new(|cx| InputState::new(window, cx).default_value("Jason Lee"));
        let color_state = cx.new(|cx| ColorPickerState::new(window, cx));

        let email_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter text here..."));
        let bio_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(5, 20)
                .placeholder("Enter text here...")
                .default_value("Hello 世界，this is GPUI component.")
        });
        let date = cx.new(|cx| DatePickerState::new(window, cx));

        Self {
            focus_handle: cx.focus_handle(),
            name_prefix_state,
            name_input,
            email_input,
            bio_input,
            date,
            color_state,
            subscribe_email: false,
            layout: Axis::Vertical,
            size: Size::default(),
            columns: 1,
        }
    }
}

impl Focusable for FormStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FormStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_multi_column = self.columns > 1;
        let is_horizontal = self.layout.is_horizontal();

        v_flex()
            .id("form-story")
            .size_full()
            .p_4()
            .justify_start()
            .gap_3()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .on_action(cx.listener(|this, action: &ToggleOption, _, cx| {
                match action {
                    ToggleOption::Horizontal => {
                        this.layout = if this.layout.is_horizontal() {
                            Axis::Vertical
                        } else {
                            Axis::Horizontal
                        };
                    }
                    ToggleOption::MultipleColumns => {
                        this.columns = if this.columns > 1 { 1 } else { 2 };
                    }
                }
                cx.notify();
            }))
            .child(story_toolbar(self.size).flex_wrap().dropdown_child(
                Button::new("form-options").label("Options"),
                {
                    let horizontal = self.layout.is_horizontal();
                    let multiple_columns = self.columns > 1;
                    move |menu, _, _| {
                        menu.menu_with_check(
                            "Horizontal",
                            horizontal,
                            Box::new(ToggleOption::Horizontal),
                        )
                        .menu_with_check(
                            "Multiple columns",
                            multiple_columns,
                            Box::new(ToggleOption::MultipleColumns),
                        )
                    }
                },
            ))
            .child(Separator::horizontal())
            .child(
                v_form()
                    .layout(self.layout)
                    .with_size(self.size)
                    .columns(self.columns)
                    .label_width(px(if is_multi_column { 100. } else { 140. }))
                    .child(
                        field().label_fn(|_, _| "Name").child(
                            Input::new(&self.name_input).pl_0().prefix(
                                div().w(px(90.)).child(
                                    Select::new(&self.name_prefix_state)
                                        .pr_0()
                                        .appearance(false),
                                ),
                            ),
                        ),
                    )
                    .child(
                        field()
                            .label("Email")
                            .child(Input::new(&self.email_input))
                            .required(true),
                    )
                    .child(
                        field()
                            .label("Bio")
                            .when(self.layout.is_vertical(), |this| this.items_start())
                            .child(Textarea::new(&self.bio_input))
                            .description_fn(|_, _| {
                                div().child("Use at most 100 words to describe yourself.")
                            }),
                    )
                    .child(
                        field()
                            .label_indent(false)
                            .when(is_multi_column, |this| this.col_span(2))
                            .child("This is a full width form field."),
                    )
                    .child(
                        field()
                            .label("Please select your birthday")
                            .description("Select your birthday, we will send you a gift.")
                            .child(DatePicker::new(&self.date)),
                    )
                    .child(
                        field()
                            .when(is_horizontal && is_multi_column, |this| {
                                this.label_indent(false)
                            })
                            .when(is_multi_column, |this| this.col_start(1))
                            .child(
                                Switch::new("subscribe-newsletter")
                                    .label("Subscribe our newsletter")
                                    .checked(self.subscribe_email)
                                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                        this.subscribe_email = *checked;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        field()
                            .when(is_horizontal && is_multi_column, |this| {
                                this.label_indent(false)
                            })
                            .child(
                                ColorPicker::new(&self.color_state)
                                    .small()
                                    .label("Theme color"),
                            ),
                    )
                    .child(
                        field()
                            .when(is_horizontal && is_multi_column, |this| {
                                this.label_indent(false)
                            })
                            .child(
                                Checkbox::new("use-vertical-layout")
                                    .label("Use this color for future events")
                                    .checked(self.subscribe_email)
                                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                        this.subscribe_email = *checked;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
    }
}
