use gpui_kit::component::AxisExt;
use gpui_kit::component::{
    Sizable as _, Size,
    button::Button,
    description_list::{DescriptionItem, DescriptionList},
    dock::PanelControl,
    text::TextView,
    v_flex,
};
use gpui_kit::*;
use gpui_kit::{
    Action, App, AppContext, Axis, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, Styled, Window,
};
use serde::Deserialize;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = description_list_story, no_json)]
enum ToggleOption {
    Vertical,
    Bordered,
}

use crate::{ChangeStorySize, story_toolbar};

pub struct DescriptionListStory {
    focus_handle: FocusHandle,
    layout: Axis,
    bordered: bool,
    size: Size,
    items: Vec<(&'static str, &'static str, usize)>,
}

impl DescriptionListStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let items = vec![
            ("Name", "GPUI Component", 1),
            (
                "Description",
                "UI components for building fantastic desktop application by using GPUI.\
                \n\n \
                Contains a lot of useful UI components, such as **Button**, **Input**, **Table**, **List**, **Select**, **DatePicker** ... \
                \n\n \
                You can easily create your native desktop application by using GPUI Component.
                ",
                3,
            ),
            ("Version", "0.1.0", 1),
            ("License", "Apache-2.0", 1),
            ("Author", "Longbridge", 1),
            ("--", "--", 1),
            (
                "Repository",
                "https://github.com/longbridge/gpui-kit",
                2,
            ),
            (
                "Category",
                "UI, Desktop, Framework",
                1,
            ),
            (
                "This is a long label for Platform",
                "macOS, Windows, Linux",
                1,
            ),
        ];

        Self {
            items,
            bordered: true,
            size: Size::default(),
            layout: Axis::Horizontal,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for DescriptionListStory {
    fn title() -> &'static str {
        "DescriptionList"
    }

    fn description() -> &'static str {
        "Present labels and values in a structured summary."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for DescriptionListStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DescriptionListStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("example")
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .on_action(cx.listener(|this, action: &ToggleOption, _, cx| {
                match action {
                    ToggleOption::Vertical => {
                        this.layout = if this.layout.is_vertical() {
                            Axis::Horizontal
                        } else {
                            Axis::Vertical
                        }
                    }
                    ToggleOption::Bordered => this.bordered = !this.bordered,
                }
                cx.notify();
            }))
            .p_4()
            .size_full()
            .items_center()
            .gap_6()
            .child(story_toolbar(self.size).dropdown_child(
                Button::new("description-list-options").label("Options"),
                {
                    let vertical = self.layout.is_vertical();
                    let bordered = self.bordered;
                    move |menu, _, _| {
                        menu.menu_with_check("Vertical", vertical, Box::new(ToggleOption::Vertical))
                            .menu_with_check("Bordered", bordered, Box::new(ToggleOption::Bordered))
                    }
                },
            ))
            .child(
                div().w(px(720.)).child(
                    DescriptionList::new()
                        .columns(3)
                        .layout(self.layout)
                        .bordered(self.bordered)
                        .with_size(self.size)
                        .children(self.items.clone().into_iter().enumerate().map(
                            |(ix, (label, value, span))| {
                                if label == "--" {
                                    return DescriptionItem::Separator;
                                }

                                DescriptionItem::new(label)
                                    .value(TextView::markdown(ix, value).into_any_element())
                                    .span(span)
                            },
                        )),
                ),
            )
    }
}
