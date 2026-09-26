//! Styled editor workflows driven through the production window and native events.

use gpui_kit::{
    AppContext, ClipboardItem, Context, Entity, InputEvent, Modifiers, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, TestAppContext, Window, WindowHandle,
    component::input::{Editor, EditorState, FoldRange},
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

fn platform_shortcut(macos: &'static str, other: &'static str) -> &'static str {
    if cfg!(target_os = "macos") {
        macos
    } else {
        other
    }
}

fn add_cursor_above() -> &'static str {
    if cfg!(target_os = "macos") {
        "cmd-alt-up"
    } else if cfg!(target_os = "windows") {
        "ctrl-alt-up"
    } else {
        "shift-alt-up"
    }
}

fn redo_shortcut() -> &'static str {
    platform_shortcut("cmd-shift-z", "ctrl-y")
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
fn multicursor_vertical_selection_replaces_and_undoes_each_range(cx: &mut TestAppContext) {
    let value = "abcd\nabcd\nabcd\nabcd\nabcd\nabcd";
    let (handle, state) = editor(cx, "plaintext", value);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press("secondary-a", cx);
        window.press("left", cx);
        window.press("right", cx);
        window.press("down", cx);
        assert_eq!(state.read(cx).cursor(), 6);
        let first = state.read(cx).cursor_layout().unwrap().0.center();
        for _ in 0..3 {
            window.press("down", cx);
        }
        assert_eq!(state.read(cx).cursor(), 21);
        let second = state.read(cx).cursor_layout().unwrap().0.center();
        assert_eq!(state.read(cx).scroll_offset().y, px(0.));

        // Native clicks at measured carets keep the two ranges apart, so
        // merging adjacent selections cannot hide a lost secondary cursor.
        for (position, alt) in [(first, false), (second, true)] {
            let modifiers = Modifiers {
                alt,
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
                    click_count: 1,
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
                    click_count: 1,
                }
                .to_platform_input(),
                cx,
            );
            window.render_frame(cx);
        }
        // Vertical selection must honor the column established by Alt-click,
        // without a horizontal movement to initialize the secondary anchor.
        assert_eq!(state.read(cx).selected_range(), 6..6);
        assert_eq!(state.read(cx).cursor(), 6);
        window.press("shift-up", cx);
        assert_eq!(state.read(cx).selected_range(), 1..6);
        assert_eq!(state.read(cx).cursor(), 1);
        assert_eq!(
            window.find(("input", state.entity_id())).value(),
            Some(value)
        );
        window.input("X", cx);
        assert_eq!(state.read(cx).value(), "aXbcd\nabcd\naXbcd\nabcd");
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), value);
        assert_eq!(state.read(cx).selected_range(), 1..6);
        assert_eq!(state.read(cx).cursor(), 1);
        // A second replacement proves Undo restored both reversed selections.
        window.input("Y", cx);
        let expected = "aYbcd\nabcd\naYbcd\nabcd";
        assert_eq!(state.read(cx).value(), expected);
        assert_eq!(
            window.find(("input", state.entity_id())).value(),
            Some(expected)
        );
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

#[gpui_kit::test]
fn block_indent_outdent_preserves_reversed_selection_and_redo(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "plaintext", "one\n  two");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press(platform_shortcut("cmd-down", "ctrl-end"), cx);
        window.press(platform_shortcut("cmd-shift-up", "ctrl-shift-home"), cx);
        assert_eq!(state.read(cx).selected_range(), 0..9);
        assert_eq!(state.read(cx).cursor(), 0);
        window.press("secondary-]", cx);
        assert_eq!(state.read(cx).value(), "  one\n    two");
        assert_eq!(state.read(cx).selected_range(), 2..13);
        assert_eq!(state.read(cx).cursor(), 2);
        window.press("secondary-[", cx);
        assert_eq!(state.read(cx).value(), "one\n  two");
        assert_eq!(state.read(cx).selected_range(), 0..9);
        assert_eq!(state.read(cx).cursor(), 0);
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), "  one\n    two");
        assert_eq!(state.read(cx).cursor(), 2);
        window.press(redo_shortcut(), cx);
        assert_eq!(state.read(cx).value(), "one\n  two");
        assert_eq!(state.read(cx).selected_range(), 0..9);
        assert_eq!(state.read(cx).cursor(), 0);
        // Moving the head right proves history kept the selection direction.
        window.press("shift-right", cx);
        assert_eq!(state.read(cx).selected_range(), 1..9);
        assert_eq!(state.read(cx).cursor(), 1);
    })
    .unwrap();
}

