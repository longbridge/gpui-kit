//! History, step mapping and order-index behaviour (exit criteria 4 and 5).

use super::*;
use crate::text::block_editor::{order::REBUILDS, step::Step};

fn generated(blocks: usize) -> String {
    (0..blocks)
        .map(|ix| format!("paragraph {ix}"))
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[test]
fn keystroke_in_large_document_records_one_text_step() {
    let mut m = model(&generated(1000));
    caret_at_end(&mut m, 500);
    m.insert_text("x", EditOrigin::Typing).unwrap();
    let steps = m.history.last_undo_steps().unwrap();
    assert_eq!(steps.len(), 1);
    assert!(matches!(steps[0], Step::ReplaceText { .. }));
}

#[test]
fn order_index_rebuilds_only_on_structural_steps() {
    let mut m = model(&generated(50));
    caret_at_end(&mut m, 10);
    let before = REBUILDS.with(|c| c.get());
    for ch in ["a", "b", "c"] {
        m.insert_text(ch, EditOrigin::Typing).unwrap();
    }
    m.delete(false).unwrap();
    assert_eq!(REBUILDS.with(|c| c.get()), before);
    m.split_block().unwrap();
    assert_eq!(REBUILDS.with(|c| c.get()), before + 1);
}

#[test]
fn backspace_coalesces_separately_from_typing() {
    let mut m = model("");
    m.insert_text("abc", EditOrigin::Typing).unwrap();
    m.delete(false).unwrap();
    m.delete(false).unwrap();
    assert_eq!(m.source(), "a");
    assert!(m.undo());
    assert_eq!(m.source(), "abc");
    assert!(m.undo());
    assert_eq!(m.source(), "");
    assert!(m.redo());
    assert!(m.redo());
    assert_eq!(m.source(), "a");
}

#[test]
fn new_edit_clears_redo() {
    let mut m = model("");
    m.insert_text("a", EditOrigin::Typing).unwrap();
    m.undo();
    assert!(m.can_redo());
    m.insert_text("b", EditOrigin::Typing).unwrap();
    assert!(!m.can_redo());
}

#[test]
fn undo_restores_selection() {
    let mut m = model("abc");
    select(&mut m, 0, 1..2);
    let selection = m.selection().clone();
    m.insert_text("x", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "axc");
    m.undo();
    assert_eq!(m.selection(), &selection);
}

#[test]
fn selection_maps_through_steps() {
    let mut m = model("abcd");
    select(&mut m, 0, 2..3);
    // A bracketed edit before the selection with no explicit caret: the
    // selection is mapped through the step.
    let leaf = m.document().first_caret().block;
    m.transact(EditOrigin::Api, |tx| {
        tx.replace_text(
            leaf,
            0..0,
            crate::text::block_editor::inline::InlineText::plain("xy", Default::default()),
        )?;
        Ok(None)
    })
    .unwrap();
    assert_eq!(
        m.selection(),
        &BlockSelection::Text {
            anchor: BlockPosition::new(leaf, 4),
            head: BlockPosition::new(leaf, 5),
        }
    );
}
