use gpui_kit::component::{Disableable, Sizable, Size, pagination::Pagination, v_flex};
use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window,
};

use crate::{ChangeStorySize, section, story_toolbar};

pub struct PaginationStory {
    basic_page: usize,
    many_pages_page: usize,
    compact_page: usize,
    focus_handle: FocusHandle,
    size: Size,
}

impl super::Story for PaginationStory {
    fn title() -> &'static str {
        "Pagination"
    }

    fn description() -> &'static str {
        "Pagination with page navigation, next and previous links."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl PaginationStory {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            basic_page: 5,
            many_pages_page: 1,
            compact_page: 3,
            focus_handle: cx.focus_handle(),
            size: Size::default(),
        })
    }
}

impl Focusable for PaginationStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PaginationStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();

        v_flex()
            .gap_6()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(
                section("Default").child(
                    Pagination::new("basic-pagination")
                        .current_page(self.basic_page)
                        .total_pages(10)
                        .with_size(self.size)
                        .on_click({
                            let entity = entity.clone();
                            move |page, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.basic_page = *page;
                                    cx.notify();
                                });
                            }
                        }),
                ),
            )
            .child(
                section("Visible Pages")
                    .description(
                        "Control how many page links remain visible in a larger result set.",
                    )
                    .child(
                        Pagination::new("many-pages-pagination")
                            .current_page(self.many_pages_page)
                            .total_pages(50)
                            .visible_pages(10)
                            .with_size(self.size)
                            .on_click({
                                let entity = entity.clone();
                                move |page, _, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.many_pages_page = *page;
                                        cx.notify();
                                    });
                                }
                            }),
                    ),
            )
            .child(
                section("Compact Style").child(
                    Pagination::new("compact-pagination")
                        .compact()
                        .current_page(self.compact_page)
                        .total_pages(10)
                        .with_size(self.size)
                        .on_click({
                            let entity = entity.clone();
                            move |page, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.compact_page = *page;
                                    cx.notify();
                                });
                            }
                        }),
                ),
            )
            .child(
                section("Disabled").child(
                    Pagination::new("disabled-pagination")
                        .current_page(4)
                        .total_pages(10)
                        .with_size(self.size)
                        .disabled(true)
                        .on_click(|_, _, _| {}),
                ),
            )
    }
}
