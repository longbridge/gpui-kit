use super::{
    input::{byte_offset, utf16_offset},
    model::{Document, Position},
};

#[test]
fn edit_ranges_keep_marks_and_unicode_boundaries() {
    let mut doc = Document::parse("**你好** world").unwrap();
    let at = Position {
        offset: "你好".len(),
        ..doc.first()
    };
    let cursor = doc.replace(at, at, "🙂", None);
    assert_eq!(doc.text(at.block), "你好🙂 world");
    assert_eq!(doc.adjacent(cursor, false), at);
    assert!(doc.source().contains("**"));
    assert_eq!(byte_offset("你🙂a", 3), 7);
    assert_eq!(utf16_offset("你🙂a", 7), 3);
}

#[test]
fn cross_block_replace_and_split_keep_the_first_identity() {
    let mut doc = Document::parse("# One\n\nTwo\n\nThree").unwrap();
    let first = doc.first();
    let last = Position {
        offset: 2,
        ..doc.last()
    };
    let cursor = doc.replace(Position { offset: 1, ..first }, last, "!", None);
    assert_eq!(doc.text(first.block), "O!ree");
    assert_eq!(doc.paragraphs().len(), 1);
    let next = doc.split(cursor);
    assert_ne!(next.block, first.block);
    assert_eq!(doc.text(next.block), "ree");
    assert!(doc.source().starts_with("# O!"));
}

#[test]
fn list_enter_creates_a_sibling_item() {
    let mut doc = Document::parse("- One\n- Two").unwrap();
    doc.split(Position {
        offset: 1,
        ..doc.first()
    });
    let source = doc.source();
    assert!(source.contains("- O"));
    assert!(source.contains("- ne"));
    assert!(source.contains("- Two"));
}

#[test]
fn unsupported_blocks_survive_text_edits() {
    let mut doc = Document::parse("Hello\n\n```rust\nlet x = 1;\n```\n\n![alt](https://example.com/a.png)\n\n| A | B |\n| - | - |\n| 1 | 2 |").unwrap();
    doc.replace(doc.first(), doc.first(), "X", None);
    let source = doc.source();
    assert!(source.contains("let x = 1;"));
    assert!(source.contains("https://example.com/a.png"));
    assert!(source.contains("| A | B |"));
}

#[test]
fn snapshots_do_not_share_mutable_document_content() {
    let mut doc = Document::parse("**before**").unwrap();
    let before = doc.clone();
    doc.replace(doc.first(), doc.last(), "after", None);
    assert_eq!(before.text(before.first().block), "before");
    assert_eq!(doc.text(doc.first().block), "after");
}

#[test]
fn toggling_format_off_preserves_text_and_other_marks() {
    let mut doc = Document::parse("**hello** *world*").unwrap();
    doc.format(
        doc.first(),
        Position {
            offset: 5,
            ..doc.first()
        },
        true,
    );
    assert!(!doc.source().contains("**hello**"));
    assert!(doc.source().contains("*world*"));
    assert_eq!(doc.text(doc.first().block), "hello world");
}

#[gpui::test]
fn ime_commit_is_one_undo_and_native_offsets_are_utf16(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::{EntityInputHandler, VisualTestContext};
    cx.update(crate::init);
    let (editor, cx) = cx.add_window_view(|_, cx| MarkdownEditorState::new("a🙂b", cx).unwrap());
    VisualTestContext::update(cx, |window, cx| {
        editor.update(cx, |state, cx| {
            state.replace_and_mark_text_in_range(Some(1..3), "你", Some(1..1), window, cx);
            assert_eq!(state.document.text(state.cursor.block), "a你b");
            state.replace_and_mark_text_in_range(None, "你好", Some(2..2), window, cx);
            state.replace_text_in_range(None, "你好", window, cx);
            assert_eq!(state.document.text(state.cursor.block), "a你好b");
            assert!(state.history.can_undo());
            state.undo(cx);
            assert_eq!(state.document.text(state.cursor.block), "a🙂b");
            state.redo(cx);
            assert_eq!(state.document.text(state.cursor.block), "a你好b");
        })
    });
}

