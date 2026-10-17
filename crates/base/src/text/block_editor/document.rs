use std::collections::HashMap;
use std::sync::Arc;

use super::{
    id::BlockId,
    inline::InlineText,
    kind::{BlockKind, BlockKindData},
    order::OrderIndex,
    position::BlockPosition,
    step::DetachedBlock,
};
use crate::text::markdown_ext::{MarkdownExtensions, MarkdownNode};

/// Payload of an atomic block.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum AtomData {
    None,
    Custom(MarkdownNode),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BlockContent {
    Children(Vec<BlockId>),
    /// Paragraph, Heading, TableCell.
    Inline(InlineText),
    /// CodeBlock (its language lives in [`BlockKindData`]).
    Plain(String),
    /// Custom, Definition, Break, HorizontalRule, Unknown.
    Atom(AtomData),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BlockEntry {
    pub(crate) parent: Option<BlockId>,
    pub(crate) kind: BlockKindData,
    pub(crate) content: BlockContent,
}

/// The editing model: block tree with stable ids (design §4.2, D4).
pub(crate) struct EditorDocument {
    pub(crate) revision: u64,
    pub(crate) root: BlockId,
    pub(crate) blocks: HashMap<BlockId, BlockEntry>,
    pub(crate) order: OrderIndex,
    /// Set by structural steps; the order index is rebuilt at commit.
    pub(crate) structure_dirty: bool,
    pub(crate) extensions: Arc<MarkdownExtensions>,
    /// Per-block content revision, bumped for a block and all its ancestors
    /// by every step that touches it; absent means never edited. The view
    /// re-projects a top-level block only when its revision changes.
    pub(crate) block_revisions: HashMap<BlockId, u64>,
    edit_counter: u64,
}

impl EditorDocument {
    /// An empty document: a root holding one empty paragraph.
    pub(crate) fn empty(extensions: Arc<MarkdownExtensions>) -> Self {
        let root = BlockId::next();
        let paragraph = BlockId::next();
        let mut blocks = HashMap::new();
        blocks.insert(
            root,
            BlockEntry {
                parent: None,
                kind: BlockKindData::Root,
                content: BlockContent::Children(vec![paragraph]),
            },
        );
        blocks.insert(
            paragraph,
            BlockEntry {
                parent: Some(root),
                kind: BlockKindData::Paragraph,
                content: BlockContent::Inline(InlineText::new()),
            },
        );
        let mut doc = Self {
            revision: 0,
            root,
            blocks,
            order: OrderIndex::default(),
            structure_dirty: true,
            extensions,
            block_revisions: HashMap::new(),
            edit_counter: 0,
        };
        doc.refresh_order();
        doc
    }

    /// Bump the content revision of `id` and its ancestors.
    pub(crate) fn touch(&mut self, id: BlockId) {
        self.edit_counter += 1;
        let mut next = Some(id);
        while let Some(id) = next {
            self.block_revisions.insert(id, self.edit_counter);
            next = self.parent(id);
        }
    }

    pub(crate) fn block_revision(&self, id: BlockId) -> u64 {
        self.block_revisions.get(&id).copied().unwrap_or(0)
    }

    /// Number of blocks, including the root.
    pub(crate) fn len(&self) -> usize {
        self.blocks.len()
    }

    pub(crate) fn get(&self, id: BlockId) -> Option<&BlockEntry> {
        self.blocks.get(&id)
    }

    pub(crate) fn get_mut(&mut self, id: BlockId) -> Option<&mut BlockEntry> {
        self.blocks.get_mut(&id)
    }

    pub(crate) fn children(&self, id: BlockId) -> &[BlockId] {
        match self.blocks.get(&id).map(|entry| &entry.content) {
            Some(BlockContent::Children(children)) => children,
            _ => &[],
        }
    }

    pub(crate) fn parent(&self, id: BlockId) -> Option<BlockId> {
        self.blocks.get(&id)?.parent
    }

    pub(crate) fn index_in_parent(&self, id: BlockId) -> Option<(BlockId, usize)> {
        let parent = self.parent(id)?;
        let index = self.children(parent).iter().position(|c| *c == id)?;
        Some((parent, index))
    }

    pub(crate) fn kind(&self, id: BlockId) -> Option<&BlockKindData> {
        self.blocks.get(&id).map(|entry| &entry.kind)
    }

    pub(crate) fn inline(&self, id: BlockId) -> Option<&InlineText> {
        match &self.blocks.get(&id)?.content {
            BlockContent::Inline(inline) => Some(inline),
            _ => None,
        }
    }

    /// Whether the block can hold a caret.
    pub(crate) fn is_caret_leaf(&self, id: BlockId) -> bool {
        self.blocks
            .get(&id)
            .is_some_and(|entry| !matches!(entry.content, BlockContent::Children(_)))
    }

    /// Length of a caret leaf in caret units: bytes of text, or 1 for an atom
    /// or code block (0 = before, 1 = after) in the spike.
    pub(crate) fn leaf_len(&self, id: BlockId) -> Option<usize> {
        match &self.blocks.get(&id)?.content {
            BlockContent::Inline(inline) => Some(inline.len()),
            BlockContent::Plain(code) => Some(code.len()),
            BlockContent::Atom(_) => Some(1),
            BlockContent::Children(_) => None,
        }
    }

    /// A code block's text.
    pub(crate) fn plain(&self, id: BlockId) -> Option<&str> {
        match &self.blocks.get(&id)?.content {
            BlockContent::Plain(code) => Some(code),
            _ => None,
        }
    }

    /// Caret-leaf text: inline text (atoms as U+FFFC) or code.
    pub(crate) fn leaf_text(&self, id: BlockId) -> Option<&str> {
        match &self.blocks.get(&id)?.content {
            BlockContent::Inline(inline) => Some(inline.text()),
            BlockContent::Plain(code) => Some(code),
            _ => None,
        }
    }

    /// Path from the root down to `id`, inclusive.
    pub(crate) fn path(&self, id: BlockId) -> Vec<BlockId> {
        let mut path = vec![id];
        let mut current = id;
        while let Some(parent) = self.parent(current) {
            path.push(parent);
            current = parent;
        }
        path.reverse();
        path
    }

    /// Caret leaves in document order, by walking the tree. Used inside
    /// transactions, where the order index may be stale.
    pub(crate) fn walk_leaves(&self) -> Vec<BlockId> {
        let mut out = Vec::new();
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            match self.blocks.get(&id).map(|entry| &entry.content) {
                Some(BlockContent::Children(children)) => stack.extend(children.iter().rev()),
                Some(_) => out.push(id),
                None => {}
            }
        }
        out
    }

    pub(crate) fn refresh_order(&mut self) {
        if self.structure_dirty {
            self.order = OrderIndex::build(self);
            self.structure_dirty = false;
        }
    }

    /// Leaves in document order from the (fresh) order index.
    pub(crate) fn leaves(&self) -> &[BlockId] {
        debug_assert!(!self.structure_dirty, "order index read while stale");
        self.order.leaves()
    }

    pub(crate) fn first_caret(&self) -> BlockPosition {
        let leaf = self.walk_leaves().first().copied().unwrap_or(self.root);
        BlockPosition::new(leaf, 0)
    }

    pub(crate) fn last_caret(&self) -> BlockPosition {
        let leaf = self.walk_leaves().last().copied().unwrap_or(self.root);
        BlockPosition::new(leaf, self.leaf_len(leaf).unwrap_or(0))
    }

    /// Insert a detached subtree (keeping its ids) under `parent` at `index`.
    pub(crate) fn attach(&mut self, parent: BlockId, index: usize, block: DetachedBlock) -> bool {
        let Some(BlockEntry {
            content: BlockContent::Children(children),
            ..
        }) = self.blocks.get_mut(&parent)
        else {
            return false;
        };
        if index > children.len() {
            return false;
        }
        children.insert(index, block.id);
        self.attach_entry(Some(parent), block);
        self.structure_dirty = true;
        true
    }

    fn attach_entry(&mut self, parent: Option<BlockId>, block: DetachedBlock) {
        let content = match block.content {
            super::step::DetachedContent::Children(children) => {
                let ids = children.iter().map(|child| child.id).collect();
                for child in children {
                    self.attach_entry(Some(block.id), child);
                }
                BlockContent::Children(ids)
            }
            super::step::DetachedContent::Leaf(content) => content,
        };
        self.blocks.insert(
            block.id,
            BlockEntry {
                parent,
                kind: block.kind,
                content,
            },
        );
    }

    /// Remove the child at `index` of `parent`, with its subtree.
    pub(crate) fn detach(&mut self, parent: BlockId, index: usize) -> Option<DetachedBlock> {
        let Some(BlockEntry {
            content: BlockContent::Children(children),
            ..
        }) = self.blocks.get_mut(&parent)
        else {
            return None;
        };
        if index >= children.len() {
            return None;
        }
        let id = children.remove(index);
        self.structure_dirty = true;
        self.detach_entry(id)
    }

    fn detach_entry(&mut self, id: BlockId) -> Option<DetachedBlock> {
        let entry = self.blocks.remove(&id)?;
        let content = match entry.content {
            BlockContent::Children(children) => super::step::DetachedContent::Children(
                children
                    .into_iter()
                    .filter_map(|child| self.detach_entry(child))
                    .collect(),
            ),
            other => super::step::DetachedContent::Leaf(other),
        };
        Some(DetachedBlock {
            id,
            kind: entry.kind,
            content,
        })
    }

    /// Structural invariants (design §5.1) plus N1–N3 for every inline leaf.
    pub(crate) fn check_invariants(&self) -> Result<(), String> {
        let mut seen = 0;
        let mut stack = vec![(self.root, None)];
        while let Some((id, parent)) = stack.pop() {
            seen += 1;
            let entry = self.blocks.get(&id).ok_or(format!("missing {id:?}"))?;
            if entry.parent != parent {
                return Err(format!("bad parent link on {id:?}"));
            }
            match &entry.content {
                BlockContent::Children(children) => {
                    if !entry.kind.is_container() {
                        return Err(format!("{:?} has children", entry.kind));
                    }
                    if children.is_empty() && entry.kind != BlockKindData::Root {
                        return Err(format!("empty container {:?}", entry.kind));
                    }
                    if let BlockKindData::List { .. } = entry.kind
                        && children
                            .iter()
                            .any(|c| !matches!(self.kind(*c), Some(BlockKindData::ListItem { .. })))
                    {
                        return Err("list holds a non-item".into());
                    }
                    if let BlockKindData::ListItem { .. } = entry.kind
                        && !children.first().is_some_and(|c| self.is_caret_leaf(*c))
                    {
                        return Err("list item does not start with a leaf".into());
                    }
                    stack.extend(children.iter().map(|c| (*c, Some(id))));
                }
                BlockContent::Inline(inline) => {
                    if !inline.is_normalized() {
                        return Err(format!("inline text of {id:?} not normalized"));
                    }
                }
                _ => {}
            }
        }
        if seen != self.blocks.len() {
            return Err("orphaned entries".into());
        }
        if self.walk_leaves().is_empty() {
            return Err("no caret leaf".into());
        }
        Ok(())
    }
}