#[gpui_kit::test]
fn block_indent_at_a_caret_changes_only_its_line(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "plaintext", "one\ntwo\nthree");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press(platform_shortcut("cmd-up", "ctrl-home"), cx);
        window.press("right", cx);
        window.press("down", cx);
        assert_eq!(state.read(cx).cursor(), 5);
        window.press("secondary-]", cx);
        assert_eq!(state.read(cx).value(), "one\n  two\nthree");
        assert_eq!(state.read(cx).selected_range(), 7..7);
        window.press("secondary-[", cx);
        assert_eq!(state.read(cx).value(), "one\ntwo\nthree");
        assert_eq!(state.read(cx).selected_range(), 5..5);
        window.press("secondary-[", cx);
        assert_eq!(state.read(cx).value(), "one\ntwo\nthree");
        assert_eq!(state.read(cx).cursor(), 5);
    })
    .unwrap();
}

#[gpui_kit::test]
fn word_and_document_movement_select_and_replace_across_lines(cx: &mut TestAppContext) {
    let value = "alpha beta\ngamma delta";
    let (handle, state) = editor(cx, "plaintext", value);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press(platform_shortcut("cmd-up", "ctrl-home"), cx);
        window.press(platform_shortcut("alt-right", "ctrl-right"), cx);
        assert_eq!(state.read(cx).selected_range(), 5..5);
        window.press(platform_shortcut("alt-shift-right", "ctrl-shift-right"), cx);
        assert_eq!(state.read(cx).selected_range(), 5..10);
        window.press(platform_shortcut("alt-left", "ctrl-left"), cx);
        assert_eq!(state.read(cx).selected_range(), 6..6);
        window.press(platform_shortcut("alt-shift-left", "ctrl-shift-left"), cx);
        assert_eq!(state.read(cx).selected_range(), 0..6);
        assert_eq!(state.read(cx).cursor(), 0);
        window.press(platform_shortcut("cmd-down", "ctrl-end"), cx);
        assert_eq!(state.read(cx).selected_range(), value.len()..value.len());
        window.press(platform_shortcut("cmd-shift-up", "ctrl-shift-home"), cx);
        assert_eq!(state.read(cx).selected_range(), 0..value.len());
        assert_eq!(state.read(cx).cursor(), 0);
        window.press("left", cx);
        window.press(platform_shortcut("cmd-shift-down", "ctrl-shift-end"), cx);
        assert_eq!(state.read(cx).cursor(), value.len());
        assert_eq!(state.read(cx).selected_range(), 0..value.len());
        window.input("中🦀", cx);
        assert_eq!(state.read(cx).value(), "中🦀");
        // Native input dispatches one keystroke per character. Replacing the
        // selection with 中 is atomic; typing 🦀 starts a separate typing run.
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), "中");
        assert_eq!(state.read(cx).selected_range(), 3..3);
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), value);
        assert_eq!(state.read(cx).selected_range(), 0..value.len());
        assert_eq!(state.read(cx).cursor(), value.len());
    })
    .unwrap();
}

#[gpui_kit::test]
fn page_movement_preserves_column_and_clamps_at_document_edges(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "plaintext", "");
    let value = vec!["abcd"; 100].join("\n");
    cx.update_window(handle.into(), |_, window, cx| {
        state.update(cx, |state, cx| state.set_value(value.clone(), window, cx));
        window.render_frame(cx);
        window.click(("input", state.entity_id()), cx);
        window.press(platform_shortcut("cmd-up", "ctrl-home"), cx);
        window.press("right", cx);
        window.press("right", cx);
        window.press("pagedown", cx);
        let next_page = state.read(cx).cursor();
        assert!(next_page > 7 && next_page < value.len() - 5);
        assert_eq!(next_page % 5, 2);
        assert!(state.read(cx).scroll_offset().y < px(0.));
        window.press("pageup", cx);
        assert_eq!(state.read(cx).selected_range(), 2..2);
        window.press("pageup", cx);
        assert_eq!(state.read(cx).cursor(), 2);
        window.press(platform_shortcut("cmd-down", "ctrl-end"), cx);
        window.press("left", cx);
        window.press("left", cx);
        window.press("pagedown", cx);
        assert_eq!(state.read(cx).cursor(), value.len() - 2);
        assert_eq!(state.read(cx).value(), value);
    })
    .unwrap();
}

