use super::{
    document::EditorDocument,
    id::BlockId,
    position::{BlockPosition, PositionError},
};

#[derive(Clone, PartialEq, Debug)]
pub enum BlockSelection {
    /// Text range, possibly spanning blocks; `anchor == head` is a caret.
    /// Atoms and code blocks between the endpoints count as fully covered.
    Text {
        anchor: BlockPosition,
        head: BlockPosition,
    },
    /// Whole blocks as objects. Endpoints must be siblings.
    Blocks { anchor: BlockId, head: BlockId },
    /// A rectangle of table cells (both ends in the same table), from
    /// dragging or Shift+arrows across cells. Copy gives a Markdown table;
    /// delete clears the cells' contents.
    Cells { anchor: BlockId, head: BlockId },
}

impl BlockSelection {
    pub fn caret(position: BlockPosition) -> Self {
        Self::Text {
            anchor: position,
            head: position,
        }
    }

    pub fn is_collapsed(&self) -> bool {
        matches!(self, Self::Text { anchor, head } if anchor == head)
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelectionError {
    Position(PositionError),
    UnknownBlock(BlockId),
    NotSiblings,
}

impl From<PositionError> for SelectionError {
    fn from(error: PositionError) -> Self {
        Self::Position(error)
    }
}

/// A text selection ordered in document order.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct ResolvedSelection {
    pub(crate) start: BlockPosition,
    pub(crate) end: BlockPosition,
}

impl ResolvedSelection {
    pub(crate) fn is_collapsed(&self) -> bool {
        self.start.block == self.end.block && self.start.offset == self.end.offset
    }
}

impl EditorDocument {
    pub(crate) fn validate_selection(
        &self,
        selection: &BlockSelection,
    ) -> Result<(), SelectionError> {
        match selection {
            BlockSelection::Text { anchor, head } => {
                self.validate(*anchor)?;
                self.validate(*head)?;
            }
            BlockSelection::Blocks { anchor, head } => {
                let a = self
                    .get(*anchor)
                    .ok_or(SelectionError::UnknownBlock(*anchor))?;
                let b = self.get(*head).ok_or(SelectionError::UnknownBlock(*head))?;
                if a.parent != b.parent || a.parent.is_none() {
                    return Err(SelectionError::NotSiblings);
                }
            }
            BlockSelection::Cells { anchor, head } => {
                let table = |id: BlockId| super::table::table_of(self, id);
                match (table(*anchor), table(*head)) {
                    (Some(a), Some(b)) if a == b => {}
                    (None, _) => return Err(SelectionError::UnknownBlock(*anchor)),
                    _ => return Err(SelectionError::NotSiblings),
                }
            }
        }
        Ok(())
    }

    /// Resolve to an ordered text range. A block selection covers its blocks'
    /// first to last caret.
    pub(crate) fn resolve_selection(
        &self,
        selection: &BlockSelection,
    ) -> Result<ResolvedSelection, SelectionError> {
        self.validate_selection(selection)?;
        let (a, b) = match selection {
            BlockSelection::Text { anchor, head } => (*anchor, *head),
            BlockSelection::Blocks { anchor, head } => {
                let first = |id: BlockId| self.first_leaf_in(id);
                let last = |id: BlockId| self.last_leaf_in(id);
                let (lo, hi) = if self.cmp_positions(first(*anchor), first(*head))?.is_le() {
                    (*anchor, *head)
                } else {
                    (*head, *anchor)
                };
                (first(lo), last(hi))
            }
            BlockSelection::Cells { anchor, head } => (
                BlockPosition::new(*anchor, 0),
                BlockPosition::new(*head, self.leaf_len(*head).unwrap_or(0)),
            ),
        };
        if self.cmp_positions(a, b)?.is_le() {
            Ok(ResolvedSelection { start: a, end: b })
        } else {
            Ok(ResolvedSelection { start: b, end: a })
        }
    }

    pub(crate) fn first_leaf_in(&self, id: BlockId) -> BlockPosition {
        let mut current = id;
        while let Some(first) = self.children(current).first() {
            current = *first;
        }
        BlockPosition::new(current, 0)
    }

    pub(crate) fn last_leaf_in(&self, id: BlockId) -> BlockPosition {
        let mut current = id;
        while let Some(last) = self.children(current).last() {
            current = *last;
        }
        BlockPosition::new(current, self.leaf_len(current).unwrap_or(0))
    }
}
