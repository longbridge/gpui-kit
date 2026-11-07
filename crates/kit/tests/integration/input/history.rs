use gpui_kit::{
    AppContext, ClipboardItem, Context, Entity, TestAppContext, Window, WindowHandle,
    component::input::{Input, InputEvent, InputState},
    div,
    prelude::*,
    test::TestWindowExt,
};

use crate::common;

#[cfg(target_os = "macos")]
const UNDO: &str = "cmd-z";
#[cfg(not(target_os = "macos"))]
const UNDO: &str = "ctrl-z";
#[cfg(target_os = "macos")]
const REDO: &str = "cmd-shift-z";
#[cfg(not(target_os = "macos"))]
const REDO: &str = "ctrl-y";
#[cfg(target_os = "macos")]
const PASTE: &str = "cmd-v";
#[cfg(not(target_os = "macos"))]
const PASTE: &str = "ctrl-v";

struct HistoryInputs {
    text: Entity<InputState>,
    other: Entity<InputState>,
}

impl Render for HistoryInputs {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .p_4()
            .gap_4()
            .child(Input::new(&self.text).id("text").w_64())
            .child(Input::new(&self.other).id("other").w_64())
    }
}

fn inputs(cx: &mut TestAppContext) -> (WindowHandle<gpui_kit::base::Root>, Entity<HistoryInputs>) {
    cx.update(gpui_kit::init);
    common::open_window(cx, None, |window, cx| {
        cx.new(|cx| HistoryInputs {
            text: cx.new(|cx| InputState::new(window, cx)),
            other: cx.new(|cx| InputState::new(window, cx)),
        })
    })
}

#[gpui_kit::test]
fn consecutive_typing_undoes_and_redoes_as_one_group(cx: &mut TestAppContext) {
    let (handle, content) = inputs(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("text", cx);
        window.input("ab", cx);
        window.input("cd", cx);
        assert_eq!(window.find("text").value(), Some("abcd"));
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some(""));
        assert_eq!(content.read(cx).text.read(cx).cursor(), 0);
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some(""));
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("abcd"));
        assert_eq!(content.read(cx).text.read(cx).selected_range(), 4..4);
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("abcd"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn moving_away_and_back_splits_typing_groups(cx: &mut TestAppContext) {
    let (handle, content) = inputs(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("text", cx);
        window.input("ab", cx);
        window.press("left", cx);
        assert_eq!(content.read(cx).text.read(cx).cursor(), 1);
        window.press("right", cx);
        window.input("cd", cx);
        assert_eq!(window.find("text").value(), Some("abcd"));
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some("ab"));
        assert_eq!(content.read(cx).text.read(cx).cursor(), 2);
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some(""));
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("ab"));
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("abcd"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn paste_is_atomic_and_separate_from_surrounding_typing(cx: &mut TestAppContext) {
    let (handle, _) = inputs(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("text", cx);
        window.input("before", cx);
        cx.write_to_clipboard(ClipboardItem::new_string("🦀中文".to_owned()));
        window.press(PASTE, cx);
        assert_eq!(window.find("text").value(), Some("before🦀中文"));
        window.input("after", cx);
        assert_eq!(window.find("text").value(), Some("before🦀中文after"));
        for value in ["before🦀中文", "before", ""] {
            window.press(UNDO, cx);
            assert_eq!(window.find("text").value(), Some(value));
        }
        for value in ["before", "before🦀中文", "before🦀中文after"] {
            window.press(REDO, cx);
            assert_eq!(window.find("text").value(), Some(value));
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn undo_selection_replacement_restores_range_and_active_end(cx: &mut TestAppContext) {
    let (handle, content) = inputs(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("text", cx);
        window.input("abcd", cx);
        window.press("shift-left", cx);
        window.press("shift-left", cx);
        let state = content.read(cx).text.read(cx);
        assert_eq!(state.selected_range(), 2..4);
        assert_eq!(state.cursor(), 2);
        window.input("X", cx);
        assert_eq!(window.find("text").value(), Some("abX"));
        assert_eq!(content.read(cx).text.read(cx).selected_range(), 3..3);
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some("abcd"));
        let state = content.read(cx).text.read(cx);
        assert_eq!(state.selected_range(), 2..4);
        assert_eq!(state.cursor(), 2);
        assert_eq!(state.selected_value(), "cd");
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("abX"));
        assert_eq!(content.read(cx).text.read(cx).selected_range(), 3..3);
    })
    .unwrap();
}

#[gpui_kit::test]
fn blur_splits_typing_without_moving_the_caret(cx: &mut TestAppContext) {
    let (handle, content) = inputs(cx);
    let blur_count = std::rc::Rc::new(std::cell::Cell::new(0));
    let _subscription = cx.update(|cx| {
        let blur_count = blur_count.clone();
        let text = content.read(cx).text.clone();
        cx.subscribe(&text, move |_, event, _| {
            if matches!(event, InputEvent::Blur) {
                blur_count.set(blur_count.get() + 1);
            }
        })
    });
    // GPUI only delivers focus/blur callbacks for an active platform window.
    cx.update_window(handle.into(), |_, window, _| window.activate_window())
        .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("text", cx);
        window.input("ab", cx);
        // Keyboard focus traversal preserves the caret, isolating blur from
        // the transaction boundary introduced by clicking inside the text.
        window.press("tab", cx);
        assert_eq!(window.find("other").focused(), Some(true));
        assert_eq!(window.find("text").focused(), Some(false));
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(blur_count.get(), 1);
    cx.update_window(handle.into(), |_, window, cx| {
        window.press("shift-tab", cx);
        assert_eq!(window.find("text").focused(), Some(true));
        assert_eq!(content.read(cx).text.read(cx).cursor(), 2);
        window.input("cd", cx);
        assert_eq!(window.find("text").value(), Some("abcd"));
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some("ab"));
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some(""));
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("ab"));
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("abcd"));
        assert_eq!(window.find("other").value(), Some(""));
    })
    .unwrap();
}

