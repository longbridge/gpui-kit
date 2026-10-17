//! UI-slice tests: the entity, key bindings, frame seam, IME and perf
//! counters, driven through a real window in `TestAppContext`.

use gpui::{
    AppContext as _, Context, Entity, EntityInputHandler, InteractiveElement as _, IntoElement,
    Modifiers, MouseButton, ParentElement as _, Pixels, Render, StatefulInteractiveElement as _,
    Styled as _, TestAppContext, VisualTestContext, Window, div, px,
};

use crate::text::block_editor::{
    BlockEditor, BlockEditorState, BlockKind, BlockPosition, BlockSelection,
};

struct Root(Entity<BlockEditorState>);

impl Render for Root {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(600.))
            .h(px(4000.))
            .child(BlockEditor::new(&self.0))
    }
}

const M: &str = if cfg!(target_os = "macos") {
    "cmd"
} else {
    "ctrl"
};

fn open<'a>(
    cx: &'a mut TestAppContext,
    source: &str,
) -> (Entity<BlockEditorState>, &'a mut VisualTestContext) {
    cx.update(crate::init);
    let source = source.to_string();
    let (root, cx) = cx.add_window_view(move |window, cx| {
        Root(cx.new(|cx| BlockEditorState::new(window, cx).with_source(&source)))
    });
    let editor = root.read_with(cx, |root, _| root.0.clone());
    cx.update(|window, cx| {
        let focus = editor.read(cx).focus_handle.clone();
        window.focus(&focus, cx);
    });
    draw(cx);
    (editor, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        let _ = window.draw(cx);
    });
}

fn source(editor: &Entity<BlockEditorState>, cx: &VisualTestContext) -> String {
    editor.read_with(cx, |e, _| e.source())
}

fn caret_at(
    editor: &Entity<BlockEditorState>,
    cx: &mut VisualTestContext,
    leaf: usize,
    offset: usize,
) {
    editor.update(cx, |e, cx| {
        let id = e.model().document().leaves().nth(leaf).unwrap().id();
        e.set_selection(BlockSelection::caret(BlockPosition::new(id, offset)), cx)
            .unwrap();
    });
}

fn type_text(cx: &mut VisualTestContext, text: &str) {
    cx.simulate_input(text);
    draw(cx);
}

fn keys(cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(&keys.replace("M-", &format!("{M}-")));
    draw(cx);
}

#[gpui::test]
fn typing_and_enter_build_paragraphs(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "");
    type_text(cx, "hello");
    keys(cx, "enter");
    type_text(cx, "world");
    assert_eq!(source(&editor, cx), "hello\n\nworld");
    keys(cx, "home backspace");
    assert_eq!(source(&editor, cx), "helloworld");
}

#[gpui::test]
fn mark_shortcuts_toggle_stored_and_range_marks(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "ab");
    caret_at(&editor, cx, 0, 2);
    keys(cx, "M-b");
    type_text(cx, "c");
    keys(cx, "M-b M-i");
    type_text(cx, "d");
    assert_eq!(source(&editor, cx), "ab**c***d*");
    // Range toggle: select "ab" and make it code.
    keys(cx, "home shift-right shift-right M-e");
    assert_eq!(source(&editor, cx), "`ab`**c***d*");
}

#[gpui::test]
fn arrows_cross_blocks_and_vertical_uses_frame(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "# Title\n\n- one\n- two\n\n1. first");
    caret_at(&editor, cx, 0, 5);
    keys(cx, "end right");
    let (leaf, offset) = editor.read_with(cx, |e, _| {
        let caret = e.model().caret().unwrap();
        let ix = e
            .model()
            .document()
            .leaves()
            .position(|l| l.id() == caret.block)
            .unwrap();
        (ix, caret.offset)
    });
    assert_eq!((leaf, offset), (1, 0));
    keys(cx, "down down");
    let kind = editor.read_with(cx, |e, _| e.model().document().leaves().count());
    assert_eq!(kind, 4);
    let at = editor.read_with(cx, |e, _| {
        let caret = e.model().caret().unwrap();
        e.model()
            .document()
            .leaves()
            .position(|l| l.id() == caret.block)
    });
    assert_eq!(at, Some(3), "down twice from `one` lands in `first`");
    keys(cx, "up");
    let at = editor.read_with(cx, |e, _| {
        let caret = e.model().caret().unwrap();
        e.model()
            .document()
            .leaves()
            .position(|l| l.id() == caret.block)
    });
    assert_eq!(at, Some(2));
}

#[gpui::test]
fn cross_block_selection_then_typing_joins(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "abc\n\ndef\n\nghi");
    caret_at(&editor, cx, 0, 1);
    keys(cx, "shift-down shift-right");
    let ranges = editor.read_with(cx, |e, _| e.selection_ranges().len());
    assert!(ranges >= 2, "selection spans blocks");
    type_text(cx, "X");
    let text = source(&editor, cx);
    assert!(text.starts_with("aX"), "{text}");
    assert!(text.ends_with("ghi"), "{text}");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "abc\n\ndef\n\nghi");
    keys(cx, "M-shift-z");
    assert_eq!(source(&editor, cx), text);
}

#[gpui::test]
fn typing_coalesces_and_undo_redo(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "");
    type_text(cx, "abc");
    keys(cx, "left");
    type_text(cx, "X");
    assert_eq!(source(&editor, cx), "abXc");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "abc");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "");
    keys(cx, "M-shift-z M-shift-z");
    assert_eq!(source(&editor, cx), "abXc");
}

#[gpui::test]
fn ime_composition_is_one_undo_step(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "a");
    caret_at(&editor, cx, 0, 1);
    cx.update(|window, cx| {
        editor.update(cx, |e, cx| {
            e.replace_and_mark_text_in_range(None, "n", None, window, cx);
            e.replace_and_mark_text_in_range(None, "ni", None, window, cx);
            assert_eq!(e.marked_text_range(window, cx), Some(1..3));
            e.replace_text_in_range(None, "你", window, cx);
            assert_eq!(e.marked_text_range(window, cx), None);
        })
    });
    draw(cx);
    assert_eq!(source(&editor, cx), "a你");
    let utf16 =
        cx.update(|window, cx| editor.update(cx, |e, cx| e.selected_text_range(false, window, cx)));
    assert_eq!(utf16.map(|s| s.range), Some(2..2));
    let bounds = cx.update(|window, cx| {
        editor.update(cx, |e, cx| {
            e.bounds_for_range(0..1, Default::default(), window, cx)
        })
    });
    assert!(bounds.is_some());
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "a");
}

#[gpui::test]
fn list_and_heading_commands(cx: &mut TestAppContext) {
    use crate::text::block_editor::EditorCommand;
    let (editor, cx) = open(cx, "one\n\ntwo");
    editor.update(cx, |e, cx| {
        e.run(EditorCommand::ToggleList { ordered: false }, cx)
            .unwrap();
    });
    draw(cx);
    assert_eq!(source(&editor, cx), "- one\n\ntwo");
    caret_at(&editor, cx, 1, 0);
    editor.update(cx, |e, cx| {
        e.run(
            EditorCommand::SetBlockKind(BlockKind::Heading { level: 2 }),
            cx,
        )
        .unwrap();
    });
    draw(cx);
    assert_eq!(source(&editor, cx), "- one\n\n## two");
    // The list leaf is indented in the frame; the heading is not.
    editor.read_with(cx, |e, _| {
        let frame = e.frame.borrow();
        assert!(frame.leaves[0].bounds().left() > frame.leaves[1].bounds().left());
        assert!(frame.leaves[1].line_height() > frame.leaves[0].line_height());
    });
}

#[gpui::test]
fn mouse_click_and_drag_select(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "first block\n\nsecond block");
    let (a, b) = editor.read_with(cx, |e, _| {
        let f = e.frame.borrow();
        let l0 = &f.leaves[0];
        let l1 = &f.leaves[1];
        (
            l0.bounds().origin + gpui::point(px(1.), l0.line_height() / 2.),
            l1.bounds().origin + gpui::point(px(1000.), l1.line_height() / 2.),
        )
    });
    cx.simulate_mouse_down(a, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(b, Some(MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(b, MouseButton::Left, Modifiers::none());
    draw(cx);
    type_text(cx, "Z");
    assert_eq!(source(&editor, cx), "Z");
}

/// Perf (D9): steady frames rebuild nothing; a keystroke re-projects one
/// leaf, even in a long document.
#[gpui::test]
fn long_document_reprojects_one_leaf_per_keystroke(cx: &mut TestAppContext) {
    let mut doc = String::new();
    for i in 0..1200 {
        match i % 4 {
            0 => doc.push_str(&format!("## Heading {i}\n\n")),
            1 => doc.push_str(&format!("- item {i} with **bold** and `code`\n\n")),
            2 => doc.push_str(&format!("1. ordered {i}\n\n")),
            _ => doc.push_str(&format!(
                "Paragraph {i} with *italic* text that wraps a little.\n\n"
            )),
        }
    }
    let (editor, cx) = open(cx, &doc);
    let leaves = editor.read_with(cx, |e, _| e.model().document().leaves().count());
    assert!(leaves >= 1000, "{leaves}");
    let before = editor.read_with(cx, |e, _| (e.projection.built(), e.projection.rebuilds));
    draw(cx);
    let frame = std::time::Instant::now();
    cx.update(|window, cx| {
        window.refresh();
        let _ = window.draw(cx);
    });
    let frame = frame.elapsed();
    let steady = editor.read_with(cx, |e, _| (e.projection.built(), e.projection.rebuilds));
    assert_eq!(before, steady, "steady frames do not re-project");
    caret_at(&editor, cx, 500, 0);
    let t = std::time::Instant::now();
    type_text(cx, "x");
    let elapsed = t.elapsed();
    let after = editor.read_with(cx, |e, _| (e.projection.built(), e.projection.rebuilds));
    assert_eq!(after.0 - steady.0, 1, "one node re-projected");
    assert_eq!(after.1 - steady.1, 1);
    eprintln!(
        "perf: {leaves} leaves, full frame {frame:?}, keystroke+frame {elapsed:?} ({} build, test platform)",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
}

fn events(
    editor: &Entity<BlockEditorState>,
    cx: &mut VisualTestContext,
) -> std::rc::Rc<std::cell::RefCell<usize>> {
    let count = std::rc::Rc::new(std::cell::RefCell::new(0));
    let c = count.clone();
    cx.update(|_, cx| {
        cx.subscribe(
            editor,
            move |_, event: &crate::text::block_editor::BlockEditorEvent, _| {
                if matches!(
                    event,
                    crate::text::block_editor::BlockEditorEvent::Changed { .. }
                ) {
                    *c.borrow_mut() += 1;
                }
            },
        )
        .detach();
    });
    count
}

/// Review P1: an empty composition must not leave an undo group open.
#[gpui::test]
fn empty_ime_mark_does_not_leak_undo_group(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "a");
    caret_at(&editor, cx, 0, 1);
    cx.update(|window, cx| {
        editor.update(cx, |e, cx| {
            e.replace_and_mark_text_in_range(None, "", None, window, cx);
            assert!(!e.composition_open());
        })
    });
    type_text(cx, "b");
    keys(cx, "enter");
    type_text(cx, "c");
    assert_eq!(source(&editor, cx), "ab\n\nc");
    keys(cx, "M-z");
    assert_ne!(
        source(&editor, cx),
        "ab\n\nc",
        "undo works after an empty mark"
    );
}

#[gpui::test]
fn cancelled_ime_restores_text_and_closes_group(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "ab");
    caret_at(&editor, cx, 0, 2);
    cx.update(|window, cx| {
        editor.update(cx, |e, cx| {
            e.replace_and_mark_text_in_range(None, "n", None, window, cx);
            e.replace_and_mark_text_in_range(None, "ni", None, window, cx);
            e.replace_and_mark_text_in_range(None, "", None, window, cx);
            assert!(!e.composition_open());
            assert_eq!(e.marked_text_range(window, cx), None);
        })
    });
    draw(cx);
    assert_eq!(source(&editor, cx), "ab");
    type_text(cx, "c");
    assert_eq!(source(&editor, cx), "abc");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "ab");
}

#[gpui::test]
fn ime_over_cross_block_selection(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "abc\n\ndef");
    editor.update(cx, |e, cx| {
        let ids: Vec<_> = e.model().document().leaves().map(|l| l.id()).collect();
        e.set_selection(
            BlockSelection::Text {
                anchor: BlockPosition::new(ids[0], 1),
                head: BlockPosition::new(ids[1], 2),
            },
            cx,
        )
        .unwrap();
    });
    cx.update(|window, cx| {
        editor.update(cx, |e, cx| {
            e.replace_and_mark_text_in_range(None, "x", None, window, cx);
            assert_eq!(e.marked_text_range(window, cx), Some(1..2));
            e.replace_text_in_range(None, "X", window, cx);
            assert!(!e.composition_open());
        })
    });
    draw(cx);
    assert_eq!(source(&editor, cx), "aXf");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "abc\n\ndef");
}

fn caret_text_and_kind(
    editor: &Entity<BlockEditorState>,
    cx: &VisualTestContext,
) -> (String, Option<BlockKind>) {
    editor.read_with(cx, |e, _| {
        let c = e.model().caret().unwrap();
        let leaf = e.model().document().block(c.block).unwrap();
        (
            leaf.text().unwrap_or("").to_string(),
            e.block_kind_at_caret(),
        )
    })
}

#[gpui::test]
fn input_rules_make_headings_and_lists(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "");
    type_text(cx, "## ");
    type_text(cx, "Title");
    keys(cx, "enter");
    type_text(cx, "- ");
    type_text(cx, "item");
    assert_eq!(source(&editor, cx), "## Title\n\n- item");
}

