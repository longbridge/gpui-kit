//! Application-facing multiline editing and viewport regressions.
use gpui_kit::{
    App, AppContext, ClipboardItem, Context, ElementId, Entity, InputEvent as _, Modifiers,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollDelta,
    Subscription, TestAppContext, Window, WindowHandle,
    component::input::{InputEvent, Textarea, TextareaState},
    div, point,
    prelude::*,
    px, size,
    test::TestWindowExt,
};

use crate::common;

#[cfg(target_os = "macos")]
const START: &str = "cmd-up";
#[cfg(not(target_os = "macos"))]
const START: &str = "ctrl-home";
#[cfg(target_os = "macos")]
const END: &str = "cmd-down";
#[cfg(not(target_os = "macos"))]
const END: &str = "ctrl-end";

#[cfg(target_os = "macos")]
const WORD_MODIFIER: &str = "alt";
#[cfg(not(target_os = "macos"))]
const WORD_MODIFIER: &str = "ctrl";
#[cfg(target_os = "macos")]
const LINE_END: &str = "cmd-right";
#[cfg(not(target_os = "macos"))]
const LINE_END: &str = "end";
#[cfg(target_os = "macos")]
const LINE_START: &str = "cmd-left";
#[cfg(not(target_os = "macos"))]
const LINE_START: &str = "home";

struct Composer {
    text: Entity<TextareaState>,
    enters: Vec<(bool, bool, String)>,
    _subscription: Subscription,
}

impl Render for Composer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .child(Textarea::new(&self.text).w_full())
    }
}

fn composer(
    cx: &mut TestAppContext,
    configure: impl FnOnce(TextareaState) -> TextareaState,
) -> (
    WindowHandle<gpui_kit::base::Root>,
    Entity<Composer>,
    Entity<TextareaState>,
) {
    cx.update(gpui_kit::init);
    let (window, view) = common::open_window(cx, Some(size(px(480.), px(480.))), |window, cx| {
        cx.new(|cx| {
            let text = cx.new(|cx| configure(TextareaState::new(window, cx)));
            let subscription = cx.subscribe(&text, |this: &mut Composer, text, event, cx| {
                if let InputEvent::PressEnter { secondary, shift } = event {
                    this.enters
                        .push((*secondary, *shift, text.read(cx).value().to_string()));
                }
            });
            Composer {
                text,
                enters: Vec::new(),
                _subscription: subscription,
            }
        })
    });
    let text = cx.update(|cx| view.read(cx).text.clone());
    (window, view, text)
}

fn target(text: &Entity<TextareaState>) -> ElementId {
    ("input", text.entity_id()).into()
}

fn caret_point(state: &TextareaState) -> Point<Pixels> {
    let caret = state.cursor_layout().expect("laid-out caret").0;
    // Stay just before the insertion boundary: converting the window point
    // back to line-local coordinates can round past the final glyph's origin,
    // where GPUI's closest_index_for_x falls through to the end of the line.
    let mut position = point(caret.left() - px(0.1), caret.center().y);
    // Caret x already includes scrolling; y is in unscrolled content space.
    position.y += state.scroll_offset().y;
    position
}

