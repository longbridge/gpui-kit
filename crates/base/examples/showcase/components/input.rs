use super::*;
use gpui::MouseButton;

impl BaseShowcase {
    pub(in super::super) fn input(&self) -> impl IntoElement {
        let state = self.input.clone();
        div()
            .w_56()
            .flex()
            .flex_col()
            .items_start()
            .gap_1()
            .text_xs()
            .child(div().h(px(16.)).flex().items_center().child("Project name"))
            .child(
                InputBase::new("example-input")
                    .w_full()
                    .h_7()
                    .px_2()
                    .border_1()
                    .border_color(super::example_rgb(0xd4d4d4))
                    .styles(|styles| {
                        styles.focused(|style| style.border_color(super::example_rgb(0x171717)))
                    })
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        state.update(cx, |state, cx| state.focus(window, cx));
                    })
                    .child(Input::new(&self.input)),
            )
    }
}