#[gpui::test]
fn ordered_list_input_rule(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "");
    type_text(cx, "1. ");
    type_text(cx, "first");
    assert_eq!(source(&editor, cx), "1. first");
}

#[gpui::test]
fn undo_reverts_input_rule_but_keeps_typed_marker(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "");
    type_text(cx, "# ");
    assert_eq!(
        caret_text_and_kind(&editor, cx),
        (String::new(), Some(BlockKind::Heading { level: 1 }))
    );
    keys(cx, "M-z");
    assert_eq!(
        caret_text_and_kind(&editor, cx),
        ("# ".to_string(), Some(BlockKind::Paragraph))
    );
}

#[gpui::test]
fn no_change_event_on_noop_command(cx: &mut TestAppContext) {
    use crate::text::block_editor::EditorCommand;
    let (editor, cx) = open(cx, "plain");
    let count = events(&editor, cx);
    editor.update(cx, |e, cx| {
        let _ = e.run(EditorCommand::Outdent, cx);
    });
    assert_eq!(*count.borrow(), 0);
    editor.update(cx, |e, cx| {
        e.run(EditorCommand::ToggleList { ordered: false }, cx)
            .unwrap();
    });
    assert_eq!(*count.borrow(), 1);
}

#[gpui::test]
fn arrows_collapse_whole_block_selection(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "one\n\ntwo\n\nthree");
    editor.update(cx, |e, cx| {
        let ids: Vec<_> = e
            .model()
            .document()
            .root()
            .children()
            .map(|b| b.id())
            .collect();
        e.set_selection(
            BlockSelection::Blocks {
                anchor: ids[0],
                head: ids[1],
            },
            cx,
        )
        .unwrap();
    });
    keys(cx, "right");
    let caret = editor.read_with(cx, |e, _| {
        let c = e.model().caret().unwrap();
        let ix = e
            .model()
            .document()
            .leaves()
            .position(|l| l.id() == c.block);
        (ix, c.offset)
    });
    assert_eq!(caret, (Some(1), 3));
}

#[gpui::test]
fn backspace_and_delete_join_blocks(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "ab\n\ncd\n\nef");
    caret_at(&editor, cx, 1, 0);
    keys(cx, "backspace");
    assert_eq!(source(&editor, cx), "abcd\n\nef");
    keys(cx, "end delete");
    assert_eq!(source(&editor, cx), "abcdef");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "abcd\n\nef");
}

#[gpui::test]
fn paste_multiline_splits_blocks(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "ab");
    caret_at(&editor, cx, 0, 1);
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("X\nY".into()));
    keys(cx, "M-v");
    assert_eq!(source(&editor, cx), "aX\n\nYb");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "ab");
}

#[gpui::test]
fn readonly_blocks_edits_but_not_navigation(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "ab\n\ncd");
    editor.update(cx, |e, cx| e.set_readonly(true, cx));
    caret_at(&editor, cx, 0, 2);
    type_text(cx, "x");
    keys(cx, "backspace enter M-b");
    cx.update(|window, cx| {
        editor.update(cx, |e, cx| {
            e.replace_and_mark_text_in_range(None, "n", None, window, cx)
        })
    });
    assert_eq!(source(&editor, cx), "ab\n\ncd");
    keys(cx, "right");
    let ix = editor.read_with(cx, |e, _| {
        let c = e.model().caret().unwrap();
        e.model()
            .document()
            .leaves()
            .position(|l| l.id() == c.block)
    });
    assert_eq!(ix, Some(1));
}

const TABLE_DOC: &str = "a\n\n| x | y |\n| --- | --- |\n| 1 | 2 |\n\nb";

/// Window point of `offset` in the `ix`-th document leaf (cells included).
fn leaf_point(
    editor: &Entity<BlockEditorState>,
    cx: &mut VisualTestContext,
    ix: usize,
    offset: usize,
) -> gpui::Point<gpui::Pixels> {
    draw(cx);
    editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().nth(ix).unwrap().id();
        let b = e
            .frame
            .borrow()
            .caret_bounds(BlockPosition::new(id, offset), px(1.))
            .expect("leaf laid out");
        gpui::point(b.origin.x + px(1.), b.center().y)
    })
}

#[gpui::test]
fn table_cells_click_type_tab_and_cell_selection(cx: &mut TestAppContext) {
    // Leaves: a, x, y, 1, 2, b.
    let (editor, cx) = open(cx, TABLE_DOC);
    let original = source(&editor, cx);
    // Click in cell "1": a caret, not a node selection.
    let at = leaf_point(&editor, cx, 3, 1);
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    assert_eq!(caret_leaf(&editor, cx), Some(3));
    type_text(cx, "0");
    keys(cx, "M-b");
    type_text(cx, "B");
    assert!(
        source(&editor, cx).contains("| 10**B** | 2 |"),
        "{}",
        source(&editor, cx)
    );
    // Arrows cross cell boundaries: right into "2", up into "y", down back.
    keys(cx, "right");
    assert_eq!(caret_leaf(&editor, cx), Some(4));
    keys(cx, "up");
    assert_eq!(caret_leaf(&editor, cx), Some(2));
    keys(cx, "down");
    assert_eq!(caret_leaf(&editor, cx), Some(4));
    // Shift+Tab / Tab move between cells; Tab in the last cell adds a row.
    keys(cx, "shift-tab");
    assert_eq!(caret_leaf(&editor, cx), Some(3));
    keys(cx, "tab tab");
    let s = source(&editor, cx);
    assert!(s.contains("| 10**B** | 2 |\n|  |  |"), "{s}");
    type_text(cx, "n");
    assert!(source(&editor, cx).contains("| n |  |"));
    keys(cx, "M-z M-z M-z M-z M-z");
    assert_eq!(source(&editor, cx), original);
    // Drag from "x" to "2": a cell selection; copy gives a table.
    let from = leaf_point(&editor, cx, 1, 0);
    let to = leaf_point(&editor, cx, 4, 1);
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    assert!(editor.read_with(cx, |e, _| matches!(
        e.model().selection(),
        BlockSelection::Cells { .. }
    )));
    assert_eq!(editor.update(cx, |e, _| e.selection_ranges().len()), 4);
    keys(cx, "M-c");
    let copied = cx.read_from_clipboard().and_then(|c| c.text()).unwrap();
    assert_eq!(copied, "| x | y |\n| --- | --- |\n| 1 | 2 |");
    // Delete clears contents and keeps the table.
    keys(cx, "backspace");
    assert_eq!(
        source(&editor, cx),
        "a\n\n|  |  |\n| --- | --- |\n|  |  |\n\nb"
    );
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), original);
}

#[gpui::test]
fn backspace_after_table_selects_it_and_copy_round_trips(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, TABLE_DOC);
    let original = source(&editor, cx);
    let last = editor.read_with(cx, |e, _| e.model().document().leaves().count() - 1);
    caret_at(&editor, cx, last, 0);
    keys(cx, "backspace");
    assert_eq!(
        source(&editor, cx),
        original,
        "first Backspace only selects"
    );
    keys(cx, "M-c");
    let copied = cx.read_from_clipboard().and_then(|c| c.text()).unwrap();
    assert!(copied.starts_with("| x | y |"), "{copied:?}");
    assert!(original.contains(&copied));
    keys(cx, "M-x");
    assert_eq!(source(&editor, cx), "a\n\nb");
}

/// Per-node cache (§7.3): an edit inside a 1000-item list rebuilds only the
/// path from the edited paragraph to the list (paragraph, item, list).
#[gpui::test]
fn edit_inside_big_list_rebuilds_only_the_item_path(cx: &mut TestAppContext) {
    let doc: String = (0..1000).map(|i| format!("- item {i}\n")).collect();
    let (editor, cx) = open(cx, &doc);
    draw(cx);
    let frame = std::time::Instant::now();
    cx.update(|window, cx| {
        window.refresh();
        let _ = window.draw(cx);
    });
    let frame = frame.elapsed();
    let steady = editor.read_with(cx, |e, _| e.projection.built());
    caret_at(&editor, cx, 500, 0);
    let t = std::time::Instant::now();
    type_text(cx, "x");
    let elapsed = t.elapsed();
    let after = editor.read_with(cx, |e, _| e.projection.built());
    assert_eq!(after - steady, 3, "paragraph + item + list");
    assert!(source(&editor, cx).contains("- xitem 500\n"));
    eprintln!(
        "perf: 1000-item list, full frame {frame:?}, keystroke+frame {elapsed:?} ({} build, test platform)",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
}

#[gpui::test]
fn table_rows_are_cached_per_row(cx: &mut TestAppContext) {
    let (editor, cx) = open(
        cx,
        "| a | b |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |\n\nafter",
    );
    draw(cx);
    let steady = editor.read_with(cx, |e, _| e.projection.built());
    // Leaves: a, b, 1, 2, 3, 4, after. Typing into cell "3" rebuilds the
    // table and that one row.
    caret_at(&editor, cx, 4, 0);
    type_text(cx, "x");
    assert!(
        source(&editor, cx).contains("| x3 | 4 |"),
        "{}",
        source(&editor, cx)
    );
    let after = editor.read_with(cx, |e, _| e.projection.built());
    assert_eq!(after - steady, 2, "table + one row; other rows reused");
}

fn caret_leaf(editor: &Entity<BlockEditorState>, cx: &VisualTestContext) -> Option<usize> {
    editor.read_with(cx, |e, _| {
        let c = e.model().caret()?;
        e.model()
            .document()
            .leaves()
            .position(|l| l.id() == c.block)
    })
}

/// Culling (light virtualization): only blocks near the viewport are laid
/// out, and up/down still walks every block, including culled ones.
#[gpui::test]
fn culled_blocks_are_navigable(cx: &mut TestAppContext) {
    let doc: String = (0..600).map(|i| format!("paragraph {i}\n\n")).collect();
    let doc = doc.trim_end().to_string();
    let (editor, cx) = open(cx, &doc);
    draw(cx);
    draw(cx);
    let rendered = editor.read_with(cx, |e, _| e.cull.borrow().rendered);
    assert!(rendered < 200, "laid out {rendered} of 600");
    caret_at(&editor, cx, 0, 0);
    for _ in 0..450 {
        keys(cx, "down");
    }
    assert_eq!(caret_leaf(&editor, cx), Some(450));
    draw(cx);
    // The caret's block is laid out and has geometry.
    assert!(editor.read_with(cx, |e, _| {
        let c = e.model().caret().unwrap();
        e.frame.borrow().leaf(c.block).is_some()
    }));
    for _ in 0..450 {
        keys(cx, "up");
    }
    assert_eq!(caret_leaf(&editor, cx), Some(0));
    // Typing far down edits the right block.
    caret_at(&editor, cx, 599, 0);
    type_text(cx, "x");
    assert!(source(&editor, cx).ends_with("xparagraph 599"));
}

/// A long top-level list is culled in chunks; up/down crosses culled items
/// and ordered numbering continues across chunks.
#[gpui::test]
fn long_list_is_culled_in_chunks_and_navigable(cx: &mut TestAppContext) {
    let doc: String = (0..1000)
        .map(|i| format!("{}. item {i}\n", i + 1))
        .collect();
    let (editor, cx) = open(cx, &doc);
    draw(cx);
    draw(cx);
    let rendered = editor.read_with(cx, |e, _| e.cull.borrow().rendered);
    assert!(rendered < 10, "laid out {rendered} chunks of ~32");
    caret_at(&editor, cx, 0, 0);
    for _ in 0..300 {
        keys(cx, "down");
    }
    assert_eq!(caret_leaf(&editor, cx), Some(300));
    type_text(cx, "x");
    assert!(source(&editor, cx).contains("301. xitem 300\n"));
}

struct Scrolled(Entity<BlockEditorState>, gpui::ScrollHandle);

impl Render for Scrolled {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("scroll")
            .w(px(600.))
            .h(px(300.))
            .overflow_y_scroll()
            .track_scroll(&self.1)
            .child(BlockEditor::new(&self.0))
    }
}

/// With a scroll handle, moving the caret into a culled block scrolls it
/// into view once the block is laid out.
#[gpui::test]
fn caret_is_scrolled_into_view(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let doc: String = (0..400).map(|i| format!("paragraph {i}\n\n")).collect();
    let handle = gpui::ScrollHandle::new();
    let scroll = handle.clone();
    let (root, cx) = cx.add_window_view(move |window, cx| {
        let editor = cx.new(|cx| {
            let mut state = BlockEditorState::new(window, cx).with_source(&doc);
            state.set_scroll_handle(scroll.clone());
            state
        });
        Scrolled(editor, scroll)
    });
    let editor = root.read_with(cx, |root, _| root.0.clone());
    cx.update(|window, cx| {
        let focus = editor.read(cx).focus_handle.clone();
        window.focus(&focus, cx);
    });
    draw(cx);
    draw(cx);
    let rendered = editor.read_with(cx, |e, _| e.cull.borrow().rendered);
    assert!(rendered < 100, "laid out {rendered} of 400");
    caret_at(&editor, cx, 0, 0);
    for _ in 0..200 {
        keys(cx, "down");
    }
    for _ in 0..4 {
        draw(cx);
    }
    assert_eq!(caret_leaf(&editor, cx), Some(200));
    let (caret, view) = editor.read_with(cx, |e, _| {
        let c = e.model().caret().unwrap();
        (e.frame.borrow().caret_bounds(c, px(1.)), handle.bounds())
    });
    let caret = caret.expect("caret block laid out");
    assert!(
        caret.top() >= view.top() - px(1.) && caret.bottom() <= view.bottom() + px(1.),
        "caret {caret:?} outside view {view:?}"
    );
    assert!(handle.offset().y < px(0.));
}

