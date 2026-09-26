//! Styled editor workflows driven through the production window and native events.

use gpui_kit::{
    AppContext, Context, Entity, TestAppContext, Window, WindowHandle,
    component::input::{Editor, EditorState},
    div,
    prelude::*,
    px, size,
    test::TestWindowExt,
};

use crate::common;

struct EditorFixture {
    state: Entity<EditorState>,
    readonly: bool,
}

impl Render for EditorFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .child(Editor::new(&self.state).readonly(self.readonly).size_full())
    }
}

fn editor(
    cx: &mut TestAppContext,
    language: &'static str,
    value: &'static str,
) -> (WindowHandle<gpui_kit::base::Root>, Entity<EditorState>) {
    editor_with_readonly(cx, language, value, false)
}

fn editor_with_readonly(
    cx: &mut TestAppContext,
    language: &'static str,
    value: &'static str,
    readonly: bool,
) -> (WindowHandle<gpui_kit::base::Root>, Entity<EditorState>) {
    cx.update(gpui_kit::init);
    let (handle, view) = common::open_window(cx, Some(size(px(800.), px(480.))), |window, cx| {
        cx.new(|cx| EditorFixture {
            readonly,
            state: cx.new(|cx| {
                EditorState::new(window, cx)
                    .language(language)
                    .default_value(value)
            }),
        })
    });
    let state = cx.update(|cx| view.read(cx).state.clone());
    (handle, state)
}

fn add_cursor_below() -> &'static str {
    if cfg!(target_os = "macos") {
        "cmd-alt-down"
    } else if cfg!(target_os = "windows") {
        "ctrl-alt-down"
    } else {
        "shift-alt-down"
    }
}

fn replace_shortcut() -> &'static str {
    if cfg!(target_os = "macos") {
        "cmd-shift-f"
    } else {
        "ctrl-h"
    }
}

#[gpui_kit::test]
fn nested_json_pairs_skip_their_generated_closers(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "json", "");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.input("[", cx);
        assert_eq!(state.read(cx).value(), "[]");
        assert_eq!(state.read(cx).selected_range(), 1..1);

        window.input("{", cx);
        assert_eq!(state.read(cx).value(), "[{}]");
        assert_eq!(state.read(cx).selected_range(), 2..2);
        window.input("}", cx);
        assert_eq!(state.read(cx).value(), "[{}]");
        assert_eq!(state.read(cx).selected_range(), 3..3);
        window.input("]", cx);
        assert_eq!(state.read(cx).value(), "[{}]");
        assert_eq!(state.read(cx).selected_range(), 4..4);
        window.input("!", cx);
        assert_eq!(
            window.find(("input", state.entity_id())).value(),
            Some("[{}]!")
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn quoted_unicode_text_keeps_one_closer_and_a_collapsed_caret(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "json", "");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.input("\"", cx);
        assert_eq!(state.read(cx).value(), "\"\"");
        assert_eq!(state.read(cx).selected_range(), 1..1);
        window.input("中🦀", cx);
        assert_eq!(state.read(cx).value(), "\"中🦀\"");
        assert_eq!(state.read(cx).selected_range(), 8..8);
        window.input("\"", cx);
        assert_eq!(state.read(cx).value(), "\"中🦀\"");
        assert_eq!(state.read(cx).selected_range(), 9..9);
        window.input(",", cx);
        assert_eq!(state.read(cx).value(), "\"中🦀\",");
    })
    .unwrap();
}

#[gpui_kit::test]
fn plaintext_language_does_not_auto_close_or_split_brackets(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "plaintext", "");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.input("{", cx);
        assert_eq!(state.read(cx).value(), "{");
        window.input("}", cx);
        window.press("left", cx);
        window.press("enter", cx);
        assert_eq!(state.read(cx).value(), "{\n}");
        assert_eq!(state.read(cx).selected_range(), 2..2);
        window.input("\"", cx);
        assert_eq!(state.read(cx).value(), "{\n\"}");
    })
    .unwrap();
}