fn pointer_click(
    window: &mut Window,
    position: Point<Pixels>,
    shift: bool,
    count: usize,
    cx: &mut App,
) {
    let modifiers = Modifiers {
        shift,
        ..Default::default()
    };
    window.dispatch_event(
        MouseMoveEvent {
            position,
            pressed_button: None,
            modifiers,
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
    window.dispatch_event(
        MouseDownEvent {
            position,
            button: MouseButton::Left,
            modifiers,
            click_count: count,
            first_mouse: false,
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
    window.dispatch_event(
        MouseUpEvent {
            position,
            button: MouseButton::Left,
            modifiers,
            click_count: count,
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
}

fn assert_caret_visible(state: &TextareaState) {
    // cursor_layout is in unscrolled coordinates; apply the viewport offset.
    let (caret, _) = state.cursor_layout().expect("laid-out caret");
    let viewport = state.input_bounds();
    let top = caret.top() + state.scroll_offset().y;
    assert!(top >= viewport.top(), "caret above viewport");
    assert!(
        top + caret.size.height <= viewport.bottom(),
        "caret below viewport"
    );
}

#[gpui_kit::test]
fn enter_and_shift_enter_insert_plain_newlines(cx: &mut TestAppContext) {
    let (handle, owner, text) = composer(cx, |state| state.rows(4));
    for (key, typed, expected) in [
        ("enter", "first", "first\n"),
        ("shift-enter", "中🦀", "first\n中🦀\n"),
    ] {
        cx.update_window(handle.into(), |_, window, cx| {
            if text.read(cx).value().is_empty() {
                window.click(target(&text), cx);
            }
            window.input(typed, cx);
            window.press(key, cx);
            assert_eq!(text.read(cx).value(), expected);
            assert_eq!(text.read(cx).cursor(), expected.len());
            assert_eq!(window.find(target(&text)).value(), Some(expected));
        })
        .unwrap();
        cx.run_until_parked();
    }
    cx.update(|cx| {
        assert_eq!(
            owner.read(cx).enters,
            vec![
                (false, false, "first\n".into()),
                (false, true, "first\n中🦀\n".into()),
            ]
        )
    });
}

#[gpui_kit::test]
fn submit_on_enter_preserves_text_but_shift_enter_inserts(cx: &mut TestAppContext) {
    let (handle, owner, text) = composer(cx, |state| state.submit_on_enter(true));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.input("send", cx);
        window.press("enter", cx);
        assert_eq!(text.read(cx).value(), "send");
        assert_eq!(text.read(cx).cursor(), 4);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| assert_eq!(owner.read(cx).enters, vec![(false, false, "send".into())]));
    cx.update_window(handle.into(), |_, window, cx| {
        window.press("shift-enter", cx);
        assert_eq!(text.read(cx).value(), "send\n");
        assert_eq!(text.read(cx).cursor(), 5);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(
            owner.read(cx).enters,
            vec![
                (false, false, "send".into()),
                (false, true, "send\n".into()),
            ]
        )
    });
}

#[gpui_kit::test]
fn keyboard_selection_cuts_and_pastes_across_unicode_lines(cx: &mut TestAppContext) {
    let (handle, _, text) = composer(cx, |state| state.default_value("a🦀\n中b"));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(END, cx);
        for _ in 0..3 {
            window.press("shift-left", cx);
        }
        assert_eq!(
            text.read(cx).selected_range(),
            "a🦀".len().."a🦀\n中b".len()
        );
        window.press("secondary-c", cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("\n中b")
        );
        assert_eq!(text.read(cx).value(), "a🦀\n中b");
        window.press("secondary-x", cx);
        assert_eq!(text.read(cx).value(), "a🦀");
        assert_eq!(text.read(cx).selected_range(), 5..5);
        window.press("secondary-v", cx);
        assert_eq!(text.read(cx).value(), "a🦀\n中b");
        assert_eq!(text.read(cx).cursor(), "a🦀\n中b".len());
    })
    .unwrap();
}

#[gpui_kit::test]
fn pasted_crlf_is_one_navigation_and_deletion_boundary(cx: &mut TestAppContext) {
    let (handle, _, text) = composer(cx, |state| state);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        cx.write_to_clipboard(ClipboardItem::new_string("é\r\n中".into()));
        window.press("secondary-v", cx);
        assert_eq!(text.read(cx).value(), "é\r\n中");
        window.press(START, cx);
        window.press("right", cx);
        assert_eq!(text.read(cx).cursor(), "é".len());
        window.press("right", cx);
        assert_eq!(text.read(cx).cursor(), "é\r\n".len());
        window.press("backspace", cx);
        assert_eq!(text.read(cx).value(), "é中");
        assert_eq!(text.read(cx).cursor(), "é".len());
    })
    .unwrap();
}

#[gpui_kit::test]
fn vertical_arrows_follow_soft_wrapped_rows(cx: &mut TestAppContext) {
    let value = "word ".repeat(80);
    let (handle, _, text) = composer(cx, |state| state.rows(6).default_value(value.clone()));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        let first = text.read(cx).cursor_layout().unwrap().0;
        window.press("down", cx);
        let state = text.read(cx);
        assert!(state.cursor() > 0 && state.cursor() < value.len());
        assert_eq!(
            state.cursor_position().line,
            0,
            "soft wrap is not a buffer newline"
        );
        assert!(state.cursor_layout().unwrap().0.top() > first.top());
        assert_caret_visible(state);
        window.press("up", cx);
        assert_eq!(text.read(cx).cursor(), 0);
        assert_eq!(text.read(cx).cursor_layout().unwrap().0.top(), first.top());
        assert_eq!(text.read(cx).value(), value);
    })
    .unwrap();
}

