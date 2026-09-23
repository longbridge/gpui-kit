//! A long press leaves a selection with grab handles and an edit menu, in an
//! `Input` and in a `TextView`.

mod common;
use gpui::{
    AppContext, Context, DispatchPhase, Entity, InputEvent as _, LongPressEvent, Modifiers,
    MouseButton, MouseDownEvent, MouseUpEvent, Pixels, Point, ScrollDelta, ScrollWheelEvent,
    StyleRefinement, TestAppContext, TouchDragEvent, TouchPhase, Window, WindowHandle, canvas, div,
    prelude::*, px,
};
use gpui_base::TextSelection;
use gpui_component::{
    Root, WindowExt as _,
    input::{Input, InputState},
    text::{TextView, TextViewState},
};
use gpui_kit::test::{TestSupportExt as _, TestWindowExt};

struct Screen {
    input: Entity<InputState>,
    text: Entity<TextViewState>,
}

impl Render for Screen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .child(Input::new(&self.input).id("input").w(px(320.)))
            .child(
                div()
                    .id("text")
                    .test_support()
                    .w(px(320.))
                    .child(TextView::new(&self.text).selectable(true)),
            )
    }
}

fn screen(cx: &mut TestAppContext) -> WindowHandle<Root> {
    cx.update(gpui_component::init);
    common::open_window(cx, None, |window, cx| {
        let view = cx.new(|cx| Screen {
            input: cx.new(|cx| InputState::new(window, cx).default_value("quick select value")),
            text: cx.new(|cx| TextViewState::markdown("quick select value", cx)),
        });
        view
    })
    .0
}

/// The text alone, for a screen that caches it.
struct Text {
    text: Entity<TextViewState>,
}

impl Render for Text {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("text")
            .test_support()
            .w(px(320.))
            .child(TextView::new(&self.text).selectable(true))
    }
}

/// A screen that shows its text through a cached view, the way a dock shows
/// its panels: a frame in which only the overlay changed replays the text
/// from the cache instead of painting it.
struct CachedScreen {
    text: Entity<Text>,
}

impl Render for CachedScreen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p_4().child(
            gpui::AnyView::from(self.text.clone())
                .cached(StyleRefinement::default().w(px(320.)).h(px(120.))),
        )
    }
}

/// A screen whose text is taller than the scroll box it sits in.
struct TallScreen {
    text: Entity<TextViewState>,
}

impl Render for TallScreen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p_4().child(
            div()
                .id("box")
                .w(px(200.))
                .h(px(48.))
                .overflow_y_scroll()
                .child(
                    div()
                        .id("text")
                        .test_support()
                        .child(TextView::new(&self.text).selectable(true)),
                ),
        )
    }
}

fn tall_screen(cx: &mut TestAppContext) -> WindowHandle<Root> {
    cx.update(gpui_component::init);
    common::open_window(cx, None, |_window, cx| {
        let view = cx.new(|cx| TallScreen {
            text: cx.new(|cx| {
                TextViewState::markdown("first line\n\nsecond line\n\nthird line\n\nlast line", cx)
            }),
        });
        view
    })
    .0
}

/// A finger's double tap: the touch is offered as a drag first, as GPUI
/// does, then arrives as a two-click press.
fn double_tap(window: &mut Window, cx: &mut gpui::App, position: Point<Pixels>) {
    window.dispatch_event(
        TouchDragEvent {
            phase: TouchPhase::Started,
            start_position: position,
            position,
        }
        .to_platform_input(),
        cx,
    );
    window.dispatch_event(
        MouseDownEvent {
            button: MouseButton::Left,
            position,
            modifiers: Modifiers::default(),
            click_count: 2,
            first_mouse: false,
        }
        .to_platform_input(),
        cx,
    );
    window.dispatch_event(
        MouseUpEvent {
            button: MouseButton::Left,
            position,
            modifiers: Modifiers::default(),
            click_count: 2,
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
}

/// A screen whose scroll container swallows every scroll packet on capture,
/// as `ScrollBounce` does while it is stretched past the end of its list.
struct SwallowingScreen {
    text: Entity<TextViewState>,
}

impl Render for SwallowingScreen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .child(
                canvas(
                    |_, _, _| (),
                    |_, _, window, _| {
                        window.on_mouse_event(|_: &ScrollWheelEvent, phase, _, cx| {
                            if phase == DispatchPhase::Capture {
                                cx.stop_propagation();
                            }
                        });
                    },
                )
                .absolute()
                .size_0(),
            )
            .child(
                div()
                    .id("text")
                    .test_support()
                    .w(px(320.))
                    .child(TextView::new(&self.text).selectable(true)),
            )
    }
}

