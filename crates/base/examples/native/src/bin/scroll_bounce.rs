//! Run with `cargo run -p gpui-base-examples --bin scroll_bounce --release`.
//! Trackpad gestures exercise the same viewport wrapper as the iOS host.

#[allow(dead_code)]
#[path = "../../../shared/palette.rs"]
mod palette;

use gpui::{
    App, AppContext as _, Context, InteractiveElement as _, IntoElement, ListAlignment, ListState,
    MouseButton, ParentElement as _, Pixels, PlatformInput, Point, Render, ScrollDelta,
    ScrollHandle, ScrollWheelEvent, StatefulInteractiveElement as _, Styled as _, TouchPhase,
    Window, WindowBounds, WindowOptions, div, list, point, px, size,
};
use gpui_base::{Button, ScrollBounce};

struct Example {
    list: ListState,
    short: ScrollHandle,
    enabled: bool,
    generation: usize,
    drag: Option<MouseScroll>,
}

/// Example-only adapter: keep targeting the viewport where the drag began,
/// even when the pointer moves over another column or outside that viewport.
struct MouseScroll {
    anchor: Point<Pixels>,
    previous: Point<Pixels>,
}

impl MouseScroll {
    fn event(&mut self, position: Point<Pixels>, phase: TouchPhase) -> ScrollWheelEvent {
        let delta = if phase == TouchPhase::Moved {
            position.y - self.previous.y
        } else {
            px(0.)
        };
        self.previous = position;
        ScrollWheelEvent {
            position: self.anchor,
            delta: ScrollDelta::Pixels(point(px(0.), delta)),
            touch_phase: phase,
            ..Default::default()
        }
    }
}

fn send_scroll(event: ScrollWheelEvent, window: &Window, cx: &mut App) {
    // Mouse listeners are temporarily taken out during dispatch. Re-entering
    // dispatch synchronously would miss the viewport's scroll listeners.
    window.defer(cx, move |window, cx| {
        window.dispatch_event(PlatformInput::ScrollWheel(event), cx);
    });
}

impl Example {
    fn begin_drag(&mut self, position: Point<Pixels>, window: &Window, cx: &mut App) {
        let mut drag = MouseScroll {
            anchor: position,
            previous: position,
        };
        send_scroll(drag.event(position, TouchPhase::Started), window, cx);
        self.drag = Some(drag);
    }