#[gpui_kit::test]
fn gutter_fold_and_unfold_change_vertical_navigation_without_editing(cx: &mut TestAppContext) {
    let value = "start\ninside\nend\ntail";
    let (handle, state) = editor(cx, "plaintext", value);
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press(platform_shortcut("cmd-up", "ctrl-home"), cx);
        // Supply a deterministic language-provider fixture; folding itself is
        // exercised through the production gutter's native mouse handler.
        state.update(cx, |state, cx| {
            state.apply_highlighter_fold_candidates(vec![FoldRange::new(0, 2)], cx);
        });
        window.render_frame(cx);
        window.click(("fold-icon", 0usize), cx);
        window.press("down", cx);
        assert_eq!(state.read(cx).cursor(), 13);
        assert_eq!(state.read(cx).value(), value);
        window.click(("fold-icon", 0usize), cx);
        window.press(platform_shortcut("cmd-up", "ctrl-home"), cx);
        window.press("down", cx);
        assert_eq!(state.read(cx).cursor(), 6);
        window.input("!", cx);
        assert_eq!(state.read(cx).value(), "start\n!inside\nend\ntail");
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), value);
    })
    .unwrap();
}

#[gpui_kit::test]
fn multicursor_copy_cut_paste_and_redo_restore_every_selection(cx: &mut TestAppContext) {
    let value = "ab\ncd\nef";
    let (handle, state) = editor(cx, "plaintext", value);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        // Add upwards so clipboard order cannot accidentally follow creation order.
        window.press(platform_shortcut("cmd-down", "ctrl-end"), cx);
        window.press("home", cx);
        window.press(add_cursor_above(), cx);
        window.press(add_cursor_above(), cx);
        window.press("shift-right", cx);
        window.press("secondary-c", cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("a\nc\ne")
        );
        assert_eq!(state.read(cx).value(), value);
        window.press("secondary-x", cx);
        assert_eq!(state.read(cx).value(), "b\nd\nf");
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("a\nc\ne")
        );
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), value);
        window.press(redo_shortcut(), cx);
        assert_eq!(state.read(cx).value(), "b\nd\nf");
        window.press("secondary-v", cx);
        assert_eq!(state.read(cx).value(), value);
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), "b\nd\nf");
        window.press(redo_shortcut(), cx);
        assert_eq!(state.read(cx).value(), value);
        window.press("escape", cx);
        window.input("!", cx);
        assert_eq!(state.read(cx).value(), "ab\ncd\ne!f");
    })
    .unwrap();
}

#[gpui_kit::test]
fn multicursor_paste_broadcasts_when_clipboard_line_count_differs(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "plaintext", "ab\ncd\nef");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press(platform_shortcut("cmd-up", "ctrl-home"), cx);
        window.press("right", cx);
        window.press(add_cursor_below(), cx);
        window.press(add_cursor_below(), cx);
        cx.write_to_clipboard(ClipboardItem::new_string("中\n🦀".into()));
        window.press("secondary-v", cx);
        assert_eq!(state.read(cx).value(), "a中\n🦀b\nc中\n🦀d\ne中\n🦀f");
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), "ab\ncd\nef");
        window.input("!", cx);
        assert_eq!(state.read(cx).value(), "a!b\nc!d\ne!f");
    })
    .unwrap();
}

#[gpui_kit::test]
fn readonly_editor_rejects_native_mutations_and_keeps_search_available(cx: &mut TestAppContext) {
    let value = "alpha\nbeta";
    let (handle, state) = editor_with_readonly(cx, "plaintext", value, true);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press("secondary-a", cx);
        cx.write_to_clipboard(ClipboardItem::new_string("replacement".into()));
        window.input("ignored", cx);
        assert_eq!(state.read(cx).value(), value);
        for key in [
            "backspace",
            "delete",
            "enter",
            "secondary-v",
            "secondary-x",
            "secondary-]",
            "secondary-[",
            "secondary-z",
            redo_shortcut(),
        ] {
            window.press(key, cx);
            assert_eq!(state.read(cx).value(), value, "readonly {key}");
            assert_eq!(
                state.read(cx).selected_range(),
                0..value.len(),
                "readonly {key}"
            );
        }
        window.press("left", cx);
        window.press("down", cx);
        assert_eq!(state.read(cx).cursor(), 6);
        window.press("secondary-f", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("replace-mode").is_none());
        window.input("beta", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(
            state
                .read(cx)
                .search_session()
                .matcher
                .matched_ranges()
                .as_ref(),
            &[6..10]
        );
        window.press(replace_shortcut(), cx);
        assert!(!state.read(cx).search_session().replace_mode);
        assert!(window.try_find("replace-all").is_none());
        window.press("escape", cx);
        assert_eq!(
            window.find(("input", state.entity_id())).focused(),
            Some(true)
        );
        assert_eq!(state.read(cx).value(), value);
    })
    .unwrap();
}