fn swallowing_screen(cx: &mut TestAppContext) -> WindowHandle<Root> {
    cx.update(gpui_component::init);
    common::open_window(cx, None, |_window, cx| {
        let view = cx.new(|cx| SwallowingScreen {
            text: cx.new(|cx| TextViewState::markdown("quick select value", cx)),
        });
        view
    })
    .0
}

fn cached_screen(cx: &mut TestAppContext) -> WindowHandle<Root> {
    cx.update(gpui_component::init);
    common::open_window(cx, None, |_window, cx| {
        let view = cx.new(|cx| CachedScreen {
            text: cx.new(|cx| Text {
                text: cx.new(|cx| TextViewState::markdown("quick select value", cx)),
            }),
        });
        view
    })
    .0
}

/// A frame as the app draws one: what is not dirty is replayed from the
/// cache. `render_frame` refreshes the window first, which paints everything
/// afresh and would hide what a cached view does.
fn frame(window: &mut Window, cx: &mut gpui::App) {
    window.draw(cx).clear(cx);
}

fn long_press(window: &mut Window, cx: &mut gpui::App, position: Point<Pixels>) {
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
        window.render_frame(cx);
    }
}

#[gpui::test]
fn long_press_in_input_offers_copy_which_closes_the_menu(cx: &mut TestAppContext) {
    let handle = screen(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let input = window.find("input").bounds();
        long_press(window, cx, point(input.left() + px(24.), input.center().y));

        assert!(window.try_find("Copy").is_some(), "the menu offers Copy");
        assert!(
            window.try_find("Cut").is_some(),
            "an editable input offers Cut"
        );
        assert!(
            window.try_find("Select All").is_some(),
            "only one word is selected, so Select All is left"
        );

        window.click("Copy", cx);
        let copied = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap_or_default();
        assert_eq!(copied, "quick");
        assert!(
            window.try_find("Copy").is_none(),
            "Copy has done its work; the menu closes"
        );
    })
    .unwrap();
}

#[gpui::test]
fn long_press_in_input_select_all_keeps_the_menu(cx: &mut TestAppContext) {
    let handle = screen(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let input = window.find("input").bounds();
        long_press(window, cx, point(input.left() + px(24.), input.center().y));
        window.click("Select All", cx);
        assert_eq!(window.find("input").value(), Some("quick select value"));
        assert!(
            window.try_find("Select All").is_none(),
            "everything is selected; nothing is left to select"
        );
        assert!(window.try_find("Copy").is_some(), "the menu stays open");
    })
    .unwrap();
}

#[gpui::test]
fn long_press_in_text_view_offers_copy_and_select_all(cx: &mut TestAppContext) {
    let handle = screen(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let text = window.find("text").bounds();
        long_press(
            window,
            cx,
            point(text.left() + px(8.), text.top() + px(10.)),
        );
        assert_eq!(TextSelection::selected_text(window, cx).trim(), "quick");
        assert!(window.try_find("Copy").is_some());
        assert!(
            window.try_find("Cut").is_none(),
            "read-only text has nothing to cut"
        );

        window.click("Select All", cx);
        assert_eq!(
            TextSelection::selected_text(window, cx).trim(),
            "quick select value"
        );

        window.click("Copy", cx);
        let copied = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap_or_default();
        assert_eq!(copied.trim(), "quick select value");
    })
    .unwrap();
}

fn point(x: Pixels, y: Pixels) -> Point<Pixels> {
    Point { x, y }
}

/// The knob the finger takes hangs below the line for the end handle.
fn end_knob(snapshot: &gpui_base::TouchSelectionSnapshot) -> Point<Pixels> {
    let end = snapshot.end();
    point(end.left(), end.bottom() + px(6.))
}

