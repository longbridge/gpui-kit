//! Table cell editing (Phase 4c): cell navigation, `BlockSelection::Cells`,
//! and row/column commands. Cells are ordinary inline leaves; a table is
//! `Table > TableRow > TableCell`, the first row being the header.

use super::{
    command::CommandError,
    document::{BlockContent, EditorDocument},
    editor::BlockEditorModel,
    id::BlockId,
    inline::InlineText,
    kind::BlockKindData,
    position::{BlockPosition, PositionError},
    selection::BlockSelection,
    step::DetachedBlock,
    transaction::EditOrigin,
};
use crate::text::node::ColumnumnAlign;

/// A table's cells as rows.
pub(crate) fn grid(doc: &EditorDocument, table: BlockId) -> Vec<Vec<BlockId>> {
    doc.children(table)
        .iter()
        .map(|row| doc.children(*row).to_vec())
        .collect()
}

pub(crate) fn table_of(doc: &EditorDocument, cell: BlockId) -> Option<BlockId> {
    if doc.kind(cell) != Some(&BlockKindData::TableCell) {
        return None;
    }
    doc.path(cell)
        .into_iter()
        .rev()
        .find(|id| matches!(doc.kind(*id), Some(BlockKindData::Table { .. })))
}

/// `(row, column)` of a cell in its table's grid.
pub(crate) fn cell_coords(
    doc: &EditorDocument,
    table: BlockId,
    cell: BlockId,
) -> Option<(usize, usize)> {
    grid(doc, table)
        .iter()
        .enumerate()
        .find_map(|(r, row)| row.iter().position(|c| *c == cell).map(|c| (r, c)))
}

fn empty_cell() -> DetachedBlock {
    DetachedBlock::leaf(
        BlockKindData::TableCell,
        BlockContent::Inline(InlineText::new()),
    )
}

fn empty_row(cols: usize) -> DetachedBlock {
    DetachedBlock::container(
        BlockKindData::TableRow,
        (0..cols).map(|_| empty_cell()).collect(),
    )
}

fn aligns(doc: &EditorDocument, table: BlockId) -> Vec<ColumnumnAlign> {
    match doc.kind(table) {
        Some(BlockKindData::Table { aligns }) => aligns.clone(),
        _ => Vec::new(),
    }
}

/// Which table edit to make at the caret's cell.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TableEdit {
    AddRowAbove,
    AddRowBelow,
    RemoveRow,
    AddColumnLeft,
    AddColumnRight,
    RemoveColumn,
}

impl BlockEditorModel {
    /// The cell holding the caret / selection head, and its table.
    pub(crate) fn current_cell(&self) -> Option<(BlockId, BlockId)> {
        let cell = match self.selection() {
            BlockSelection::Text { head, .. } => head.block,
            BlockSelection::Cells { head, .. } => *head,
            BlockSelection::Blocks { .. } => return None,
        };
        Some((table_of(&self.doc, cell)?, cell))
    }

    /// The cells of a `BlockSelection::Cells` rectangle, row by row.
    pub fn selected_cells(&self) -> Option<Vec<Vec<BlockId>>> {
        let BlockSelection::Cells { anchor, head } = self.selection() else {
            return None;
        };
        let table = table_of(&self.doc, *anchor)?;
        let (ar, ac) = cell_coords(&self.doc, table, *anchor)?;
        let (hr, hc) = cell_coords(&self.doc, table, *head)?;
        let grid = grid(&self.doc, table);
        Some(
            (ar.min(hr)..=ar.max(hr))
                .map(|r| {
                    (ac.min(hc)..=ac.max(hc))
                        .filter_map(|c| grid[r].get(c).copied())
                        .collect()
                })
                .collect(),
        )
    }

