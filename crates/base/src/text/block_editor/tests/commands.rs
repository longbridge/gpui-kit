//! Commands on nested blocks and the editing primitives.

use super::*;

#[test]
fn split_and_join_paragraphs() {
    let mut m = model("abcd");
    caret_in(&mut m, 0, 2);
    m.split_block().unwrap();
    assert_eq!(m.source(), "ab\n\ncd");
    m.delete(false).unwrap();
    assert_eq!(m.source(), "abcd");
    assert_eq!(m.caret().unwrap().offset, 2);
}

#[test]
fn split_in_list_item_creates_item() {
    let mut m = model("- [x] ab");
    caret_in(&mut m, 0, 1);
    m.split_block().unwrap();
    assert_eq!(m.source(), "- [x] a\n- [ ] b");
    m.insert_text("z", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "- [x] a\n- [ ] zb");
}

#[test]
fn newline_in_inserted_text_splits() {
    let mut m = model("");
    m.insert_text("a\nb", EditOrigin::Paste).unwrap();
    assert_eq!(m.source(), "a\n\nb");
}

#[test]
fn delete_forward_joins_next() {
    let mut m = model("ab\n\ncd");
    caret_at_end(&mut m, 0);
    m.delete(true).unwrap();
    assert_eq!(m.source(), "abcd");
}

#[test]
fn backspace_removes_preceding_rule() {
    let mut m = model("a\n\n---\n\nb");
    caret_in(&mut m, 2, 0);
    m.delete(false).unwrap();
    assert_eq!(m.source(), "a\n\nb");
}

#[test]
fn backspace_removes_whole_grapheme() {
    let mut m = model("e\u{301}x");
    caret_in(&mut m, 0, 3);
    m.delete(false).unwrap();
    assert_eq!(m.source(), "x");
}

#[test]
fn cross_block_delete_joins_ends() {
    let mut m = model("# abc\n\n- def\n\nghi");
    let leaves: Vec<_> = m.document().leaves().map(|b| b.id()).collect();
    m.set_selection(BlockSelection::Text {
        anchor: BlockPosition::new(leaves[2], 1),
        head: BlockPosition::new(leaves[0], 1),
    })
    .unwrap();
    m.run(EditorCommand::DeleteSelection).unwrap();
    assert_eq!(m.source(), "# ahi");
    assert_invariants(&m);
}

#[test]
fn nested_list_toggle_converts_inner_list_only() {
    let mut m = model("- a\n  - b");
    caret_in(&mut m, 1, 0);
    m.run(EditorCommand::ToggleList { ordered: true }).unwrap();
    assert_eq!(m.source(), "- a\n  1. b");
    m.run(EditorCommand::ToggleList { ordered: true }).unwrap();
    assert_eq!(m.source(), "- a\n\n  b");
    assert_invariants(&m);
}

#[test]
fn toggle_list_off_splits_list() {
    let mut m = model("- a\n- b\n- c");
    caret_in(&mut m, 1, 0);
    m.run(EditorCommand::ToggleList { ordered: false }).unwrap();
    assert_eq!(m.source(), "- a\n\nb\n\n- c");
    assert_invariants(&m);
}

#[test]
fn indent_then_outdent() {
    let mut m = model("- a\n- b\n- c");
    caret_in(&mut m, 1, 0);
    m.run(EditorCommand::Indent).unwrap();
    assert_eq!(m.source(), "- a\n  - b\n- c");
    caret_in(&mut m, 2, 0);
    m.run(EditorCommand::Indent).unwrap();
    assert_eq!(m.source(), "- a\n  - b\n  - c");
    caret_in(&mut m, 1, 0);
    m.run(EditorCommand::Outdent).unwrap();
    assert_eq!(m.source(), "- a\n- b\n  - c");
    assert_invariants(&m);
}

#[test]
fn move_block_inside_blockquote() {
    let mut m = model("> a\n>\n> b");
    caret_in(&mut m, 0, 0);
    m.run(EditorCommand::MoveBlocks { down: true }).unwrap();
    assert_eq!(m.source(), "> b\n>\n> a");
}

#[test]
fn set_block_kind_and_query() {
    let mut m = model("a");
    caret_in(&mut m, 0, 0);
    m.run(EditorCommand::SetBlockKind(BlockKind::Heading { level: 2 }))
        .unwrap();
    assert_eq!(m.source(), "## a");
    assert_eq!(
        m.block_kind_at_caret(),
        Some(BlockKind::Heading { level: 2 })
    );
}