#[gpui::test]
fn readonly_rejects_typing_formatting_and_history(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::{EntityInputHandler, VisualTestContext};
    cx.update(crate::init);
    let (editor, cx) = cx.add_window_view(|_, cx| MarkdownEditorState::new("hello", cx).unwrap());
    VisualTestContext::update(cx, |window, cx| {
        editor.update(cx, |state, cx| {
            let source = state.source();
            state.set_readonly(true, cx);
            state.replace_text_in_range(None, "changed", window, cx);
            state.replace_and_mark_text_in_range(None, "你", None, window, cx);
            state.set_heading(2, cx);
            state.undo(cx);
            assert_eq!(state.source(), source);
        })
    });
}

#[gpui::test]
fn keyboard_edits_the_rendered_document(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::VisualTestContext;
    cx.update(crate::init);
    let (editor, cx) = cx.add_window_view(|_, cx| MarkdownEditorState::new("hello", cx).unwrap());
    VisualTestContext::update(cx, |window, cx| {
        editor.update(cx, |state, cx| state.focus(window, cx));
        window.draw(cx).clear(cx);
    });
    cx.simulate_keystrokes("end enter w o r l d");
    cx.read(|cx| {
        let state = editor.read(cx);
        assert_eq!(state.document.paragraphs().len(), 2);
        assert_eq!(state.document.text(state.cursor.block), "world");
    });
}

#[gpui::test]
fn collapsed_format_can_disable_inherited_bold(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::AppContext;
    cx.update(crate::init);
    let editor = cx.new(|cx| MarkdownEditorState::new("**hello**", cx).unwrap());
    editor.update(cx, |state, cx| {
        state.place(state.document.last(), false, cx);
        state.toggle_bold(cx);
        state.insert(" plain", cx);
        assert!(!state.document.mark_at(state.cursor).bold);
        assert!(state.source().contains("**hello**"));
        state.toggle_bold(cx);
        state.insert(" bold", cx);
        assert!(state.document.mark_at(state.cursor).bold);
    });
}

#[gpui::test]
fn readonly_round_trip_preserves_selection_and_history(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::VisualTestContext;
    cx.update(crate::init);
    let (editor, cx) = cx.add_window_view(|_, cx| MarkdownEditorState::new("hello", cx).unwrap());
    VisualTestContext::update(cx, |window, cx| {
        editor.update(cx, |state, cx| {
            state.focus(window, cx);
            state.place(state.document.last(), false, cx);
            state.insert("!", cx);
            state.anchor = state.document.first();
            state.set_readonly(true, cx);
        });
        window.draw(cx).clear(cx);
    });
    cx.simulate_keystrokes("backspace enter x");
    VisualTestContext::update(cx, |window, cx| {
        editor.update(cx, |state, cx| {
            assert_eq!(state.document.text(state.cursor.block), "hello!");
            assert_eq!(
                state.document.selected(state.anchor, state.cursor),
                "hello!"
            );
            assert!(!state.active);
            state.copy(cx);
            assert_eq!(
                cx.read_from_clipboard().and_then(|item| item.text()),
                Some("hello!".to_string())
            );
            state.set_readonly(false, cx);
            state.focus(window, cx);
        });
    });
    cx.simulate_keystrokes("x");
    editor.update(cx, |state, cx| {
        assert_eq!(state.document.text(state.cursor.block), "x");
        state.undo(cx);
        assert_eq!(state.document.text(state.cursor.block), "hello!");
        state.undo(cx);
        assert_eq!(state.document.text(state.cursor.block), "hello");
    });
}

#[gpui::test]
fn readonly_transition_settles_composition_once(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::{EntityInputHandler, VisualTestContext};
    cx.update(crate::init);
    let (editor, cx) = cx.add_window_view(|_, cx| MarkdownEditorState::new("", cx).unwrap());
    VisualTestContext::update(cx, |window, cx| {
        editor.update(cx, |state, cx| {
            state.replace_and_mark_text_in_range(None, "你好", None, window, cx);
            state.set_readonly(true, cx);
            state.set_readonly(true, cx);
            state.replace_text_in_range(None, "late commit", window, cx);
            assert!(state.composition.is_none());
            assert!(state.history.can_undo());
            assert_eq!(state.document.text(state.cursor.block), "你好");
            state.set_readonly(false, cx);
            state.undo(cx);
            assert_eq!(state.document.text(state.cursor.block), "");
            state.redo(cx);
            assert_eq!(state.document.text(state.cursor.block), "你好");
        });
    });
}

