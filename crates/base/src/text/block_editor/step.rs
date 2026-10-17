use std::ops::Range;

use super::{
    document::{BlockContent, EditorDocument},
    id::BlockId,
    inline::InlineText,
    kind::BlockKindData,
    position::BlockPosition,
};

/// A subtree taken out of the document. It keeps its ids, so undoing a
/// removal brings back the same `BlockId`s (design §5.1).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DetachedBlock {
    pub(crate) id: BlockId,
    pub(crate) kind: BlockKindData,
    pub(crate) content: DetachedContent,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DetachedContent {
    Children(Vec<DetachedBlock>),
    Leaf(BlockContent),
}

impl DetachedBlock {
    /// A new leaf with a fresh id. Ids are allocated when a transaction is
    /// built and recorded in its steps, so redo reproduces them.
    pub(crate) fn leaf(kind: BlockKindData, content: BlockContent) -> Self {
        Self {
            id: BlockId::next(),
            kind,
            content: DetachedContent::Leaf(content),
        }
    }

    pub(crate) fn container(kind: BlockKindData, children: Vec<DetachedBlock>) -> Self {
        Self {
            id: BlockId::next(),
            kind,
            content: DetachedContent::Children(children),
        }
    }

    pub(crate) fn paragraph(inline: InlineText) -> Self {
        Self::leaf(BlockKindData::Paragraph, BlockContent::Inline(inline))
    }

    /// Every id in the subtree.
    pub(crate) fn ids(&self, out: &mut Vec<BlockId>) {
        out.push(self.id);
        if let DetachedContent::Children(children) = &self.content {
            for child in children {
                child.ids(out);
            }
        }
    }
}

/// An invertible edit. `SplitBlock`, `JoinBlocks`, `MoveBlocks` and mark
/// changes from the design are composites of these four in the spike (see
/// the PR notes): that kept inversion trivial.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Step {
    ReplaceText {
        block: BlockId,
        range: Range<usize>,
        insert: InlineText,
    },
    InsertBlocks {
        parent: BlockId,
        index: usize,
        blocks: Vec<DetachedBlock>,
    },
    RemoveBlocks {
        parent: BlockId,
        range: Range<usize>,
    },
    SetKind {
        block: BlockId,
        kind: BlockKindData,
    },
    /// Edit a code block's text (Phase 4b).
    ReplacePlain {
        block: BlockId,
        range: Range<usize>,
        insert: String,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StepError {
    UnknownBlock(BlockId),
    NotText(BlockId),
    OutOfRange,
    KindMismatch,
}

/// How a step moves positions.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum StepMap {
    Identity,
    Text {
        block: BlockId,
        range: Range<usize>,
        insert_len: usize,
    },
    Removed(Vec<BlockId>),
}

impl StepMap {
    /// Map a position through the step. `None` when its block was removed.
    pub(crate) fn map(&self, p: BlockPosition) -> Option<BlockPosition> {
        match self {
            Self::Identity => Some(p),
            Self::Text {
                block,
                range,
                insert_len,
            } if *block == p.block => {
                let offset = if p.offset <= range.start {
                    p.offset
                } else if p.offset >= range.end {
                    p.offset - range.len() + insert_len
                } else {
                    range.start + insert_len
                };
                Some(BlockPosition { offset, ..p })
            }
            Self::Text { .. } => Some(p),
            Self::Removed(ids) => (!ids.contains(&p.block)).then_some(p),
        }
    }
}

impl Step {
    /// Apply the step and return its inverse and position map.
    pub(crate) fn apply(&self, doc: &mut EditorDocument) -> Result<(Step, StepMap), StepError> {
        let result = self.apply_inner(doc)?;
        let touched = match self {
            Self::ReplaceText { block, .. }
            | Self::SetKind { block, .. }
            | Self::ReplacePlain { block, .. } => *block,
            Self::InsertBlocks { parent, .. } | Self::RemoveBlocks { parent, .. } => *parent,
        };
        doc.touch(touched);
        Ok(result)
    }