#[test]
fn toggle_task() {
    let mut m = model("- a");
    caret_in(&mut m, 0, 0);
    m.run(EditorCommand::ToggleTask).unwrap();
    assert_eq!(m.source(), "- [ ] a");
}

#[test]
fn mark_state_and_active_marks() {
    let mut m = model("a **b** c");
    select(&mut m, 0, 0..3);
    assert_eq!(m.mark_state(MarkKind::Bold), MarkState::Mixed);
    caret_in(&mut m, 0, 3);
    assert!(m.active_marks().has(MarkKind::Bold));
    caret_in(&mut m, 0, 1);
    assert!(!m.active_marks().has(MarkKind::Bold));
}

#[test]
fn link_does_not_extend_at_trailing_edge() {
    let mut m = model("[ab](https://x.dev)");
    caret_at_end(&mut m, 0);
    m.insert_text("c", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "[ab](https://x.dev)c");
    caret_in(&mut m, 0, 1);
    m.insert_text("z", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "[azb](https://x.dev)c");
}

#[test]
fn code_block_text_editing() {
    let mut m = model("```rust\nfn main() {}\n```");
    caret_in(&mut m, 0, 11);
    m.insert_text("\n    x", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "```rust\nfn main() {\n    x}\n```");
    // Marks never apply inside code.
    m.run(EditorCommand::ToggleMark(MarkKind::Bold)).unwrap();
    m.insert_text("y", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "```rust\nfn main() {\n    xy}\n```");
    m.delete(false).unwrap();
    m.delete(false).unwrap();
    assert_eq!(m.source(), "```rust\nfn main() {\n    }\n```");
    // Tab / Shift+Tab indent the caret's line.
    m.run(EditorCommand::Outdent).unwrap();
    assert_eq!(m.source(), "```rust\nfn main() {\n}\n```");
    m.run(EditorCommand::Indent).unwrap();
    assert_eq!(m.source(), "```rust\nfn main() {\n    }\n```");
    assert!(m.undo());
    assert_eq!(m.source(), "```rust\nfn main() {\n}\n```");
    assert_invariants(&m);
}

#[test]
fn code_block_enter_and_exit() {
    let mut m = model("```\na\n```");
    caret_in(&mut m, 0, 1);
    m.split_block().unwrap();
    assert_eq!(m.document().leaves().count(), 1, "Enter is a newline");
    // Enter on the trailing empty line leaves the block.
    m.split_block().unwrap();
    m.insert_text("after", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "```\na\n```\n\nafter");
    assert_invariants(&m);
}

#[test]
fn backspace_in_empty_code_block_makes_a_paragraph() {
    let mut m = model("x\n\n```\n\n```");
    let last = m.document().leaves().count() - 1;
    caret_in(&mut m, last, 0);
    m.delete(false).unwrap();
    m.insert_text("p", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "x\n\np");
    // Backspace at the start of the paragraph after a code block steps into
    // the code instead of deleting it.
    let mut m = model("```\ncode\n```\n\nafter");
    caret_in(&mut m, 1, 0);
    m.delete(false).unwrap();
    assert_eq!(m.source(), "```\ncode\n```\n\nafter");
    assert_eq!(m.caret().unwrap().offset, 4);
}

#[test]
fn backspace_at_start_of_code_block_moves_to_previous_block() {
    let source = "x\n\n```\ncode\n```";
    let mut m = model(source);
    let x = m.document().leaves().next().unwrap().id();
    caret_in(&mut m, 1, 0);
    m.delete(false).unwrap();
    assert_eq!(m.source(), source, "no merge");
    assert_eq!(m.caret(), Some(BlockPosition::new(x, 1)));
    // Delete at the end of the code block steps into the next block.
    let source = "```\ncode\n```\n\ny";
    let mut m = model(source);
    let y = m.document().leaves().nth(1).unwrap().id();
    caret_in(&mut m, 0, 4);
    m.delete(true).unwrap();
    assert_eq!(m.source(), source);
    assert_eq!(m.caret(), Some(BlockPosition::new(y, 0)));
}