#[gpui::test]
fn inline_code_fragments_map_back_to_document_positions(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::{EntityInputHandler, VisualTestContext, point, px};
    cx.update(crate::init);
    let (editor, cx) = cx.add_window_view(|_, cx| {
        MarkdownEditorState::new("# before `代码` [after](https://example.com)", cx).unwrap()
    });
    VisualTestContext::update(cx, |window, cx| {
        editor.update(cx, |state, cx| state.focus(window, cx));
        window.draw(cx).clear(cx);
        editor.update(cx, |state, cx| {
            let id = state.document.first().block;
            let mut fragments = state
                .layouts
                .iter()
                .filter(|((block, _), _)| *block == id)
                .map(|(_, layout)| layout)
                .collect::<Vec<_>>();
            fragments.sort_by_key(|layout| layout.source.start);
            assert!(
                fragments.len() >= 3,
                "inline code must use shared fragment layout"
            );
            assert_eq!(fragments.first().unwrap().source.start, 0);
            assert_eq!(
                fragments.last().unwrap().source.end,
                state.document.text(id).len()
            );
            for pair in fragments.windows(2) {
                assert_eq!(pair[0].source.end, pair[1].source.start);
            }
            let target = Position {
                block: id,
                offset: "before 代".len(),
            };
            let layout = state.layout_for(target).unwrap();
            let origin = layout
                .text
                .position_for_index(target.offset - layout.source.start)
                .unwrap();
            let hit = state
                .hit(point(
                    origin.x + px(0.1),
                    origin.y + layout.text.line_height() / 2.,
                ))
                .unwrap();
            assert_eq!(hit, target);
            state.place(hit, false, cx);
            let bounds = state
                .bounds_for_range(8..8, gpui::Bounds::default(), window, cx)
                .unwrap();
            assert!((bounds.left() - origin.x).abs() < px(1.));
            state.insert("新", cx);
            assert_eq!(state.document.text(id), "before 代新码 after");
            assert!(state.source().contains("https://example.com"));
            state.undo(cx);
            assert_eq!(state.document.first().block, id);
            assert_eq!(state.document.text(id), "before 代码 after");
        });
    });
}

#[gpui::test]
fn shared_history_discards_redo_after_a_new_edit(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::AppContext;
    cx.update(crate::init);
    let editor = cx.new(|cx| MarkdownEditorState::new("a", cx).unwrap());
    editor.update(cx, |state, cx| {
        state.place(state.document.last(), false, cx);
        state.enter(cx);
        let second = state.cursor.block;
        state.insert("b", cx);
        state.undo(cx);
        state.undo(cx);
        assert_eq!(state.document.paragraphs().len(), 1);
        state.redo(cx);
        assert_eq!(state.cursor.block, second);
        state.insert("c", cx);
        assert!(!state.history.can_redo());
        assert_eq!(state.document.text(second), "c");
    });
}

#[gpui::test]
fn table_cells_do_not_register_editable_positions(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::VisualTestContext;
    cx.update(crate::init);
    let (editor, cx) = cx.add_window_view(|_, cx| {
        MarkdownEditorState::new("text\n\n| A | B |\n| - | - |\n| 1 | 2 |", cx).unwrap()
    });
    VisualTestContext::update(cx, |window, cx| {
        window.draw(cx).clear(cx);
        editor.read_with(cx, |state, _| {
            let text_id = state.document.first().block;
            assert!(!state.layouts.is_empty());
            assert!(state.layouts.keys().all(|(id, _)| *id == text_id));
        });
    });
}

#[test]
fn list_indent_outdent_preserves_order_and_markdown_nesting() {
    for source in ["- one\n- two\n- three", "1. one\n2. two\n3. three"] {
        let mut doc = Document::parse(source).unwrap();
        let ids = doc
            .paragraphs()
            .iter()
            .map(|p| p.editor_id())
            .collect::<Vec<_>>();
        assert!(!doc.indent_list(ids[0]));
        assert!(doc.indent_list(ids[1]));
        assert!(doc.indent_list(ids[2]));
        let nested = doc.source();
        assert!(nested.contains("\n  - two") || nested.contains("\n   1. two"));
        let parsed = Document::parse(&nested).unwrap();
        assert_eq!(parsed.source(), nested);
        assert!(doc.outdent_list(ids[1]));
        assert_eq!(
            doc.paragraphs()
                .iter()
                .map(|p| p.editor_id())
                .collect::<Vec<_>>(),
            ids
        );
        // Following nested siblings become children of the promoted item.
        assert!(doc.outdent_list(ids[2]));
        assert_eq!(doc.source(), Document::parse(source).unwrap().source());
    }
}

