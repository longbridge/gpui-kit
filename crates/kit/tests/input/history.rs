use gpui_kit::{
    AppContext, ClipboardItem, Context, Entity, TestAppContext, Window, WindowHandle,
    component::input::{Input, InputState},
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
fn combining_mark_scalar_navigation_and_deletion_round_trip(cx: &mut TestAppContext) {
    let (handle, content) = inputs(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("text", cx);
        window.input("e\u{301}x", cx);
        window.press("left", cx);
        assert_eq!(content.read(cx).text.read(cx).cursor(), "e\u{301}".len());
        // Input currently navigates Unicode scalars, not extended graphemes.
        window.press("left", cx);
        assert_eq!(content.read(cx).text.read(cx).cursor(), "e".len());
        window.press("right", cx);
        assert_eq!(content.read(cx).text.read(cx).cursor(), "e\u{301}".len());
        window.press("backspace", cx);
        assert_eq!(window.find("text").value(), Some("ex"));
        assert_eq!(content.read(cx).text.read(cx).cursor(), "e".len());
        window.press(UNDO, cx);
        assert_eq!(window.find("text").value(), Some("e\u{301}x"));
        assert_eq!(content.read(cx).text.read(cx).cursor(), "e\u{301}".len());
        window.press(REDO, cx);
        assert_eq!(window.find("text").value(), Some("ex"));
    })
    .unwrap();
}