#[gpui_kit::test]
fn search_case_toggle_recomputes_matches_and_buttons_wrap(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "plaintext", "Cat cat CAT");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press(platform_shortcut("cmd-up", "ctrl-home"), cx);
        window.press("secondary-f", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.input("cat", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(
            state
                .read(cx)
                .search_session()
                .matcher
                .matched_ranges()
                .as_ref(),
            &[0..3, 4..7, 8..11]
        );
        window.within("search-panel").click("prev", cx);
        assert_eq!(state.read(cx).search_session().matcher.current(), Some(2));
        window.within("search-panel").click("next", cx);
        assert_eq!(state.read(cx).search_session().matcher.current(), Some(0));
        window.within("search-panel").click("case-insensitive", cx);
        assert!(!state.read(cx).search_session().case_insensitive);
        assert_eq!(
            state
                .read(cx)
                .search_session()
                .matcher
                .matched_ranges()
                .as_ref(),
            &[4..7]
        );
        window.within("search-panel").click("next", cx);
        assert_eq!(state.read(cx).search_session().matcher.current(), Some(0));
        window.within("search-panel").click("case-insensitive", cx);
        assert!(state.read(cx).search_session().case_insensitive);
        assert_eq!(state.read(cx).search_session().matcher.len(), 3);
        assert_eq!(state.read(cx).value(), "Cat cat CAT");
    })
    .unwrap();
}

#[gpui_kit::test]
fn search_treats_regex_metacharacters_literally_and_handles_no_matches(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "plaintext", "a.b axb a.b");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press(platform_shortcut("cmd-up", "ctrl-home"), cx);
        window.press("secondary-f", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.input("a.b", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(
            state
                .read(cx)
                .search_session()
                .matcher
                .matched_ranges()
                .as_ref(),
            &[0..3, 8..11]
        );
        window.press("secondary-a", cx);
        window.input("[", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert!(state.read(cx).search_session().matcher.is_empty());
        window.within("search-panel").click("next", cx);
        window.within("search-panel").click("prev", cx);
        assert_eq!(state.read(cx).search_session().query, "[");
        assert_eq!(state.read(cx).value(), "a.b axb a.b");
        // Scoped dispatch verifies an observed input retains focus, without
        // assuming the panel or buttons expose focus/disabled snapshots.
        window.within("search-panel").press("enter", cx);
        window.within("search-panel").press("shift-enter", cx);
        assert_eq!(state.read(cx).search_session().matcher.current(), None);
        window.within("search-panel").press("secondary-a", cx);
        window.within("search-panel").press("backspace", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(state.read(cx).search_session().query, "");
        assert!(state.read(cx).search_session().matcher.is_empty());
        window.press("escape", cx);
        assert_eq!(state.read(cx).value(), "a.b axb a.b");
    })
    .unwrap();
}

#[gpui_kit::test]
fn replace_one_updates_unicode_match_offsets_and_undoes_atomically(cx: &mut TestAppContext) {
    let value = "🦀 cat cat 中";
    let (handle, state) = editor(cx, "plaintext", value);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press(platform_shortcut("cmd-up", "ctrl-home"), cx);
        window.press(replace_shortcut(), cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.input("cat", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(
            state
                .read(cx)
                .search_session()
                .matcher
                .matched_ranges()
                .as_ref(),
            &[5..8, 9..12]
        );
        window.press("tab", cx);
        window.input("猫", cx);
        window.within("search-panel").click("replace-one", cx);
        assert_eq!(state.read(cx).value(), "🦀 猫 cat 中");
        assert_eq!(
            state
                .read(cx)
                .search_session()
                .matcher
                .matched_ranges()
                .as_ref(),
            &[9..12]
        );
        window.within("search-panel").click("replace-one", cx);
        assert_eq!(state.read(cx).value(), "🦀 猫 猫 中");
        assert!(state.read(cx).search_session().matcher.is_empty());
        window.within("search-panel").click("replace-one", cx);
        assert_eq!(state.read(cx).value(), "🦀 猫 猫 中");
        assert!(state.read(cx).search_session().matcher.is_empty());
        window.within("search-panel").press("tab", cx);
        window.within("search-panel").click("close", cx);
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), "🦀 猫 cat 中");
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), value);
        window.press(redo_shortcut(), cx);
        assert_eq!(state.read(cx).value(), "🦀 猫 cat 中");
    })
    .unwrap();
}

