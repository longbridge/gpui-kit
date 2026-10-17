//! Direct projection of the editing model into the shared renderer's
//! `BlockNode`s (design §7.3): no Markdown round trip, so no render-mode
//! placeholders and no re-parse of a whole list for an edit inside it.
//! Every text leaf (paragraph, heading, code block, table cell) gets its
//! `BlockId` as `editor_key`.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use super::{
    document::{AtomData, BlockContent, EditorDocument},
    id::BlockId,
    inline::{InlineAtom, InlineText},
    kind::BlockKindData,
};
use crate::text::{
    inline::InlineState,
    node::{BlockNode, BlockState, CodeBlock, InlineNode, Paragraph, Table, TableCell, TableRow},
};

fn keyed(state: &Arc<Mutex<InlineState>>, id: BlockId) {
    if let Ok(mut state) = state.lock() {
        state.editor_key = Some(id.as_u64());
    }
}

/// A paragraph holding `inline`, keyed by `id`.
pub(crate) fn paragraph(inline: &InlineText, id: BlockId) -> Paragraph {
    let mut paragraph = Paragraph::default();
    for (range, mark, atom) in inline.pieces() {
        match atom {
            Some(InlineAtom::Image(image)) => paragraph.push(InlineNode::image(image.clone())),
            Some(InlineAtom::Custom(node)) => {
                let node = InlineNode::custom(node.clone());
                let len = node.text.len();
                paragraph.push(node.marks(vec![(0..len, mark.clone())]));
            }
            None => {
                let text = &inline.text()[range];
                paragraph.push(
                    InlineNode::new(text.to_string()).marks(vec![(0..text.len(), mark.clone())]),
                );
            }
        }
    }
    keyed(&paragraph.state, id);
    paragraph
}

/// Project one block (and its subtree), uncached.
#[cfg(test)]
pub(crate) fn project_block(doc: &EditorDocument, id: BlockId) -> Option<BlockNode> {
    NodeCache::default().node(doc, id)
}

/// Per-node projection cache (design §7.3). Every block and table row is
/// cached under its id with the block revision it was built at; `touch`
/// bubbles revisions to ancestors, so an edit rebuilds only the path from
/// the edited leaf to its top-level block and reuses every other subtree
/// (sharing its retained `InlineState`s).
#[derive(Default)]
pub(crate) struct NodeCache {
    nodes: HashMap<BlockId, (u64, BlockNode)>,
    rows: HashMap<BlockId, (u64, TableRow)>,
    /// Nodes and rows built (not reused), for the perf tests (D9).
    pub(crate) built: usize,
}

impl NodeCache {
    /// Drop entries of blocks that left the document, once the cache has
    /// grown well past the document (amortized O(1) per pass).
    pub(crate) fn collect_garbage(&mut self, doc: &EditorDocument) {
        if self.nodes.len() + self.rows.len() > 2 * doc.len() + 64 {
            self.nodes.retain(|id, _| doc.get(*id).is_some());
            self.rows.retain(|id, _| doc.get(*id).is_some());
        }
    }

    pub(crate) fn clear(&mut self) {
        self.nodes.clear();
        self.rows.clear();
    }

    pub(crate) fn node(&mut self, doc: &EditorDocument, id: BlockId) -> Option<BlockNode> {
        let revision = doc.block_revision(id);
        let node = match self.nodes.get(&id) {
            Some((rev, node)) if *rev == revision => node.clone(),
            _ => {
                let node = self.build(doc, id)?;
                self.built += 1;
                self.nodes.insert(id, (revision, node.clone()));
                node
            }
        };
        Some(node)
    }

    fn row(&mut self, doc: &EditorDocument, id: BlockId) -> TableRow {
        let revision = doc.block_revision(id);
        let row = match self.rows.get(&id) {
            Some((rev, row)) if *rev == revision => row.clone(),
            _ => {
                self.built += 1;
                let row = TableRow {
                    children: doc
                        .children(id)
                        .iter()
                        .map(|cell| TableCell {
                            children: match doc.inline(*cell) {
                                Some(inline) => paragraph(inline, *cell),
                                None => paragraph(&InlineText::new(), *cell),
                            },
                            width: None,
                        })
                        .collect(),
                };
                self.rows.insert(id, (revision, row.clone()));
                row
            }
        };
        row
    }

    fn build(&mut self, doc: &EditorDocument, id: BlockId) -> Option<BlockNode> {
        let entry = doc.get(id)?;
        let children = |this: &mut Self| -> Vec<BlockNode> {
            doc.children(id)
                .iter()
                .filter_map(|child| this.node(doc, *child))
                .collect()
        };
        let inline = || match &entry.content {
            BlockContent::Inline(inline) => paragraph(inline, id),
            _ => paragraph(&InlineText::new(), id),
        };
        Some(match &entry.kind {
            BlockKindData::Root => BlockNode::Root {
                children: children(self),
                span: None,
            },
            BlockKindData::Paragraph => BlockNode::Paragraph(inline()),
            BlockKindData::Heading { level } => BlockNode::Heading {
                level: *level,
                children: inline(),
                span: None,
            },
            BlockKindData::Blockquote => BlockNode::Blockquote {
                children: children(self),
                span: None,
            },
            BlockKindData::List { ordered, start } => BlockNode::List {
                children: children(self),
                ordered: *ordered,
                start: *start,
                span: None,
            },
            BlockKindData::ListItem { checked, spread } => BlockNode::ListItem {
                children: children(self),
                spread: *spread,
                checked: *checked,
                span: None,
            },
            BlockKindData::CodeBlock { lang, meta } => {
                let code = match &entry.content {
                    BlockContent::Plain(code) => code.clone(),
                    _ => String::new(),
                };
                let code =
                    CodeBlock::new(code.into(), lang.clone(), None::<crate::text::node::Span>)
                        .with_meta(meta.clone());
                keyed(code.inline_state(), id);
                BlockNode::CodeBlock(code)
            }
            BlockKindData::Table { aligns } => BlockNode::Table(Table {
                children: doc
                    .children(id)
                    .iter()
                    .map(|row| self.row(doc, *row))
                    .collect(),
                column_aligns: aligns.clone(),
                span: None,
                table_data_cache: Default::default(),
                column_widths_cache: Default::default(),
            }),
            BlockKindData::TableRow | BlockKindData::TableCell => return None,
            BlockKindData::HorizontalRule => BlockNode::HorizontalRule { span: None },
            BlockKindData::Break { html } => BlockNode::Break {
                html: *html,
                span: None,
            },
            BlockKindData::Custom => match &entry.content {
                BlockContent::Atom(AtomData::Custom(node)) => BlockNode::Custom {
                    node: node.clone(),
                    state: BlockState::default(),
                },
                _ => BlockNode::Unknown,
            },
            BlockKindData::Definition {
                identifier,
                url,
                title,
            } => BlockNode::Definition {
                identifier: identifier.clone(),
                url: url.clone(),
                title: title.clone(),
                span: None,
            },
            BlockKindData::Unknown => BlockNode::Unknown,
        })
    }
}