#[test]
fn exiting_code_block_then_typing_is_two_undo_steps() {
    let mut m = model("```\ncode\n```");
    caret_in(&mut m, 0, 4);
    m.split_block().unwrap(); // newline
    m.split_block().unwrap(); // exit on the trailing empty line
    m.insert_text("p", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "```\ncode\n```\n\np");
    assert!(m.undo());
    assert_eq!(m.document().leaves().count(), 2, "typing undone alone");
    assert_eq!(m.source(), "```\ncode\n```\n\n");
    assert!(m.undo());
    // Undo the exit too: the newline belongs to the editable code body,
    // distinct from the exterior empty paragraph restored above.
    let code = m
        .document()
        .leaves()
        .find_map(|l| m.doc().plain(l.id()).map(str::to_string));
    assert_eq!(code.as_deref(), Some("code\n"), "then the exit");
}

#[test]
fn set_code_block_converts_paragraph() {
    let mut m = model("let x = 1;");
    caret_in(&mut m, 0, 3);
    m.run(EditorCommand::SetCodeBlock {
        lang: Some("rust".into()),
    })
    .unwrap();
    assert_eq!(m.source(), "```rust\nlet x = 1;\n```");
    assert_eq!(m.caret().unwrap().offset, 3);
    assert!(m.undo());
    assert_eq!(m.source(), "let x = 1;");
}

#[test]
fn code_blocks_round_trip() {
    for source in [
        "```rust\nfn main() {\n    println!(\"hi\");\n}\n```",
        "```\nplain\n\nwith blank line\n```",
        "````md\n```\nnested fence\n```\n````",
        "```\n**not bold** <u>x</u>\n```",
    ] {
        let m = model(source);
        assert_eq!(m.source(), source);
    }
}

#[test]
fn invalid_offsets_are_errors() {
    let mut m = model("é");
    let id = m.document().first_caret().block;
    assert_eq!(
        m.set_selection(BlockSelection::caret(BlockPosition::new(id, 1))),
        Err(SelectionError::Position(
            PositionError::NotOnGraphemeBoundary
        ))
    );
    assert_eq!(
        m.set_selection(BlockSelection::caret(BlockPosition::new(id, 9))),
        Err(SelectionError::Position(PositionError::OffsetOutOfBounds {
            len: 2
        }))
    );
    let root = m.document().root().id();
    assert_eq!(
        m.set_selection(BlockSelection::caret(BlockPosition::new(root, 0))),
        Err(SelectionError::Position(PositionError::NotACaretBlock(
            root
        )))
    );
}

#[test]
fn input_order_is_document_order() {
    let m = model("> a\n\n- b\n  - c\n\n| d | e |\n| --- | --- |\n| f | g |");
    let texts: Vec<_> = m.document().leaves().filter_map(|b| b.text()).collect();
    assert_eq!(texts, ["a", "b", "c", "d", "e", "f", "g"]);
}

/// Enter, Enter at the end of `leaf`, then type `x`; one undo reverts the exit.
fn exit_list_after(source: &str, leaf: usize, offset: usize) -> (String, BlockEditorModel) {
    let mut m = model(source);
    caret_in(&mut m, leaf, offset);
    m.split_block().unwrap();
    let before = m.source();
    let revision = m.revision();
    m.split_block().unwrap();
    assert_invariants(&m);
    assert!(m.revision() > revision);
    let after = m.source();
    assert!(m.undo());
    assert_eq!(m.source(), before, "exit is one undo step");
    assert!(m.redo());
    assert_eq!(m.source(), after);
    m.insert_text("x", EditOrigin::Typing).unwrap();
    (m.source(), m)
}

#[test]
fn enter_in_empty_last_item_exits_list() {
    let (source, m) = exit_list_after("- a\n- b", 1, 1);
    assert_eq!(source, "- a\n- b\n\nx");
    assert_invariants(&m);
}

#[test]
fn enter_in_empty_middle_item_splits_list() {
    let (source, m) = exit_list_after("1. a\n2. b", 0, 1);
    assert_eq!(source, "1. a\n\nx\n\n1. b");
    assert_invariants(&m);
}

#[test]
fn enter_in_empty_nested_item_outdents() {
    let (source, m) = exit_list_after("- a\n  - b", 1, 1);
    assert_eq!(source, "- a\n  - b\n- x");
    assert_invariants(&m);
}