#[gpui::test]
fn list_keyboard_shortcuts_and_ordered_continuation(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::VisualTestContext;
    cx.update(crate::init);
    let (editor, cx) = cx.add_window_view(|_, cx| MarkdownEditorState::new("", cx).unwrap());
    VisualTestContext::update(cx, |window, cx| {
        editor.update(cx, |state, cx| state.focus(window, cx));
        window.draw(cx).clear(cx);
    });
    cx.simulate_keystrokes("1 . space a enter b tab");
    editor.update(cx, |state, _| {
        assert!(state.source().contains("1. a\n   1. b"))
    });
    cx.simulate_keystrokes("shift-tab");
    editor.update(cx, |state, cx| {
        assert!(state.source().contains("1. a\n2. b"));
        state.undo(cx);
        assert!(state.source().contains("1. a\n   1. b"));
        state.redo(cx);
        assert!(state.source().contains("1. a\n2. b"));
        state.set_readonly(true, cx);
    });
    cx.simulate_keystrokes("tab shift-tab");
    editor.update(cx, |state, _| {
        assert!(state.source().contains("1. a\n2. b"))
    });
}

#[gpui::test]
fn empty_list_backspace_keeps_the_current_paragraph(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::AppContext;
    cx.update(crate::init);
    for source in ["previous\n\n- ", "previous\n\n1. "] {
        let editor = cx.new(|cx| MarkdownEditorState::new(source, cx).unwrap());
        editor.update(cx, |state, cx| {
            state.place(state.document.last(), false, cx);
            let cursor = state.cursor;
            let count = state.document.paragraphs().len();
            let before = state.source();
            state.delete(false, cx);
            assert_eq!(state.cursor, cursor);
            assert_eq!(state.document.paragraphs().len(), count);
            assert_eq!(state.document.text(cursor.block), "");
            assert_ne!(state.source(), before);
            state.undo(cx);
            assert_eq!(state.source(), before);
        });
    }
}

#[test]
fn enter_on_a_parent_list_item_keeps_its_children() {
    let mut doc = Document::parse("- parent\n  - child\n- sibling").unwrap();
    let at = Position {
        offset: 3,
        ..doc.first()
    };
    doc.split(at);
    let source = doc.source();
    assert!(source.contains("- par\n- ent\n  - child\n- sibling"));
    assert_eq!(Document::parse(&source).unwrap().source(), source);
}

#[gpui::test]
fn empty_nested_list_backspace_outdents_without_joining(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::AppContext;
    cx.update(crate::init);
    let editor = cx.new(|cx| MarkdownEditorState::new("- parent", cx).unwrap());
    editor.update(cx, |state, cx| {
        state.place(state.document.last(), false, cx);
        state.enter(cx);
        state.indent_list(cx);
        let cursor = state.cursor;
        let nested = state.source();
        state.delete(false, cx);
        assert_eq!(state.cursor, cursor);
        assert_eq!(state.document.paragraphs().len(), 2);
        assert_eq!(state.document.text(cursor.block), "");
        assert_ne!(state.source(), nested);
        state.delete(false, cx);
        assert_eq!(state.cursor, cursor);
        assert_eq!(state.document.paragraphs().len(), 2);
        assert!(!state.document.outdent_list(cursor.block));
        state.undo(cx);
        state.undo(cx);
        assert_eq!(state.source(), nested);
    });
}

