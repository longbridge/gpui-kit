//! Touch selection demo: the handles and the edit menu a long press leaves
//! behind in an `Input`, a `Textarea` and a `TextView`, driven from a desktop.
//!
//! A desktop has no long press, so each `Long press` button injects the
//! gesture GPUI would recognize from a finger held on the control. From there
//! everything is the real thing:
//!
//! - The word under the press is selected, with a grab handle at each end and
//!   an edit menu above it (Cut / Copy / Paste / Select All as they apply).
//! - Drag a handle with the mouse to move that end; the other end stays put.
//! - Drag one handle past the other: they swap, and the selection runs the
//!   other way.
//! - Scroll the page or the textarea: a handle whose end left the view goes
//!   away, and the menu steps aside until the scroll ends.
//! - Click anywhere else to drop the handles; an `Input` also drops them when
//!   you type or press `Escape`, and brings the menu back when you click the
//!   selected text.
//!
//! Run: `cargo run -p touch_selection`

use std::{cell::Cell, rc::Rc};

use gpui_kit::assets::Assets;
use gpui_kit::component::{
    button::Button,
    input::{Input, InputState, Textarea, TextareaState},
    text::TextView,
    *,
};
use gpui_kit::*;

struct TouchSelectionExample {
    input: Entity<InputState>,
    textarea: Entity<TextareaState>,
    /// Where each control was painted this frame, so a press can land in it.
    input_bounds: Rc<Cell<Bounds<Pixels>>>,
    textarea_bounds: Rc<Cell<Bounds<Pixels>>>,
    text_bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl TouchSelectionExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            input: cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value("The quick brown fox jumps over the lazy dog")
            }),
            textarea: cx.new(|cx| {
                // Enough lines to scroll inside its fixed height.
                let lines = (1..=12)
                    .map(|n| format!("Line {n}: online content authoring isn't a solved problem."))
                    .collect::<Vec<_>>();
                TextareaState::new(window, cx).default_value(lines.join("\n"))
            }),
            input_bounds: Rc::default(),
            textarea_bounds: Rc::default(),
            text_bounds: Rc::default(),
        }
    }

    /// Wraps a control so its painted bounds are known to the press button.
    fn measured(control: impl IntoElement, bounds: &Rc<Cell<Bounds<Pixels>>>) -> impl IntoElement {
        let bounds = bounds.clone();
        div().relative().child(control).child(
            canvas(move |painted, _, _| bounds.set(painted), |_, _, _, _| {})
                .absolute()
                .inset_0(),
        )
    }

    /// A long press a finger's width into the control's first line of text.
    fn press_button(
        id: &'static str,
        bounds: &Rc<Cell<Bounds<Pixels>>>,
        window: &Window,
    ) -> impl IntoElement {
        let bounds = bounds.clone();
        let first_line = window.line_height() * 0.5 + px(12.);
        Button::new(id)
            .label("Long press")
            .on_click(move |_, window, cx| {
                let bounds = bounds.get();
                let position = point(bounds.left() + px(48.), bounds.top() + first_line);
                // The click that runs this is itself being dispatched; an
                // event sent from inside it would find no listeners. Send the
                // gesture once the click is over.
                window.defer(cx, move |window, cx| {
                    for phase in [TouchPhase::Started, TouchPhase::Ended] {
                        window.dispatch_event(
                            LongPressEvent {
                                phase,
                                start_position: position,
                                position,
                            }
                            .to_platform_input(),
                            cx,
                        );
                    }
                });
            })
    }

    /// Paragraphs above and below the pressed one, so the page scrolls with
    /// the selection somewhere in the middle of it.
    fn filler(id: &'static str, paragraphs: usize) -> impl IntoElement {
        let text = (0..paragraphs)
            .map(|_| {
                "Online content authoring isn't a solved problem. You might go with \
                 an HTML-based editor, and hope it gives you the kind of HTML you \
                 want. Or you might use a text-based markup format, and hope your \
                 users understand how to use it."
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        TextView::markdown(id, text).selectable(true)
    }

    fn row(label: &'static str, button: impl IntoElement, cx: &App) -> impl IntoElement {
        h_flex()
            .items_center()
            .justify_between()
            .child(div().text_color(cx.theme().muted_foreground).child(label))
            .child(button)
    }
}

impl Render for TouchSelectionExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("page")
            .size_full()
            .overflow_y_scroll()
            .p_6()
            .gap_6()
            .child(
                v_flex()
                    .gap_2()
                    .child(Self::row(
                        "Input",
                        Self::press_button("press-input", &self.input_bounds, window),
                        cx,
                    ))
                    .child(Self::measured(Input::new(&self.input), &self.input_bounds)),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(Self::row(
                        "Textarea",
                        Self::press_button("press-textarea", &self.textarea_bounds, window),
                        cx,
                    ))
                    .child(Self::measured(
                        Textarea::new(&self.textarea).h(px(120.)),
                        &self.textarea_bounds,
                    )),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(Self::row(
                        "TextView",
                        Self::press_button("press-text", &self.text_bounds, window),
                        cx,
                    ))
                    // One paragraph above keeps the pressed one on screen at
                    // start; scroll to move it out either way.
                    .child(Self::filler("text-before", 1))
                    .child(Self::measured(
                        TextView::markdown(
                            "text",
                            "ProseMirror tries to bridge the gap between **rich text** and \
                             structured content, by providing a drop-in editor component \
                             with a rigid semantic document model that can be customized \
                             to fit your application.",
                        )
                        .selectable(true),
                        &self.text_bounds,
                    ))
                    .child(Self::filler("text-after", 6)),
            )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_kit::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(520.), px(640.)), cx)),
            ..Default::default()
        };

        gpui_kit::open_window(window_options, cx, |window, cx| {
            cx.new(|cx| TouchSelectionExample::new(window, cx))
        })
        .expect("Failed to open window");
    });
}