#[test]
fn table_node_selection_moves_and_deletes() {
    let mut m = model("a\n\n| x | y |\n| --- | --- |\n| 1 | 2 |\n\nb");
    let original = m.source();
    let last = m.document().leaves().count() - 1;
    caret_in(&mut m, last, 0);
    m.delete(false).unwrap();
    assert_eq!(m.source(), original, "Backspace after a table selects it");
    assert!(matches!(m.selection(), BlockSelection::Blocks { .. }));
    assert!(m.run(EditorCommand::MoveBlocks { down: false }).unwrap());
    assert!(m.source().starts_with("| x | y |"), "{}", m.source());
    assert_invariants(&m);
    m.delete(false).unwrap();
    assert_eq!(m.source(), "a\n\nb");
    assert!(m.undo());
    assert!(m.source().starts_with("| x | y |"));
    assert_invariants(&m);
}

const GRID: &str = "| a | b |\n| :-: | --: |\n| 1 | 2 |";

fn table_ids(m: &BlockEditorModel) -> Vec<BlockId> {
    m.document().leaves().map(|b| b.id()).collect()
}

#[test]
fn table_alignment_round_trips() {
    for source in [
        GRID,
        "| l | c | r |\n| :-- | :-: | --: |\n| 1 | **2** | `3` |",
        "x\n\n| a |\n| --- |\n|  |\n\ny",
    ] {
        assert_eq!(model(source).source(), source);
    }
}

#[test]
fn typing_and_marks_in_cells() {
    let mut m = model(GRID);
    caret_in(&mut m, 2, 1);
    m.insert_text("0", EditOrigin::Typing).unwrap();
    m.split_block().ok(); // Enter in a cell does nothing.
    assert_eq!(m.source(), "| a | b |\n| :-: | --: |\n| 10 | 2 |");
    let ids = table_ids(&m);
    m.set_selection(BlockSelection::Text {
        anchor: BlockPosition::new(ids[2], 0),
        head: BlockPosition::new(ids[2], 2),
    })
    .unwrap();
    m.run(EditorCommand::ToggleMark(MarkKind::Italic)).unwrap();
    assert_eq!(m.source(), "| a | b |\n| :-: | --: |\n| *10* | 2 |");
    assert_invariants(&m);
}

#[test]
fn tab_moves_between_cells_and_adds_rows() {
    let mut m = model(GRID);
    caret_in(&mut m, 0, 0);
    m.run(EditorCommand::Indent).unwrap();
    let ids = table_ids(&m);
    assert_eq!(
        m.selection(),
        &BlockSelection::Text {
            anchor: BlockPosition::new(ids[1], 0),
            head: BlockPosition::new(ids[1], 1),
        },
        "Tab selects the next cell's text"
    );
    m.run(EditorCommand::Outdent).unwrap();
    assert_eq!(m.selection().clone(), {
        let len = 1;
        BlockSelection::Text {
            anchor: BlockPosition::new(ids[0], 0),
            head: BlockPosition::new(ids[0], len),
        }
    });
    caret_in(&mut m, 3, 1);
    m.run(EditorCommand::Indent).unwrap();
    assert_eq!(m.source(), format!("{GRID}\n|  |  |"));
    m.insert_text("z", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), format!("{GRID}\n| z |  |"));
    assert!(m.undo());
    assert!(m.undo());
    assert_eq!(m.source(), GRID);
    assert_invariants(&m);
}

#[test]
fn cell_selection_marks_clears_and_rejects_other_tables() {
    let mut m = model("| a | b | c |\n| --- | --- | --- |\n| 1 | 2 | 3 |");
    let ids = table_ids(&m);
    // b..2 is the 2×1... no: b (row 0, col 1) to 3 (row 1, col 2) is 2×2.
    m.set_selection(BlockSelection::Cells {
        anchor: ids[1],
        head: ids[5],
    })
    .unwrap();
    assert_eq!(
        m.selected_cells().unwrap(),
        vec![vec![ids[1], ids[2]], vec![ids[4], ids[5]]]
    );
    m.run(EditorCommand::ToggleMark(MarkKind::Bold)).unwrap();
    assert_eq!(
        m.source(),
        "| a | **b** | **c** |\n| --- | --- | --- |\n| 1 | **2** | **3** |"
    );
    m.delete(false).unwrap();
    assert_eq!(m.source(), "| a |  |  |\n| --- | --- | --- |\n| 1 |  |  |");
    assert_eq!(m.caret(), Some(BlockPosition::new(ids[1], 0)));
    // Typing over a cell selection clears it, then types into the first cell.
    m.set_selection(BlockSelection::Cells {
        anchor: ids[0],
        head: ids[3],
    })
    .unwrap();
    m.insert_text("q", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "| q |  |  |\n| --- | --- | --- |\n|  |  |  |");
    // A cell selection cannot leave its table.
    let mut two = model("| a |\n| --- |\n| 1 |\n\n| b |\n| --- |\n| 2 |");
    let ids = table_ids(&two);
    assert!(
        two.set_selection(BlockSelection::Cells {
            anchor: ids[0],
            head: ids[2],
        })
        .is_err()
    );
}

