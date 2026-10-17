use std::ops::Range;

use super::{
    document::{BlockContent, EditorDocument},
    id::BlockId,
    inline::InlineText,
    kind::BlockKindData,
    selection::BlockSelection,
    step::{DetachedBlock, Step, StepError, StepMap},
};

/// Why an edit happened; drives undo coalescing.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditOrigin {
    Typing,
    Backspace,
    DeleteForward,
    Ime,
    Paste,
    Cut,
    Command(&'static str),
    InputRule(&'static str),
    Api,
    Undo,
    Redo,
}

/// A committed group of steps. Opaque.
#[derive(Clone, Debug)]
pub struct Transaction {
    pub(crate) steps: Vec<Step>,
    pub(crate) inverse: Vec<Step>,
    pub(crate) maps: Vec<StepMap>,
    pub(crate) selection_before: BlockSelection,
    pub(crate) selection_after: BlockSelection,
    pub(crate) origin: EditOrigin,
}

impl Transaction {
    /// Map a selection through every step; `None` if an endpoint's block
    /// was removed.
    pub(crate) fn map_selection(&self, selection: &BlockSelection) -> Option<BlockSelection> {
        let map = |mut p| {
            for step_map in &self.maps {
                p = step_map.map(p)?;
            }
            Some(p)
        };
        match selection {
            BlockSelection::Text { anchor, head } => Some(BlockSelection::Text {
                anchor: map(*anchor)?,
                head: map(*head)?,
            }),
            BlockSelection::Blocks { .. } | BlockSelection::Cells { .. } => Some(selection.clone()),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Re-apply recorded steps (redo) or their inverses (undo).
    pub(crate) fn replay(steps: &[Step], doc: &mut EditorDocument) -> Result<(), StepError> {
        for step in steps {
            step.apply(doc)?;
        }
        doc.refresh_order();
        doc.revision += 1;
        Ok(())
    }
}

/// Builds a transaction against the live document. Every change goes
/// through [`TransactionBuilder::step`], which records the step and its
/// inverse.
pub(crate) struct TransactionBuilder<'a> {
    pub(crate) doc: &'a mut EditorDocument,
    steps: Vec<Step>,
    inverse: Vec<Step>,
    maps: Vec<StepMap>,
}

impl<'a> TransactionBuilder<'a> {
    pub(crate) fn new(doc: &'a mut EditorDocument) -> Self {
        Self {
            doc,
            steps: Vec::new(),
            inverse: Vec::new(),
            maps: Vec::new(),
        }
    }

    pub(crate) fn step(&mut self, step: Step) -> Result<StepMap, StepError> {
        let (inverse, map) = step.apply(self.doc)?;
        self.steps.push(step);
        self.inverse.push(inverse);
        self.maps.push(map.clone());
        Ok(map)
    }

    pub(crate) fn replace_text(
        &mut self,
        block: BlockId,
        range: Range<usize>,
        insert: InlineText,
    ) -> Result<(), StepError> {
        if range.is_empty() && insert.is_empty() {
            return Ok(());
        }
        self.step(Step::ReplaceText {
            block,
            range,
            insert,
        })
        .map(|_| ())
    }

    pub(crate) fn replace_plain(
        &mut self,
        block: BlockId,
        range: Range<usize>,
        insert: &str,
    ) -> Result<(), StepError> {
        if range.is_empty() && insert.is_empty() {
            return Ok(());
        }
        self.step(Step::ReplacePlain {
            block,
            range,
            insert: insert.to_string(),
        })
        .map(|_| ())
    }

    pub(crate) fn insert(
        &mut self,
        parent: BlockId,
        index: usize,
        blocks: Vec<DetachedBlock>,
    ) -> Result<(), StepError> {
        if blocks.is_empty() {
            return Ok(());
        }
        self.step(Step::InsertBlocks {
            parent,
            index,
            blocks,
        })
        .map(|_| ())
    }

    /// Remove blocks and return copies of them (ids kept) so callers can
    /// re-insert them elsewhere: a move is remove + insert.
    pub(crate) fn remove(
        &mut self,
        parent: BlockId,
        range: Range<usize>,
    ) -> Result<Vec<DetachedBlock>, StepError> {
        if range.is_empty() {
            return Ok(Vec::new());
        }
        self.step(Step::RemoveBlocks { parent, range })?;
        match self.inverse.last() {
            Some(Step::InsertBlocks { blocks, .. }) => Ok(blocks.clone()),
            _ => Err(StepError::OutOfRange),
        }
    }

    pub(crate) fn remove_block(&mut self, id: BlockId) -> Result<DetachedBlock, StepError> {
        let (parent, index) = self
            .doc
            .index_in_parent(id)
            .ok_or(StepError::UnknownBlock(id))?;
        self.remove(parent, index..index + 1)?
            .pop()
            .ok_or(StepError::OutOfRange)
    }

    /// Move `id` (keeping ids) to `index` of `parent`, where `index` counts
    /// the target's children *after* the block has been taken out.
    pub(crate) fn move_block(
        &mut self,
        id: BlockId,
        parent: BlockId,
        index: usize,
    ) -> Result<(), StepError> {
        let block = self.remove_block(id)?;
        self.insert(parent, index, vec![block])
    }

    pub(crate) fn set_kind(
        &mut self,
        block: BlockId,
        kind: BlockKindData,
    ) -> Result<(), StepError> {
        if self.doc.kind(block) == Some(&kind) {
            return Ok(());
        }
        self.step(Step::SetKind { block, kind }).map(|_| ())
    }

    /// Structural normalization (design §5.1), appended as ordinary steps so
    /// undo stays exact:
    /// - a `List` holds only `ListItem`s,
    /// - a `ListItem` starts with a leaf,
    /// - no container other than `Root` is empty,
    /// - adjacent sibling lists of the same type are merged (Markdown cannot
    ///   write them apart),
    /// - there is at least one caret leaf.
    pub(crate) fn normalize(&mut self) -> Result<(), StepError> {
        loop {
            let Some(fix) = self.find_violation() else {
                return Ok(());
            };
            match fix {
                Fix::RemoveEmpty(id) => {
                    self.remove_block(id)?;
                }
                Fix::WrapInItem(id) => {
                    let (parent, index) = self
                        .doc
                        .index_in_parent(id)
                        .ok_or(StepError::UnknownBlock(id))?;
                    let block = self.remove_block(id)?;
                    let item = DetachedBlock::container(
                        BlockKindData::ListItem {
                            checked: None,
                            spread: false,
                        },
                        vec![block],
                    );
                    self.insert(parent, index, vec![item])?;
                }
                Fix::LeadingParagraph(item) => {
                    self.insert(item, 0, vec![DetachedBlock::paragraph(InlineText::new())])?;
                }
                Fix::MergeLists(first, second) => {
                    let count = self.doc.children(second).len();
                    let items = self.remove(second, 0..count)?;
                    let len = self.doc.children(first).len();
                    self.insert(first, len, items)?;
                    self.remove_block(second)?;
                }
                Fix::NoLeaf => {
                    let root = self.doc.root;
                    let len = self.doc.children(root).len();
                    self.insert(root, len, vec![DetachedBlock::paragraph(InlineText::new())])?;
                }
            }
        }
    }

    fn find_violation(&self) -> Option<Fix> {
        let doc = &*self.doc;
        let mut stack = vec![doc.root];
        while let Some(id) = stack.pop() {
            let entry = doc.get(id)?;
            if let BlockContent::Children(children) = &entry.content {
                if children.is_empty() && entry.kind != BlockKindData::Root {
                    return Some(Fix::RemoveEmpty(id));
                }
                if let BlockKindData::List { .. } = entry.kind
                    && let Some(stray) = children
                        .iter()
                        .find(|c| !matches!(doc.kind(**c), Some(BlockKindData::ListItem { .. })))
                {
                    return Some(Fix::WrapInItem(*stray));
                }
                if let BlockKindData::ListItem { .. } = entry.kind
                    && !children.first().is_some_and(|c| doc.is_caret_leaf(*c))
                {
                    return Some(Fix::LeadingParagraph(id));
                }
                for pair in children.windows(2) {
                    if let (
                        Some(BlockKindData::List { ordered: a, .. }),
                        Some(BlockKindData::List { ordered: b, .. }),
                    ) = (doc.kind(pair[0]), doc.kind(pair[1]))
                        && a == b
                    {
                        return Some(Fix::MergeLists(pair[0], pair[1]));
                    }
                }
                stack.extend(children.iter().copied());
            }
        }
        if doc.walk_leaves().is_empty() {
            return Some(Fix::NoLeaf);
        }
        None
    }

    pub(crate) fn finish(
        mut self,
        origin: EditOrigin,
        selection_before: BlockSelection,
        selection_after: BlockSelection,
    ) -> Result<Transaction, StepError> {
        self.normalize()?;
        self.doc.refresh_order();
        if !self.steps.is_empty() {
            self.doc.revision += 1;
        }
        debug_assert_eq!(self.doc.check_invariants(), Ok(()));
        let mut inverse = self.inverse;
        inverse.reverse();
        Ok(Transaction {
            steps: self.steps,
            inverse,
            maps: self.maps,
            selection_before,
            selection_after,
            origin,
        })
    }

    /// Undo whatever was applied so far (used when a command fails halfway).
    pub(crate) fn rollback(self) {
        for step in self.inverse.iter().rev() {
            let _ = step.apply(self.doc);
        }
        self.doc.refresh_order();
    }
}

enum Fix {
    RemoveEmpty(BlockId),
    WrapInItem(BlockId),
    LeadingParagraph(BlockId),
    MergeLists(BlockId, BlockId),
    NoLeaf,
}