#[gpui::test]
fn end_handle_drags_the_input_selection_with_a_finger(cx: &mut TestAppContext) {
    let handle = screen(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let input_bounds = window.find("input").bounds();
        long_press(
            window,
            cx,
            point(input_bounds.left() + px(24.), input_bounds.center().y),
        );
        let input = window
            .focused_input(cx)
            .and_then(|state| state.as_input().cloned())
            .expect("the long press focused the input");
        let snapshot = input.read(cx).touch_selection().expect("handles are shown");
        assert!(snapshot.is_menu_open());
        let start = end_knob(&snapshot);

        // The drag is offered on the first touch; the handle claims it and
        // the menu steps aside until the finger lifts.
        let far_right = point(input_bounds.right() - px(8.), start.y);
        for (phase, position) in [(TouchPhase::Started, start), (TouchPhase::Moved, far_right)] {
            window.dispatch_event(
                TouchDragEvent {
                    phase,
                    start_position: start,
                    position,
                }
                .to_platform_input(),
                cx,
            );
            window.render_frame(cx);
        }
        assert!(
            window.try_find("Copy").is_none(),
            "the menu hides while dragging"
        );
        assert_eq!(
            input.read(cx).selected_text().to_string(),
            "quick select value"
        );

        window.dispatch_event(
            TouchDragEvent {
                phase: TouchPhase::Ended,
                start_position: start,
                position: far_right,
            }
            .to_platform_input(),
            cx,
        );
        window.render_frame(cx);
        assert!(
            window.try_find("Copy").is_some(),
            "the menu is back once the finger lifts"
        );
        assert_eq!(
            input.read(cx).selected_text().to_string(),
            "quick select value"
        );
    })
    .unwrap();
}

#[gpui::test]
fn end_handle_drags_the_text_view_selection_with_a_mouse(cx: &mut TestAppContext) {
    let handle = screen(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let text = window.find("text").bounds();
        long_press(
            window,
            cx,
            point(text.left() + px(8.), text.top() + px(10.)),
        );
        let snapshot = TextSelection::touch_selection(window, cx).expect("handles are shown");
        let start = end_knob(&snapshot);
        window.drag(start, point(text.right() - px(8.), start.y), cx);
        assert_eq!(
            TextSelection::selected_text(window, cx).trim(),
            "quick select value"
        );
        let snapshot = TextSelection::touch_selection(window, cx).unwrap();
        assert!(
            snapshot.is_menu_open(),
            "the menu is back once the button is released"
        );
        assert_eq!(snapshot.dragging(), None);
    })
    .unwrap();
}

#[gpui::test]
fn a_handle_stops_at_the_other_end_instead_of_collapsing_the_selection(cx: &mut TestAppContext) {
    let handle = screen(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let text = window.find("text").bounds();
        long_press(
            window,
            cx,
            point(text.left() + px(8.), text.top() + px(10.)),
        );
        assert_eq!(TextSelection::selected_text(window, cx).trim(), "quick");

        // Pull the end handle back over the word, one short drag at a time,
        // then past its start where the paragraph has nothing more to select.
        for delta in [-6., -12., -20., -30., -40.] {
            let snapshot = TextSelection::touch_selection(window, cx).expect("still live");
            let start = end_knob(&snapshot);
            window.drag(start, point(start.x + px(delta), start.y), cx);
        }
        assert_eq!(TextSelection::selected_text(window, cx).trim(), "q");
        let snapshot = TextSelection::touch_selection(window, cx).expect("a character is left");
        assert!(snapshot.is_menu_open());
    })
    .unwrap();

    cx.update_window(handle.into(), |_, window, cx| {
        // The text's menu floats over the input; put it away first.
        TextSelection::clear(window, cx);
        window.render_frame(cx);
        let input_bounds = window.find("input").bounds();
        long_press(
            window,
            cx,
            point(input_bounds.left() + px(24.), input_bounds.center().y),
        );
        let input = window
            .focused_input(cx)
            .and_then(|state| state.as_input().cloned())
            .expect("the long press focused the input");
        assert_eq!(input.read(cx).selected_text().to_string(), "quick");
        for delta in [-6., -12., -20., -30.] {
            let snapshot = input.read(cx).touch_selection().expect("still live");
            let start = end_knob(&snapshot);
            window.drag(start, point(start.x + px(delta), start.y), cx);
        }
        assert_eq!(input.read(cx).selected_text().to_string(), "q");
        assert!(input.read(cx).touch_selection().is_some());
    })
    .unwrap();
}

