mod commands;
mod host;
mod projection;
mod props;
mod repro;
mod round_trip;
mod undo;
mod view;

use super::*;
use crate::text::MarkdownExtensions;

pub(super) fn model(source: &str) -> BlockEditorModel {
    BlockEditorModel::from_markdown(source, MarkdownExtensions::default()).unwrap()
}

/// Caret at `offset` in the `ix`-th caret leaf.
pub(super) fn caret_in(model: &mut BlockEditorModel, ix: usize, offset: usize) -> BlockPosition {
    let id = model.document().leaves().nth(ix).unwrap().id();
    let pos = BlockPosition::new(id, offset);
    model.set_selection(BlockSelection::caret(pos)).unwrap();
    pos
}

/// Caret at the end of the `ix`-th caret leaf.
pub(super) fn caret_at_end(model: &mut BlockEditorModel, ix: usize) -> BlockPosition {
    let leaf = model.document().leaves().nth(ix).unwrap();
    let len = leaf.text().map(str::len).unwrap_or(1);
    let id = leaf.id();
    let pos = BlockPosition::new(id, len);
    model.set_selection(BlockSelection::caret(pos)).unwrap();
    pos
}

pub(super) fn select(model: &mut BlockEditorModel, ix: usize, range: std::ops::Range<usize>) {
    let id = model.document().leaves().nth(ix).unwrap().id();
    model
        .set_selection(BlockSelection::Text {
            anchor: BlockPosition::new(id, range.start),
            head: BlockPosition::new(id, range.end),
        })
        .unwrap();
}

pub(super) fn assert_invariants(model: &BlockEditorModel) {
    model.doc().check_invariants().unwrap();
}
