use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window, prelude::FluentBuilder as _, px,
};

use gpui_kit::component::{
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    v_flex,
};

use crate::section;

pub struct BreadcrumbStory {
    focus_handle: gpui_kit::FocusHandle,
    clicked_item: Option<String>,
}

impl BreadcrumbStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            clicked_item: None,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for BreadcrumbStory {
    fn title() -> &'static str {
        "Breadcrumb"
    }

    fn description() -> &'static str {
        "A breadcrumb navigation element that shows the current location in a hierarchy."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for BreadcrumbStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BreadcrumbStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Default")
                    .description("Shows the current location in a hierarchy.")
                    .w(px(480.))
                    .child(
                        Breadcrumb::new()
                            .child("Home")
                            .child("Documents")
                            .child("Projects"),
                    ),
            )
            .child(
                section("Interactive")
                    .description("Earlier levels can respond to navigation clicks.")
                    .w(px(480.))
                    .child(
                        v_flex()
                            .gap_4()
                            .items_center()
                            .child(
                                Breadcrumb::new()
                                    .child("Home")
                                    .child(BreadcrumbItem::new("Documents").on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.clicked_item = Some("Documents".to_string());
                                            cx.notify();
                                        },
                                    )))
                                    .child(BreadcrumbItem::new("Projects").on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.clicked_item = Some("Projects".to_string());
                                            cx.notify();
                                        },
                                    )))
                                    .child(BreadcrumbItem::new("Current").on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.clicked_item = Some("Current".to_string());
                                            cx.notify();
                                        },
                                    ))),
                            )
                            .when_some(self.clicked_item.clone(), |this, item| {
                                this.child(format!("Selected: {}", item))
                            }),
                    ),
            )
    }
}