#[gpui_kit::test]
fn selection_across_soft_wraps_copies_and_replaces_buffer_text(cx: &mut TestAppContext) {
    let value = "中🦀 word ".repeat(80);
    let (handle, _, text) = composer(cx, |state| state.rows(6).default_value(value.clone()));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        let first_row = text.read(cx).cursor_layout().unwrap().0.top();
        // Measure the native Down destination in this layout instead of
        // assuming a font-dependent wrap offset. Shift should retain the
        // anchor while reaching the same destination.
        window.press("down", cx);
        let next_row_cursor = text.read(cx).cursor();
        assert!(next_row_cursor > 0 && next_row_cursor < value.len());
        assert!(text.read(cx).cursor_layout().unwrap().0.top() > first_row);
        window.press(START, cx);
        window.press("shift-down", cx);
        let selected = text.read(cx).selected_range();
        assert_eq!(
            selected,
            0..next_row_cursor,
            "Shift-Down should extend selection to the same visual row as Down"
        );
        assert_eq!(text.read(cx).cursor_position().line, 0);
        assert!(text.read(cx).cursor_layout().unwrap().0.top() > first_row);
        window.press("secondary-c", cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some(&value[selected.clone()])
        );
        assert_eq!(text.read(cx).value(), value);
        window.input("X", cx);
        let expected = format!("X{}", &value[selected.end..]);
        assert_eq!(text.read(cx).value(), expected);
        assert_eq!(text.read(cx).selected_range(), 1..1);
        assert_eq!(window.find(target(&text)).value(), Some(expected.as_str()));
        window.press("secondary-z", cx);
        assert_eq!(text.read(cx).value(), value);
        assert_eq!(text.read(cx).selected_range(), selected);
    })
    .unwrap();
}