    fn end_drag(&mut self, window: &Window, cx: &mut App) {
        if let Some(mut drag) = self.drag.take() {
            send_scroll(drag.event(drag.previous, TouchPhase::Ended), window, cx);
        }
    }
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        palette::activate(window, cx);
        let colors = gpui_base::Theme::global(cx).tokens.colors;
        let border = colors.border;
        let surface = colors.surface;
        let generation = self.generation;
        div().id("drag-surface").size_full().flex().flex_col().p_6().gap_4()
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, window, cx| {
                if event.pressed_button != Some(MouseButton::Left) {
                    this.end_drag(window, cx);
                } else if let Some(drag) = &mut this.drag {
                    send_scroll(drag.event(event.position, TouchPhase::Moved), window, cx);
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, window, cx| this.end_drag(window, cx)))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _, window, cx| this.end_drag(window, cx)))
            .bg(colors.background).text_color(colors.foreground)
            .child(div().text_xl().child("Scroll bounce"))
            .child("Hold the left mouse button on the list and drag past an edge, then release. Trackpad scrolling also works.")
            .child(div().flex().gap_3()
                .child(Button::new("toggle").border_1().border_color(border).px_3().py_2()
                    .child(if self.enabled { "Bounce: on" } else { "Bounce: off" })
                    .on_click(cx.listener(|this, _, _, cx| { this.enabled = !this.enabled; cx.notify(); })))
                .child(Button::new("top").border_1().border_color(border).px_3().py_2().child("Top")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.list.scroll_to_reveal_item(0); this.generation += 1; cx.notify();
                    })))
                .child(Button::new("bottom").border_1().border_color(border).px_3().py_2().child("Bottom")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.list.scroll_to_end(); this.generation += 1; cx.notify();
                    })))
                .child(Button::new("motion").border_1().border_color(border).px_3().py_2()
                    .child(if cx.reduce_motion() { "Reduced motion: on" } else { "Reduced motion: off" })
                    .on_click(cx.listener(|_, _, window, cx| {
                        let reduced = !cx.reduce_motion();
                        cx.set_reduce_motion(reduced); window.refresh();
                    }))))
            .child(div().flex().items_stretch().flex_1().min_h_0().gap_4()
                .child(div().flex().flex_col().flex_1().min_w_0().min_h_0().gap_2()
                    .child("Virtual list · 120 rows")
                    .child(div().id("list-drag").flex().flex_col().flex_1().min_h_0()
                        .border_1().border_color(border).overflow_hidden()
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                            this.begin_drag(event.position, window, cx);
                            cx.stop_propagation();
                        }))
                        .child(ScrollBounce::new(("long", generation), &self.list,
                        list(self.list.clone(), move |ix, _, _| {
                            div().p_4().border_b_1().border_color(border).bg(surface)
                                .child(format!("Message {} — swipe, hold, release, reverse", ix + 1))
                                .into_any_element()
                        }).flex_1()).enabled(self.enabled))))
                .child(div().flex().flex_col().flex_1().min_w_0().min_h_0().gap_2()
                    .child("Short content · both edges")
                    .child(ScrollBounce::new(("short", generation), &self.short,
                        div().id("short-viewport").flex_1().min_h_0()
                            .overflow_y_scroll().track_scroll(&self.short).bg(surface)
                            .child(div().flex().flex_col().p_4().gap_4()
                                .child("This content fits inside the viewport. Pull down or up.")
                                .child(div().id("horizontal").overflow_x_scroll().h_16()
                                    .child(div().w(gpui::rems(48.)).child("← Horizontal content — drag sideways across this row to verify nested scrolling →")))))
                        .enabled(self.enabled))))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        gpui_base::init(cx);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(900.), px(650.)), cx)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| Example {
                    list: ListState::new(120, ListAlignment::Top, px(200.)).measure_all(),
                    short: ScrollHandle::new(),
                    enabled: true,
                    generation: 0,
                    drag: None,
                })
            },
        )
        .expect("open scroll bounce example");
        cx.activate(true);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{MouseDownEvent, MouseMoveEvent, MouseUpEvent, TestAppContext};
    use gpui_base::ScrollbarHandle;

    #[gpui::test]
    fn mouse_drag_scrolls_and_stretches_the_example_list(cx: &mut TestAppContext) {
        cx.update(gpui_base::init);
        let (view, cx) = cx.add_window_view(|_, _| Example {
            list: ListState::new(120, ListAlignment::Top, px(200.)).measure_all(),
            short: ScrollHandle::new(),
            enabled: true,
            generation: 0,
            drag: None,
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let handle = view.read_with(cx, |view, _| view.list.clone());
        let bounds = handle.viewport_bounds();
        assert!(bounds.size.height > px(100.));
        let anchor = bounds.origin + point(px(50.), px(50.));
        cx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: anchor,
            ..Default::default()
        });
        cx.simulate_event(MouseMoveEvent {
            pressed_button: Some(MouseButton::Left),
            position: anchor + point(px(0.), px(100.)),
            ..Default::default()
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(handle.offset().y, px(0.));
        assert!(handle.viewport_bounds().origin.y > bounds.origin.y);
        cx.simulate_event(MouseMoveEvent {
            pressed_button: Some(MouseButton::Left),
            position: anchor - point(px(0.), px(40.)),
            ..Default::default()
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(handle.offset().y, px(-40.));
        cx.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position: anchor,
            ..Default::default()
        });
        assert!(view.read_with(cx, |view, _| view.drag.is_none()));

        // Pull past the top again, then release without reversing. Verify
        // real prepaint displacement decreases as the spring clock advances.
        cx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: anchor,
            ..Default::default()
        });
        cx.simulate_event(MouseMoveEvent {
            pressed_button: Some(MouseButton::Left),
            position: anchor + point(px(0.), px(140.)),
            ..Default::default()
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let stretched = handle.viewport_bounds().origin.y;
        assert!(stretched > bounds.origin.y);
        cx.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position: anchor + point(px(0.), px(140.)),
            ..Default::default()
        });
        // ScrollBounce uses the real monotonic clock, not the test executor's
        // virtual timer. A delayed frame may settle fully; either is valid.
        std::thread::sleep(std::time::Duration::from_millis(30));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let returning = handle.viewport_bounds().origin.y;
        assert!(returning >= bounds.origin.y && returning < stretched);
    }
}