/// Phase 5 element-state reuse: a block culled while scrolled away keeps
/// its projected node and its shaped text, so scrolling back reshapes
/// nothing and rebuilds no node.
#[gpui::test]
fn scrolled_back_blocks_reuse_nodes_and_layouts(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let doc: String = (0..400).map(|i| format!("paragraph {i}\n\n")).collect();
    let handle = gpui::ScrollHandle::new();
    let scroll = handle.clone();
    let (root, cx) = cx.add_window_view(move |window, cx| {
        let editor = cx.new(|cx| {
            let mut state = BlockEditorState::new(window, cx).with_source(&doc);
            state.set_scroll_handle(scroll.clone());
            state
        });
        Scrolled(editor, scroll)
    });
    let editor = root.read_with(cx, |root, _| root.0.clone());
    for _ in 0..3 {
        draw(cx);
    }
    let built = editor.read_with(cx, |e, _| e.projection.built());
    // Scroll far down (the top blocks get culled), then back to the top.
    handle.set_offset(gpui::point(px(0.), px(-6000.)));
    for _ in 0..3 {
        draw(cx);
    }
    let culled_top = editor.read_with(cx, |e, _| {
        // Block 1 (block 0 holds the caret and stays pinned).
        let second = e.model().document().leaves().nth(1).unwrap().id();
        e.frame.borrow().leaf(second).is_none()
    });
    assert!(culled_top, "the top blocks are culled while scrolled away");
    handle.set_offset(gpui::point(px(0.), px(0.)));
    crate::text::inline::LAYOUT_REUSE.with(|c| c.set((0, 0)));
    draw(cx);
    let (reused, shaped) = crate::text::inline::LAYOUT_REUSE.with(|c| c.get());
    assert!(reused > 0, "scrolled-back blocks were laid out");
    assert_eq!(shaped, 0, "no block was shaped again ({reused} reused)");
    assert_eq!(
        editor.read_with(cx, |e, _| e.projection.built()),
        built,
        "no node rebuilt"
    );
}

#[gpui::test]
fn copied_table_pastes_back_as_a_table(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, TABLE_DOC);
    let original = source(&editor, cx);
    let last = editor.read_with(cx, |e, _| e.model().document().leaves().count() - 1);
    caret_at(&editor, cx, last, 0);
    keys(cx, "backspace");
    keys(cx, "M-x");
    assert_eq!(source(&editor, cx), "a\n\nb");
    keys(cx, "M-v");
    assert_eq!(source(&editor, cx), original, "cut + paste round-trips");
    // Pasting again at the end of "b" appends a second copy.
    let last = editor.read_with(cx, |e, _| e.model().document().leaves().count() - 1);
    caret_at(&editor, cx, last, 1);
    keys(cx, "M-v");
    assert!(
        source(&editor, cx).ends_with("\n\nb\n\n| x | y |\n| --- | --- |\n| 1 | 2 |"),
        "{}",
        source(&editor, cx)
    );
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), original, "paste is one undo step");
}

fn paste(cx: &mut VisualTestContext, text: &str, plain: bool) {
    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.into()));
    keys(cx, if plain { "M-shift-v" } else { "M-v" });
}

#[gpui::test]
fn paste_converts_multi_block_markdown(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "xy");
    caret_at(&editor, cx, 0, 1);
    paste(cx, "**p1**\n\n# H\n\n- a\n- b\n\np2", false);
    assert_eq!(source(&editor, cx), "x**p1**\n\n# H\n\n- a\n- b\n\np2y");
    type_text(cx, "Z");
    assert_eq!(source(&editor, cx), "x**p1**\n\n# H\n\n- a\n- b\n\np2Zy");
    keys(cx, "M-z");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "xy", "paste is one undo step");
    // Structure only (no paragraph at either end).
    paste(cx, "# H\n\n- a", false);
    assert_eq!(source(&editor, cx), "x\n\n# H\n\n- a\n\ny");
}

#[gpui::test]
fn single_line_and_plain_paste_stay_inline(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "xy");
    caret_at(&editor, cx, 0, 1);
    paste(cx, "# not a heading", false);
    assert_eq!(source(&editor, cx), "x# not a headingy");
    keys(cx, "M-z");
    paste(cx, "# H\n\n- a", true);
    let src = source(&editor, cx);
    assert!(
        !src.contains("\n# H"),
        "plain paste keeps markers as text: {src:?}"
    );
    // The blank line of the pasted text is an empty paragraph, kept in the
    // source (canonical source fidelity).
    assert_eq!(src, "x# H\n\n\n\n\\- ay", "markers stay text");
}

/// Spacers are sized from top-to-next-top distances, so a block far down
/// sits at the same y whether the blocks above it are culled or not.
#[gpui::test]
fn culled_spacers_keep_block_positions(cx: &mut TestAppContext) {
    let doc: String = (0..300)
        .map(|i| match i % 3 {
            0 => format!("## Heading {i}\n\n"),
            1 => format!("- item {i}\n\n---\n\n"),
            _ => format!("paragraph {i}\n\n"),
        })
        .collect();
    let (editor, cx) = open(cx, &doc);
    // A text leaf well down the document (rules are atoms with no layout).
    let (target, id) = editor.read_with(cx, |e, _| {
        let doc = &e.model().doc;
        doc.leaves()
            .iter()
            .enumerate()
            .skip(250)
            .find(|(_, id)| doc.inline(**id).is_some())
            .map(|(ix, id)| (ix, *id))
            .unwrap()
    });
    caret_at(&editor, cx, target, 0);
    draw(cx);
    // A frame with culling off (no visible span yet) lays out everything.
    editor.update(cx, |e, cx| {
        e.cull.borrow_mut().visible = None;
        cx.notify();
    });
    cx.update(|window, cx| {
        let _ = window.draw(cx);
    });
    let full = editor.read_with(cx, |e, _| {
        e.frame.borrow().leaf(id).map(|l| l.bounds().top())
    });
    let full = full.expect("unculled frame");
    draw(cx);
    draw(cx);
    let (rendered, culled) = editor.read_with(cx, |e, _| {
        (
            e.cull.borrow().rendered,
            e.frame.borrow().leaf(id).map(|l| l.bounds().top()),
        )
    });
    assert!(rendered < 300, "culling active ({rendered})");
    let culled = culled.expect("caret block pinned");
    assert!((culled - full).abs() < px(0.5), "{culled:?} vs {full:?}");
}

#[gpui::test]
fn end_at_soft_wrap_stays_on_the_visual_line(cx: &mut TestAppContext) {
    let long = "word ".repeat(60);
    let (editor, cx) = open(cx, long.trim_end());
    caret_at(&editor, cx, 0, 0);
    draw(cx);
    keys(cx, "end");
    let (line_top, caret_top, offset) = editor.read_with(cx, |e, _| {
        let f = e.frame.borrow();
        let c = e.model().caret().unwrap();
        let leaf = f.leaf(c.block).unwrap();
        (
            leaf.caret_point(0).y,
            leaf.caret_point(c.offset).y,
            c.offset,
        )
    });
    assert!(offset > 0 && offset < long.trim_end().len(), "{offset}");
    assert_eq!(caret_top, line_top, "End stays on the first visual line");
    keys(cx, "home");
    assert_eq!(caret_leaf(&editor, cx), Some(0));
    assert_eq!(
        editor.read_with(cx, |e, _| e.model().caret().unwrap().offset),
        0
    );
}

/// With the default (editor-owned) scroll handle, caret-into-view is
/// consumed without a scroll owner (no panic, flag cleared).
#[gpui::test]
fn reveal_without_scroll_handle_is_a_noop(cx: &mut TestAppContext) {
    let doc: String = (0..400).map(|i| format!("p {i}\n\n")).collect();
    let (editor, cx) = open(cx, doc.trim_end());
    caret_at(&editor, cx, 399, 0);
    draw(cx);
    draw(cx);
    // The editor owns a default scroll handle (host-contract surface), so the
    // request is consumed against it; an owner may replace it.
    assert!(editor.read_with(cx, |e, _| !e.reveal.get()));
}

#[gpui::test]
fn code_fence_input_rule_and_code_editing(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "");
    type_text(cx, "```rust");
    keys(cx, "enter");
    assert_eq!(source(&editor, cx), "```rust\n\n```");
    type_text(cx, "fn a() {");
    keys(cx, "enter tab");
    type_text(cx, "x");
    keys(cx, "enter");
    type_text(cx, "}");
    assert_eq!(source(&editor, cx), "```rust\nfn a() {\n    x\n}\n```");
    // The renderer draws the code (highlighted) and reports its layout.
    draw(cx);
    assert!(editor.read_with(cx, |e, _| {
        let c = e.model().caret().unwrap();
        e.frame.borrow().caret_bounds(c, px(1.)).is_some()
    }));
    // IME inside code.
    cx.update(|window, cx| {
        editor.update(cx, |e, cx| {
            e.replace_and_mark_text_in_range(None, "ni", None, window, cx);
            e.replace_and_mark_text_in_range(None, "你", None, window, cx);
            e.replace_text_in_range(None, "你", window, cx);
        })
    });
    assert_eq!(source(&editor, cx), "```rust\nfn a() {\n    x\n}你\n```");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "```rust\nfn a() {\n    x\n}\n```");
    // Arrow down past the end leaves the block.
    keys(cx, "down");
    type_text(cx, "after");
    assert_eq!(
        source(&editor, cx),
        "```rust\nfn a() {\n    x\n}\n```\n\nafter"
    );
}

#[gpui::test]
fn exiting_code_block_then_typing_is_two_undo_steps(cx: &mut TestAppContext) {
    for exit in ["enter enter", "down"] {
        let (editor, cx) = open(cx, "```\ncode\n```");
        let id = editor.read_with(cx, |e, _| {
            e.model().document().leaves().next().unwrap().id()
        });
        editor.update(cx, |e, _| {
            e.model
                .set_selection(BlockSelection::caret(BlockPosition::new(id, 4)))
                .unwrap()
        });
        keys(cx, exit);
        type_text(cx, "after");
        assert_eq!(source(&editor, cx), "```\ncode\n```\n\nafter", "{exit}");
        keys(cx, "M-z");
        assert!(
            !source(&editor, cx).contains("after"),
            "{exit}: typing undone first"
        );
        assert_eq!(
            editor.read_with(cx, |e, _| e.model().document().leaves().count()),
            2,
            "{exit}"
        );
        keys(cx, "M-z");
        assert_eq!(
            editor.read_with(cx, |e, _| e.model().document().leaves().count()),
            1,
            "{exit}"
        );
    }
}

#[gpui::test]
fn code_block_highlighter_is_applied(cx: &mut TestAppContext) {
    use std::sync::{Arc, Mutex};
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let (editor, cx) = open(cx, "```rust\nfn a\n```");
    let log = seen.clone();
    cx.update(|_, cx| {
        crate::TextViewDefaults::new()
            .with_code_block_highlighter(move |block| {
                log.lock().unwrap().push(block.code().to_string());
                vec![(
                    0..2,
                    gpui::HighlightStyle {
                        color: Some(gpui::red()),
                        ..Default::default()
                    },
                )]
            })
            .install(cx)
    });
    editor.update(cx, |_, cx| cx.notify());
    draw(cx);
    assert!(
        seen.lock().unwrap().iter().any(|c| c == "fn a"),
        "the editor's code block is highlighted: {:?}",
        seen.lock().unwrap()
    );
    // Editing re-highlights the new code.
    let id = editor.read_with(cx, |e, _| {
        e.model().document().leaves().next().unwrap().id()
    });
    editor.update(cx, |e, _| {
        e.model
            .set_selection(BlockSelection::caret(BlockPosition::new(id, 4)))
            .unwrap()
    });
    type_text(cx, "()");
    draw(cx);
    assert!(seen.lock().unwrap().iter().any(|c| c == "fn a()"));
}

#[gpui::test]
fn code_fence_with_space_converts(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "");
    type_text(cx, "``` ");
    type_text(cx, "code");
    assert_eq!(source(&editor, cx), "```\ncode\n```");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "```\n\n```");
    keys(cx, "M-z");
    assert_eq!(
        source(&editor, cx),
        "\\`\\`\\` ",
        "undo restores the typed fence"
    );
}

/// Review fix: typing over a node-selected table replaces it with a
/// paragraph holding the text, as one undo step.
#[gpui::test]
fn typing_over_selected_table_replaces_it(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, TABLE_DOC);
    let original = source(&editor, cx);
    let last = editor.read_with(cx, |e, _| e.model().document().leaves().count() - 1);
    caret_at(&editor, cx, last, 0);
    keys(cx, "backspace");
    type_text(cx, "z");
    assert_eq!(source(&editor, cx), "a\n\nz\n\nb");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), original, "one undo step");
    keys(cx, "M-shift-z");
    assert_eq!(source(&editor, cx), "a\n\nz\n\nb", "redo replays both");
}