#[gpui_kit::test]
fn paired_backspace_and_undo_preserve_unicode_neighbors(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "json", "中 ");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press("secondary-a", cx);
        window.press("right", cx);
        window.input("[", cx);
        assert_eq!(state.read(cx).value(), "中 []");
        assert_eq!(state.read(cx).selected_range(), 5..5);
        // Navigation establishes a distinct undo boundary before deletion.
        window.press("left", cx);
        window.press("right", cx);
        window.press("backspace", cx);
        assert_eq!(state.read(cx).value(), "中 ");
        assert_eq!(state.read(cx).selected_range(), 4..4);
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), "中 []");
        assert_eq!(state.read(cx).selected_range(), 5..5);
        window.input("]", cx);
        assert_eq!(state.read(cx).value(), "中 []");
        assert_eq!(state.read(cx).selected_range(), 6..6);
    })
    .unwrap();
}

#[gpui_kit::test]
fn enter_between_braces_indents_the_body_and_retains_the_closing_line(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "json", "{}");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press("secondary-a", cx);
        window.press("left", cx);
        window.press("right", cx);
        window.press("enter", cx);
        assert_eq!(state.read(cx).value(), "{\n  \n}");
        assert_eq!(state.read(cx).selected_range(), 4..4);
        window.input("\"key\": 1", cx);
        assert_eq!(state.read(cx).value(), "{\n  \"key\": 1\n}");
        window.press("down", cx);
        window.press("end", cx);
        window.press("enter", cx);
        assert_eq!(state.read(cx).value(), "{\n  \"key\": 1\n}\n");
        assert_eq!(state.read(cx).selected_range(), 15..15);
    })
    .unwrap();
}

#[gpui_kit::test]
fn python_enter_uses_colon_indent_and_outdents_before_a_closer(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "python", "if ready:");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press("secondary-a", cx);
        window.press("right", cx);
        window.press("enter", cx);
        assert_eq!(state.read(cx).value(), "if ready:\n  ");
        assert_eq!(state.read(cx).selected_range(), 12..12);
        window.input("value)", cx);
        window.press("left", cx);
        window.press("enter", cx);
        assert_eq!(state.read(cx).value(), "if ready:\n  value\n)");
        assert_eq!(state.read(cx).selected_range(), 18..18);
    })
    .unwrap();
}

#[gpui_kit::test]
fn tab_and_shift_tab_preserve_multiline_selection_and_undo(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "rust", "one\n  two");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press("secondary-a", cx);
        assert_eq!(state.read(cx).selected_range(), 0..9);
        window.press("tab", cx);
        assert_eq!(state.read(cx).value(), "  one\n    two");
        assert_eq!(state.read(cx).selected_range(), 2..13);
        window.press("shift-tab", cx);
        assert_eq!(state.read(cx).value(), "one\n  two");
        assert_eq!(state.read(cx).selected_range(), 0..9);
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), "  one\n    two");
        assert_eq!(state.read(cx).selected_range(), 2..13);
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), "one\n  two");
        assert_eq!(state.read(cx).selected_range(), 0..9);
    })
    .unwrap();
}