    /// Clear the contents of the selected cells (one undo step); the caret
    /// goes to the first of them.
    pub(crate) fn clear_cells(&mut self, origin: EditOrigin) -> Result<bool, CommandError> {
        let Some(cells) = self.selected_cells() else {
            return Ok(false);
        };
        let first = cells.first().and_then(|r| r.first()).copied();
        self.transact(origin, |tx| {
            for cell in cells.iter().flatten() {
                let len = tx.doc.leaf_len(*cell).unwrap_or(0);
                if len > 0 {
                    tx.replace_text(*cell, 0..len, InlineText::new())?;
                }
            }
            Ok(first.map(|c| BlockSelection::caret(BlockPosition::new(c, 0))))
        })?;
        if let Some(first) = first {
            // A no-op transaction leaves the selection: collapse it anyway.
            let _ = self.set_selection(BlockSelection::caret(BlockPosition::new(first, 0)));
        }
        Ok(true)
    }

    /// The selected cells as a Markdown table (header = first selected row).
    pub(crate) fn cells_markdown(&self) -> Option<String> {
        let cells = self.selected_cells()?;
        let cell_text = |id: &BlockId| {
            let inline = self.doc.inline(*id).cloned().unwrap_or_default();
            super::serialize::inline_markdown(&inline)
        };
        let mut out = String::new();
        for (r, row) in cells.iter().enumerate() {
            let line: Vec<String> = row.iter().map(cell_text).collect();
            out.push_str(&format!("| {} |\n", line.join(" | ")));
            if r == 0 {
                let sep: Vec<&str> = row.iter().map(|_| "---").collect();
                out.push_str(&format!("| {} |\n", sep.join(" | ")));
            }
        }
        Some(out.trim_end().to_string())
    }

    /// Tab / Shift+Tab in a cell: move to the next / previous cell (all of
    /// its text selected, like spreadsheets do). Tab in the last cell adds
    /// a row. Returns `false` when the caret is not in a cell.
    pub(crate) fn tab_cell(&mut self, back: bool) -> Result<bool, CommandError> {
        let Some((table, cell)) = self.current_cell() else {
            return Ok(false);
        };
        let cells: Vec<BlockId> = grid(&self.doc, table).into_iter().flatten().collect();
        let Some(ix) = cells.iter().position(|c| *c == cell) else {
            return Ok(false);
        };
        let target = if back {
            match ix.checked_sub(1) {
                Some(i) => cells[i],
                None => return Ok(true),
            }
        } else if let Some(next) = cells.get(ix + 1) {
            *next
        } else {
            let cols = self
                .doc
                .children(*self.doc.children(table).last().unwrap_or(&table))
                .len();
            let row = empty_row(cols.max(1));
            let first = match &row.content {
                super::step::DetachedContent::Children(c) => c[0].id,
                _ => unreachable!(),
            };
            let rows = self.doc.children(table).len();
            self.transact(EditOrigin::Command("table_add_row"), |tx| {
                tx.insert(table, rows, vec![row])?;
                Ok(Some(BlockSelection::caret(BlockPosition::new(first, 0))))
            })?;
            return Ok(true);
        };
        let len = self.doc.leaf_len(target).unwrap_or(0);
        self.set_selection(BlockSelection::Text {
            anchor: BlockPosition::new(target, 0),
            head: BlockPosition::new(target, len),
        })?;
        Ok(true)
    }

