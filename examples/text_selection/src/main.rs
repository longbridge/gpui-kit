//! Window-level text selection demo: drag across multiple chat bubbles and
//! press cmd-c / ctrl-c to copy the merged text.
//!
//! - Drag from anywhere inside the window (even the blank space between
//!   bubbles) to start a selection that spans multiple `TextView`s.
//! - The copied text keeps the top-to-bottom order, joined by newlines.
//! - Clicking the `Button` does NOT start a selection.
//! - Dragging inside the `Input` only drives the `Input`'s own selection.
//!
//! Run: `cargo run -p text_selection`

use gpui_kit::assets::Assets;
use gpui_kit::component::{
    button::Button,
    input::{Input, InputState},
    text::TextView,
    *,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

struct ChatExample {
    input: Entity<InputState>,
}

impl ChatExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            input: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Type here (selection must NOT start from here)")
            }),
        }
    }

    fn bubble(&self, ix: usize, text: &str, mine: bool, cx: &App) -> impl IntoElement {
        div().flex().when(mine, |this| this.justify_end()).child(
            div()
                .max_w(px(420.))
                .p_3()
                .rounded(cx.theme().radius_lg)
                .bg(if mine {
                    cx.theme().primary.opacity(0.1)
                } else {
                    cx.theme().muted
                })
                // `selectable(true)` opts this TextView into window-level
                // selection, so a drag started anywhere can extend into it.
                .child(TextView::markdown(("msg", ix), text).selectable(true)),
        )
    }
}

impl Render for ChatExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(self.bubble(0, "**Hello!** How can I help you today?", false, cx))
            .child(self.bubble(
                1,
                "I want to select text *across* multiple bubbles.",
                true,
                cx,
            ))
            .child(self.bubble(
                2,
                "Sure — drag from anywhere, even from the blank space between \
                 bubbles, then press `cmd-c` to copy everything.",
                false,
                cx,
            ))
            .child(self.bubble(3, "Nice, it also keeps the top-to-bottom order.", true, cx))
            .child(div().flex_1())
            .child(Button::new("noop").label("Clicking me must not start selection"))
            .child(Input::new(&self.input))
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_kit::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(800.), px(600.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| ChatExample::new(window, cx));
                // The first level view on the window should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