/// Borrowed read projection of the document, like the dock's `PaneRef`.
#[derive(Clone, Copy)]
pub struct DocumentRef<'a> {
    pub(crate) doc: &'a EditorDocument,
}

impl<'a> DocumentRef<'a> {
    pub fn root(&self) -> BlockRef<'a> {
        BlockRef {
            doc: self.doc,
            id: self.doc.root,
        }
    }

    pub fn block(&self, id: BlockId) -> Option<BlockRef<'a>> {
        self.doc.get(id).map(|_| BlockRef { doc: self.doc, id })
    }

    pub fn first_caret(&self) -> BlockPosition {
        self.doc.first_caret()
    }

    pub fn last_caret(&self) -> BlockPosition {
        self.doc.last_caret()
    }

    /// Caret-bearing leaves in document order (O(1) per step, from the order
    /// index).
    pub fn leaves(&self) -> impl Iterator<Item = BlockRef<'a>> + 'a {
        let doc = self.doc;
        doc.leaves().iter().map(move |id| BlockRef { doc, id: *id })
    }
}

/// Borrowed read projection of one block.
#[derive(Clone, Copy)]
pub struct BlockRef<'a> {
    doc: &'a EditorDocument,
    id: BlockId,
}

impl<'a> BlockRef<'a> {
    pub fn id(&self) -> BlockId {
        self.id
    }

    pub fn kind(&self) -> BlockKind {
        self.doc
            .kind(self.id)
            .map(BlockKindData::public)
            .unwrap_or(BlockKind::Unknown)
    }

    /// The flat text of a text-bearing leaf (inline atoms show as U+FFFC).
    pub fn text(&self) -> Option<&'a str> {
        match &self.doc.get(self.id)?.content {
            BlockContent::Inline(inline) => Some(inline.text()),
            BlockContent::Plain(text) => Some(text),
            _ => None,
        }
    }

    pub fn parent(&self) -> Option<BlockRef<'a>> {
        self.doc
            .parent(self.id)
            .map(|id| BlockRef { doc: self.doc, id })
    }

    pub fn children(&self) -> impl Iterator<Item = BlockRef<'a>> + 'a {
        let doc = self.doc;
        doc.children(self.id)
            .iter()
            .map(move |id| BlockRef { doc, id: *id })
    }
}