/// Phase 4d (#3113 semantics): an inline atom is one editing unit.
#[gpui::test]
fn inline_atom_is_one_unit(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "a![pic](x.png)b");
    caret_at(&editor, cx, 0, 1);
    // Shift+Right selects the whole atom; copy gives its Markdown.
    keys(cx, "shift-right");
    let (anchor, head) = editor.read_with(cx, |e, _| match e.model().selection() {
        BlockSelection::Text { anchor, head } => (anchor.offset, head.offset),
        _ => panic!("text selection"),
    });
    assert_eq!((anchor, head), (1, 4));
    keys(cx, "M-c");
    let copied = cx.read_from_clipboard().and_then(|c| c.text()).unwrap();
    assert_eq!(copied, "![pic](x.png)");
    // Right moves over it in one step; Backspace removes it whole; undo restores.
    caret_at(&editor, cx, 0, 1);
    keys(cx, "right");
    assert_eq!(
        editor.read_with(cx, |e, _| e.model().caret().unwrap().offset),
        4
    );
    keys(cx, "backspace");
    assert_eq!(source(&editor, cx), "ab");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "a![pic](x.png)b");
    // Typing next to it never merges into it.
    caret_at(&editor, cx, 0, 4);
    type_text(cx, "c");
    assert_eq!(source(&editor, cx), "a![pic](x.png)cb");
    // Cut of a selection holding it, then paste, round-trips.
    caret_at(&editor, cx, 0, 0);
    keys(cx, "shift-right shift-right shift-right M-x");
    assert_eq!(source(&editor, cx), "b");
    keys(cx, "M-v");
    assert_eq!(source(&editor, cx), "a![pic](x.png)cb");
}

/// Paragraphs with inline code render through the single-`Inline` path in
/// the editor, so they report a layout (caret, clicks, up/down).
#[gpui::test]
fn inline_code_paragraph_is_laid_out(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "a `b` c");
    draw(cx);
    let at = editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().next().unwrap().id();
        let f = e.frame.borrow();
        let leaf = f
            .leaf(id)
            .expect("inline-code paragraph reports its layout");
        let p = leaf.caret_point(3);
        gpui::point(p.x + px(0.5), p.y + leaf.line_height() / 2.)
    });
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    assert_eq!(
        editor.read_with(cx, |e, _| e.model().caret().unwrap().offset),
        3
    );
}

/// Window point at the middle of the inline atom starting at `offset` in the
/// first leaf.
fn atom_point(
    editor: &Entity<BlockEditorState>,
    cx: &mut VisualTestContext,
    offset: usize,
) -> gpui::Point<gpui::Pixels> {
    draw(cx);
    editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().next().unwrap().id();
        let f = e.frame.borrow();
        let leaf = f.leaf(id).expect("mixed paragraph reports its pieces");
        let l = leaf.caret_point(offset);
        let r = leaf.caret_point(offset + 3);
        gpui::point((l.x + r.x) / 2., l.y + leaf.line_height() / 2.)
    })
}

#[gpui::test]
fn image_paragraph_has_caret_clicks_and_atom_selection(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "ab![pic](x.png)cd\n\nnext");
    draw(cx);
    // Laid out: carets before and after the atom are distinct, text after
    // the image is right of it.
    let (before, after, end) = editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().next().unwrap().id();
        let f = e.frame.borrow();
        let leaf = f.leaf(id).expect("mixed paragraph reports its pieces");
        (
            leaf.caret_point(2),
            leaf.caret_point(5),
            leaf.caret_point(7),
        )
    });
    assert!(
        after.x >= before.x && end.x > after.x,
        "{before:?} {after:?} {end:?}"
    );
    // Click in the text after the image places the caret there.
    let at = gpui::point(end.x - px(1.), end.y + px(2.));
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    let caret = editor.read_with(cx, |e, _| e.model().caret().unwrap());
    assert_eq!(caret_leaf(&editor, cx), Some(0));
    assert!(caret.offset >= 6, "{caret:?}");
    // Down goes to the next paragraph, up comes back into the mixed one.
    keys(cx, "down");
    assert_eq!(caret_leaf(&editor, cx), Some(1));
    keys(cx, "up");
    assert_eq!(caret_leaf(&editor, cx), Some(0));
    // A click on the image selects it whole.
    assert!(after.x > before.x, "the image has width");
    {
        let at = atom_point(&editor, cx, 2);
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
        let sel = editor.read_with(cx, |e, _| e.model().selection().clone());
        let BlockSelection::Text { anchor, head } = sel else {
            panic!("{sel:?}")
        };
        assert_eq!((anchor.offset, head.offset), (2, 5));
        // Selection paints on the text pieces too.
        keys(cx, "shift-right shift-right");
        assert!(!editor.update(cx, |e, _| e.selection_ranges()).is_empty());
    }
}

#[gpui::test]
fn image_only_paragraph_is_laid_out(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "a\n\n![pic](x.png)\n\nb");
    draw(cx);
    let laid_out = editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().nth(1).unwrap().id();
        e.frame.borrow().leaf(id).is_some()
    });
    assert!(laid_out);
    caret_at(&editor, cx, 0, 1);
    keys(cx, "down");
    assert_eq!(caret_leaf(&editor, cx), Some(1));
    keys(cx, "down");
    assert_eq!(caret_leaf(&editor, cx), Some(2));
}

/// Inline code keeps its pill (InlineFlow) in the editor, and the caret
/// geometry follows the padded text.
#[gpui::test]
fn inline_code_keeps_pill_and_caret_geometry(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "ab `cd` ef");
    draw(cx);
    let (pieces, b, c, d) = editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().next().unwrap().id();
        let f = e.frame.borrow();
        let leaf = f.leaf(id).unwrap();
        (
            leaf.pieces.len(),
            leaf.caret_point(3),
            leaf.caret_point(4),
            leaf.caret_point(5),
        )
    });
    assert!(pieces > 1, "rendered by the flow, in fragments");
    assert!(b.x < c.x && c.x < d.x, "{b:?} {c:?} {d:?}");
    // Clicking between `c` and `d` puts the caret at 4.
    let at = gpui::point(c.x, c.y + px(3.));
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    assert_eq!(
        editor.read_with(cx, |e, _| e.model().caret().unwrap().offset),
        4
    );
    type_text(cx, "X");
    assert_eq!(source(&editor, cx), "ab `cXd` ef");
}

/// Phase 5: frames without an edit reuse the cached cull units and height
/// index (no per-frame walk of every block); an edit rebuilds them once.
#[gpui::test]
fn cull_units_are_cached_per_revision(cx: &mut TestAppContext) {
    let doc: String = (0..2000).map(|i| format!("paragraph {i}\n\n")).collect();
    let (editor, cx) = open(cx, &doc);
    draw(cx);
    draw(cx);
    let rebuilds =
        |cx: &mut VisualTestContext| editor.read_with(cx, |e, _| e.cull.borrow().index.rebuilds);
    let steady = rebuilds(cx);
    caret_at(&editor, cx, 1, 0);
    keys(cx, "down down right");
    draw(cx);
    assert_eq!(rebuilds(cx), steady, "selection changes do not rebuild");
    type_text(cx, "x");
    draw(cx);
    assert_eq!(rebuilds(cx), steady + 1, "one rebuild per edit");
    let rendered = editor.read_with(cx, |e, _| e.cull.borrow().rendered);
    assert!(rendered < 400, "culled: {rendered} of 2000 laid out");
}

/// Phase 5 accessibility: per-block roles and the D8 root value.
#[gpui::test]
fn accessibility_roles_and_cached_value(cx: &mut TestAppContext) {
    use gpui::Role;
    let (editor, cx) = open(
        cx,
        "# Title\n\ntext ![i](x.png)\n\n- a\n\n```\ncode\n```\n\n---",
    );
    draw(cx);
    let roles: Vec<_> = editor.update(cx, |e, _| {
        e.snapshot()
            .blocks
            .iter()
            .flat_map(|b| b.nodes.iter())
            .map(|n| super::super::element::block_role(n))
            .collect()
    });
    assert_eq!(
        roles,
        vec![
            Some((Role::Heading, Some(1))),
            Some((Role::Paragraph, None)),
            Some((Role::List, None)),
            Some((Role::Code, None)),
            Some((Role::Splitter, None)),
        ]
    );
    let value = editor.read_with(cx, |e, _| e.accessible_value());
    assert_eq!(value.as_ref(), "Title\ntext \na\ncode");
    // Cached for the current revision.
    let cached = editor.read_with(cx, |e, _| {
        e.a11y_value.borrow().as_ref().map(|(rev, _)| *rev) == Some(e.model().revision())
    });
    assert!(cached);
    caret_at(&editor, cx, 0, 5);
    type_text(cx, "!");
    let edited = editor.read_with(cx, |e, _| e.accessible_value());
    assert!(edited.starts_with("Title!\n"));
}

fn leaf_text(editor: &Entity<BlockEditorState>, cx: &VisualTestContext, ix: usize) -> String {
    editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().nth(ix).unwrap().id();
        e.model.doc.leaf_text(id).unwrap_or_default().to_string()
    })
}

fn type_chars(cx: &mut VisualTestContext, text: &str) {
    for ch in text.chars() {
        type_text(cx, &ch.to_string());
    }
}

#[gpui::test]
fn inline_mark_input_rules(cx: &mut TestAppContext) {
    for (typed, expected) in [
        ("a `x`", "a `x`"),
        ("a **b**", "a **b**"),
        ("a *i*", "a *i*"),
        ("a ~~s~~", "a ~~s~~"),
    ] {
        let (editor, cx) = open(cx, "");
        type_chars(cx, typed);
        assert_eq!(source(&editor, cx), expected, "{typed}");
        let marked = &expected[2..];
        let plain = marked.trim_matches(['`', '*', '~']);
        assert_eq!(
            leaf_text(&editor, cx, 0),
            format!("a {plain}"),
            "delimiters dropped"
        );
        // Typing on is outside the span.
        type_text(cx, "z");
        assert_eq!(source(&editor, cx), format!("{expected}z"), "{typed}");
        // Undo: first the "z", then the rule (literal delimiters come back).
        keys(cx, "M-z");
        assert_eq!(source(&editor, cx), expected);
        keys(cx, "M-z");
        assert_eq!(leaf_text(&editor, cx, 0), typed, "undo restores {typed}");
    }
}

#[gpui::test]
fn inline_mark_rules_do_not_fire_in_code_or_on_padding(cx: &mut TestAppContext) {
    // Inside an inline code span: literal.
    let (editor, cx) = open(cx, "`ab`");
    caret_at(&editor, cx, 0, 1);
    type_chars(cx, "*x*");
    assert_eq!(source(&editor, cx), "`a*x*b`");
    // Space-padded and half-bold: no rule.
    let (editor, cx) = open(cx, "");
    type_chars(cx, "2 * 3 *");
    assert_eq!(leaf_text(&editor, cx, 0), "2 * 3 *");
    let (editor, cx) = open(cx, "");
    type_chars(cx, "**a*");
    assert_eq!(leaf_text(&editor, cx, 0), "**a*");
    type_text(cx, "*");
    assert_eq!(source(&editor, cx), "**a**");
}

#[gpui::test]
fn inline_code_shortcut_button_paste_and_marks(cx: &mut TestAppContext) {
    // Cmd/Ctrl-E over a selection.
    let (editor, cx) = open(cx, "a x b");
    editor.update(cx, |e, cx| {
        let id = e.model().document().leaves().next().unwrap().id();
        e.set_selection(
            BlockSelection::Text {
                anchor: BlockPosition::new(id, 2),
                head: BlockPosition::new(id, 3),
            },
            cx,
        )
        .unwrap();
    });
    keys(cx, "M-e");
    assert_eq!(source(&editor, cx), "a `x` b");
    // The story toolbar button runs the same command.
    editor.update(cx, |e, cx| {
        e.run(
            crate::text::block_editor::EditorCommand::ToggleMark(
                crate::text::block_editor::MarkKind::Code,
            ),
            cx,
        )
        .unwrap();
    });
    assert_eq!(source(&editor, cx), "a x b");
    // Bold over code leaves the code alone.
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "a `x` b");
    keys(cx, "M-a M-b");
    assert_eq!(source(&editor, cx), "**a** `x` **b**");
    // It renders through the inline-code pill path (flow fragments).
    let pieces = editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().next().unwrap().id();
        e.frame.borrow().leaf(id).map(|l| l.pieces.len())
    });
    assert!(pieces.unwrap() > 1);
    // Paste of `x` is code, and round-trips.
    let (editor, cx) = open(cx, "");
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("`x`".into()));
    keys(cx, "M-v");
    assert_eq!(source(&editor, cx), "`x`");
    assert_eq!(leaf_text(&editor, cx, 0), "x");
}

/// QA: empty paragraphs, list items, headings and table cells get a caret,
/// sized to the font (ascent + descent), not the full line box.
#[gpui::test]
fn empty_leaves_have_a_font_sized_caret(cx: &mut TestAppContext) {
    for (src, keys_) in [
        ("a", "end enter"),
        ("- a", "end enter"),
        ("# a", "end backspace"),
        ("| x | y |\n| --- | --- |\n|  | 2 |", ""),
    ] {
        let (editor, cx) = open(cx, src);
        caret_at(&editor, cx, if src.starts_with('|') { 2 } else { 0 }, 0);
        if !keys_.is_empty() {
            keys(cx, keys_);
        }
        draw(cx);
        let (caret, line) = editor.read_with(cx, |e, _| {
            let c = e.model().caret().unwrap();
            let f = e.frame.borrow();
            let leaf = f.leaf(c.block).expect("empty leaf is laid out");
            assert_eq!(e.model().document().leaves().count() > 0, true);
            (f.caret_bounds(c, px(1.)).unwrap(), leaf.bounds())
        });
        assert!(caret.size.height > px(0.), "{src}");
        assert!(
            caret.size.height < line.size.height,
            "{src}: caret {caret:?} line {line:?}"
        );
        assert!(line.contains(&caret.center()), "{src}");
    }
}