#[gpui::test]
fn copy_after_select_all_keeps_the_whole_selection(cx: &mut TestAppContext) {
    let handle = screen(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let text = window.find("text").bounds();
        long_press(
            window,
            cx,
            point(text.left() + px(8.), text.top() + px(10.)),
        );
        window.click("Select All", cx);
        assert_eq!(
            TextSelection::selected_text(window, cx).trim(),
            "quick select value"
        );
        window.click("Copy", cx);
        assert_eq!(
            TextSelection::selected_text(window, cx).trim(),
            "quick select value",
            "the tap on Copy must not touch the selection"
        );
        let snapshot = TextSelection::touch_selection(window, cx).expect("handles stay");
        assert!(!snapshot.is_menu_open());
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("quick select value")
        );
    })
    .unwrap();
}

#[gpui::test]
fn the_first_move_after_taking_a_handle_is_not_lost(cx: &mut TestAppContext) {
    let handle = screen(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let input_bounds = window.find("input").bounds();
        long_press(
            window,
            cx,
            point(input_bounds.left() + px(24.), input_bounds.center().y),
        );
        let input = window
            .focused_input(cx)
            .and_then(|state| state.as_input().cloned())
            .expect("the long press focused the input");
        let snapshot = input.read(cx).touch_selection().unwrap();
        let start = end_knob(&snapshot);
        let far_right = point(input_bounds.right() - px(8.), start.y);
        // No frame between the three phases: the drag begins, moves and ends
        // before the handles could re-render around it.
        for (phase, position) in [
            (TouchPhase::Started, start),
            (TouchPhase::Moved, far_right),
            (TouchPhase::Ended, far_right),
        ] {
            window.dispatch_event(
                TouchDragEvent {
                    phase,
                    start_position: start,
                    position,
                }
                .to_platform_input(),
                cx,
            );
        }
        window.render_frame(cx);
        assert_eq!(
            input.read(cx).selected_text().to_string(),
            "quick select value"
        );
        let snapshot = input.read(cx).touch_selection().unwrap();
        assert_eq!(snapshot.dragging(), None);
        assert!(snapshot.is_menu_open());
    })
    .unwrap();
}

#[gpui::test]
fn select_all_after_a_drag_stays_put_frame_after_frame(cx: &mut TestAppContext) {
    let handle = screen(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let text = window.find("text").bounds();
        long_press(
            window,
            cx,
            point(text.left() + px(8.), text.top() + px(10.)),
        );
        let snapshot = TextSelection::touch_selection(window, cx).unwrap();
        let start = end_knob(&snapshot);
        window.drag(start, point(text.left() + px(120.), start.y), cx);
        assert_eq!(TextSelection::selected_text(window, cx), "quick select \n");

        window.click("Select All", cx);
        let mut seen = Vec::new();
        for _ in 0..6 {
            window.render_frame(cx);
            let snapshot = TextSelection::touch_selection(window, cx).expect("still live");
            seen.push((
                TextSelection::selected_text(window, cx),
                snapshot.start().origin,
                snapshot.end().origin,
                snapshot.is_menu_open(),
            ));
        }
        assert_eq!(seen[0].0.trim(), "quick select value");
        assert!(seen[0].3, "the menu stays open over the whole selection");
        assert!(
            seen.iter().all(|frame| frame == &seen[0]),
            "nothing may change from one frame to the next: {seen:#?}"
        );
    })
    .unwrap();
}

