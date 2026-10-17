//! The review's model-level repros on #3049 (R1, R2, R3, R6, R7).

use super::*;

#[test]
fn r3_bold_typing_after_bold_stays_valid() {
    let mut m = model("**ab**");
    caret_at_end(&mut m, 0);
    m.insert_text("c", EditOrigin::Typing).unwrap();
    m.insert_text("d", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "**abcd**");
    let leaf = m.document().leaves().next().unwrap().id();
    let runs = m.doc().inline(leaf).unwrap().runs();
    assert_eq!(runs.len(), 1);
    assert!(runs[0].marks.bold);
}

const MIXED: &str = "# T\n\n- a\n\n```rs\ncode\n```";

#[test]
fn r2_select_all_type_removes_code_block() {
    let mut m = model(MIXED);
    m.run(EditorCommand::SelectAll).unwrap();
    m.insert_text("x", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "x");
    let leaves: Vec<_> = m.document().leaves().map(|b| b.kind()).collect();
    assert_eq!(leaves, [BlockKind::Paragraph]);
    assert_invariants(&m);
}

#[test]
fn r2_code_block_is_addressable() {
    let m = model(MIXED);
    let leaves: Vec<_> = m.document().leaves().collect();
    assert_eq!(
        leaves.iter().map(|b| b.kind()).collect::<Vec<_>>(),
        [
            BlockKind::Heading { level: 1 },
            BlockKind::Paragraph,
            BlockKind::CodeBlock
        ]
    );
    assert_eq!(leaves[2].text(), Some("code"));
    let pos = BlockPosition::new(leaves[2].id(), 0);
    assert_eq!(m.doc().validate(pos), Ok(pos));
    assert_eq!(
        leaves[1].parent().map(|p| p.kind()),
        Some(BlockKind::ListItem { checked: None })
    );
}

#[test]
fn r1_stale_position_errors_not_panics() {
    let mut m = model("a\n\nb\n\nc");
    let b = m.document().leaves().nth(1).unwrap().id();
    m.transact(EditOrigin::Api, |tx| {
        tx.remove_block(b)?;
        Ok(None)
    })
    .unwrap();
    assert_eq!(m.source(), "a\n\nc");
    let stale = BlockPosition::new(b, 0);
    assert_eq!(
        m.set_selection(BlockSelection::caret(stale)),
        Err(SelectionError::Position(PositionError::UnknownBlock(b)))
    );
}

#[test]
fn r1_position_from_before_set_source_is_unknown() {
    let mut m = model("same text");
    let old = m.document().first_caret();
    m.set_source("same text").unwrap();
    assert_eq!(
        m.set_selection(BlockSelection::caret(old)),
        Err(SelectionError::Position(PositionError::UnknownBlock(
            old.block
        )))
    );
    assert!(!m.can_undo());
}

#[test]
fn r1_ids_survive_undo() {
    let mut m = model("a\n\nb\n\nc");
    let b = m.document().leaves().nth(1).unwrap().id();
    m.transact(EditOrigin::Api, |tx| {
        tx.remove_block(b)?;
        Ok(None)
    })
    .unwrap();
    assert!(m.undo());
    assert_eq!(m.source(), "a\n\nb\n\nc");
    assert_eq!(m.document().leaves().nth(1).unwrap().id(), b);
    assert!(m.redo());
    assert_eq!(m.source(), "a\n\nc");
}

#[test]
fn r7_toggle_list_inside_blockquote() {
    let mut m = model("> para");
    caret_in(&mut m, 0, 2);
    assert!(m.run(EditorCommand::ToggleList { ordered: false }).unwrap());
    assert_eq!(m.source(), "> - para");
    assert_invariants(&m);
}

#[test]
fn r7_outdent_nested_list_item() {
    let mut m = model("- a\n  - b");
    caret_in(&mut m, 1, 0);
    assert!(m.run(EditorCommand::Outdent).unwrap());
    assert_eq!(m.source(), "- a\n- b");
    assert_invariants(&m);
}

#[test]
fn r7_move_block_inside_list() {
    let mut m = model("- a\n- b");
    caret_in(&mut m, 1, 0);
    assert!(m.run(EditorCommand::MoveBlocks { down: false }).unwrap());
    assert_eq!(m.source(), "- b\n- a");
    assert_invariants(&m);
}

#[test]
fn r6_typing_coalesces_into_one_undo() {
    let mut m = model("");
    for ch in ["h", "e", "l", "l", "o"] {
        m.insert_text(ch, EditOrigin::Typing).unwrap();
    }
    assert_eq!(m.source(), "hello");
    assert!(m.undo());
    assert_eq!(m.source(), "");
    assert!(!m.can_undo());
}

#[test]
fn r6_caret_move_breaks_coalescing() {
    let mut m = model("");
    m.insert_text("a", EditOrigin::Typing).unwrap();
    m.insert_text("b", EditOrigin::Typing).unwrap();
    m.break_coalescing();
    m.insert_text("c", EditOrigin::Typing).unwrap();
    m.insert_text("d", EditOrigin::Typing).unwrap();
    assert!(m.undo());
    assert_eq!(m.source(), "ab");
}

#[test]
fn r6_mark_toggle_is_its_own_undo_step() {
    let mut m = model("abc");
    select(&mut m, 0, 1..2);
    m.run(EditorCommand::ToggleMark(MarkKind::Bold)).unwrap();
    assert_eq!(m.source(), "a**b**c");
    assert_eq!(m.mark_state(MarkKind::Bold), MarkState::On);
    assert!(m.undo());
    assert_eq!(m.source(), "abc");
}