#[gpui_kit::test]
fn wrapped_selection_repeats_and_reverses_across_its_anchor(cx: &mut TestAppContext) {
    let value = "中🦀 word ".repeat(80);
    let (handle, _, text) = composer(cx, |state| state.rows(6).default_value(value.clone()));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        // Measure four visual rows through native movement, without assuming
        // a font-dependent wrap offset. Start selection on the third row.
        let mut rows = vec![(
            text.read(cx).cursor(),
            text.read(cx).cursor_layout().unwrap().0,
        )];
        for _ in 0..3 {
            window.press("down", cx);
            let state = text.read(cx);
            let caret = state.cursor_layout().unwrap().0;
            assert!(state.cursor() > rows.last().unwrap().0);
            assert!(caret.top() > rows.last().unwrap().1.top());
            rows.push((state.cursor(), caret));
        }
        window.press(START, cx);
        window.press("down", cx);
        window.press("down", cx);
        let anchor = rows[2].0;
        assert_eq!(text.read(cx).cursor(), anchor);
        for (key, row) in [
            ("shift-up", 1),
            ("shift-up", 0),
            ("shift-down", 1),
            ("shift-down", 2),
            ("shift-down", 3),
            ("shift-up", 2),
        ] {
            window.press(key, cx);
            let state = text.read(cx);
            let cursor = rows[row].0;
            assert_eq!(state.cursor(), cursor, "{key} to visual row {row}");
            assert_eq!(
                state.selected_range(),
                anchor.min(cursor)..anchor.max(cursor)
            );
            assert_eq!(state.cursor_layout().unwrap().0.top(), rows[row].1.top());
            assert_eq!(state.cursor_position().line, 0);
            assert_caret_visible(state);
            assert_eq!(window.find(target(&text)).value(), Some(value.as_str()));
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn vertical_selection_preserves_column_across_short_and_empty_lines(cx: &mut TestAppContext) {
    let value = "abcdefghij\nx\n\nabcdefghij";
    let (handle, _, text) = composer(cx, |state| state.rows(4).default_value(value));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        for _ in 0..6 {
            window.press("right", cx);
        }
        let anchor = 6;
        assert_eq!(text.read(cx).cursor(), anchor);
        let original_x = text.read(cx).cursor_layout().unwrap().0.left();
        // Clamp at the short/empty row, then recover column six in either
        // direction. The selection anchor stays on the first line throughout.
        for (key, cursor) in [
            ("shift-down", 12),
            ("shift-down", 13),
            ("shift-down", 20),
            ("shift-up", 13),
            ("shift-up", 12),
            ("shift-up", 6),
        ] {
            window.press(key, cx);
            let state = text.read(cx);
            assert_eq!(state.cursor(), cursor, "{key}");
            assert_eq!(state.selected_range(), anchor..cursor);
            if cursor == 20 || cursor == anchor {
                assert_eq!(state.cursor_layout().unwrap().0.left(), original_x);
            }
            assert_caret_visible(state);
            assert_eq!(window.find(target(&text)).value(), Some(value));
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn document_navigation_reveals_both_ends_of_a_fixed_viewport(cx: &mut TestAppContext) {
    let value = (0..40).map(|n| format!("line {n}\n")).collect::<String>();
    let (handle, _, text) = composer(cx, |state| state.rows(3).default_value(value.clone()));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        let initial_height = window.find(target(&text)).bounds().size.height;
        window.press(END, cx);
        assert_eq!(text.read(cx).cursor(), value.len());
        assert!(text.read(cx).scroll_offset().y < px(0.));
        assert_caret_visible(text.read(cx));
        window.press(START, cx);
        assert_eq!(text.read(cx).cursor(), 0);
        assert_eq!(text.read(cx).scroll_offset().y, px(0.));
        assert_caret_visible(text.read(cx));
        assert_eq!(
            window.find(target(&text)).bounds().size.height,
            initial_height
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn typing_reveals_caret_after_user_scrolls_away(cx: &mut TestAppContext) {
    let value = (0..40).map(|n| format!("line {n}\n")).collect::<String>();
    let (handle, _, text) = composer(cx, |state| {
        state.auto_grow(1, 4).default_value(value.clone())
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(END, cx);
        let at_end = text.read(cx).scroll_offset().y;
        window.scroll(target(&text), ScrollDelta::Lines(point(0., 100.)), cx);
        assert!(
            text.read(cx).scroll_offset().y > at_end,
            "wheel must move the viewport"
        );
        assert_eq!(
            text.read(cx).cursor(),
            value.len(),
            "wheel must not move the caret"
        );
        window.input("X", cx);
        assert_eq!(text.read(cx).value(), format!("{value}X"));
        assert_eq!(text.read(cx).cursor(), value.len() + 1);
        assert!(text.read(cx).scroll_offset().y < px(0.));
        assert_caret_visible(text.read(cx));
    })
    .unwrap();
}

#[gpui_kit::test]
fn auto_grow_obeys_minimum_and_maximum_then_shrinks_after_delete(cx: &mut TestAppContext) {
    let (handle, _, text) = composer(cx, |state| state.auto_grow(2, 4));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        let minimum = window.find(target(&text)).bounds().size.height;
        window.input("one", cx);
        window.press("enter", cx);
        assert_eq!(window.find(target(&text)).bounds().size.height, minimum);
        window.press("enter", cx);
        let three_rows = window.find(target(&text)).bounds().size.height;
        assert!(three_rows > minimum);
        window.press("enter", cx);
        let maximum = window.find(target(&text)).bounds().size.height;
        assert!(maximum > three_rows);
        for _ in 0..8 {
            window.press("enter", cx);
        }
        assert_eq!(window.find(target(&text)).bounds().size.height, maximum);
        assert_caret_visible(text.read(cx));
        window.press("secondary-a", cx);
        window.press("backspace", cx);
        assert_eq!(text.read(cx).value(), "");
        assert_eq!(text.read(cx).cursor(), 0);
        assert_eq!(window.find(target(&text)).bounds().size.height, minimum);
    })
    .unwrap();
}

#[gpui_kit::test]
fn resizing_reflows_auto_grow_without_changing_text_or_caret(cx: &mut TestAppContext) {
    let value = "word ".repeat(32);
    let (handle, _, text) = composer(cx, |state| {
        state.auto_grow(1, 30).default_value(value.clone())
    });
    let initial = cx
        .update_window(handle.into(), |_, window, cx| {
            window.click(target(&text), cx);
            window.press(END, cx);
            window.find(target(&text)).bounds().size
        })
        .unwrap();
    cx.simulate_window_resize(handle.into(), size(px(240.), px(720.)));
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let narrow = window.find(target(&text)).bounds().size;
        assert!(narrow.width < initial.width);
        assert!(narrow.height > initial.height);
        assert_eq!(text.read(cx).value(), value);
        assert_eq!(text.read(cx).cursor(), value.len());
    })
    .unwrap();
    cx.simulate_window_resize(handle.into(), size(px(480.), px(480.)));
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find(target(&text)).bounds().size, initial);
        assert_eq!(text.read(cx).cursor(), value.len());
        assert_eq!(text.read(cx).value(), value);
    })
    .unwrap();
}

#[gpui_kit::test]
fn vertical_selection_reaches_document_edges_from_inside_the_only_row(cx: &mut TestAppContext) {
    let (handle, _, text) = composer(cx, |state| state.rows(4).default_value("abcdef"));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        for (key, range, cursor) in [("shift-up", 0..3, 0), ("shift-down", 3..6, 6)] {
            window.press(START, cx);
            for _ in 0..3 {
                window.press("right", cx);
            }
            window.press(key, cx);
            assert_eq!(text.read(cx).selected_range(), range, "{key}");
            assert_eq!(text.read(cx).cursor(), cursor);
            window.press(key, cx);
            assert_eq!(text.read(cx).selected_range(), range, "repeated {key}");
            window.press("secondary-c", cx);
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some(&"abcdef"[range])
            );
        }
        assert_eq!(text.read(cx).value(), "abcdef");
    })
    .unwrap();
}