    /// Insert an empty `rows` × `cols` table (header row included) after the
    /// caret's top-level-sibling block, with the caret in its first cell.
    pub(crate) fn insert_table(&mut self, rows: usize, cols: usize) -> Result<bool, CommandError> {
        let pos = self
            .caret()
            .ok_or(CommandError::Unsupported("block selection"))?;
        if table_of(&self.doc, pos.block).is_some() {
            return Err(CommandError::Unsupported("table inside a table"));
        }
        let (rows, cols) = (rows.max(1), cols.max(1));
        let table = DetachedBlock::container(
            BlockKindData::Table {
                aligns: vec![ColumnumnAlign::default(); cols],
            },
            (0..rows).map(|_| empty_row(cols)).collect(),
        );
        let mut ids = Vec::new();
        table.ids(&mut ids);
        let first = ids[2];
        let empty = self.doc.leaf_len(pos.block) == Some(0)
            && self.doc.kind(pos.block) == Some(&BlockKindData::Paragraph);
        self.transact(EditOrigin::Command("insert_table"), |tx| {
            let (parent, index) = tx
                .doc
                .index_in_parent(pos.block)
                .ok_or(PositionError::UnknownBlock(pos.block))?;
            tx.insert(parent, index + 1, vec![table])?;
            // An empty paragraph is replaced by the table.
            if empty {
                tx.remove_block(pos.block)?;
            }
            Ok(Some(BlockSelection::caret(BlockPosition::new(first, 0))))
        })?;
        Ok(true)
    }

    /// Add or remove a row or column at the caret's cell. Removing the last
    /// row or column removes the table.
    pub(crate) fn edit_table(&mut self, edit: TableEdit) -> Result<bool, CommandError> {
        let Some((table, cell)) = self.current_cell() else {
            return Err(CommandError::Unsupported("caret is not in a table"));
        };
        let Some((r, c)) = cell_coords(&self.doc, table, cell) else {
            return Ok(false);
        };
        let grid = grid(&self.doc, table);
        let rows = grid.len();
        let cols = grid.iter().map(Vec::len).max().unwrap_or(0);
        let row_ids = self.doc.children(table).to_vec();
        let mut aligns = aligns(&self.doc, table);
        aligns.resize(cols, ColumnumnAlign::default());
        let remove_table = matches!(edit, TableEdit::RemoveRow if rows == 1)
            || matches!(edit, TableEdit::RemoveColumn if cols == 1);
        if remove_table {
            self.set_selection(BlockSelection::Blocks {
                anchor: table,
                head: table,
            })?;
            return self.run(super::command::EditorCommand::DeleteSelection);
        }
        self.transact(EditOrigin::Command("table_edit"), |tx| {
            let caret_cell = match edit {
                TableEdit::AddRowAbove | TableEdit::AddRowBelow => {
                    let at = if edit == TableEdit::AddRowBelow {
                        r + 1
                    } else {
                        r
                    };
                    let row = empty_row(cols);
                    let mut ids = Vec::new();
                    row.ids(&mut ids);
                    tx.insert(table, at, vec![row])?;
                    ids[1 + c.min(cols - 1)]
                }
                TableEdit::RemoveRow => {
                    tx.remove(table, r..r + 1)?;
                    let next = if r < rows - 1 { r + 1 } else { r - 1 };
                    grid[next][c.min(grid[next].len().saturating_sub(1))]
                }
                TableEdit::AddColumnLeft | TableEdit::AddColumnRight => {
                    let at = if edit == TableEdit::AddColumnRight {
                        c + 1
                    } else {
                        c
                    };
                    let mut caret_cell = cell;
                    for (ri, row) in row_ids.iter().enumerate() {
                        let new = empty_cell();
                        if ri == r {
                            caret_cell = new.id;
                        }
                        let at = at.min(tx.doc.children(*row).len());
                        tx.insert(*row, at, vec![new])?;
                    }
                    aligns.insert(at, ColumnumnAlign::default());
                    tx.set_kind(
                        table,
                        BlockKindData::Table {
                            aligns: aligns.clone(),
                        },
                    )?;
                    caret_cell
                }
                TableEdit::RemoveColumn => {
                    for row in &row_ids {
                        if c < tx.doc.children(*row).len() {
                            tx.remove(*row, c..c + 1)?;
                        }
                    }
                    aligns.remove(c);
                    tx.set_kind(
                        table,
                        BlockKindData::Table {
                            aligns: aligns.clone(),
                        },
                    )?;
                    let nc = if c < cols - 1 { c + 1 } else { c - 1 };
                    grid[r][nc.min(grid[r].len() - 1)]
                }
            };
            Ok(Some(BlockSelection::caret(BlockPosition::new(
                caret_cell, 0,
            ))))
        })?;
        Ok(true)
    }