#[gpui_kit::test]
fn replace_all_with_empty_replacement_preserves_unicode_suffix(cx: &mut TestAppContext) {
    let value = "🦀 cat cat 中";
    let (handle, state) = editor(cx, "plaintext", value);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press(platform_shortcut("cmd-up", "ctrl-home"), cx);
        window.press(replace_shortcut(), cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.input("cat", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(state.read(cx).search_session().matcher.len(), 2);
        window.within("search-panel").click("replace-all", cx);
        assert_eq!(state.read(cx).value(), "🦀   中");
        assert!(state.read(cx).search_session().matcher.is_empty());
        window.within("search-panel").click("close", cx);
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), value);
        window.press(redo_shortcut(), cx);
        assert_eq!(state.read(cx).value(), "🦀   中");
    })
    .unwrap();
}

#[gpui_kit::test]
fn multicursor_block_indent_outdent_is_one_history_entry(cx: &mut TestAppContext) {
    let value = "ab\ncd\nef";
    let (handle, state) = editor(cx, "plaintext", value);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press(platform_shortcut("cmd-up", "ctrl-home"), cx);
        window.press("right", cx);
        window.press(add_cursor_below(), cx);
        window.press(add_cursor_below(), cx);
        window.press("secondary-]", cx);
        assert_eq!(state.read(cx).value(), "  ab\n  cd\n  ef");
        assert_eq!(state.read(cx).cursor(), 3);
        window.press("secondary-[", cx);
        assert_eq!(state.read(cx).value(), value);
        assert_eq!(state.read(cx).cursor(), 1);
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), "  ab\n  cd\n  ef");
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), value);
        window.press(redo_shortcut(), cx);
        assert_eq!(state.read(cx).value(), "  ab\n  cd\n  ef");
        window.input("!", cx);
        assert_eq!(state.read(cx).value(), "  a!b\n  c!d\n  e!f");
    })
    .unwrap();
}

#[gpui_kit::test]
fn word_deletion_at_multiple_cursors_restores_ranges_on_undo(cx: &mut TestAppContext) {
    let value = "one two\nred fox";
    let (handle, state) = editor(cx, "plaintext", value);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press(platform_shortcut("cmd-up", "ctrl-home"), cx);
        window.press(add_cursor_below(), cx);
        window.press(platform_shortcut("alt-delete", "ctrl-delete"), cx);
        assert_eq!(state.read(cx).value(), " two\n fox");
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), value);
        window.press("end", cx);
        window.press(platform_shortcut("alt-backspace", "ctrl-backspace"), cx);
        assert_eq!(state.read(cx).value(), "one \nred ");
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), value);
        window.input("!", cx);
        assert_eq!(state.read(cx).value(), "one two!\nred fox!");
    })
    .unwrap();
}

#[gpui_kit::test]
fn unwrapped_editor_scrolls_horizontally_and_navigates_logical_lines(cx: &mut TestAppContext) {
    let (handle, state) = editor(cx, "plaintext", "");
    let first_line = "abcdefghij".repeat(80);
    let value = format!("{first_line}\nlast");
    cx.update_window(handle.into(), |_, window, cx| {
        state.update(cx, |state, cx| {
            state.set_soft_wrap(false, window, cx);
            state.set_value(value.clone(), window, cx);
        });
        window.render_frame(cx);
        window.click(("input", state.entity_id()), cx);
        window.press(platform_shortcut("cmd-up", "ctrl-home"), cx);
        window.press("end", cx);
        assert_eq!(state.read(cx).cursor(), first_line.len());
        assert!(state.read(cx).scroll_offset().x < px(0.));
        assert_eq!(state.read(cx).scroll_offset().y, px(0.));
        window.press("down", cx);
        assert_eq!(state.read(cx).cursor(), value.len());
        window.press("up", cx);
        assert_eq!(state.read(cx).cursor(), first_line.len());
        window.press("shift-home", cx);
        assert_eq!(state.read(cx).selected_range(), 0..first_line.len());
        window.press("secondary-c", cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some(first_line.as_str())
        );
        window.input("X", cx);
        assert_eq!(state.read(cx).value(), "X\nlast");
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), value);
        assert_eq!(state.read(cx).selected_range(), 0..first_line.len());
    })
    .unwrap();
}