#[gpui::test]
fn a_selection_under_a_cached_view_holds_still_and_keeps_its_handles(cx: &mut TestAppContext) {
    let handle = cached_screen(cx);
    // One update per step: what a frame defers — the sweep of participants
    // that did not paint, the overlay's notification — runs when the update
    // ends, as it does between the app's frames.
    let text = cx
        .update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            let text = window.find("text").bounds();
            long_press(
                window,
                cx,
                point(text.left() + px(8.), text.top() + px(10.)),
            );
            assert_eq!(TextSelection::selected_text(window, cx).trim(), "quick");
            text
        })
        .unwrap();

    // Only the overlay drew after the press; the text was replayed from the
    // cache. The selection must not be taken for gone.
    let mut seen = Vec::new();
    for _ in 0..6 {
        let frame = cx
            .update_window(handle.into(), |_, window, cx| {
                frame(window, cx);
                (
                    TextSelection::touch_selection(window, cx).map(|snapshot| {
                        (
                            snapshot.start().origin,
                            snapshot.end().origin,
                            snapshot.is_menu_open(),
                        )
                    }),
                    window.try_find("Copy").is_some(),
                )
            })
            .unwrap();
        seen.push(frame);
    }
    assert!(
        seen[0].0.is_some_and(|(_, _, menu_open)| menu_open) && seen[0].1,
        "the handles and the menu are up: {seen:#?}"
    );
    assert!(
        seen.iter().all(|frame| frame == &seen[0]),
        "nothing may change from one frame to the next: {seen:#?}"
    );

    // And the end handle takes a finger, which needs its hitbox in the frame
    // the replayed text left behind.
    let start = cx
        .update_window(handle.into(), |_, window, cx| {
            end_knob(&TextSelection::touch_selection(window, cx).unwrap())
        })
        .unwrap();
    let far_right = point(text.right() - px(8.), start.y);
    for (phase, position) in [
        (TouchPhase::Started, start),
        (TouchPhase::Moved, far_right),
        (TouchPhase::Ended, far_right),
    ] {
        cx.update_window(handle.into(), |_, window, cx| {
            window.dispatch_event(
                TouchDragEvent {
                    phase,
                    start_position: start,
                    position,
                }
                .to_platform_input(),
                cx,
            );
            frame(window, cx);
        })
        .unwrap();
    }
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(
            TextSelection::selected_text(window, cx).trim(),
            "quick select value",
            "the handle claimed the drag"
        );
    })
    .unwrap();
}

#[gpui::test]
fn a_double_tap_in_text_view_selects_nothing(cx: &mut TestAppContext) {
    let handle = screen(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let text = window.find("text").bounds();
        double_tap(
            window,
            cx,
            point(text.left() + px(8.), text.top() + px(10.)),
        );
        window.render_frame(cx);
        // Read-only text takes a finger's long press only. (The mouse's
        // double click still selects the word; a touch just went down here,
        // so that is for another test.)
        assert_eq!(TextSelection::selected_text(window, cx).trim(), "");
        assert!(TextSelection::touch_selection(window, cx).is_none());
        assert!(window.try_find("Copy").is_none());
    })
    .unwrap();
}

#[gpui::test]
fn select_all_takes_the_text_beyond_the_viewport(cx: &mut TestAppContext) {
    let handle = tall_screen(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let text = window.find("text").bounds();
        long_press(
            window,
            cx,
            point(text.left() + px(8.), text.top() + px(10.)),
        );
        assert_eq!(TextSelection::selected_text(window, cx).trim(), "first");

        window.click("Select All", cx);
        assert_eq!(
            TextSelection::selected_text(window, cx).trim(),
            "first line\nsecond line\nthird line\nlast line",
            "the whole text, not the lines the box shows"
        );
    })
    .unwrap();
}

#[gpui::test]
fn the_menu_comes_back_after_a_scroll_the_container_swallowed(cx: &mut TestAppContext) {
    let handle = swallowing_screen(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let text = window.find("text").bounds();
        long_press(
            window,
            cx,
            point(text.left() + px(8.), text.top() + px(10.)),
        );
        assert!(window.try_find("Copy").is_some(), "the menu is up");

        // A finger scrolls: the menu steps aside while it moves, and comes
        // back when it lifts — even when the scroll container stopped every
        // packet before the selection's own listener, as a bounce does.
        let position = point(text.center().x, text.bottom() + px(40.));
        let mut menu_seen = Vec::new();
        for (touch_phase, dy) in [
            (TouchPhase::Started, 0.),
            (TouchPhase::Moved, -12.),
            (TouchPhase::Ended, 0.),
        ] {
            window.dispatch_event(
                ScrollWheelEvent {
                    position,
                    delta: ScrollDelta::Pixels(point(px(0.), px(dy))),
                    modifiers: Modifiers::default(),
                    touch_phase,
                }
                .to_platform_input(),
                cx,
            );
            window.render_frame(cx);
            menu_seen.push(window.try_find("Copy").is_some());
        }
        assert_eq!(
            menu_seen,
            [false, false, true],
            "aside while the finger moves, back when it lifts"
        );
        assert_eq!(TextSelection::selected_text(window, cx).trim(), "quick");
    })
    .unwrap();
}