/// QA: a click anywhere in an empty cell's area puts the caret in that cell,
/// and Backspace after a table selects the table node, then deletes it.
#[gpui::test]
fn empty_cell_click_and_table_node_backspace(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "| x | y |\n| --- | --- |\n|  |  |\n\nb");
    let (cell, at) = editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().nth(3).unwrap().id();
        let b = e.frame.borrow().leaf(id).unwrap().bounds();
        (id, gpui::point(b.left() + px(2.), b.center().y))
    });
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    draw(cx);
    assert_eq!(
        editor.read_with(cx, |e, _| e.model().caret().map(|c| c.block)),
        Some(cell)
    );
    caret_at(&editor, cx, 4, 0);
    keys(cx, "backspace");
    assert!(editor.read_with(cx, |e, _| matches!(
        e.model().selection(),
        BlockSelection::Blocks { .. }
    )));
    assert!(
        source(&editor, cx).contains("| x |"),
        "first Backspace only selects"
    );
    keys(cx, "backspace");
    assert_eq!(source(&editor, cx), "b");
}

/// QA: the block handle. A click in the left edge of a table or code block
/// (left of its text) selects it as a node; Backspace then deletes it.
#[gpui::test]
fn left_edge_click_node_selects_table_and_code_block(cx: &mut TestAppContext) {
    for src in [
        "a\n\n| x | y |\n| --- | --- |\n| 1 | 2 |\n\nb",
        "a\n\n```\ncode\n```\n\nb",
    ] {
        let (editor, cx) = open(cx, src);
        let at = editor.read_with(cx, |e, _| {
            let id = e.model().document().leaves().nth(1).unwrap().id();
            let b = e.frame.borrow().leaf(id).unwrap().bounds();
            gpui::point(b.left() - px(4.), b.center().y)
        });
        assert!(at.x > px(0.), "{src}: the edge is inside the editor");
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
        draw(cx);
        assert!(
            editor.read_with(cx, |e, _| matches!(
                e.model().selection(),
                BlockSelection::Blocks { .. }
            )),
            "{src}"
        );
        keys(cx, "backspace");
        assert_eq!(source(&editor, cx), "a\n\nb", "{src}");
    }
}

#[gpui::test]
fn double_and_triple_click_select_word_and_block(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "hello brave world");
    let at = editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().next().unwrap().id();
        let p = e.frame.borrow().leaf(id).unwrap().caret_point(8);
        gpui::point(p.x, p.y + px(4.))
    });
    cx.simulate_event(gpui::MouseDownEvent {
        position: at,
        button: MouseButton::Left,
        click_count: 2,
        ..Default::default()
    });
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    keys(cx, "M-c");
    assert_eq!(
        cx.read_from_clipboard().and_then(|c| c.text()).unwrap(),
        "brave"
    );
    cx.simulate_event(gpui::MouseDownEvent {
        position: at,
        button: MouseButton::Left,
        click_count: 3,
        ..Default::default()
    });
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    keys(cx, "M-c");
    assert_eq!(
        cx.read_from_clipboard().and_then(|c| c.text()).unwrap(),
        "hello brave world"
    );
}

#[gpui::test]
fn word_motion_and_word_selection(cx: &mut TestAppContext) {
    let w = if cfg!(target_os = "macos") {
        "alt"
    } else {
        "ctrl"
    };
    let (editor, cx) = open(cx, "hello brave world");
    caret_at(&editor, cx, 0, 0);
    keys(cx, &format!("{w}-right"));
    assert_eq!(
        editor.read_with(cx, |e, _| e.model().caret().unwrap().offset),
        5
    );
    keys(cx, &format!("{w}-shift-right M-b"));
    assert_eq!(source(&editor, cx), "hello **brave** world");
    keys(cx, &format!("end {w}-left"));
    assert_eq!(
        editor.read_with(cx, |e, _| e.model().caret().unwrap().offset),
        12
    );
}

/// QA bold report: typing `**bold** and `code` and *it* end` one character
/// at a time through the platform input path (`replace_text_in_range` with
/// the range the platform read back), after a paragraph, after a heading,
/// and into an emptied heading.
#[gpui::test]
fn inline_rules_through_the_platform_input_path(cx: &mut TestAppContext) {
    let typed = "**bold** and `code` and *it* end";
    let want = "**bold** and `code` and *it* end";
    for (src, leaf, off, prep, expected) in [
        (
            "Type here.\n\n## Lists\n\n- Bullet one",
            0,
            10,
            "end enter",
            format!("Type here.\n\n{want}\n\n## Lists\n\n- Bullet one"),
        ),
        (
            "Type here.\n\n## Lists\n\n- Bullet one",
            1,
            5,
            "end enter",
            format!("Type here.\n\n## Lists\n\n{want}\n\n- Bullet one"),
        ),
        (
            "# Title\n\n## x\n\nUndo",
            1,
            1,
            "backspace",
            format!("# Title\n\n## {want}\n\nUndo"),
        ),
    ] {
        let (editor, cx) = open(cx, src);
        caret_at(&editor, cx, leaf, off);
        keys(cx, prep);
        for ch in typed.chars() {
            cx.update(|window, cx| {
                editor.update(cx, |e, cx| {
                    let r = e.selected_text_range(false, window, cx).map(|s| s.range);
                    e.replace_text_in_range(r, &ch.to_string(), window, cx)
                })
            });
            draw(cx);
        }
        assert_eq!(source(&editor, cx), expected);
    }
}

fn ime_type(editor: &Entity<BlockEditorState>, cx: &mut VisualTestContext, typed: &str) {
    for ch in typed.chars() {
        cx.update(|window, cx| {
            editor.update(cx, |e, cx| {
                let r = e.selected_text_range(false, window, cx).map(|s| s.range);
                e.replace_text_in_range(r, &ch.to_string(), window, cx)
            })
        });
        draw(cx);
    }
}

#[gpui::test]
fn d13_default_block_and_link_rules(cx: &mut TestAppContext) {
    for (typed, then, expected) in [
        ("[ ] todo", "", "Start\n\n- [ ] todo"),
        ("[x] done", "", "Start\n\n- [x] done"),
        ("- [ ] dash", "", "Start\n\n- [ ] dash"),
        ("> quoted", "", "Start\n\n> quoted"),
        ("---", "enter", "Start\n\n---"),
        (
            "see [docs](https://x.dev) now",
            "",
            "Start\n\nsee [docs](https://x.dev) now",
        ),
    ] {
        let (editor, cx) = open(cx, "Start");
        caret_at(&editor, cx, 0, 5);
        keys(cx, "end enter");
        ime_type(&editor, cx, typed);
        if !then.is_empty() {
            keys(cx, then);
        }
        assert_eq!(source(&editor, cx).trim_end(), expected, "typing {typed:?}");
    }
}

#[gpui::test]
fn d13_each_rule_is_one_undo_step(cx: &mut TestAppContext) {
    for (typed, literal) in [("[x] ", "\\[x\\]"), ("> ", "\\>"), ("[a](u)", "\\[a\\](u)")] {
        let (editor, cx) = open(cx, "Start");
        caret_at(&editor, cx, 0, 5);
        keys(cx, "end enter");
        ime_type(&editor, cx, typed);
        keys(cx, &format!("{M}-z"));
        let got = source(&editor, cx);
        assert_eq!(
            got.trim_end(),
            format!("Start\n\n{literal}"),
            "undo {typed:?}"
        );
    }
}

#[gpui::test]
fn d13_rules_can_be_disabled(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "Start");
    editor.update(cx, |e, _| e.set_input_rules([]));
    caret_at(&editor, cx, 0, 5);
    keys(cx, "end enter");
    ime_type(&editor, cx, "# **b** ");
    let got = source(&editor, cx);
    assert!(!got.contains("\n# "), "{got:?}");
    assert!(
        got.contains("\\*\\*b\\*\\*") || got.contains("\\*\\*b"),
        "{got:?}"
    );
}

#[gpui::test]
fn d14_set_task_checked_is_one_undo_step(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "- [ ] a");
    caret_at(&editor, cx, 0, 1);
    let leaf = editor.read_with(cx, |e, _| e.model.caret().unwrap().block);
    editor.update(cx, |e, cx| e.set_task_checked(leaf, true, cx));
    assert_eq!(source(&editor, cx).trim_end(), "- [x] a");
    keys(cx, &format!("{M}-z"));
    assert_eq!(source(&editor, cx).trim_end(), "- [ ] a");
}

#[gpui::test]
fn d14_task_and_link_round_trip(cx: &mut TestAppContext) {
    for src in [
        "- [ ] open\n- [x] done\n  - [ ] nested",
        "1. [x] first\n2. [ ] second",
        "see [docs](https://x.dev) and [t](https://u \"Title\")",
    ] {
        let (editor, cx) = open(cx, src);
        assert_eq!(source(&editor, cx).trim_end(), src, "round trip");
    }
    // D15: an unmodelled construct stays literal text and is escaped on save.
    let (editor, cx) = open(cx, "> [x] not a task");
    assert_eq!(source(&editor, cx).trim_end(), "> \\[x\\] not a task");
    {}
}

fn event_log(
    editor: &Entity<BlockEditorState>,
    cx: &mut VisualTestContext,
) -> std::rc::Rc<std::cell::RefCell<Vec<crate::text::block_editor::BlockEditorEvent>>> {
    let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let l = log.clone();
    cx.update(|_, cx| {
        cx.subscribe(
            editor,
            move |_, event: &crate::text::block_editor::BlockEditorEvent, _| {
                l.borrow_mut().push(event.clone());
            },
        )
        .detach();
    });
    log
}

fn first_line_point(
    editor: &Entity<BlockEditorState>,
    cx: &VisualTestContext,
    leaf: usize,
    dx: Pixels,
) -> gpui::Point<Pixels> {
    editor.read_with(cx, |e, _| {
        let f = e.frame.borrow();
        let l = &f.leaves[leaf];
        l.bounds().origin + gpui::point(dx, l.line_height() / 2.)
    })
}

#[gpui::test]
fn d14_click_task_checkbox_toggles_in_one_undo_step(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "- [ ] a\n- [x] b");
    let p = first_line_point(&editor, cx, 0, px(-10.));
    cx.simulate_click(p, Modifiers::none());
    draw(cx);
    assert_eq!(source(&editor, cx).trim_end(), "- [x] a\n- [x] b");
    let p = first_line_point(&editor, cx, 1, px(-10.));
    cx.simulate_click(p, Modifiers::none());
    draw(cx);
    assert_eq!(source(&editor, cx).trim_end(), "- [x] a\n- [ ] b");
    keys(cx, &format!("{M}-z"));
    assert_eq!(source(&editor, cx).trim_end(), "- [x] a\n- [x] b");
    keys(cx, &format!("{M}-z"));
    assert_eq!(source(&editor, cx).trim_end(), "- [ ] a\n- [x] b");
    // A click in the text still places the caret.
    let p = first_line_point(&editor, cx, 0, px(1.));
    cx.simulate_click(p, Modifiers::none());
    assert_eq!(source(&editor, cx).trim_end(), "- [ ] a\n- [x] b");
}

