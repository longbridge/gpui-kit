use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Window, div, px,
};

use gpui_kit::component::{
    ActiveTheme, Sizable, Size,
    radio::{Radio, RadioGroup},
    v_flex,
};

use crate::{ChangeStorySize, section, story_toolbar};

pub struct RadioStory {
    focus_handle: gpui_kit::FocusHandle,
    delivery: Option<usize>,
    billing: Option<usize>,
    size: Size,
}

impl super::Story for RadioStory {
    fn title() -> &'static str {
        "Radio"
    }

    fn description() -> &'static str {
        "Choose one option from a set."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl RadioStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            delivery: Some(0),
            billing: Some(1),
            size: Size::default(),
        }
    }
}

impl Focusable for RadioStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RadioStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_3()
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .child(story_toolbar(self.size))
            .child(
                section("Delivery")
                    .description("Choose one option from a clearly described set.")
                    .w(px(320.))
                    .items_center()
                    .child(
                        RadioGroup::vertical("delivery")
                            .w(px(320.))
                            .gap_3()
                            .child(
                                Radio::new("standard")
                                    .with_size(self.size)
                                    .w_full()
                                    .label("Standard delivery")
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Arrives in 3–5 business days."),
                                    ),
                            )
                            .child(
                                Radio::new("express")
                                    .with_size(self.size)
                                    .w_full()
                                    .label("Express delivery")
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Arrives the next business day."),
                                    ),
                            )
                            .child(
                                Radio::new("pickup")
                                    .with_size(self.size)
                                    .w_full()
                                    .label("Store pickup")
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Unavailable for this order."),
                                    )
                                    .disabled(true),
                            )
                            .selected_index(self.delivery)
                            .on_click(cx.listener(|this, selected: &usize, _, cx| {
                                this.delivery = Some(*selected);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("Billing cycle")
                    .description("Horizontal groups work for short, related choices.")
                    .w(px(320.))
                    .items_center()
                    .child(
                        RadioGroup::horizontal("billing")
                            .w(px(320.))
                            .justify_between()
                            .child(Radio::new("monthly").with_size(self.size).label("Monthly"))
                            .child(Radio::new("yearly").with_size(self.size).label("Yearly"))
                            .child(
                                Radio::new("lifetime")
                                    .with_size(self.size)
                                    .label("Lifetime"),
                            )
                            .selected_index(self.billing)
                            .on_click(cx.listener(|this, selected_ix: &usize, _, cx| {
                                this.billing = Some(*selected_ix);
                                cx.notify();
                            })),
                    ),
            )
    }
}