#[gpui_kit::test]
fn horizontal_selection_sets_the_column_for_vertical_extension(cx: &mut TestAppContext) {
    let (handle, _, text) = composer(cx, |state| state.rows(4).default_value("abcdef\nabcdef"));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        for (key, range, cursor) in [
            ("shift-right", 0..1, 1),
            ("shift-down", 0..8, 8),
            ("shift-left", 0..7, 7),
            ("shift-up", 0..0, 0),
            ("shift-down", 0..7, 7),
        ] {
            window.press(key, cx);
            assert_eq!(text.read(cx).selected_range(), range, "{key}");
            assert_eq!(text.read(cx).cursor(), cursor, "{key}");
        }
        window.press(END, cx);
        window.press("shift-left", cx);
        window.press("shift-up", cx);
        assert_eq!(text.read(cx).selected_range(), 5..13);
        assert_eq!(text.read(cx).cursor(), 5);
    })
    .unwrap();
}

#[gpui_kit::test]
fn arrows_recover_preferred_column_after_short_rows_and_reset_it_after_horizontal_motion(
    cx: &mut TestAppContext,
) {
    let (handle, _, text) = composer(cx, |state| {
        state.rows(4).default_value("abcdefghij\nx\n\nabcdefghij")
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        for _ in 0..6 {
            window.press("right", cx);
        }
        for (key, cursor) in [
            ("down", 12),
            ("down", 13),
            ("down", 20),
            ("up", 13),
            ("up", 12),
            ("up", 6),
        ] {
            window.press(key, cx);
            assert_eq!(text.read(cx).selected_range(), cursor..cursor, "{key}");
        }
        window.press("down", cx);
        window.press("left", cx);
        assert_eq!(text.read(cx).cursor(), 11);
        window.press("down", cx);
        window.press("down", cx);
        assert_eq!(
            text.read(cx).cursor(),
            14,
            "horizontal motion resets the preferred column"
        );
        window.press("up", cx);
        window.press("up", cx);
        window.press("up", cx);
        assert_eq!(text.read(cx).cursor(), 0);
    })
    .unwrap();
}

#[gpui_kit::test]
fn document_selection_reverses_across_anchor_and_deletes_only_selected_text(
    cx: &mut TestAppContext,
) {
    let value = "ab\n中🦀\ncd";
    let anchor = "ab\n中".len();
    let (handle, _, text) = composer(cx, |state| state.rows(4).default_value(value));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        for _ in 0..4 {
            window.press("right", cx);
        }
        assert_eq!(text.read(cx).cursor(), anchor);
        for (key, range, cursor) in [
            (format!("shift-{START}"), 0..anchor, 0),
            (format!("shift-{END}"), anchor..value.len(), value.len()),
            (format!("shift-{START}"), 0..anchor, 0),
        ] {
            window.press(&key, cx);
            assert_eq!(text.read(cx).selected_range(), range);
            assert_eq!(text.read(cx).cursor(), cursor);
        }
        window.press("delete", cx);
        assert_eq!(text.read(cx).value(), "🦀\ncd");
        assert_eq!(text.read(cx).selected_range(), 0..0);
        window.press("secondary-z", cx);
        assert_eq!(text.read(cx).value(), value);
        assert_eq!(text.read(cx).selected_range(), 0..anchor);
    })
    .unwrap();
}

#[gpui_kit::test]
fn word_navigation_selection_and_deletion_preserve_adjacent_lines(cx: &mut TestAppContext) {
    let (handle, _, text) = composer(cx, |state| {
        state.rows(3).default_value("one two\nthree four")
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        window.press(&format!("{WORD_MODIFIER}-right"), cx);
        assert_eq!(text.read(cx).cursor(), 3);
        window.press(&format!("{WORD_MODIFIER}-shift-right"), cx);
        assert_eq!(text.read(cx).selected_range(), 3..7);
        window.press("delete", cx);
        assert_eq!(text.read(cx).value(), "one\nthree four");
        window.press(END, cx);
        window.press(&format!("{WORD_MODIFIER}-left"), cx);
        assert_eq!(text.read(cx).cursor(), "one\nthree ".len());
        window.press(&format!("{WORD_MODIFIER}-delete"), cx);
        assert_eq!(text.read(cx).value(), "one\nthree ");
        window.press("backspace", cx);
        window.press(&format!("{WORD_MODIFIER}-shift-left"), cx);
        assert_eq!(text.read(cx).selected_range(), 4..9);
        window.press(&format!("{WORD_MODIFIER}-backspace"), cx);
        assert_eq!(text.read(cx).value(), "one\n");
        window.press(START, cx);
        window.press(&format!("{WORD_MODIFIER}-right"), cx);
        window.press(&format!("{WORD_MODIFIER}-backspace"), cx);
        assert_eq!(text.read(cx).value(), "\n");
        assert_eq!(text.read(cx).cursor(), 0);
    })
    .unwrap();
}

