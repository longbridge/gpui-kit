use std::cmp::Ordering;

use super::{
    document::{BlockContent, EditorDocument},
    id::BlockId,
};

/// Caret side at a soft wrap or mark boundary. Not part of ordering.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Affinity {
    Upstream,
    #[default]
    Downstream,
}

/// A caret location. Plain value type: its fields are the definition.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BlockPosition {
    pub block: BlockId,
    /// UTF-8 byte offset into the block's flat text, on a grapheme boundary.
    /// Byte offset in the leaf's text (inline text or code). For atomic
    /// blocks: 0 = before, 1 = after.
    pub offset: usize,
    pub affinity: Affinity,
}

impl BlockPosition {
    pub fn new(block: BlockId, offset: usize) -> Self {
        Self {
            block,
            offset,
            affinity: Affinity::Downstream,
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PositionError {
    /// Removed, from another document, or from before `set_source`.
    UnknownBlock(BlockId),
    /// The block holds other blocks (e.g. List, TableRow).
    NotACaretBlock(BlockId),
    OffsetOutOfBounds {
        len: usize,
    },
    NotOnGraphemeBoundary,
}

impl EditorDocument {
    pub(crate) fn validate(&self, p: BlockPosition) -> Result<BlockPosition, PositionError> {
        let entry = self
            .get(p.block)
            .ok_or(PositionError::UnknownBlock(p.block))?;
        match &entry.content {
            BlockContent::Children(_) => Err(PositionError::NotACaretBlock(p.block)),
            BlockContent::Inline(inline) => {
                if p.offset > inline.len() {
                    Err(PositionError::OffsetOutOfBounds { len: inline.len() })
                } else if !inline.is_grapheme_boundary(p.offset) {
                    Err(PositionError::NotOnGraphemeBoundary)
                } else {
                    Ok(p)
                }
            }
            BlockContent::Plain(code) => {
                if p.offset > code.len() {
                    Err(PositionError::OffsetOutOfBounds { len: code.len() })
                } else if !code.is_char_boundary(p.offset) {
                    Err(PositionError::NotOnGraphemeBoundary)
                } else {
                    Ok(p)
                }
            }
            BlockContent::Atom(_) => {
                if p.offset > 1 {
                    Err(PositionError::OffsetOutOfBounds { len: 1 })
                } else {
                    Ok(p)
                }
            }
        }
    }

    /// The nearest valid position, falling back to the document start.
    pub(crate) fn clamp(&self, p: BlockPosition) -> BlockPosition {
        match self.validate(p) {
            Ok(p) => p,
            Err(PositionError::OffsetOutOfBounds { len }) => BlockPosition { offset: len, ..p },
            Err(PositionError::NotOnGraphemeBoundary) => {
                let offset = match self.plain(p.block) {
                    Some(code) => (0..=p.offset.min(code.len()))
                        .rev()
                        .find(|i| code.is_char_boundary(*i))
                        .unwrap_or(0),
                    None => self
                        .inline(p.block)
                        .and_then(|inline| inline.prev_grapheme(p.offset))
                        .unwrap_or(0),
                };
                BlockPosition { offset, ..p }
            }
            Err(_) => self.first_caret(),
        }
    }

    /// Document order, O(1) via the order index.
    pub(crate) fn cmp_positions(
        &self,
        a: BlockPosition,
        b: BlockPosition,
    ) -> Result<Ordering, PositionError> {
        let a = self.validate(a)?;
        let b = self.validate(b)?;
        let rank = |p: BlockPosition| {
            self.order
                .rank(p.block)
                .ok_or(PositionError::UnknownBlock(p.block))
        };
        Ok(rank(a)?.cmp(&rank(b)?).then(a.offset.cmp(&b.offset)))
    }
}