#[test]
fn insert_table_and_row_column_commands() {
    let mut m = model("");
    m.run(EditorCommand::InsertTable { rows: 2, cols: 2 })
        .unwrap();
    assert_eq!(m.source(), "|  |  |\n| --- | --- |\n|  |  |");
    m.insert_text("h", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "| h |  |\n| --- | --- |\n|  |  |");

    let mut m = model(GRID);
    caret_in(&mut m, 1, 0); // "b", right-aligned column
    m.run(EditorCommand::EditTable(TableEdit::AddColumnLeft))
        .unwrap();
    assert_eq!(
        m.source(),
        "| a |  | b |\n| :-: | --- | --: |\n| 1 |  | 2 |"
    );
    m.insert_text("n", EditOrigin::Typing).unwrap();
    assert_eq!(
        m.source(),
        "| a | n | b |\n| :-: | --- | --: |\n| 1 |  | 2 |"
    );
    m.run(EditorCommand::EditTable(TableEdit::RemoveColumn))
        .unwrap();
    assert_eq!(m.source(), GRID);
    m.run(EditorCommand::EditTable(TableEdit::AddColumnRight))
        .unwrap();
    assert_eq!(
        m.source(),
        "| a | b |  |\n| :-: | --: | --- |\n| 1 | 2 |  |"
    );
    assert!(m.undo());
    assert_eq!(m.source(), GRID);
    caret_in(&mut m, 2, 0);
    m.run(EditorCommand::EditTable(TableEdit::AddRowAbove))
        .unwrap();
    assert_eq!(m.source(), "| a | b |\n| :-: | --: |\n|  |  |\n| 1 | 2 |");
    m.run(EditorCommand::EditTable(TableEdit::RemoveRow))
        .unwrap();
    assert_eq!(m.source(), GRID);
    m.run(EditorCommand::EditTable(TableEdit::AddRowBelow))
        .unwrap();
    assert_eq!(m.source(), format!("{GRID}\n|  |  |"));
    assert_invariants(&m);
    // Removing the header row promotes the next row.
    let mut m = model(GRID);
    caret_in(&mut m, 0, 0);
    m.run(EditorCommand::EditTable(TableEdit::RemoveRow))
        .unwrap();
    assert_eq!(m.source(), "| 1 | 2 |\n| :-: | --: |");
    // Removing the only row removes the table.
    m.run(EditorCommand::EditTable(TableEdit::RemoveRow))
        .unwrap();
    assert_eq!(m.source(), "");
    assert!(m.undo());
    assert_eq!(m.source(), "| 1 | 2 |\n| :-: | --: |");
    assert_invariants(&m);
}

#[test]
fn typing_over_node_selected_blocks_replaces_them() {
    for (source, expected) in [
        ("a\n\n```rs\ncode\n```\n\nb", "a\n\nz\n\nb"),
        ("a\n\n---\n\nb", "a\n\nz\n\nb"),
        ("a\n\n![img](x.png)\n\nb", "a\n\nz\n\nb"),
        ("a\n\n| x |\n| --- |\n| 1 |\n\nb", "a\n\nz\n\nb"),
    ] {
        let mut m = model(source);
        let original = m.source();
        let target = m.document().root().children().nth(1).unwrap().id();
        m.set_selection(BlockSelection::Blocks {
            anchor: target,
            head: target,
        })
        .unwrap();
        m.insert_text("z", EditOrigin::Typing).unwrap();
        assert_eq!(m.source(), expected, "{source:?}");
        assert_invariants(&m);
        assert!(m.undo());
        assert_eq!(m.source(), original, "one undo step for {source:?}");
    }
}
