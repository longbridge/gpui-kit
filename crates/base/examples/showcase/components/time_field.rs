use gpui::{
    Context, IntoElement, ParentElement as _, Styled as _, div, prelude::FluentBuilder as _,
};
use gpui_base::TimeField;

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn time_field(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let value = self.time_field.read(cx).time();

        div()
            .w_56()
            .flex()
            .flex_col()
            .gap_1()
            .text_xs()
            .child(div().child("Reminder time"))
            .child(
                TimeField::new("example-time-field", &self.time_field)
                    .flex()
                    .items_center()
                    .h_7()
                    .px_2()
                    .border_1()
                    .border_color(super::example_rgb(0xa3a3a3))
                    .bg(super::example_rgb(0xffffff))
                    .render_segment(|segment, state, _, _| {
                        segment
                            .px_0p5()
                            .when(state.is_selected(), |this| {
                                this.bg(super::example_rgb(0xdbeafe))
                            })
                            .into_any_element()
                    }),
            )
            .child(
                div()
                    .text_color(super::example_rgb(0x737373))
                    .child(format!("Selected {}", value.format("%H:%M:%S"))),
            )
    }
}