#[gpui_kit::test]
fn backspace_at_start_preserves_redo(cx: &mut TestAppContext) {
    let (handle, content) = inputs(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("text", cx);
        window.input("abc", cx);
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some(""));
        window.press("backspace", cx);
        assert_eq!(window.find("text").value(), Some(""));
        assert_eq!(content.read(cx).text.read(cx).cursor(), 0);
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("abc"));
        assert_eq!(content.read(cx).text.read(cx).cursor(), 3);
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some(""));
    })
    .unwrap();
}

#[gpui_kit::test]
fn empty_paste_at_caret_preserves_redo(cx: &mut TestAppContext) {
    let (handle, content) = inputs(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("text", cx);
        window.input("keep", cx);
        cx.write_to_clipboard(ClipboardItem::new_string("🦀".into()));
        window.press(PASTE, cx);
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some("keep"));
        cx.write_to_clipboard(ClipboardItem::new_string(String::new()));
        window.press(PASTE, cx);
        assert_eq!(window.find("text").value(), Some("keep"));
        assert_eq!(content.read(cx).text.read(cx).selected_range(), 4..4);
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("keep🦀"));
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some("keep"));
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some(""));
    })
    .unwrap();
}

#[gpui_kit::test]
fn new_edit_after_undo_discards_the_redo_branch(cx: &mut TestAppContext) {
    let (handle, _) = inputs(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("text", cx);
        window.input("ab", cx);
        window.press("left", cx);
        window.press("right", cx);
        window.input("old", cx);
        assert_eq!(window.find("text").value(), Some("abold"));
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some("ab"));
        window.input("new", cx);
        assert_eq!(window.find("text").value(), Some("abnew"));
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("abnew"));
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some("ab"));
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("abnew"));
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("abnew"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn emoji_navigation_and_both_delete_directions_round_trip(cx: &mut TestAppContext) {
    let (handle, content) = inputs(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("text", cx);
        window.input("a🦀b", cx);
        window.press("left", cx);
        assert_eq!(content.read(cx).text.read(cx).cursor(), "a🦀".len());
        window.press("left", cx);
        assert_eq!(content.read(cx).text.read(cx).cursor(), "a".len());
        window.press("delete", cx);
        assert_eq!(window.find("text").value(), Some("ab"));
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some("a🦀b"));
        assert_eq!(content.read(cx).text.read(cx).cursor(), "a".len());
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("ab"));
        window.press(UNDO, cx);
        window.press("right", cx);
        assert_eq!(content.read(cx).text.read(cx).cursor(), "a🦀".len());
        window.press("backspace", cx);
        assert_eq!(window.find("text").value(), Some("ab"));
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some("a🦀b"));
        assert_eq!(content.read(cx).text.read(cx).cursor(), "a🦀".len());
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("ab"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn unicode_selection_deletion_restores_text_and_active_end_on_undo(cx: &mut TestAppContext) {
    let (handle, content) = inputs(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("text", cx);
        let text = "e\u{301}👩‍💻中文";
        window.input(text, cx);
        window.press("shift-home", cx);
        assert_eq!(content.read(cx).text.read(cx).selected_value(), text);
        window.press("backspace", cx);
        assert_eq!(window.find("text").value(), Some(""));
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some(text));
        let state = content.read(cx).text.read(cx);
        assert_eq!(state.selected_range(), 0..text.len());
        assert_eq!(state.selected_value(), text);
        assert_eq!(state.cursor(), 0);
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some(""));
        assert_eq!(content.read(cx).text.read(cx).selected_range(), 0..0);
    })
    .unwrap();
}