#[gpui_kit::test]
fn open_replace_panel_tracks_live_readonly_changes(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, view) = common::open_window(cx, Some(size(px(800.), px(480.))), |window, cx| {
        cx.new(|cx| EditorFixture {
            readonly: false,
            state: cx.new(|cx| {
                EditorState::new(window, cx)
                    .language("plaintext")
                    .default_value("cat cat")
            }),
        })
    });
    let state = cx.update(|cx| view.read(cx).state.clone());
    cx.update_window(handle.into(), |_, window, cx| {
        window.click(("input", state.entity_id()), cx);
        window.press(platform_shortcut("cmd-up", "ctrl-home"), cx);
        window.press(replace_shortcut(), cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.input("cat", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(state.read(cx).search_session().matcher.len(), 2);
        window.press("tab", cx);
        window.input("dog", cx);
        assert!(window.within("search-panel").find("replace-all").visible());
        // An application may revoke edit permission while replacement has focus.
        view.update(cx, |view, cx| {
            view.readonly = true;
            cx.notify();
        });
        window.render_frame(cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(state.read(cx).search_session().open);
        assert!(!state.read(cx).search_session().replace_mode);
        assert!(window.try_find("replace-mode").is_none());
        assert!(window.try_find("replace-one").is_none());
        assert!(window.try_find("replace-all").is_none());
        assert_eq!(state.read(cx).value(), "cat cat");
        window.within("search-panel").click("next", cx);
        assert_eq!(state.read(cx).search_session().matcher.current(), Some(1));
        view.update(cx, |view, cx| {
            view.readonly = false;
            cx.notify();
        });
        window.render_frame(cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.within("search-panel").click("replace-mode", cx);
        assert!(state.read(cx).search_session().replace_mode);
        window.within("search-panel").press("secondary-a", cx);
        window.within("search-panel").input("fox", cx);
        window.within("search-panel").click("replace-all", cx);
        assert_eq!(state.read(cx).value(), "fox fox");
        window.within("search-panel").click("close", cx);
        window.press("secondary-z", cx);
        assert_eq!(state.read(cx).value(), "cat cat");
    })
    .unwrap();
}

#[gpui_kit::test]
fn shift_end_then_down_preserves_editor_visual_row_end_affinity(cx: &mut TestAppContext) {
    let value = "x".repeat(600);
    let (handle, state) = editor(cx, "plaintext", "");
    cx.update_window(handle.into(), |_, window, cx| {
        state.update(cx, |state, cx| {
            state.set_soft_wrap(true, window, cx);
            state.set_value(value.clone(), window, cx);
        });
        window.render_frame(cx);
        window.click(("input", state.entity_id()), cx);
        let start = platform_shortcut("cmd-up", "ctrl-home");
        let end = platform_shortcut("cmd-right", "end");
        let select_end = platform_shortcut("cmd-shift-right", "shift-end");
        window.press(start, cx);
        let first_row_top = state.read(cx).cursor_layout().unwrap().0.top();
        window.press(end, cx);
        let first_end = state.read(cx).cursor();
        let first_caret = state.read(cx).cursor_layout().unwrap().0;
        assert!(first_end > 3 && first_end < value.len());
        assert_eq!(first_caret.top(), first_row_top);

        // Measure the next visual row independently, starting at its left edge.
        // No font width or wrap-column constant is part of the contract.
        window.press(start, cx);
        window.press("down", cx);
        window.press(end, cx);
        let second_end = state.read(cx).cursor();
        let second_caret = state.read(cx).cursor_layout().unwrap().0;
        assert!(second_end > first_end && second_end < value.len());
        assert!(second_caret.top() > first_caret.top());

        for down in ["shift-down", "down"] {
            window.press(start, cx);
            for _ in 0..3 {
                window.press("right", cx);
            }
            window.press(select_end, cx);
            assert_eq!(state.read(cx).selected_range(), 3..first_end);
            assert_eq!(state.read(cx).cursor(), first_end);
            assert_eq!(state.read(cx).cursor_layout().unwrap().0, first_caret);
            window.press(down, cx);
            assert_eq!(state.read(cx).cursor(), second_end, "{down}");
            assert_eq!(
                state.read(cx).cursor_layout().unwrap().0,
                second_caret,
                "{down}"
            );
            let expected = if down == "shift-down" {
                3..second_end
            } else {
                second_end..second_end
            };
            assert_eq!(state.read(cx).selected_range(), expected, "{down}");
        }
        assert_eq!(state.read(cx).value(), value);
    })
    .unwrap();
}