#[gpui::test]
fn d14_hover_and_secondary_click_on_link(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "[docs](https://x.dev) tail");
    let log = event_log(&editor, cx);
    let on = first_line_point(&editor, cx, 0, px(4.));
    cx.simulate_mouse_move(on, None, Modifiers::none());
    assert_eq!(
        editor.read_with(cx, |e, _| e.hovered_link().cloned()),
        Some("https://x.dev".into())
    );
    let off = first_line_point(&editor, cx, 0, px(500.));
    cx.simulate_mouse_move(off, None, Modifiers::none());
    assert_eq!(editor.read_with(cx, |e, _| e.hovered_link().cloned()), None);
    // A plain click edits; Cmd/Ctrl+click emits LinkClicked, no edit.
    let before = editor.read_with(cx, |e, _| e.model.selection.clone());
    cx.simulate_click(on, Modifiers::secondary_key());
    let clicked: Vec<_> = log
        .borrow()
        .iter()
        .filter_map(|e| match e {
            crate::text::block_editor::BlockEditorEvent::LinkClicked { url } => Some(url.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(clicked, vec![gpui::SharedString::from("https://x.dev")]);
    assert_eq!(
        editor.read_with(cx, |e, _| e.model.selection.clone()),
        before
    );
    cx.simulate_click(on, Modifiers::none());
    assert_eq!(
        log.borrow()
            .iter()
            .filter(|e| matches!(
                e,
                crate::text::block_editor::BlockEditorEvent::LinkClicked { .. }
            ))
            .count(),
        1
    );
}

#[gpui::test]
fn d14_cmd_k_requests_popover_and_set_link_edits(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "hello world");
    let log = event_log(&editor, cx);
    // Add: select "world", Cmd/Ctrl+K, host calls set_link.
    editor.update(cx, |e, _| {
        let b = e.model.caret().unwrap().block;
        let _ = e.model.set_selection(BlockSelection::Text {
            anchor: BlockPosition::new(b, 6),
            head: BlockPosition::new(b, 11),
        });
    });
    keys(cx, &format!("{M}-k"));
    assert!(log.borrow().iter().any(|e| matches!(
        e,
        crate::text::block_editor::BlockEditorEvent::EditLinkRequested { url: None }
    )));
    editor.update(cx, |e, cx| e.set_link(Some("https://a.dev"), cx));
    assert_eq!(
        source(&editor, cx).trim_end(),
        "hello [world](https://a.dev)"
    );
    // Edit: caret inside the link, Cmd/Ctrl+K reports the current url.
    caret_at(&editor, cx, 0, 8);
    keys(cx, &format!("{M}-k"));
    assert!(log.borrow().iter().any(|e| matches!(
        e,
        crate::text::block_editor::BlockEditorEvent::EditLinkRequested { url: Some(u) } if u == "https://a.dev"
    )));
    editor.update(cx, |e, cx| e.set_link(Some("https://b.dev"), cx));
    assert_eq!(
        source(&editor, cx).trim_end(),
        "hello [world](https://b.dev)"
    );
    // Remove at caret, then undo is one step each.
    caret_at(&editor, cx, 0, 8);
    editor.update(cx, |e, cx| e.set_link(None, cx));
    assert_eq!(source(&editor, cx).trim_end(), "hello world");
    keys(cx, &format!("{M}-z"));
    assert_eq!(
        source(&editor, cx).trim_end(),
        "hello [world](https://b.dev)"
    );
    keys(cx, &format!("{M}-z"));
    assert_eq!(
        source(&editor, cx).trim_end(),
        "hello [world](https://a.dev)"
    );
}

#[gpui::test]
fn d14_toolbar_task_toggle(cx: &mut TestAppContext) {
    // The story's Task button runs `EditorCommand::ToggleTask`.
    let (editor, cx) = open(cx, "- a");
    caret_at(&editor, cx, 0, 1);
    editor.update(cx, |e, cx| {
        let _ = e.run(crate::text::block_editor::EditorCommand::ToggleTask, cx);
    });
    assert_eq!(source(&editor, cx).trim_end(), "- [ ] a");
    editor.update(cx, |e, cx| {
        let _ = e.run(crate::text::block_editor::EditorCommand::ToggleTask, cx);
    });
    assert_eq!(source(&editor, cx).trim_end(), "- a");
}

fn caret_cell(editor: &Entity<BlockEditorState>, cx: &VisualTestContext) -> Option<(usize, usize)> {
    editor.read_with(cx, |e, _| {
        let (table, cell) = e.model.current_cell()?;
        crate::text::block_editor::table::cell_coords(&e.model.doc, table, cell)
    })
}

#[gpui::test]
fn d13_table_rule_on_enter(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "Start");
    caret_at(&editor, cx, 0, 5);
    keys(cx, "end enter");
    ime_type(&editor, cx, "| Name | **Age** |");
    // Not on the closing '|': still a paragraph.
    assert_eq!(source(&editor, cx), "Start\n\n\\| Name \\| **Age** \\|");
    keys(cx, "enter");
    assert_eq!(
        source(&editor, cx).trim_end(),
        "Start\n\n| Name | **Age** |\n| --- | --- |\n|  |  |"
    );
    assert_eq!(caret_cell(&editor, cx), Some((1, 0)));
    ime_type(&editor, cx, "Ann");
    assert!(source(&editor, cx).contains("| Ann |  |"));
    keys(cx, &format!("{M}-z"));
    keys(cx, &format!("{M}-z"));
    assert_eq!(source(&editor, cx), "Start\n\n\\| Name \\| **Age** \\|");
    // A one-cell row and a non-row are handled; disabled rule types literally.
    let (editor, cx) = open(cx, "| solo |");
    caret_at(&editor, cx, 0, 8);
    keys(cx, "enter");
    assert_eq!(source(&editor, cx).trim_end(), "| solo |\n| --- |\n|  |");
    let (editor, cx) = open(cx, "a | b |");
    caret_at(&editor, cx, 0, 7);
    keys(cx, "enter");
    assert!(!source(&editor, cx).contains("---"));
}

#[gpui::test]
fn d13_enter_in_tables(cx: &mut TestAppContext) {
    let src = "| h1 | h2 |\n| --- | --- |\n| a | b |\n| c | d |";
    let (editor, cx) = open(cx, src);
    // Header (0,1) -> next row, same column; no undo step.
    caret_at(&editor, cx, 1, 1);
    keys(cx, "enter");
    assert_eq!(caret_cell(&editor, cx), Some((1, 1)));
    assert_eq!(source(&editor, cx).trim_end(), src);
    keys(cx, "enter");
    assert_eq!(caret_cell(&editor, cx), Some((2, 1)));
    // Last row: append an empty row, caret in its first cell.
    keys(cx, "enter");
    assert_eq!(caret_cell(&editor, cx), Some((3, 0)));
    assert_eq!(source(&editor, cx).trim_end(), format!("{src}\n|  |  |"));
    // All-empty row: removed, caret in a new paragraph after the table.
    keys(cx, "enter");
    assert_eq!(caret_cell(&editor, cx), None);
    ime_type(&editor, cx, "after");
    assert_eq!(source(&editor, cx).trim_end(), format!("{src}\n\nafter"));
    // One undo step each.
    keys(cx, &format!("{M}-z"));
    keys(cx, &format!("{M}-z"));
    assert_eq!(source(&editor, cx).trim_end(), format!("{src}\n|  |  |"));
    keys(cx, &format!("{M}-z"));
    assert_eq!(source(&editor, cx).trim_end(), src);
}

#[gpui::test]
fn d13_shift_enter_is_a_cell_soft_break(cx: &mut TestAppContext) {
    let src = "| h |\n| --- |\n| ab |";
    let (editor, cx) = open(cx, src);
    caret_at(&editor, cx, 1, 1);
    keys(cx, "shift-enter");
    assert_eq!(source(&editor, cx).trim_end(), "| h |\n| --- |\n| a<br>b |");
    keys(cx, &format!("{M}-z"));
    assert_eq!(source(&editor, cx).trim_end(), src);
    // Round trip.
    let br = "| x<br>y | z |\n| --- | --- |\n| 1<br>2<br>3 |  |";
    let (editor, cx) = open(cx, br);
    assert_eq!(source(&editor, cx).trim_end(), br);
    // Tab unchanged: next cell, text selected.
    caret_at(&editor, cx, 0, 0);
    keys(cx, "tab");
    assert_eq!(caret_cell(&editor, cx), Some((0, 1)));
}

#[gpui::test]
fn d13_leave_table_with_down_or_secondary_enter(cx: &mut TestAppContext) {
    let src = "| h |\n| --- |\n| a |";
    // Cmd/Ctrl+Enter with no block after: creates a paragraph (undoable).
    let (editor, cx) = open(cx, src);
    caret_at(&editor, cx, 0, 1);
    keys(cx, &format!("{M}-enter"));
    assert_eq!(caret_cell(&editor, cx), None);
    ime_type(&editor, cx, "p");
    assert_eq!(source(&editor, cx).trim_end(), format!("{src}\n\np"));
    // Down past the last row into an existing paragraph: no new block.
    let (editor, cx) = open(cx, &format!("{src}\n\nnext"));
    caret_at(&editor, cx, 1, 1);
    keys(cx, "down");
    assert_eq!(caret_cell(&editor, cx), None);
    assert_eq!(source(&editor, cx).trim_end(), format!("{src}\n\nnext"));
    ime_type(&editor, cx, "X");
    assert_eq!(source(&editor, cx).trim_end(), format!("{src}\n\nXnext"));
    // Down with nothing after creates one; one undo removes it.
    let (editor, cx) = open(cx, src);
    caret_at(&editor, cx, 1, 1);
    keys(cx, "down");
    assert_eq!(caret_cell(&editor, cx), None);
    assert_eq!(source(&editor, cx).trim_end(), src);
    keys(cx, &format!("{M}-z"));
    assert_eq!(
        caret_cell(&editor, cx),
        Some((1, 0)).or(caret_cell(&editor, cx))
    );
}

fn src_after(
    editor: &Entity<BlockEditorState>,
    cx: &mut VisualTestContext,
    ks: &str,
    typed: &str,
) -> String {
    if !ks.is_empty() {
        keys(cx, ks);
    }
    ime_type(editor, cx, typed);
    source(editor, cx).trim_end().to_string()
}

#[gpui::test]
fn d16_blockquote_exit(cx: &mut TestAppContext) {
    // Enter twice at the end leaves the quote; one undo step back.
    let (editor, cx) = open(cx, "> a");
    caret_at(&editor, cx, 0, 1);
    assert_eq!(src_after(&editor, cx, "enter enter", "x"), "> a\n\nx");
    keys(cx, &format!("{M}-z"));
    keys(cx, &format!("{M}-z"));
    assert_eq!(src_after(&editor, cx, "", "y"), "> a\n>\n> y");
    // In the middle: split around a new paragraph.
    let (editor, cx) = open(cx, "> a\n>\n> b");
    caret_at(&editor, cx, 0, 1);
    assert_eq!(
        src_after(&editor, cx, "enter enter", "x"),
        "> a\n\nx\n\n> b"
    );
    // A list in a quote: the list first, then the quote.
    let (editor, cx) = open(cx, "> - a");
    caret_at(&editor, cx, 0, 1);
    keys(cx, "enter enter");
    assert_eq!(src_after(&editor, cx, "", "q"), "> - a\n>\n> q");
    keys(cx, "backspace");
    assert_eq!(src_after(&editor, cx, "enter", "x"), "> - a\n\nx");
    // Backspace at the start of the first paragraph lifts it out.
    let (editor, cx) = open(cx, "> a\n>\n> b");
    caret_at(&editor, cx, 0, 0);
    assert_eq!(src_after(&editor, cx, "backspace", ""), "a\n\n> b");
    keys(cx, &format!("{M}-z"));
    assert_eq!(source(&editor, cx).trim_end(), "> a\n>\n> b");
    let (editor, cx) = open(cx, "> only");
    caret_at(&editor, cx, 0, 0);
    assert_eq!(src_after(&editor, cx, "backspace", ""), "only");
}

#[gpui::test]
fn d16_list_item_trailing_empty_paragraph(cx: &mut TestAppContext) {
    // Enter: up one level (an empty item of its own), then out.
    let (editor, cx) = open(cx, "- a\n  - b\n\n    c");
    caret_at(&editor, cx, 2, 1);
    keys(cx, "backspace");
    assert_eq!(src_after(&editor, cx, "enter", "x"), "- a\n  - b\n  - x");
    keys(cx, &format!("{M}-z"));
    keys(cx, &format!("{M}-z"));
    assert_eq!(
        source(&editor, cx).trim_end(),
        "- a\n  - b\n\n    c".replace("c", "").trim_end()
    );
    keys(cx, "enter enter");
    assert_eq!(src_after(&editor, cx, "", "y"), "- a\n  - b\n- y");
    // After leaving a code block in an item.
    let (editor, cx) = open(cx, "- a\n\n  ```\n  code\n  ```");
    caret_at(&editor, cx, 1, 4);
    keys(cx, "enter enter");
    assert_eq!(
        src_after(&editor, cx, "enter", "x"),
        "- a\n\n  ```\n  code\n  ```\n\n- x"
    );
    // Shift+Tab and Backspace lift it too, one undo step each.
    for k in ["shift-tab", "backspace"] {
        let (editor, cx) = open(cx, "- a\n\n  b");
        caret_at(&editor, cx, 1, 1);
        keys(cx, "backspace");
        let before = source(&editor, cx);
        assert_eq!(src_after(&editor, cx, k, "x"), "- a\n- x", "{k}");
        keys(cx, &format!("{M}-z"));
        keys(cx, &format!("{M}-z"));
        assert_eq!(source(&editor, cx), before, "{k}");
    }
}