#[gpui_kit::test]
fn cut_then_paste_replacement_restore_each_selection_and_clipboard(cx: &mut TestAppContext) {
    let (handle, content) = inputs(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("text", cx);
        window.input("abcDEF", cx);
        for _ in 0..3 {
            window.press("shift-left", cx);
        }
        window.press("secondary-x", cx);
        assert_eq!(window.find("text").value(), Some("abc"));
        window.press("home", cx);
        window.press("shift-right", cx);
        window.press("shift-right", cx);
        window.press(PASTE, cx);
        assert_eq!(window.find("text").value(), Some("DEFc"));
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some("abc"));
        let state = content.read(cx).text.read(cx);
        assert_eq!(state.selected_range(), 0..2);
        assert_eq!(state.cursor(), 2);
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some("abcDEF"));
        let state = content.read(cx).text.read(cx);
        assert_eq!(state.selected_range(), 3..6);
        assert_eq!(state.cursor(), 3);
        for (value, cursor) in [("abc", 3), ("DEFc", 3)] {
            window.press(REDO, cx);
            assert_eq!(window.find("text").value(), Some(value));
            assert_eq!(
                content.read(cx).text.read(cx).selected_range(),
                cursor..cursor
            );
        }
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("DEF")
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn normalized_paste_undo_restores_unicode_selection_in_both_directions(cx: &mut TestAppContext) {
    let (handle, content) = inputs(cx);
    for reversed in [false, true] {
        cx.update_window(handle.into(), |_, window, cx| {
            window.click("text", cx);
            window.press("secondary-a", cx);
            window.input("[e\u{301}🦀]", cx);
            // Select the interior using line boundaries, excluding the ASCII
            // brackets. This isolates history from scalar/grapheme step counts.
            if reversed {
                window.press("left", cx);
                window.press("shift-home", cx);
                window.press("shift-right", cx);
            } else {
                window.press("home", cx);
                window.press("right", cx);
                window.press("shift-end", cx);
                window.press("shift-left", cx);
            }
            assert_eq!(
                content.read(cx).text.read(cx).selected_value(),
                "e\u{301}🦀"
            );
            cx.write_to_clipboard(ClipboardItem::new_string("中\r\n\t文".into()));
            window.press(PASTE, cx);
            assert_eq!(window.find("text").value(), Some("[中\t文]"));
            window.press(UNDO, cx);
            assert_eq!(window.find("text").value(), Some("[e\u{301}🦀]"));
            let state = content.read(cx).text.read(cx);
            assert_eq!(state.selected_range(), 1..8);
            assert_eq!(state.cursor(), if reversed { 1 } else { 8 });
            window.press(REDO, cx);
            assert_eq!(window.find("text").value(), Some("[中\t文]"));
            assert_eq!(content.read(cx).text.read(cx).selected_range(), 8..8);
        })
        .unwrap();
    }
}
