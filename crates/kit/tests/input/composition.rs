//! Simulated IME protocol coverage through GPUI's public ElementInputHandler.
//!
//! Focus, ordinary typing and history commands use Window events. Composition
//! calls use a public bridge to the rendered control's retained state because
//! GPUI keeps the installed platform handler private. These tests do not drive
//! a native OS IME, candidate window, or platform event translation.
use gpui_kit::{App, AppContext, InputHandler, TestAppContext, Window, test::TestWindowExt};

use super::lifecycle::mount;

fn undo(window: &mut Window, cx: &mut App) {
    window.press(
        if cfg!(target_os = "macos") {
            "cmd-z"
        } else {
            "ctrl-z"
        },
        cx,
    );
}

fn selection(
    handler: &mut dyn InputHandler,
    window: &mut Window,
    cx: &mut App,
) -> std::ops::Range<usize> {
    handler
        .selected_text_range(false, window, cx)
        .expect("focused text selection")
        .range
}

#[gpui_kit::test]
fn composition_replaces_marked_text_in_utf16_and_commits_as_one_undo_step(cx: &mut TestAppContext) {
    let (handle, fields, ids) = mount(cx);
    for (ix, id) in ids.into_iter().enumerate() {
        cx.update_window(handle.into(), |_, window, cx| {
            window.click(id.clone(), cx);
            window.input("A🦀Z", cx);
            let mut handler = fields.read(cx).handler(ix, window);
            assert_eq!(selection(&mut *handler, window, cx), 4..4);
            // Replace the astral character: UTF-16 1..3, UTF-8 1..5.
            handler.replace_and_mark_text_in_range(Some(1..3), "に🦀", Some(1..3), window, cx);
            window.render_frame(cx);
            assert_eq!(window.find(id.clone()).value(), Some("Aに🦀Z"));
            assert_eq!(handler.marked_text_range(window, cx), Some(1..4));
            assert_eq!(selection(&mut *handler, window, cx), 2..4);
            let mut adjusted = None;
            assert_eq!(
                handler.text_for_range(2..4, &mut adjusted, window, cx),
                Some("🦀".into())
            );
            assert_eq!(adjusted, Some(2..4));

            // An omitted replacement range must replace the whole preedit,
            // even when only part of it is selected.
            handler.replace_and_mark_text_in_range(None, "日本", Some(2..2), window, cx);
            window.render_frame(cx);
            assert_eq!(window.find(id.clone()).value(), Some("A日本Z"));
            assert_eq!(handler.marked_text_range(window, cx), Some(1..3));
            assert_eq!(selection(&mut *handler, window, cx), 3..3);
            handler.replace_text_in_range(None, "日本語", window, cx);
            window.render_frame(cx);
            assert_eq!(window.find(id.clone()).value(), Some("A日本語Z"));
            assert_eq!(handler.marked_text_range(window, cx), None);
            assert_eq!(selection(&mut *handler, window, cx), 4..4);

            window.input("x", cx);
            assert_eq!(window.find(id.clone()).value(), Some("A日本語xZ"));
            undo(window, cx);
            assert_eq!(window.find(id.clone()).value(), Some("A日本語Z"));
            undo(window, cx);
            assert_eq!(window.find(id.clone()).value(), Some("A🦀Z"));
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn cancelling_preedit_leaves_no_undo_entry_and_does_not_swallow_later_typing(
    cx: &mut TestAppContext,
) {
    let (handle, fields, ids) = mount(cx);
    for (ix, id) in ids.into_iter().enumerate() {
        cx.update_window(handle.into(), |_, window, cx| {
            window.click(id.clone(), cx);
            window.input("A🦀", cx);
            let mut handler = fields.read(cx).handler(ix, window);
            handler.replace_and_mark_text_in_range(None, "ni", Some(2..2), window, cx);
            assert_eq!(handler.marked_text_range(window, cx), Some(3..5));
            // An empty preedit cancels the composition; unmark alone retains
            // its text and is covered separately below.
            handler.replace_and_mark_text_in_range(None, "", None, window, cx);
            window.render_frame(cx);
            assert_eq!(window.find(id.clone()).value(), Some("A🦀"));
            assert_eq!(handler.marked_text_range(window, cx), None);
            assert_eq!(selection(&mut *handler, window, cx), 3..3);
            window.input("x", cx);
            undo(window, cx);
            assert_eq!(window.find(id.clone()).value(), Some("A🦀"));
            undo(window, cx);
            assert_eq!(window.find(id).value(), Some(""));
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn unmark_retains_preedit_and_separates_its_history_from_later_typing(cx: &mut TestAppContext) {
    let (handle, fields, ids) = mount(cx);
    for (ix, id) in ids.into_iter().enumerate() {
        cx.update_window(handle.into(), |_, window, cx| {
            window.click(id.clone(), cx);
            let mut handler = fields.read(cx).handler(ix, window);
            handler.replace_and_mark_text_in_range(None, "に", None, window, cx);
            handler.replace_and_mark_text_in_range(None, "日本", None, window, cx);
            handler.unmark_text(window, cx);
            window.render_frame(cx);
            assert_eq!(handler.marked_text_range(window, cx), None);
            assert_eq!(selection(&mut *handler, window, cx), 2..2);
            assert_eq!(window.find(id.clone()).value(), Some("日本"));
            window.input("x", cx);
            undo(window, cx);
            assert_eq!(window.find(id.clone()).value(), Some("日本"));
            undo(window, cx);
            assert_eq!(window.find(id).value(), Some(""));
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn input_handler_reports_backward_unicode_selection_direction(cx: &mut TestAppContext) {
    let (handle, fields, ids) = mount(cx);
    for (ix, id) in ids.into_iter().enumerate() {
        cx.update_window(handle.into(), |_, window, cx| {
            window.click(id, cx);
            window.input("A🦀", cx);
            window.press("shift-left", cx);
            let mut handler = fields.read(cx).handler(ix, window);
            let selected = handler.selected_text_range(false, window, cx).unwrap();
            assert_eq!(selected.range, 1..3);
            assert!(
                selected.reversed,
                "Shift-Left leaves the caret at the start of the UTF-16 selection"
            );
            window.press("right", cx);
            let selected = handler.selected_text_range(false, window, cx).unwrap();
            assert_eq!(selected.range, 3..3);
            assert!(!selected.reversed);
        })
        .unwrap();
    }
}
