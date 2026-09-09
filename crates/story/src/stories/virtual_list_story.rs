use std::{ops::Range, rc::Rc};

use gpui_kit::component::{
    ActiveTheme as _, VirtualListScrollHandle,
    button::Button,
    h_flex,
    scroll::{ScrollableElement, ScrollbarAxis},
    v_flex, v_virtual_list,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::story_toolbar_group;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = virtual_list_story, no_json)]
enum VirtualListAction {
    Dataset(usize),
    Both,
    Vertical,
    Horizontal,
}

pub struct VirtualListStory {
    focus_handle: FocusHandle,
    scroll_handle: VirtualListScrollHandle,
    items: Vec<String>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    columns_count: usize,
    axis: ScrollbarAxis,
    size_mode: usize,
    visible_range: Range<usize>,
}

const ITEM_SIZE: Size<Pixels> = size(px(100.), px(30.));

impl VirtualListStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let items = (0..5000).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
        let item_sizes = items.iter().map(|_| ITEM_SIZE).collect::<Vec<_>>();

        Self {
            focus_handle: cx.focus_handle(),
            scroll_handle: VirtualListScrollHandle::new(),
            items,
            item_sizes: Rc::new(item_sizes),
            columns_count: 100,
            axis: ScrollbarAxis::Both,
            size_mode: 0,
            visible_range: (0..0),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn change_test_cases(&mut self, n: usize, cx: &mut Context<Self>) {
        self.size_mode = n;
        if n == 0 {
            self.items = (0..5000).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
            self.columns_count = 30;
        } else if n == 1 {
            self.items = (0..100).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
            self.columns_count = 100;
        } else if n == 2 {
            self.items = (0..500000)
                .map(|i| format!("Item {}", i))
                .collect::<Vec<_>>();
            self.columns_count = 100;
        } else {
            self.items = (0..5).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
            self.columns_count = 10;
        }

        self.item_sizes = Rc::new(self.items.iter().map(|_| ITEM_SIZE).collect());
        cx.notify();
    }

    pub fn change_axis(&mut self, axis: ScrollbarAxis, cx: &mut Context<Self>) {
        self.axis = axis;
        cx.notify();
    }

    fn render_buttons(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                story_toolbar_group()
                    .dropdown_child(
                        Button::new("virtual-list-dataset").label(format!(
                            "Dataset: {}",
                            ["Standard", "Wide", "Stress", "Short"][self.size_mode]
                        )),
                        {
                            let selected = self.size_mode;
                            move |menu, _, _| {
                                ["Standard", "Wide", "Stress", "Short"]
                                    .into_iter()
                                    .enumerate()
                                    .fold(menu, |menu, (index, label)| {
                                        menu.menu_with_check(
                                            label,
                                            selected == index,
                                            Box::new(VirtualListAction::Dataset(index)),
                                        )
                                    })
                            }
                        },
                    )
                    .dropdown_child(
                        Button::new("virtual-list-axis").label(format!(
                            "Axis: {}",
                            if self.axis.is_both() {
                                "Both"
                            } else if self.axis.is_vertical() {
                                "Vertical"
                            } else {
                                "Horizontal"
                            }
                        )),
                        {
                            let axis = self.axis;
                            move |menu, _, _| {
                                menu.menu_with_check(
                                    "Both",
                                    axis.is_both(),
                                    Box::new(VirtualListAction::Both),
                                )
                                .menu_with_check(
                                    "Vertical",
                                    axis.is_vertical(),
                                    Box::new(VirtualListAction::Vertical),
                                )
                                .menu_with_check(
                                    "Horizontal",
                                    axis.is_horizontal(),
                                    Box::new(VirtualListAction::Horizontal),
                                )
                            }
                        },
                    )
                    .child(Button::new("scroll-to0").label("Top").on_click(cx.listener(
                        |this, _, _, cx| {
                            this.scroll_handle.scroll_to_item(0, ScrollStrategy::Top);
                            cx.notify();
                        },
                    )))
                    .child(
                        Button::new("scroll-to1")
                            .label("Row 50")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.scroll_handle.scroll_to_item(50, ScrollStrategy::Top);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("scroll-to2")
                            .label("Center 25")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.scroll_handle
                                    .scroll_to_item(25, ScrollStrategy::Center);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("scroll-to-bottom")
                            .label("Bottom")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.scroll_handle.scroll_to_bottom();
                                cx.notify();
                            })),
                    ),
            )
            .child(format!("Visible: {:?}", self.visible_range))
    }
}

impl super::Story for VirtualListStory {
    fn title() -> &'static str {
        "VirtualList"
    }

    fn description() -> &'static str {
        "Add vertical or horizontal, or both scrollbars to a container, \
        and use `virtual_list` to render a large number of items."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for VirtualListStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for VirtualListStory {
    fn render(
        &mut self,
        _: &mut gpui_kit::Window,
        cx: &mut gpui_kit::Context<Self>,
    ) -> impl gpui_kit::IntoElement {
        let columns_count = self.columns_count;

        fn render_item(cx: &App) -> Div {
            div()
                .flex()
                .h_full()
                .items_center()
                .justify_center()
                .text_sm()
                .w(ITEM_SIZE.width)
                .h(ITEM_SIZE.height)
                .bg(cx.theme().secondary)
        }

        v_flex()
            .size_full()
            .gap_4()
            .on_action(
                cx.listener(|this, action: &VirtualListAction, _, cx| match action {
                    VirtualListAction::Dataset(index) => this.change_test_cases(*index, cx),
                    VirtualListAction::Both => this.change_axis(ScrollbarAxis::Both, cx),
                    VirtualListAction::Vertical => this.change_axis(ScrollbarAxis::Vertical, cx),
                    VirtualListAction::Horizontal => {
                        this.change_axis(ScrollbarAxis::Horizontal, cx)
                    }
                }),
            )
            .child(self.render_buttons(cx))
            .child(
                div().w_full().flex_1().min_h_64().child(
                    div().relative().size_full().child(
                        v_flex()
                            .id("list")
                            .relative()
                            .size_full()
                            .child(
                                v_virtual_list(
                                    cx.entity().clone(),
                                    "items",
                                    self.item_sizes.clone(),
                                    move |story, visible_range, _, cx| {
                                        story.visible_range = visible_range.clone();

                                        visible_range
                                            .map(|ix| {
                                                h_flex().gap_1().items_center().children(
                                                    (0..columns_count).map(|i| {
                                                        render_item(cx).child(if i == 0 {
                                                            format!("row: {}", ix)
                                                        } else {
                                                            format!("{}", i)
                                                        })
                                                    }),
                                                )
                                            })
                                            .collect()
                                    },
                                )
                                .track_scroll(&self.scroll_handle)
                                .p_4()
                                .border_1()
                                .border_color(cx.theme().border)
                                .gap_1(),
                            )
                            .scrollbar(&self.scroll_handle, self.axis),
                    ),
                ),
            )
    }
}