#[gpui_kit::test]
fn pointer_drag_across_unicode_rows_preserves_direction_and_replaces_selection(
    cx: &mut TestAppContext,
) {
    let value = "a🦀\n中b\nend";
    let (handle, _, text) = composer(cx, |state| state.auto_grow(4, 4).default_value(value));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        window.press("right", cx);
        let from = caret_point(text.read(cx));
        for _ in 0..4 {
            window.press("right", cx);
        }
        assert_eq!(text.read(cx).cursor(), "a🦀\n中b".len());
        let to = caret_point(text.read(cx));
        pointer_click(window, from, false, 1, cx);
        assert_eq!(
            text.read(cx).cursor(),
            1,
            "click at the measured insertion boundary before the emoji"
        );
        for (start, end, cursor) in [(from, to, 10), (to, from, 1)] {
            window.drag(start, end, cx);
            assert_eq!(text.read(cx).selected_range(), 1..10);
            assert_eq!(text.read(cx).cursor(), cursor);
            window.press("secondary-c", cx);
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some("🦀\n中b")
            );
        }
        window.input("X", cx);
        assert_eq!(text.read(cx).value(), "aX\nend");
        assert_eq!(text.read(cx).selected_range(), 2..2);
    })
    .unwrap();
}

#[gpui_kit::test]
fn page_navigation_preserves_column_and_clamps_at_document_boundaries(cx: &mut TestAppContext) {
    let value = vec!["abcdef"; 30].join("\n");
    let (handle, _, text) = composer(cx, |state| state.rows(4).default_value(value.clone()));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        for _ in 0..3 {
            window.press("right", cx);
        }
        let state = text.read(cx);
        let page_rows =
            (state.input_bounds().size.height / state.cursor_layout().unwrap().1) as usize;
        assert!(
            page_rows > 0 && page_rows < 29,
            "fixture must have a partial-document viewport"
        );
        window.press("pagedown", cx);
        let cursor = text.read(cx).cursor();
        assert_eq!(
            cursor,
            3 + page_rows * 7,
            "PageDown moves one visible page at the same column"
        );
        assert_eq!(cursor % 7, 3);
        assert_caret_visible(text.read(cx));
        window.press("pageup", cx);
        assert_eq!(text.read(cx).cursor(), 3);
        window.press(END, cx);
        window.press("pagedown", cx);
        assert_eq!(text.read(cx).selected_range(), value.len()..value.len());
        window.press(START, cx);
        window.press("pageup", cx);
        assert_eq!(text.read(cx).selected_range(), 0..0);
        assert_eq!(text.read(cx).value(), value);
    })
    .unwrap();
}

#[gpui_kit::test]
fn empty_document_navigation_and_forward_crlf_deletion_respect_scalar_boundaries(
    cx: &mut TestAppContext,
) {
    let (handle, _, text) = composer(cx, |state| state.rows(4));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        for key in [
            "up",
            "down",
            "left",
            "right",
            "shift-up",
            "shift-down",
            "pageup",
            "pagedown",
            "delete",
            "backspace",
            START,
            END,
        ] {
            window.press(key, cx);
            assert_eq!(text.read(cx).selected_range(), 0..0, "{key}");
            assert_eq!(text.read(cx).value(), "");
        }
        cx.write_to_clipboard(ClipboardItem::new_string("e\u{301}\r\n🦀".into()));
        window.press("secondary-v", cx);
        window.press(START, cx);
        // Text editing moves through Unicode scalar boundaries; CRLF is
        // deliberately one boundary. A combining mark remains its own scalar.
        window.press("right", cx);
        assert_eq!(text.read(cx).cursor(), 1);
        window.press("right", cx);
        assert_eq!(text.read(cx).cursor(), "e\u{301}".len());
        window.press("shift-right", cx);
        assert_eq!(text.read(cx).selected_range(), 3..5);
        window.press("left", cx);
        assert_eq!(text.read(cx).cursor(), 3);
        window.press("delete", cx);
        assert_eq!(text.read(cx).value(), "e\u{301}🦀");
        window.press("delete", cx);
        assert_eq!(text.read(cx).value(), "e\u{301}");
        window.press("backspace", cx);
        assert_eq!(text.read(cx).value(), "e");
        assert_eq!(text.read(cx).cursor(), 1);
        window.press("backspace", cx);
        assert_eq!(text.read(cx).value(), "");
        assert_eq!(text.read(cx).cursor(), 0);
    })
    .unwrap();
}