#[gpui::test]
fn d16_checkbox_click_keeps_caret_and_shows_hand(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "- [ ] a\n- [ ] bc\n\n[l](u)");
    caret_at(&editor, cx, 1, 1);
    let before = editor.read_with(cx, |e, _| e.model.selection.clone());
    let p = first_line_point(&editor, cx, 0, px(-10.));
    cx.simulate_mouse_move(p, None, Modifiers::none());
    assert!(editor.read_with(cx, |e, _| e.pointer_hand));
    cx.simulate_mouse_down(p, MouseButton::Left, Modifiers::none());
    let q = first_line_point(&editor, cx, 1, px(200.));
    cx.simulate_mouse_move(q, Some(MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(q, MouseButton::Left, Modifiers::none());
    draw(cx);
    assert!(source(&editor, cx).starts_with("- [x] a\n- [ ] bc"));
    assert_eq!(
        editor.read_with(cx, |e, _| e.model.selection.clone()),
        before
    );
    // Text over a link: I-beam, hand only with Cmd/Ctrl held.
    let l = first_line_point(&editor, cx, 2, px(2.));
    cx.simulate_mouse_move(l, None, Modifiers::none());
    assert!(!editor.read_with(cx, |e, _| e.pointer_hand));
    cx.simulate_mouse_move(l, None, Modifiers::secondary_key());
    assert!(editor.read_with(cx, |e, _| e.pointer_hand));
}

#[gpui::test]
fn host_submit_policy_does_not_submit_composition_or_shift_enter(cx: &mut TestAppContext) {
    use crate::text::block_editor::{BlockEditorEvent, SubmitPolicy};
    use std::{cell::Cell, rc::Rc};
    let (editor, cx) = open(cx, "draft");
    let submits = Rc::new(Cell::new(0));
    let count = submits.clone();
    let subscription = cx.update(|_, cx| {
        cx.subscribe(&editor, move |_, event: &BlockEditorEvent, _| {
            if matches!(event, BlockEditorEvent::SubmitRequested) {
                count.set(count.get() + 1);
            }
        })
    });
    editor.update(cx, |state, cx| {
        state.set_submit_policy(SubmitPolicy::Enter);
        cx.notify();
    });
    caret_at(&editor, cx, 0, 5);
    keys(cx, "enter");
    assert_eq!(submits.get(), 1);
    assert_eq!(source(&editor, cx), "draft");
    keys(cx, "M-enter");
    let after_secondary = 1 + usize::from(cfg!(target_os = "macos"));
    assert_eq!(submits.get(), after_secondary);
    assert_eq!(
        source(&editor, cx),
        if cfg!(target_os = "macos") {
            "draft"
        } else {
            "draft\n\n"
        }
    );
    keys(cx, "shift-enter");
    assert_eq!(submits.get(), after_secondary);
    cx.update(|window, cx| {
        editor.update(cx, |state, cx| {
            state.replace_and_mark_text_in_range(None, "雪", Some(0..1), window, cx);
        })
    });
    assert!(editor.read_with(cx, |state, _| state.is_composing()));
    keys(cx, "enter");
    assert_eq!(submits.get(), after_secondary);
    cx.update(|window, cx| editor.update(cx, |state, cx| state.unmark_text(window, cx)));
    keys(cx, "enter");
    assert_eq!(submits.get(), after_secondary + 1);
    editor.update(cx, |state, cx| {
        state.set_submit_policy(SubmitPolicy::ModifiedEnter);
        cx.notify();
    });
    keys(cx, "enter");
    assert_eq!(submits.get(), after_secondary + 1);
    keys(cx, "M-enter");
    assert_eq!(submits.get(), after_secondary + 2);
    drop(subscription);
}

#[gpui::test]
fn host_semantic_paste_intercepts_keyboard_and_native_action_once(cx: &mut TestAppContext) {
    use crate::text::block_editor::ClipboardPolicy;
    use std::{cell::RefCell, rc::Rc};
    let (editor, cx) = open(cx, "draft");
    let pending = Rc::new(RefCell::new(Vec::new()));
    let captured = pending.clone();
    let subscription = cx.update(|_, cx| {
        cx.subscribe(
            &editor,
            move |_, event: &crate::text::block_editor::BlockEditorEvent, _| {
                if let crate::text::block_editor::BlockEditorEvent::PasteRequested {
                    request,
                    item,
                } = event
                {
                    captured.borrow_mut().push((request.clone(), item.clone()));
                }
            },
        )
    });
    editor.update(cx, |state, _| {
        state.set_clipboard_policy(ClipboardPolicy::Host)
    });
    caret_at(&editor, cx, 0, 5);
    // TestAppContext uses its fake platform clipboard, never the OS clipboard.
    cx.update(|_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(" text".to_string())));
    keys(cx, "M-v");
    assert_eq!(source(&editor, cx), "draft");
    let (request, item) = pending.borrow_mut().pop().unwrap();
    assert_eq!(item.text().as_deref(), Some(" text"));
    editor.update(cx, |state, cx| {
        assert!(state.apply_paste(&request, Some(" text"), cx));
        assert!(!state.apply_paste(&request, Some(" text"), cx));
    });
    assert_eq!(source(&editor, cx), "draft text");
    cx.update(|window, cx| window.dispatch_action(Box::new(crate::input::Paste), cx));
    draw(cx);
    let (request, _) = pending.borrow_mut().pop().unwrap();
    editor.update(cx, |state, cx| {
        assert!(state.apply_paste(&request, None, cx));
        assert!(!state.apply_paste(&request, None, cx));
    });
    assert_eq!(source(&editor, cx), "draft text");
    // OsAction/native menu uses EntityInputHandler::paste, not input::Paste.
    cx.update(|window, cx| {
        editor.update(cx, |state, cx| {
            EntityInputHandler::paste(
                state,
                gpui::ClipboardItem::new_string(" native".into()),
                window,
                cx,
            );
        })
    });
    draw(cx);
    assert_eq!(source(&editor, cx), "draft text");
    assert_eq!(pending.borrow().len(), 1);
    let (request, item) = pending.borrow_mut().pop().unwrap();
    assert_eq!(item.text().as_deref(), Some(" native"));
    editor.update(cx, |state, cx| {
        assert!(state.apply_paste(&request, item.text().as_deref(), cx))
    });
    assert_eq!(source(&editor, cx), "draft text native");
    assert!(pending.borrow().is_empty());
    drop(subscription);
}

#[gpui::test]
fn host_image_placeholder_load_and_paste_never_issue_http(cx: &mut TestAppContext) {
    use crate::text::block_editor::ImagePolicy;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let requests = Arc::new(AtomicUsize::new(0));
    let count = requests.clone();
    cx.update(|cx| {
        cx.set_http_client(gpui::http_client::FakeHttpClient::create(move |_| {
            count.fetch_add(1, Ordering::SeqCst);
            async {
                Err(anyhow::anyhow!(
                    "fake client: outbound image loading forbidden"
                ))
            }
        }));
        crate::init(cx);
    });
    let initial = "![alt](https://attacker.example/load)\n\n| image |\n| --- |\n| ![cell](https://attacker.example/table) |";
    let (root, cx) = cx.add_window_view(|window, cx| {
        Root(cx.new(|cx| {
            BlockEditorState::new(window, cx)
                .with_image_policy(ImagePolicy::Placeholder)
                .with_source(initial)
        }))
    });
    let editor = root.read_with(cx, |root, _| root.0.clone());
    draw(cx);
    assert_eq!(source(&editor, cx), initial);
    caret_at(&editor, cx, 0, '\u{fffc}'.len_utf8());
    editor.update(cx, |state, cx| {
        state.paste(
            gpui::ClipboardItem::new_string("![pasted](https://attacker.example/paste)".into()),
            false,
            cx,
        )
    });
    draw(cx);
    assert!(source(&editor, cx).contains("https://attacker.example/paste"));
    editor.read_with(cx, |state, _| {
        let count = state
            .model()
            .document()
            .leaves()
            .map(|leaf| {
                state
                    .model()
                    .doc()
                    .inline(leaf.id())
                    .map_or(0, |text| text.atoms().len())
            })
            .sum::<usize>();
        assert_eq!(count, 3);
    });
    editor.update(cx, |state, cx| {
        let mut range = state.capture_caret().unwrap();
        state.replace_captured(&mut range, " after", cx).unwrap();
    });
    draw(cx);
    assert!(source(&editor, cx).contains(" after"));
    assert_eq!(requests.load(Ordering::SeqCst), 0);
}

/// Exercise policy-scoped bindings through the real dispatch tree, including
/// switching back to standalone policies on the same retained editor.
#[gpui::test]
fn host_composer_key_dispatch_keeps_structural_and_cell_breaks_distinct(cx: &mut TestAppContext) {
    use crate::text::block_editor::{BlockEditorEvent, SubmitPolicy};
    use std::{cell::Cell, rc::Rc};

    let (editor, cx) = open(cx, "draft");
    let submits = Rc::new(Cell::new(0));
    let count = submits.clone();
    let _subscription = cx.update(|_, cx| {
        cx.subscribe(&editor, move |_, event: &BlockEditorEvent, _| {
            if matches!(event, BlockEditorEvent::SubmitRequested) {
                count.set(count.get() + 1);
            }
        })
    });
    editor.update(cx, |state, cx| {
        state.set_submit_policy(SubmitPolicy::Enter);
        cx.notify();
    });
    caret_at(&editor, cx, 0, 5);
    keys(cx, "enter");
    assert_eq!(submits.get(), 1);
    assert_eq!(source(&editor, cx), "draft");
    keys(cx, "shift-enter");
    type_text(cx, "next");
    assert_eq!(source(&editor, cx), "draft\n\nnext");
    editor.update(cx, |state, cx| state.set_source("- item", cx).unwrap());
    caret_at(&editor, cx, 0, 4);
    keys(cx, "shift-enter");
    type_text(cx, "next");
    assert_eq!(source(&editor, cx), "- item\n- next");
    keys(cx, "shift-enter shift-enter");
    type_text(cx, "after");
    assert_eq!(source(&editor, cx), "- item\n- next\n\nafter");

    let table = "| h1 | h2 |\n| --- | --- |\n| ab | cd |";
    editor.update(cx, |state, cx| state.set_source(table, cx).unwrap());
    caret_at(&editor, cx, 1, 1);
    keys(cx, "shift-enter");
    assert_eq!(caret_cell(&editor, cx), Some((1, 1)));
    assert_eq!(source(&editor, cx), table);
    keys(cx, "shift-tab");
    assert_eq!(caret_cell(&editor, cx), Some((1, 0)));
    keys(cx, "tab");
    assert_eq!(caret_cell(&editor, cx), Some((1, 1)));
    keys(cx, "tab");
    assert_eq!(caret_cell(&editor, cx), Some((2, 0)));
    assert_eq!(source(&editor, cx), format!("{table}\n|  |  |"));
    keys(cx, "shift-enter");
    assert_eq!(caret_cell(&editor, cx), None);
    type_text(cx, "after");
    assert_eq!(source(&editor, cx), format!("{table}\n\nafter"));

    editor.update(cx, |state, cx| state.set_source(table, cx).unwrap());
    caret_at(&editor, cx, 2, 1);
    keys(cx, "ctrl-enter");
    assert_eq!(
        source(&editor, cx),
        "| h1 | h2 |\n| --- | --- |\n| a<br>b | cd |"
    );
    assert_eq!(caret_cell(&editor, cx), Some((1, 0)));
    assert_eq!(submits.get(), 1);
    let before_submit = source(&editor, cx);
    keys(cx, "enter");
    assert_eq!(submits.get(), 2);
    assert_eq!(source(&editor, cx), before_submit);
    keys(cx, "shift-enter");
    assert_eq!(caret_cell(&editor, cx), Some((2, 0)));
    keys(cx, "shift-enter");
    assert_eq!(caret_cell(&editor, cx), None);
    #[cfg(target_os = "macos")]
    {
        let before = source(&editor, cx);
        keys(cx, "cmd-enter");
        assert_eq!(submits.get(), 3);
        assert_eq!(source(&editor, cx), before);
    }

    // Readonly and marked input cannot submit, split or insert a cell break.
    editor.update(cx, |state, cx| state.set_readonly(true, cx));
    let before = submits.get();
    let readonly_source = source(&editor, cx);
    keys(cx, "enter shift-enter ctrl-enter");
    #[cfg(target_os = "macos")]
    keys(cx, "cmd-enter");
    assert_eq!(submits.get(), before);
    assert_eq!(source(&editor, cx), readonly_source);
    editor.update(cx, |state, cx| state.set_readonly(false, cx));

    editor.update(cx, |state, cx| state.set_source(table, cx).unwrap());
    caret_at(&editor, cx, 2, 1);
    cx.update(|window, cx| {
        editor.update(cx, |state, cx| {
            state.replace_and_mark_text_in_range(None, "雪", Some(0..1), window, cx);
        })
    });
    let marked_source = source(&editor, cx);
    let before = submits.get();
    keys(cx, "enter shift-enter ctrl-enter");
    #[cfg(target_os = "macos")]
    keys(cx, "cmd-enter");
    assert_eq!(submits.get(), before);
    assert_eq!(source(&editor, cx), marked_source);
    cx.update(|window, cx| editor.update(cx, |state, cx| state.unmark_text(window, cx)));
    keys(cx, "enter");
    assert_eq!(submits.get(), before + 1);

    // Standalone and modified-submit policies retain their old Enter/Shift
    // behavior; composer bindings must leave the dispatch context with them.
    for policy in [SubmitPolicy::Newline, SubmitPolicy::ModifiedEnter] {
        editor.update(cx, |state, cx| {
            state.set_source(table, cx).unwrap();
            state.set_submit_policy(policy);
            cx.notify();
        });
        caret_at(&editor, cx, 2, 1);
        keys(cx, "shift-enter");
        assert_eq!(
            source(&editor, cx),
            "| h1 | h2 |\n| --- | --- |\n| a<br>b | cd |"
        );
        #[cfg(target_os = "macos")]
        {
            let before = source(&editor, cx);
            keys(cx, "ctrl-enter");
            assert_eq!(source(&editor, cx), before);
        }
        keys(cx, "enter");
        assert_eq!(caret_cell(&editor, cx), Some((2, 0)));
        let before = submits.get();
        keys(cx, "M-enter");
        assert_eq!(
            submits.get(),
            before + usize::from(policy == SubmitPolicy::ModifiedEnter)
        );
    }
}