#[gpui::test]
fn task_prefixes_export_standard_markdown_and_continue_unchecked(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::AppContext;
    cx.update(crate::init);
    for (prefix, checked) in [
        ("-[]", false),
        ("-[ ]", false),
        ("-[x]", true),
        ("-[X]", true),
        ("- [ ]", false),
        ("- [x]", true),
    ] {
        let editor = cx.new(|cx| MarkdownEditorState::new("", cx).unwrap());
        editor.update(cx, |state, cx| {
            // Commit each character separately, including the list's first space.
            for ch in prefix.chars() {
                state.insert(&ch.to_string(), cx);
            }
            state.insert(" ", cx);
            let id = state.cursor.block;
            assert_eq!(state.document.text(id), "");
            state.insert("任务", cx);
            let expected = if checked {
                "- [x] 任务"
            } else {
                "- [ ] 任务"
            };
            assert_eq!(state.source().trim(), expected);
            assert_eq!(
                Document::parse(&state.source()).unwrap().source(),
                state.source()
            );
            state.enter(cx);
            state.insert("下一项", cx);
            assert_eq!(state.source().trim(), format!("{expected}\n- [ ] 下一项"));
            state.undo(cx);
            state.undo(cx);
            assert_eq!(state.source().trim(), expected);
        });
    }
}

#[gpui::test]
fn task_conversion_is_undoable_and_plain_brackets_stay_text(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::AppContext;
    cx.update(crate::init);
    let editor = cx.new(|cx| MarkdownEditorState::new("", cx).unwrap());
    editor.update(cx, |state, cx| {
        state.insert("-[x]", cx);
        state.insert(" ", cx);
        assert_eq!(state.source().trim(), "- [x]");
        state.undo(cx);
        assert_eq!(state.document.text(state.cursor.block), "-[x]");
        state.redo(cx);
        assert_eq!(state.source().trim(), "- [x]");
        state.delete(false, cx);
        assert_eq!(state.source().trim(), "");
        state.insert("[x]", cx);
        state.insert(" ", cx);
        assert_eq!(state.document.text(state.cursor.block), "[x] ");
    });
}

#[gpui::test]
fn task_prefix_keyboard_input(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::VisualTestContext;
    cx.update(crate::init);
    let (editor, cx) = cx.add_window_view(|_, cx| MarkdownEditorState::new("", cx).unwrap());
    VisualTestContext::update(cx, |window, cx| {
        editor.update(cx, |state, cx| state.focus(window, cx));
        window.draw(cx).clear(cx);
    });
    cx.simulate_keystrokes("- [ x ] space a enter b");
    editor.update(cx, |state, _| {
        assert_eq!(state.source().trim(), "- [x] a\n- [ ] b")
    });
}

#[gpui::test]
fn double_click_selects_words_in_editable_and_readonly_markdown(cx: &mut gpui::TestAppContext) {
    use super::MarkdownEditorState;
    use gpui::{Modifiers, MouseButton, MouseDownEvent, VisualTestContext, point, px};
    cx.update(crate::init);
    for readonly in [false, true] {
        for (source, offset, expected) in [
            ("before **hel**lo after", 9, "hello"),
            ("before `hello` after", 9, "hello"),
            ("中文", 3, "文"),
        ] {
            let (editor, cx) =
                cx.add_window_view(|_, cx| MarkdownEditorState::new(source, cx).unwrap());
            VisualTestContext::update(cx, |window, cx| {
                editor.update(cx, |state, cx| state.set_readonly(readonly, cx));
                window.draw(cx).clear(cx);
            });
            let position = editor.read_with(cx, |state, _| {
                let at = Position {
                    offset,
                    ..state.document.first()
                };
                let layout = state.layout_for(at).unwrap();
                let origin = layout
                    .text
                    .position_for_index(offset - layout.source.start)
                    .unwrap();
                point(
                    origin.x + px(0.1),
                    origin.y + layout.text.line_height() / 2.,
                )
            });
            cx.simulate_event(MouseDownEvent {
                position,
                modifiers: Modifiers::default(),
                button: MouseButton::Left,
                click_count: 2,
                first_mouse: false,
            });
            editor.read_with(cx, |state, _| {
                assert_eq!(
                    state.document.selected(state.anchor, state.cursor),
                    expected
                )
            });
            cx.simulate_mouse_move(position, MouseButton::Left, Modifiers::default());
            cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
            editor.update(cx, |state, cx| {
                assert_eq!(
                    state.document.selected(state.anchor, state.cursor),
                    expected
                );
                state.copy(cx);
                assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap(), expected);
                if !readonly {
                    state.insert("new", cx);
                    assert!(!state.document.text(state.cursor.block).contains(expected));
                    state.undo(cx);
                    assert_eq!(
                        state.document.selected(state.anchor, state.cursor),
                        expected
                    );
                }
            });
        }
    }
}