#[gpui_kit::test]
fn wrapped_logical_line_end_and_last_row_selection_reach_document_end(cx: &mut TestAppContext) {
    let value = "word ".repeat(80);
    let (handle, _, text) = composer(cx, |state| state.rows(6).default_value(value.clone()));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        window.press(LINE_END, cx);
        assert_eq!(
            text.read(cx).cursor(),
            value.len(),
            "Textarea End addresses the logical line"
        );
        window.press(LINE_START, cx);
        assert_eq!(text.read(cx).cursor(), 0);
        window.press(LINE_END, cx);
        window.press("left", cx);
        let anchor = value.len() - 1;
        window.press("shift-down", cx);
        assert_eq!(text.read(cx).selected_range(), anchor..value.len());
        window.press("shift-down", cx);
        assert_eq!(text.read(cx).selected_range(), anchor..value.len());
        assert_caret_visible(text.read(cx));
    })
    .unwrap();
}

#[gpui_kit::test]
fn line_boundary_selection_reverses_without_consuming_crlf(cx: &mut TestAppContext) {
    let value = "first\r\na中🦀z\r\nlast";
    let (handle, _, text) = composer(cx, |state| state.rows(4).default_value(value));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        window.press("down", cx);
        window.press(LINE_START, cx);
        assert_eq!(text.read(cx).cursor(), 7);
        window.press("right", cx);
        window.press("right", cx);
        let anchor = "first\r\na中".len();
        assert_eq!(text.read(cx).cursor(), anchor);
        window.press(&format!("shift-{LINE_END}"), cx);
        assert_eq!(text.read(cx).selected_range(), anchor..16);
        window.press("secondary-c", cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("🦀z")
        );
        window.press(&format!("shift-{LINE_START}"), cx);
        assert_eq!(text.read(cx).selected_range(), 7..anchor);
        assert_eq!(text.read(cx).cursor(), 7);
        window.press("backspace", cx);
        assert_eq!(text.read(cx).value(), "first\r\n🦀z\r\nlast");
        assert_eq!(text.read(cx).selected_range(), 7..7);
        window.press(LINE_END, cx);
        assert_eq!(text.read(cx).cursor(), 12);
        window.press("delete", cx);
        assert_eq!(text.read(cx).value(), "first\r\n🦀zlast");
    })
    .unwrap();
}

#[gpui_kit::test]
fn shift_click_extends_and_reverses_multiline_selection_from_the_original_anchor(
    cx: &mut TestAppContext,
) {
    let value = "abcd\nabcd\nabcd";
    let (handle, _, text) = composer(cx, |state| state.auto_grow(5, 5).default_value(value));
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        window.press("right", cx);
        let first = caret_point(text.read(cx));
        window.press("down", cx);
        let anchor = caret_point(text.read(cx));
        window.press("down", cx);
        let last = caret_point(text.read(cx));
        pointer_click(window, anchor, false, 1, cx);
        assert_eq!(text.read(cx).selected_range(), 6..6);
        for (position, range, cursor) in [(last, 6..11, 11), (first, 1..6, 1), (anchor, 6..6, 6)] {
            pointer_click(window, position, true, 1, cx);
            assert_eq!(text.read(cx).selected_range(), range);
            assert_eq!(text.read(cx).cursor(), cursor);
        }
        pointer_click(window, last, true, 1, cx);
        window.input("X", cx);
        assert_eq!(text.read(cx).value(), "abcd\naXbcd");
        assert_eq!(text.read(cx).selected_range(), 7..7);
    })
    .unwrap();
}

#[gpui_kit::test]
fn double_click_selects_a_word_and_triple_click_selects_the_whole_wrapped_paragraph(
    cx: &mut TestAppContext,
) {
    let paragraph = "alpha beta ".repeat(16);
    let value = format!("first\n{paragraph}\nlast");
    let (handle, _, text) = composer(cx, |state| {
        state.auto_grow(8, 8).default_value(value.clone())
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        for _ in 0..8 {
            window.press("right", cx);
        }
        assert_eq!(text.read(cx).cursor(), 8);
        let position = caret_point(text.read(cx));
        pointer_click(window, position, false, 2, cx);
        assert_eq!(text.read(cx).selected_range(), 6..11);
        window.press("secondary-c", cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("alpha")
        );
        pointer_click(window, position, false, 3, cx);
        assert_eq!(text.read(cx).selected_range(), 6..6 + paragraph.len());
        window.press("secondary-c", cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some(paragraph.as_str())
        );
        cx.write_to_clipboard(ClipboardItem::new_string("replacement".into()));
        window.press("secondary-v", cx);
        assert_eq!(text.read(cx).value(), "first\nreplacement\nlast");
        window.press("secondary-z", cx);
        assert_eq!(text.read(cx).value(), value);
        assert_eq!(text.read(cx).selected_range(), 6..6 + paragraph.len());
    })
    .unwrap();
}

