//! Application-facing multiline editing and viewport regressions.
use gpui_kit::{
    AppContext, ClipboardItem, Context, ElementId, Entity, ScrollDelta, Subscription,
    TestAppContext, Window, WindowHandle,
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