    fn apply_inner(&self, doc: &mut EditorDocument) -> Result<(Step, StepMap), StepError> {
        match self {
            Self::ReplaceText {
                block,
                range,
                insert,
            } => {
                let entry = doc.get_mut(*block).ok_or(StepError::UnknownBlock(*block))?;
                let BlockContent::Inline(inline) = &mut entry.content else {
                    return Err(StepError::NotText(*block));
                };
                if range.start > range.end
                    || range.end > inline.len()
                    || !inline.text().is_char_boundary(range.start)
                    || !inline.text().is_char_boundary(range.end)
                {
                    return Err(StepError::OutOfRange);
                }
                let removed = inline.replace(range.clone(), insert.clone());
                let inverse = Step::ReplaceText {
                    block: *block,
                    range: range.start..range.start + insert.len(),
                    insert: removed,
                };
                let map = StepMap::Text {
                    block: *block,
                    range: range.clone(),
                    insert_len: insert.len(),
                };
                Ok((inverse, map))
            }
            Self::InsertBlocks {
                parent,
                index,
                blocks,
            } => {
                if doc.get(*parent).is_none() {
                    return Err(StepError::UnknownBlock(*parent));
                }
                if *index > doc.children(*parent).len() {
                    return Err(StepError::OutOfRange);
                }
                for (ix, block) in blocks.iter().enumerate() {
                    if !doc.attach(*parent, index + ix, block.clone()) {
                        return Err(StepError::OutOfRange);
                    }
                }
                let inverse = Step::RemoveBlocks {
                    parent: *parent,
                    range: *index..index + blocks.len(),
                };
                Ok((inverse, StepMap::Identity))
            }
            Self::RemoveBlocks { parent, range } => {
                if doc.get(*parent).is_none() {
                    return Err(StepError::UnknownBlock(*parent));
                }
                if range.end > doc.children(*parent).len() || range.start > range.end {
                    return Err(StepError::OutOfRange);
                }
                let mut removed = Vec::with_capacity(range.len());
                for _ in range.clone() {
                    removed.push(
                        doc.detach(*parent, range.start)
                            .ok_or(StepError::OutOfRange)?,
                    );
                }
                let mut ids = Vec::new();
                for block in &removed {
                    block.ids(&mut ids);
                }
                let inverse = Step::InsertBlocks {
                    parent: *parent,
                    index: range.start,
                    blocks: removed,
                };
                Ok((inverse, StepMap::Removed(ids)))
            }
            Self::ReplacePlain {
                block,
                range,
                insert,
            } => {
                let entry = doc.get_mut(*block).ok_or(StepError::UnknownBlock(*block))?;
                let BlockContent::Plain(code) = &mut entry.content else {
                    return Err(StepError::NotText(*block));
                };
                if range.start > range.end
                    || range.end > code.len()
                    || !code.is_char_boundary(range.start)
                    || !code.is_char_boundary(range.end)
                {
                    return Err(StepError::OutOfRange);
                }
                let removed = code[range.clone()].to_string();
                code.replace_range(range.clone(), insert);
                Ok((
                    Step::ReplacePlain {
                        block: *block,
                        range: range.start..range.start + insert.len(),
                        insert: removed,
                    },
                    StepMap::Text {
                        block: *block,
                        range: range.clone(),
                        insert_len: insert.len(),
                    },
                ))
            }
            Self::SetKind { block, kind } => {
                let entry = doc.get_mut(*block).ok_or(StepError::UnknownBlock(*block))?;
                if entry.kind.is_container() != kind.is_container()
                    || entry.kind.is_textblock() != kind.is_textblock()
                {
                    return Err(StepError::KindMismatch);
                }
                let prev = std::mem::replace(&mut entry.kind, kind.clone());
                doc.structure_dirty = true;
                Ok((
                    Step::SetKind {
                        block: *block,
                        kind: prev,
                    },
                    StepMap::Identity,
                ))
            }
        }
    }
}
