use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme as _,
    button::Button,
    scroll::{Scrollbar, ScrollbarPlacement},
    v_flex,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::story_toolbar_group;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = scrollbar_story, no_json)]
struct ChangeDataset(pub usize);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = scrollbar_story, no_json)]
struct ChangePlacement(pub usize);

pub struct ScrollbarStory {
    focus_handle: FocusHandle,
    items: Rc<Vec<String>>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    test_width: Pixels,
    size_mode: usize,
    placement: ScrollbarPlacement,
    scroll_handle: UniformListScrollHandle,
}

const ITEM_HEIGHT: Pixels = px(50.);
const PLACEMENTS: [ScrollbarPlacement; 4] = [
    ScrollbarPlacement::TopLeft,
    ScrollbarPlacement::TopRight,
    ScrollbarPlacement::BottomLeft,
    ScrollbarPlacement::BottomRight,
];

impl ScrollbarStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let items: Rc<Vec<String>> = Rc::new((0..5000).map(|i| format!("Item {}", i)).collect());
        let test_width = px(3000.);
        let item_sizes = items
            .iter()
            .map(|_| size(test_width, ITEM_HEIGHT))
            .collect::<Vec<_>>();

        Self {
            focus_handle: cx.focus_handle(),
            items,
            item_sizes: Rc::new(item_sizes),
            test_width,
            size_mode: 0,
            placement: ScrollbarPlacement::default(),
            scroll_handle: UniformListScrollHandle::new(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn change_test_cases(&mut self, n: usize, cx: &mut Context<Self>) {
        self.size_mode = n;
        if n == 0 {
            self.items = Rc::new((0..5000).map(|i| format!("Item {}", i)).collect());
            self.test_width = px(3000.);
        } else if n == 1 {
            self.items = Rc::new((0..100).map(|i| format!("Item {}", i)).collect());
            self.test_width = px(10000.);
        } else if n == 2 {
            self.items = Rc::new((0..500000).map(|i| format!("Item {}", i)).collect());
            self.test_width = px(10000.);
        } else {
            self.items = Rc::new((0..5).map(|i| format!("Item {}", i)).collect());
            self.test_width = px(10000.);
        }

        self.item_sizes = self
            .items
            .iter()
            .map(|_| size(self.test_width, ITEM_HEIGHT))
            .collect::<Vec<_>>()
            .into();
        cx.notify();
    }
}

impl super::Story for ScrollbarStory {
    fn title() -> &'static str {
        "Scrollbar"
    }

    fn description() -> &'static str {
        "Add scrollbar to a scrollable element."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ScrollbarStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ScrollbarStory {
    fn render(
        &mut self,
        _: &mut gpui_kit::Window,
        cx: &mut gpui_kit::Context<Self>,
    ) -> impl gpui_kit::IntoElement {
        v_flex()
            .size_full()
            .gap_4()
            .on_action(cx.listener(|this, action: &ChangeDataset, _, cx| {
                this.change_test_cases(action.0, cx);
            }))
            .on_action(cx.listener(|this, action: &ChangePlacement, _, cx| {
                this.placement = PLACEMENTS[action.0];
                cx.notify();
            }))
            .child(story_toolbar_group().dropdown_child(
                Button::new("scrollbar-options").label("Options"),
                {
                    let dataset = self.size_mode;
                    let placement = self.placement;
                    move |menu, window, cx| {
                        menu.submenu("Dataset", window, cx, move |menu, _, _| {
                            ["Standard", "Wide", "Stress", "Short"]
                                .into_iter()
                                .enumerate()
                                .fold(menu, |menu, (ix, label)| {
                                    menu.menu_with_check(
                                        label,
                                        dataset == ix,
                                        Box::new(ChangeDataset(ix)),
                                    )
                                })
                        })
                        .submenu(
                            "Placement",
                            window,
                            cx,
                            move |menu, _, _| {
                                PLACEMENTS.into_iter().enumerate().fold(
                                    menu,
                                    |menu, (ix, value)| {
                                        menu.menu_with_check(
                                            format!("{:?}", value),
                                            placement == value,
                                            Box::new(ChangePlacement(ix)),
                                        )
                                    },
                                )
                            },
                        )
                    }
                },
            ))
            .child({
                div()
                    .relative()
                    .border_1()
                    .border_color(cx.theme().border)
                    .flex_1()
                    .child(
                        uniform_list("list", self.items.len(), {
                            let items = self.items.clone();
                            let width = self.test_width;
                            move |visible_range, _, cx| {
                                let mut elements = Vec::with_capacity(visible_range.len());
                                for ix in visible_range {
                                    let item = &items[ix];
                                    elements.push(
                                        div()
                                            .w(width)
                                            .h(ITEM_HEIGHT)
                                            .pt_1()
                                            .items_center()
                                            .justify_center()
                                            .text_sm()
                                            .child(
                                                div()
                                                    .p_2()
                                                    .bg(cx.theme().secondary)
                                                    .child(item.to_string()),
                                            ),
                                    );
                                }
                                elements
                            }
                        })
                        .with_horizontal_sizing_behavior(
                            ListHorizontalSizingBehavior::Unconstrained,
                        )
                        .py_1()
                        .px_3()
                        .size_full()
                        .track_scroll(&self.scroll_handle),
                    )
                    .child(Scrollbar::new(&self.scroll_handle).placement(self.placement))
            })
    }
}
