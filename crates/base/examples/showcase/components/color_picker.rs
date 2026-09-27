use super::*;
use gpui::{Focusable as _, Hsla, MouseButton};

impl BaseShowcase {
    pub(in super::super) fn color_picker(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // A builder-supplied default cannot reach the hex field and the sliders
        // without a window, so flush it on the first render.
        self.color_picker
            .update(cx, |state, cx| state.sync_pending_value(window, cx));

        let picker = self.color_picker.read(cx);
        let open = picker.is_open();
        let selected = picker.value();
        let displayed = picker
            .displayed_color()
            .unwrap_or(super::example_rgb(0x171717).into());
        let hex = picker.hex_input().read(cx).value();
        let focus_handle = picker.focus_handle(cx);
        let hex_input = picker.hex_input().clone();
        let state = self.color_picker.clone();

        let trigger_state = state.clone();
        let trigger = div()
            .id("color-trigger")
            .w_full()
            .h_7()
            .px_2()
            .flex()
            .items_center()
            .gap_2()
            .border_1()
            .border_color(super::example_rgb(0x171717))
            .bg(super::example_rgb(0xffffff))
            .on_click(move |_, _, cx| {
                trigger_state.update(cx, |state, cx| state.toggle_open(cx));
            })
            .child(
                div()
                    .size(px(14.))
                    .bg(displayed)
                    .border_1()
                    .border_color(super::example_rgb(0x171717)),
            )
            .child(hex)
            .child(div().flex_1())
            .child(super::chevron(open));

        let swatches = div().flex().gap_1().children(
            [0xdc2626u32, 0xd97706, 0x16a34a, 0x2563eb, 0x7c3aed]
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    let color: Hsla = super::example_rgb(value).into();
                    let hover_state = state.clone();
                    let click_state = state.clone();
                    ColorSwatch::new(("swatch", index), color)
                        .selected(selected == Some(color))
                        .size(px(24.))
                        .bg(color)
                        .border_1()
                        .border_color(if selected == Some(color) {
                            super::example_rgb(0x171717)
                        } else {
                            super::example_rgb(0xffffff)
                        })
                        // Hovering previews without committing; leaving restores
                        // the committed color.
                        .on_hover(move |color, entered, window, cx| {
                            hover_state.update(cx, |state, cx| {
                                if entered {
                                    state.preview_color(color, window, cx);
                                } else {
                                    state.clear_preview(window, cx);
                                }
                            });
                        })
                        .on_click(move |color, _, window, cx| {
                            click_state
                                .update(cx, |state, cx| state.select_color(color, window, cx));
                        })
                }),
        );

        let content = div()
            .w(px(220.))
            .mt_1()
            .p_2()
            .flex()
            .flex_col()
            .gap_2()
            .border_1()
            .border_color(super::example_rgb(0x171717))
            .bg(super::example_rgb(0xffffff))
            .child(swatches)
            .child(
                InputBase::new("color-hex-input")
                    .w_full()
                    .h_7()
                    .px_2()
                    .border_1()
                    .border_color(super::example_rgb(0xd4d4d4))
                    .styles(|styles| {
                        styles.focused(|style| style.border_color(super::example_rgb(0x171717)))
                    })
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        hex_input.update(cx, |input, cx| input.focus(window, cx));
                    })
                    .child(picker.hex_input().clone()),
            );

        let open_state = state.clone();
        let root = ColorPicker::new("example-color-picker")
            .open(open)
            .track_focus(&focus_handle)
            .accessibility_label("Brand color")
            .on_open_change(move |open, _, cx| {
                open_state.update(cx, |state, cx| state.set_open(open, cx));
            })
            .w(px(220.))
            .text_xs()
            .child(trigger);

        Popup::new("example-color-picker-popup", root).when(open, |this| this.content(content))
    }
}