#[gpui_kit::test]
fn readonly_editor_rejects_indentation_and_keeps_multiline_text_copyable(cx: &mut TestAppContext) {
    let value = "  one\n    中🦀";
    let (handle, state) = editor_with_readonly(cx, "rust", value, true);
    cx.update_window(handle.into(), |_, window, cx| {
        for key in ["tab", "shift-tab"] {
            window.click(("input", state.entity_id()), cx);
            window.press("secondary-a", cx);
            window.press(key, cx);
            assert_eq!(state.read(cx).value(), value, "read-only {key}");
            assert_eq!(state.read(cx).selected_range(), 0..value.len());
            assert_eq!(
                window.find(("input", state.entity_id())).value(),
                Some(value)
            );
        }
        window.click(("input", state.entity_id()), cx);
        window.press("secondary-a", cx);
        window.press("secondary-c", cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some(value)
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn keyboard_multicursor_replacement_undo_and_escape_keep_the_active_cursor(
    cx: &mut TestAppContext,
) {
    let (handle, state) = editor(cx, "rust", "ab\nab\nab");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press("secondary-a", cx);
        window.press("left", cx);
        window.press("right", cx);
        window.press(add_cursor_below(), cx);
        window.press(add_cursor_below(), cx);
        window.press("shift-right", cx);
        // Additional cursors do not replace the original primary selection.
        assert_eq!(state.read(cx).selected_range(), 1..2);
        window.input("X", cx);
        assert_eq!(state.read(cx).value(), "aX\naX\naX");
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), "ab\nab\nab");
        assert_eq!(state.read(cx).selected_range(), 1..2);
        // Replacing again proves undo restored all three selections.
        window.input("Y", cx);
        assert_eq!(state.read(cx).value(), "aY\naY\naY");
        window.press("escape", cx);
        window.input("!", cx);
        assert_eq!(state.read(cx).value(), "aY!\naY\naY");
        assert_eq!(state.read(cx).selected_range(), 3..3);
    })
    .unwrap();
}

#[gpui_kit::test]
fn search_keyboard_navigation_wraps_and_escape_returns_editor_focus(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "plaintext", "one two one");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press("secondary-a", cx);
        window.press("left", cx);
        window.press("secondary-f", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.within("search-panel").find("next").visible());
        window.input("one", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(state.read(cx).value(), "one two one");
        assert_eq!(state.read(cx).search_session().query, "one");
        assert_eq!(
            state
                .read(cx)
                .search_session()
                .matcher
                .matched_ranges()
                .as_ref(),
            &[0..3, 8..11]
        );
        assert_eq!(state.read(cx).search_session().matcher.current(), Some(0));
        window.press("enter", cx);
        assert_eq!(state.read(cx).search_session().matcher.current(), Some(1));
        window.press("enter", cx);
        assert_eq!(state.read(cx).search_session().matcher.current(), Some(0));
        window.press("shift-enter", cx);
        assert_eq!(state.read(cx).search_session().matcher.current(), Some(1));
        window.press("escape", cx);
        assert!(!state.read(cx).search_session().open);
        assert!(window.try_find("next").is_none());
        assert_eq!(
            window.find(("input", state.entity_id())).focused(),
            Some(true)
        );
        window.input("!", cx);
        assert_eq!(state.read(cx).value(), "!one two one");
    })
    .unwrap();
}

#[gpui_kit::test]
fn replace_overlay_tabs_to_replacement_and_replace_all_is_one_undo(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "plaintext", "cat dog cat");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press("secondary-a", cx);
        window.press("left", cx);
        window.press(replace_shortcut(), cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(state.read(cx).search_session().replace_mode);
        assert!(window.within("search-panel").find("replace-all").visible());
        window.input("cat", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(state.read(cx).search_session().query, "cat");
        assert_eq!(state.read(cx).search_session().matcher.len(), 2);
        window.press("tab", cx);
        window.input("fox", cx);
        window.press("shift-tab", cx);
        // Retyping the query proves Shift-Tab returned focus to search; if
        // focus stayed in replacement, Replace All would insert "cat".
        window.press("secondary-a", cx);
        window.input("cat", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(state.read(cx).search_session().query, "cat");
        assert_eq!(state.read(cx).value(), "cat dog cat");
        window.within("search-panel").click("replace-all", cx);
        assert_eq!(state.read(cx).value(), "fox dog fox");
        assert_eq!(state.read(cx).search_session().matcher.len(), 0);
        window.within("search-panel").click("close", cx);
        assert_eq!(
            window.find(("input", state.entity_id())).focused(),
            Some(true)
        );
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), "cat dog cat");
    })
    .unwrap();
}