    /// Enter in a cell (D13). Returns `false` when the caret is not in a
    /// cell. Each branch is one undo step:
    /// - a body row whose cells are all empty: remove it and leave the
    ///   table to a new paragraph after it;
    /// - the last row: append an empty row, caret in its first cell;
    /// - any other row: the same column of the next row.
    pub(crate) fn table_enter(&mut self) -> Result<bool, CommandError> {
        let Some((table, cell)) = self.current_cell() else {
            return Ok(false);
        };
        let Some((r, c)) = cell_coords(&self.doc, table, cell) else {
            return Ok(false);
        };
        let grid = grid(&self.doc, table);
        let row_empty = grid[r]
            .iter()
            .all(|id| self.doc.leaf_len(*id).unwrap_or(0) == 0);
        if r > 0 && row_empty {
            let para = empty_paragraph();
            let target = para.id;
            self.transact(EditOrigin::Command("table_exit_row"), |tx| {
                tx.remove(table, r..r + 1)?;
                let (parent, index) = tx
                    .doc
                    .index_in_parent(table)
                    .ok_or(PositionError::UnknownBlock(table))?;
                tx.insert(parent, index + 1, vec![para])?;
                Ok(Some(BlockSelection::caret(BlockPosition::new(target, 0))))
            })?;
            return Ok(true);
        }
        if r + 1 == grid.len() {
            let cols = grid.iter().map(Vec::len).max().unwrap_or(1).max(1);
            let row = empty_row(cols);
            let mut ids = Vec::new();
            row.ids(&mut ids);
            let first = ids[1];
            let rows = grid.len();
            self.transact(EditOrigin::Command("table_add_row"), |tx| {
                tx.insert(table, rows, vec![row])?;
                Ok(Some(BlockSelection::caret(BlockPosition::new(first, 0))))
            })?;
            return Ok(true);
        }
        let next = &grid[r + 1];
        let target = next[c.min(next.len().saturating_sub(1))];
        self.set_selection(BlockSelection::caret(BlockPosition::new(target, 0)))?;
        Ok(true)
    }

    /// Shift+Enter in a cell: a soft line break (`<br>` in Markdown).
    pub(crate) fn cell_soft_break(&mut self) -> Result<bool, CommandError> {
        let Some((_, cell)) = self.current_cell() else {
            return Ok(false);
        };
        let BlockSelection::Text { anchor, head } = self.selection().clone() else {
            return Ok(false);
        };
        if anchor.block != cell || head.block != cell {
            return Ok(false);
        }
        let (lo, hi) = (
            anchor.offset.min(head.offset),
            anchor.offset.max(head.offset),
        );
        let marks = self
            .doc
            .inline(cell)
            .and_then(|i| i.mark_before(lo).cloned())
            .unwrap_or_default();
        self.transact(EditOrigin::Command("cell_soft_break"), |tx| {
            tx.replace_text(cell, lo..hi, InlineText::plain("\n", marks))?;
            Ok(Some(BlockSelection::caret(BlockPosition::new(
                cell,
                lo + 1,
            ))))
        })?;
        Ok(true)
    }

    /// Down past the last row or Cmd/Ctrl+Enter: the caret goes to the
    /// text block after the table, creating an empty paragraph (one undo
    /// step) when there is none. Returns `false` outside a table.
    pub(crate) fn exit_table(&mut self) -> Result<bool, CommandError> {
        let Some((table, _)) = self.current_cell() else {
            return Ok(false);
        };
        let Some((parent, index)) = self.doc.index_in_parent(table) else {
            return Ok(false);
        };
        if let Some(next) = self.doc.children(parent).get(index + 1).copied()
            && self.doc.inline(next).is_some()
        {
            self.set_selection(BlockSelection::caret(BlockPosition::new(next, 0)))?;
            return Ok(true);
        }
        let para = empty_paragraph();
        let target = para.id;
        self.transact(EditOrigin::Command("table_exit"), |tx| {
            tx.insert(parent, index + 1, vec![para])?;
            Ok(Some(BlockSelection::caret(BlockPosition::new(target, 0))))
        })?;
        Ok(true)
    }