#[gpui_kit::test]
fn dragging_beyond_viewport_autoscrolls_selection_and_release_stops_it(cx: &mut TestAppContext) {
    let value = vec!["abcdef"; 80].join("\n");
    let (handle, _, text) = composer(cx, |state| {
        state.auto_grow(4, 4).default_value(value.clone())
    });
    let (outside, initial_cursor, initial_scroll) = cx
        .update_window(handle.into(), |_, window, cx| {
            window.click(target(&text), cx);
            window.press(START, cx);
            window.press("right", cx);
            let from = caret_point(text.read(cx));
            let outside = point(from.x, text.read(cx).input_bounds().bottom() + px(40.));
            window.dispatch_event(
                MouseMoveEvent {
                    position: from,
                    pressed_button: None,
                    modifiers: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
            window.render_frame(cx);
            window.dispatch_event(
                MouseDownEvent {
                    position: from,
                    button: MouseButton::Left,
                    click_count: 1,
                    first_mouse: false,
                    modifiers: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
            window.render_frame(cx);
            assert_eq!(text.read(cx).cursor(), 1);
            window.dispatch_event(
                MouseMoveEvent {
                    position: outside,
                    pressed_button: Some(MouseButton::Left),
                    modifiers: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
            window.render_frame(cx);
            assert_eq!(text.read(cx).selected_range().start, 1);
            assert!(text.read(cx).cursor() < value.len(), "dragging just below the viewport must not immediately select to EOF; cursor={}, document length={}", text.read(cx).cursor(), value.len());
            (
                outside,
                text.read(cx).cursor(),
                text.read(cx).scroll_offset().y,
            )
        })
        .unwrap();
    cx.run_until_parked();
    for _ in 0..8 {
        cx.background_executor
            .advance_clock(std::time::Duration::from_millis(16));
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
            .unwrap();
    }
    let released = cx
        .update_window(handle.into(), |_, window, cx| {
            assert!(
                text.read(cx).scroll_offset().y < initial_scroll,
                "holding beyond the viewport must scroll without additional pointer moves"
            );
            assert!(
                text.read(cx).cursor() > initial_cursor,
                "the selection must follow newly revealed rows"
            );
            assert_eq!(text.read(cx).selected_range().start, 1);
            assert_eq!(text.read(cx).value(), value);
            window.dispatch_event(
                MouseUpEvent {
                    position: outside,
                    button: MouseButton::Left,
                    click_count: 1,
                    modifiers: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
            window.render_frame(cx);
            (
                text.read(cx).selected_range(),
                text.read(cx).scroll_offset(),
            )
        })
        .unwrap();
    for _ in 0..4 {
        cx.background_executor
            .advance_clock(std::time::Duration::from_millis(16));
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(text.read(cx).selected_range(), released.0);
            assert_eq!(text.read(cx).scroll_offset(), released.1);
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn shift_line_end_then_down_retains_wrapped_textarea_selection_and_caret(cx: &mut TestAppContext) {
    let value = "x".repeat(600);
    let (handle, _, text) = composer(cx, |state| {
        state.auto_grow(6, 6).default_value(value.clone())
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(target(&text), cx);
        window.press(START, cx);
        let first_row_top = text.read(cx).cursor_layout().unwrap().0.top();
        window.press("down", cx);
        assert!(text.read(cx).cursor_layout().unwrap().0.top() > first_row_top);
        window.press(LINE_END, cx);
        let end = text.read(cx).cursor();
        let end_caret = text.read(cx).cursor_layout().unwrap().0;
        // Textarea Home/End address logical lines even when soft-wrapped.
        // The visual-row End affinity regression needs an Editor fixture.
        assert_eq!(end, value.len());

        window.press(START, cx);
        for _ in 0..3 {
            window.press("right", cx);
        }
        window.press(&format!("shift-{LINE_END}"), cx);
        assert_eq!(text.read(cx).selected_range(), 3..end);
        assert_eq!(text.read(cx).cursor_layout().unwrap().0, end_caret);
        window.press("shift-down", cx);
        assert_eq!(text.read(cx).selected_range(), 3..end);
        assert_eq!(text.read(cx).cursor(), end);
        assert_eq!(text.read(cx).cursor_layout().unwrap().0, end_caret);
        assert_caret_visible(text.read(cx));
    })
    .unwrap();
}