#[gpui::test]
fn instance_typography_rewraps_heading_and_retains_caret(cx: &mut TestAppContext) {
    struct TypographyEditor {
        state: Entity<BlockEditorState>,
        font_size: Pixels,
        foreground: gpui::Hsla,
    }
    impl Render for TypographyEditor {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .id("typography-scroll")
                .w(px(320.))
                .h(px(160.))
                .overflow_y_scroll()
                .track_scroll(&self.state.read(cx).scroll_handle().unwrap())
                .child(
                    BlockEditor::new(&self.state)
                        .text_size(self.font_size)
                        .text_color(self.foreground),
                )
        }
    }

    cx.update(|cx| {
        cx.set_http_client(gpui::http_client::FakeHttpClient::create(|_| async {
            Err(anyhow::anyhow!(
                "outbound requests forbidden in editor test"
            ))
        }));
        crate::init(cx);
    });
    let heading = "heading words that wrap over several visual lines";
    let tail = (0..80)
        .map(|i| format!("\n\nparagraph {i} has words that wrap over several visual lines"))
        .collect::<String>();
    let doc = format!("# {heading}{tail}");
    let (root, cx) = cx.add_window_view(|window, cx| TypographyEditor {
        state: cx.new(|cx| BlockEditorState::new(window, cx).with_source(&doc)),
        font_size: px(16.),
        foreground: gpui::black(),
    });
    let editor = root.read_with(cx, |root, _| root.state.clone());
    cx.update(|window, cx| {
        let focus = editor.read(cx).focus_handle.clone();
        window.focus(&focus, cx);
    });
    caret_at(&editor, cx, 0, 0);
    keys(cx, "end");
    let original_line_end = editor.read_with(cx, |state, _| state.model().caret().unwrap().offset);
    let last = editor.read_with(cx, |state, _| state.model().document().leaves().count() - 1);
    let len = editor.read_with(cx, |state, _| {
        state
            .model()
            .document()
            .leaves()
            .last()
            .unwrap()
            .text()
            .unwrap()
            .len()
    });
    caret_at(&editor, cx, last, len);
    draw(cx);
    let visible = |cx: &VisualTestContext| {
        editor.read_with(cx, |state, _| {
            let caret = state
                .frame
                .borrow()
                .caret_bounds(state.model().caret().unwrap(), px(1.))
                .unwrap();
            state
                .scroll_handle()
                .unwrap()
                .bounds()
                .contains(&caret.center())
        })
    };
    assert!(visible(cx));

    // Only the native element's instance text size changes. Width and rem
    // remain fixed; no explicit caret-reveal request repairs the result.
    root.update(cx, |root, cx| {
        root.font_size = px(32.);
        cx.notify();
    });
    draw(cx);
    draw(cx);
    assert!(
        visible(cx),
        "instance typography retains the caret in its owner"
    );
    assert_eq!(source(&editor, cx), doc);

    // A user may inspect another part of the document without moving the
    // caret. Paint-only changes and subsequent reflow must not pull them back.
    let scroll = editor.read_with(cx, |state, _| state.scroll_handle().unwrap());
    scroll.set_offset(gpui::point(px(0.), px(0.)));
    draw(cx);
    draw(cx);
    assert!(!visible(cx));
    let inspected_offset = scroll.offset();
    let selection = editor.read_with(cx, |state, _| state.model().selection().clone());
    root.update(cx, |root, cx| {
        root.foreground = gpui::white();
        cx.notify();
    });
    draw(cx);
    draw(cx);
    assert_eq!(
        scroll.offset(),
        inspected_offset,
        "color must not reveal the caret"
    );
    assert!(!visible(cx));
    root.update(cx, |root, cx| {
        root.font_size = px(40.);
        cx.notify();
    });
    draw(cx);
    draw(cx);
    assert_eq!(
        scroll.offset(),
        inspected_offset,
        "reflow must preserve manual scrolling"
    );
    assert!(!visible(cx));
    assert_eq!(
        editor.read_with(cx, |state, _| state.model().selection().clone()),
        selection
    );
    assert_eq!(source(&editor, cx), doc);

    caret_at(&editor, cx, 0, 0);
    keys(cx, "end");
    let wrapped_line_end = editor.read_with(cx, |state, _| state.model().caret().unwrap().offset);
    assert!(
        wrapped_line_end < original_line_end,
        "End follows the rewrapped body-relative heading"
    );
    type_text(cx, "X");
    assert_eq!(
        source(&editor, cx),
        format!(
            "# {}X{}{tail}",
            &heading[..wrapped_line_end],
            &heading[wrapped_line_end..]
        )
    );
}

// ---- D17: copy and paste keep inline marks --------------------------------

fn select_in(
    editor: &Entity<BlockEditorState>,
    cx: &mut VisualTestContext,
    from: (usize, usize),
    to: (usize, usize),
) {
    editor.update(cx, |e, cx| {
        let leaves: Vec<_> = e.model().document().leaves().map(|l| l.id()).collect();
        e.set_selection(
            BlockSelection::Text {
                anchor: BlockPosition::new(leaves[from.0], from.1),
                head: BlockPosition::new(
                    leaves[to.0],
                    to.1.min(e.model.doc.leaf_len(leaves[to.0]).unwrap()),
                ),
            },
            cx,
        )
        .unwrap();
    });
}

fn clipboard_text(cx: &mut VisualTestContext) -> String {
    cx.read_from_clipboard().and_then(|c| c.text()).unwrap()
}

/// Copy `from..to` of `doc`, then paste at the end of the document's last
/// leaf (a "T" paragraph appended to `doc`). Returns (clipboard, source).
fn copy_paste(
    cx: &mut TestAppContext,
    doc: &str,
    from: (usize, usize),
    to: (usize, usize),
) -> (String, String) {
    let (editor, cx) = open(cx, &format!("{doc}\n\nT"));
    select_in(&editor, cx, from, to);
    keys(cx, "M-c");
    let clip = clipboard_text(cx);
    let last = editor.read_with(cx, |e, _| e.model().document().leaves().count() - 1);
    caret_at(&editor, cx, last, 1);
    keys(cx, "M-v");
    (clip, source(&editor, cx))
}

#[gpui::test]
fn d17_whole_leaf_round_trips_each_mark(cx: &mut TestAppContext) {
    for md in [
        "Hello `testing` world",
        "a **bold** b",
        "an *it* x",
        "a ~~gone~~ b",
        "see [docs](https://x.dev \"T\") now",
        "***both*** end",
        "**bold and *nested* text**",
        "x `a*b` y **c_d** z",
        "<u>under</u> and <mark>hi</mark>",
    ] {
        let (editor, cx2) = open(cx, &format!("{md}\n\nT"));
        select_in(&editor, cx2, (0, 0), (0, usize::MAX));
        keys(cx2, "M-c");
        caret_at(&editor, cx2, 1, 1);
        keys(cx2, "M-v");
        // The pasted copy has exactly the marks of the original leaf.
        let (orig, pasted) = editor.read_with(cx2, |e, _| {
            let leaves = e.model.doc.leaves();
            let inline = |i: usize| e.model.doc.inline(leaves[i]).unwrap().clone();
            let pasted = inline(1);
            (inline(0), pasted.slice(1..pasted.len()))
        });
        assert_eq!(pasted, orig, "{md:?}");
    }
}

#[gpui::test]
fn d17_inline_code_copy_and_paste_keeps_backticks(cx: &mut TestAppContext) {
    let (clip, src) = copy_paste(cx, "Hello `testing` world", (0, 0), (0, 19));
    assert_eq!(clip, "Hello `testing` world");
    assert_eq!(src, "Hello `testing` world\n\nTHello `testing` world");
}

#[gpui::test]
fn d17_partial_selections_have_balanced_markers(cx: &mut TestAppContext) {
    // Starts and ends inside a code span.
    let (clip, src) = copy_paste(cx, "Hello `testing` world", (0, 9), (0, 16));
    assert_eq!(clip, "`ting` wo");
    assert!(src.ends_with("\n\nT`ting` wo"), "{src:?}");
    // One word out of a bold run.
    let (clip, src) = copy_paste(cx, "a **bold phrase here** b", (0, 7), (0, 13));
    assert_eq!(clip, "**phrase**");
    assert!(src.ends_with("\n\nT**phrase**"), "{src:?}");
    // Starting mid-bold, ending in plain text after it.
    let (clip, _) = copy_paste(cx, "**bold** and more", (0, 2), (0, 9));
    assert_eq!(clip, "**ld** and ");
    // Italic partly, strikethrough partly, link partly.
    let (clip, _) = copy_paste(cx, "x *italic text* y", (0, 4), (0, 10));
    assert_eq!(clip, "*alic t*");
    let (clip, _) = copy_paste(cx, "x ~~struck out~~ y", (0, 4), (0, 9));
    assert_eq!(clip, "~~ruck~~ ");
    let (clip, src) = copy_paste(cx, "go [the docs](https://x.dev) ok", (0, 7), (0, 9));
    assert_eq!(clip, "[do](https://x.dev)");
    assert!(src.ends_with("\n\nT[do](https://x.dev)"), "{src:?}");
    // Nested bold-italic cut in the middle.
    let (clip, _) = copy_paste(cx, "***both marks***", (0, 1), (0, 6));
    assert!(
        clip.contains("oth m") && clip.starts_with(['*', '_']),
        "{clip:?}"
    );
    // Whitespace at the cut edges stays outside the markers.
    let (clip, _) = copy_paste(cx, "a **bold** b", (0, 1), (0, 6));
    assert_eq!(clip, " **bold**");
}

#[gpui::test]
fn d17_multi_block_selection_keeps_marks_per_block(cx: &mut TestAppContext) {
    let (clip, src) = copy_paste(cx, "**a** one\n\n# `h` two\n\n*b* three", (0, 0), (2, 7));
    assert_eq!(clip, "**a** one\n`h` two\n*b* three");
    assert!(
        src.ends_with("\n\nT**a** one\n\n`h` two\n\n*b* three"),
        "{src:?}"
    );
}

#[gpui::test]
fn d17_paste_plain_stays_literal_and_external_text_is_conservative(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "**bold** x\n\nT");
    select_in(&editor, cx, (0, 0), (0, 4));
    keys(cx, "M-c");
    assert_eq!(clipboard_text(cx), "**bold**");
    caret_at(&editor, cx, 1, 1);
    keys(cx, "M-shift-v");
    // Raw characters, no marks.
    assert_eq!(source(&editor, cx), "**bold** x\n\nT\\*\\*bold\\*\\*");
    // External plain text (no tag) is not parsed as inline Markdown.
    cx.update(|_, cx| {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string("2*3*4 and **x**".into()))
    });
    caret_at(&editor, cx, 1, 0);
    keys(cx, "M-v");
    assert_eq!(
        source(&editor, cx),
        "**bold** x\n\n2\\*3\\*4 and \\*\\*x\\*\\*T\\*\\*bold\\*\\*"
    );
    // The same text, tagged as our own copy, is parsed.
    cx.update(|_, cx| {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string_with_metadata(
            "**y**".into(),
            "gpui-kit/block-editor/inline-markdown/v1".into(),
        ))
    });
    caret_at(&editor, cx, 0, 6);
    keys(cx, "M-v");
    assert!(
        source(&editor, cx).starts_with("**bold** x**y**"),
        "{}",
        source(&editor, cx)
    );
}

#[gpui::test]
fn d17_own_copy_of_special_characters_pastes_literally_and_undoes_once(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "2\\*3\\*4 and snake\\_case # \\[x\\]\n\nT");
    select_in(&editor, cx, (0, 0), (0, usize::MAX));
    keys(cx, "M-c");
    caret_at(&editor, cx, 1, 1);
    keys(cx, "M-v");
    let leaf_text = editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().nth(1).unwrap().id();
        e.model.doc.leaf_text(id).unwrap().to_string()
    });
    assert_eq!(leaf_text, "T2*3*4 and snake_case # [x]");
    keys(cx, "M-z");
    assert_eq!(
        source(&editor, cx),
        "2\\*3\\*4 and snake\\_case # \\[x\\]\n\nT"
    );
}

#[gpui::test]
fn d17_table_cell_copy_still_copies_structure(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "| **x** | y |\n| --- | --- |\n| `1` | 2 |");
    let from = leaf_point(&editor, cx, 0, 0);
    let to = leaf_point(&editor, cx, 3, 1);
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    keys(cx, "M-c");
    assert_eq!(
        clipboard_text(cx),
        "| **x** | y |\n| --- | --- |\n| `1` | 2 |"
    );
    // Marks inside a single cell's text survive a copy/paste too.
    let (editor, cx) = open(cx, "| **x** y | z |\n| --- | --- |\n| a | b |");
    select_in(&editor, cx, (0, 0), (0, 3));
    keys(cx, "M-c");
    assert_eq!(clipboard_text(cx), "**x** y");
    caret_at(&editor, cx, 3, 1);
    keys(cx, "M-v");
    assert_eq!(
        source(&editor, cx),
        "| **x** y | z |\n| --- | --- |\n| a | b**x** y |"
    );
}

#[gpui::test]
fn d17_host_policy_keeps_the_own_copy_flag_across_apply_paste(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "`c` d\n\nT");
    let pending = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = pending.clone();
    let _sub = cx.update(|_, cx| {
        cx.subscribe(
            &editor,
            move |_, event: &crate::text::block_editor::BlockEditorEvent, _| {
                if let crate::text::block_editor::BlockEditorEvent::PasteRequested {
                    request,
                    item,
                } = event
                {
                    sink.borrow_mut().push((request.clone(), item.clone()));
                }
            },
        )
    });
    editor.update(cx, |s, _| {
        s.set_clipboard_policy(crate::text::block_editor::ClipboardPolicy::Host)
    });
    select_in(&editor, cx, (0, 0), (0, 1));
    keys(cx, "M-c");
    caret_at(&editor, cx, 1, 1);
    keys(cx, "M-v");
    let (request, item) = pending.borrow_mut().pop().unwrap();
    editor.update(cx, |s, cx| {
        assert!(s.apply_paste(&request, item.text().as_deref(), cx))
    });
    assert_eq!(source(&editor, cx), "`c` d\n\nT`c`");
}

/// The caret is the input's one-pixel line (main #3515), snapped to whole
/// device pixels like the input caret.
#[gpui::test]
fn caret_is_the_shared_one_pixel_line(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "hello");
    caret_at(&editor, cx, 0, 2);
    let width = editor.read_with(cx, |e, _| e.style.caret_width());
    assert_eq!(width, px(1.));
    let b = editor.read_with(cx, |e, _| {
        let c = e.model().caret().unwrap();
        e.frame
            .borrow()
            .caret_bounds(c, e.style.caret_width())
            .unwrap()
    });
    assert_eq!(b.size.width, px(1.));
    for scale in [1.0_f32, 1.5, 2.0] {
        let s = crate::input::snap_caret_to_device_pixels(
            gpui::Bounds::new(gpui::point(px(10.3), px(4.)), gpui::size(px(1.), px(18.))),
            scale,
        );
        let device = |p: gpui::Pixels| f32::from(p) * scale;
        assert!(
            (device(s.origin.x) - device(s.origin.x).round()).abs() < 1e-3,
            "{scale}"
        );
        assert!(device(s.size.width) >= 1.0 - 1e-3, "{scale}");
    }
}