    /// Whether the caret is in the last row of a table.
    pub(crate) fn in_last_table_row(&self) -> bool {
        self.current_cell().is_some_and(|(table, cell)| {
            cell_coords(&self.doc, table, cell)
                .is_some_and(|(r, _)| r + 1 == self.doc.children(table).len())
        })
    }

    /// The table input rule (D13): a paragraph `| a | b |` becomes a table
    /// with those cells as the header row and one empty body row, caret in
    /// its first cell; no alignment. One undo step. Returns `false` when
    /// `block` is not such a paragraph.
    pub(crate) fn paragraph_to_table(&mut self, block: BlockId) -> Result<bool, CommandError> {
        if self.doc.kind(block) != Some(&BlockKindData::Paragraph)
            || table_of(&self.doc, block).is_some()
        {
            return Ok(false);
        }
        let Some(inline) = self.doc.inline(block).cloned() else {
            return Ok(false);
        };
        let Some(ranges) = table_row_cells(inline.text()) else {
            return Ok(false);
        };
        let cols = ranges.len();
        let header = DetachedBlock::container(
            BlockKindData::TableRow,
            ranges
                .into_iter()
                .map(|r| {
                    DetachedBlock::leaf(
                        BlockKindData::TableCell,
                        BlockContent::Inline(inline.slice(r)),
                    )
                })
                .collect(),
        );
        let body = empty_row(cols);
        let mut ids = Vec::new();
        body.ids(&mut ids);
        let first = ids[1];
        let table = DetachedBlock::container(
            BlockKindData::Table {
                aligns: vec![ColumnumnAlign::None; cols],
            },
            vec![header, body],
        );
        self.transact(EditOrigin::InputRule("table"), |tx| {
            let (parent, index) = tx
                .doc
                .index_in_parent(block)
                .ok_or(PositionError::UnknownBlock(block))?;
            tx.remove_block(block)?;
            tx.insert(parent, index, vec![table])?;
            Ok(Some(BlockSelection::caret(BlockPosition::new(first, 0))))
        })?;
        Ok(true)
    }
}

fn empty_paragraph() -> DetachedBlock {
    DetachedBlock::leaf(
        BlockKindData::Paragraph,
        BlockContent::Inline(InlineText::new()),
    )
}

/// The trimmed byte ranges of the cells of a `| a | b |` line, if it is one
/// (starts and ends with `|`, at least one cell; `\|` does not split).
pub(crate) fn table_row_cells(text: &str) -> Option<Vec<std::ops::Range<usize>>> {
    let start = text.len() - text.trim_start().len();
    let end = text.trim_end().len();
    let line = &text[start..end];
    if line.len() < 2 || !line.starts_with('|') || !line.ends_with('|') || line.contains('\n') {
        return None;
    }
    let mut bars = Vec::new();
    let bytes = text.as_bytes();
    for i in start..end {
        if bytes[i] == b'|' && (i == 0 || bytes[i - 1] != b'\\') {
            bars.push(i);
        }
    }
    if bars.len() < 2 {
        return None;
    }
    Some(
        bars.windows(2)
            .map(|w| {
                let (a, b) = (w[0] + 1, w[1]);
                let cell = &text[a..b];
                let lead = cell.len() - cell.trim_start().len();
                a + lead..a + cell.trim_end().len()
            })
            .map(|r| if r.start > r.end { r.start..r.start } else { r })
            .collect(),
    )
}
