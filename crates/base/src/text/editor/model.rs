use crate::text::{
    MarkdownExtensions,
    document::ParsedDocument,
    format::markdown,
    node::{BlockNode, InlineNode, NodeContext, Paragraph, TextMark},
};
use gpui::SharedString;
use std::{collections::HashSet, ops::Range, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;

// Positions use stable paragraph identities and UTF-8 boundaries.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(super) struct Position {
    pub block: u64,
    pub offset: usize,
}

#[derive(Clone, Default)]
pub(super) struct Document {
    pub blocks: Vec<BlockNode>,
    pub context: NodeContext,
}

// Paragraph render state is retained across edits and history snapshots.
// Its allocation identifies a paragraph only within this in-memory document.
impl Paragraph {
    pub(super) fn editor_id(&self) -> u64 {
        Arc::as_ptr(&self.state) as usize as u64
    }
}

fn children(node: &BlockNode) -> Option<&Vec<BlockNode>> {
    match node {
        BlockNode::Root { children, .. }
        | BlockNode::List { children, .. }
        | BlockNode::ListItem { children, .. }
        | BlockNode::Blockquote { children, .. } => Some(children),
        _ => None,
    }
}

fn children_mut(node: &mut BlockNode) -> Option<&mut Vec<BlockNode>> {
    match node {
        BlockNode::Root { children, .. }
        | BlockNode::List { children, .. }
        | BlockNode::ListItem { children, .. }
        | BlockNode::Blockquote { children, .. } => Some(children),
        _ => None,
    }
}

fn paragraph(node: &BlockNode) -> Option<&Paragraph> {
    match node {
        BlockNode::Paragraph(p) | BlockNode::Heading { children: p, .. }
            if p.children
                .iter()
                .all(|n| n.image.is_none() && n.custom.is_none()) =>
        {
            Some(p)
        }
        _ => None,
    }
}

fn paragraph_mut(node: &mut BlockNode) -> Option<&mut Paragraph> {
    match node {
        BlockNode::Paragraph(p) | BlockNode::Heading { children: p, .. }
            if p.children
                .iter()
                .all(|n| n.image.is_none() && n.custom.is_none()) =>
        {
            Some(p)
        }
        _ => None,
    }
}

fn visit<'a>(nodes: &'a [BlockNode], result: &mut Vec<&'a Paragraph>) {
    for node in nodes {
        if let Some(p) = paragraph(node) {
            result.push(p);
        }
        if let Some(nodes) = children(node) {
            visit(nodes, result);
        }
    }
}

fn find_mut(nodes: &mut [BlockNode], id: u64) -> Option<&mut Paragraph> {
    for node in nodes {
        if paragraph(node).is_some_and(|p| p.editor_id() == id) {
            return paragraph_mut(node);
        }
        if let Some(nodes) = children_mut(node) {
            if let Some(p) = find_mut(nodes, id) {
                return Some(p);
            }
        }
    }
    None
}

// Slice content and format ranges without retaining mutable render caches.
fn slice(p: &Paragraph, range: Range<usize>) -> Vec<InlineNode> {
    let mut result = Vec::new();
    let mut offset = 0;
    for node in &p.children {
        let start = range.start.saturating_sub(offset).min(node.text.len());
        let end = range.end.saturating_sub(offset).min(node.text.len());
        if start < end {
            let marks = node
                .marks
                .iter()
                .filter_map(|(r, mark)| {
                    let a = r.start.max(start);
                    let b = r.end.min(end);
                    (a < b).then(|| ((a - start)..(b - start), mark.clone()))
                })
                .collect();
            result.push(InlineNode::new(node.text[start..end].to_string()).marks(marks));
        }
        offset += node.text.len();
    }
    result
}

impl Document {
    #[cfg(test)]
    pub fn parse(source: &str) -> Result<Self, SharedString> {
        Self::parse_with_extensions(source, MarkdownExtensions::default())
    }

    pub fn parse_with_extensions(
        source: &str,
        extensions: MarkdownExtensions,
    ) -> Result<Self, SharedString> {
        let mut context = NodeContext {
            markdown_extensions: Arc::new(extensions),
            ..Default::default()
        };
        let parsed = markdown::parse(source, &mut context)?;
        let mut doc = Self {
            blocks: parsed.blocks.as_ref().clone(),
            context,
        };
        fn fill_empty_items(nodes: &mut [BlockNode]) {
            for node in nodes {
                if let BlockNode::ListItem { children, .. } = node {
                    if children.is_empty() {
                        children.push(BlockNode::Paragraph(Paragraph::default()));
                    }
                }
                if let Some(nested) = children_mut(node) {
                    fill_empty_items(nested);
                }
            }
        }
        fill_empty_items(&mut doc.blocks);
        doc.ensure_paragraph();
        Ok(doc)
    }

    pub fn source(&self) -> String {
        ParsedDocument {
            source: "".into(),
            blocks: Arc::new(self.blocks.clone()),
        }
        .to_markdown()
    }

    pub fn paragraphs(&self) -> Vec<&Paragraph> {
        let mut result = Vec::new();
        visit(&self.blocks, &mut result);
        result
    }

    pub fn text(&self, id: u64) -> String {
        self.paragraphs()
            .iter()
            .find(|p| p.editor_id() == id)
            .map(|p| p.text())
            .unwrap_or_default()
    }

    pub fn first(&self) -> Position {
        Position {
            block: self.paragraphs()[0].editor_id(),
            offset: 0,
        }
    }

    pub fn last(&self) -> Position {
        let p = self.paragraphs().last().copied().unwrap();
        Position {
            block: p.editor_id(),
            offset: p.text_len(),
        }
    }

    pub fn ordered(&self, a: Position, b: Position) -> (Position, Position) {
        let ids: Vec<_> = self.paragraphs().iter().map(|p| p.editor_id()).collect();
        let key = |p: Position| {
            (
                ids.iter().position(|id| *id == p.block).unwrap_or(0),
                p.offset,
            )
        };
        if key(a) <= key(b) { (a, b) } else { (b, a) }
    }

    pub fn selected(&self, a: Position, b: Position) -> String {
        let (a, b) = self.ordered(a, b);
        self.paragraphs()
            .into_iter()
            .skip_while(|p| p.editor_id() != a.block)
            .scan(false, |done, p| {
                if *done {
                    return None;
                }
                *done = p.editor_id() == b.block;
                let text = p.text();
                let start = if p.editor_id() == a.block {
                    a.offset
                } else {
                    0
                };
                let end = if p.editor_id() == b.block {
                    b.offset
                } else {
                    text.len()
                };
                Some(text[start..end].to_string())
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn selection_in(&self, id: u64, a: Position, b: Position) -> Option<Range<usize>> {
        let (a, b) = self.ordered(a, b);
        let ps = self.paragraphs();
        let ix = |id| ps.iter().position(|p| p.editor_id() == id);
        let current = ix(id)?;
        if current < ix(a.block)? || current > ix(b.block)? {
            return None;
        }
        Some(
            (if id == a.block { a.offset } else { 0 })..(if id == b.block {
                b.offset
            } else {
                ps[current].text_len()
            }),
        )
    }

    fn new_paragraph(&mut self, nodes: Vec<InlineNode>) -> Paragraph {
        let mut p = Paragraph::default();
        p.children = nodes;
        p
    }

    fn ensure_paragraph(&mut self) {
        if self.paragraphs().is_empty() {
            let p = self.new_paragraph(vec![]);
            self.blocks.push(BlockNode::Paragraph(p));
        }
    }

    // Typing inherits formatting unless a toolbar command overrides it.
    pub fn mark_at(&self, at: Position) -> TextMark {
        let mut mark = TextMark::default();
        if at.offset > 0 {
            if let Some(p) = self
                .paragraphs()
                .into_iter()
                .find(|p| p.editor_id() == at.block)
            {
                for node in slice(p, previous_boundary(&p.text(), at.offset)..at.offset) {
                    for (_, inherited) in node.marks {
                        mark.merge(inherited);
                    }
                }
            }
        }
        mark
    }

    pub fn replace(
        &mut self,
        a: Position,
        b: Position,
        text: &str,
        stored: Option<&TextMark>,
    ) -> Position {
        let (a, b) = self.ordered(a, b);
        let ps = self.paragraphs();
        let first = ps.iter().find(|p| p.editor_id() == a.block).unwrap();
        let last = ps.iter().find(|p| p.editor_id() == b.block).unwrap();
        let mut nodes = slice(first, 0..a.offset);
        let mark = stored.cloned().unwrap_or_else(|| self.mark_at(a));
        if !text.is_empty() {
            nodes.push(InlineNode::new(text.to_string()).marks(vec![(0..text.len(), mark)]));
        }
        nodes.extend(slice(last, b.offset..last.text_len()));
        let remove: HashSet<_> = ps
            .iter()
            .skip_while(|p| p.editor_id() != a.block)
            .skip(1)
            .take_while(|p| p.editor_id() != b.block)
            .map(|p| p.editor_id())
            .chain((a.block != b.block).then_some(b.block))
            .collect();
        find_mut(&mut self.blocks, a.block).unwrap().children = nodes;
        fn prune(nodes: &mut Vec<BlockNode>, remove: &HashSet<u64>) {
            nodes.retain_mut(|node| {
                if paragraph(node).is_some_and(|p| remove.contains(&p.editor_id())) {
                    return false;
                }
                if let Some(nodes) = children_mut(node) {
                    prune(nodes, remove);
                    return !nodes.is_empty();
                }
                true
            });
        }
        if a.block != b.block {
            prune(&mut self.blocks, &remove);
        }
        Position {
            block: a.block,
            offset: a.offset + text.len(),
        }
    }

    pub fn split(&mut self, at: Position) -> Position {
        let p = self
            .paragraphs()
            .into_iter()
            .find(|p| p.editor_id() == at.block)
            .unwrap();
        let tail = slice(p, at.offset..p.text_len());
        let head = slice(p, 0..at.offset);
        let new_p = self.new_paragraph(tail);
        let result = Position {
            block: new_p.editor_id(),
            offset: 0,
        };
        find_mut(&mut self.blocks, at.block).unwrap().children = head;
        fn insert(nodes: &mut Vec<BlockNode>, id: u64, new_p: &Paragraph) -> bool {
            for ix in 0..nodes.len() {
                if paragraph(&nodes[ix]).is_some_and(|p| p.editor_id() == id) {
                    nodes.insert(ix + 1, BlockNode::Paragraph(new_p.clone()));
                    return true;
                }
                if let BlockNode::List {
                    children: items, ..
                } = &mut nodes[ix]
                {
                    for item_ix in 0..items.len() {
                        if let BlockNode::ListItem {
                            children, checked, ..
                        } = &items[item_ix]
                        {
                            if children
                                .first()
                                .and_then(paragraph)
                                .is_some_and(|p| p.editor_id() == id)
                            {
                                let mut tail = vec![BlockNode::Paragraph(new_p.clone())];
                                let checked = checked.map(|_| false);
                                tail.extend(children[1..].iter().cloned());
                                children_mut(&mut items[item_ix]).unwrap().truncate(1);
                                items.insert(
                                    item_ix + 1,
                                    BlockNode::ListItem {
                                        children: tail,
                                        spread: false,
                                        checked,
                                        span: None,
                                    },
                                );
                                return true;
                            }
                        }
                    }
                }
                if let Some(children) = children_mut(&mut nodes[ix]) {
                    if insert(children, id, new_p) {
                        return true;
                    }
                }
            }
            false
        }
        insert(&mut self.blocks, at.block, &new_p);
        result
    }

    pub fn adjacent(&self, at: Position, forward: bool) -> Position {
        let ps = self.paragraphs();
        let ix = ps.iter().position(|p| p.editor_id() == at.block).unwrap();
        let text = ps[ix].text();
        if forward {
            if at.offset < text.len() {
                Position {
                    offset: next_boundary(&text, at.offset),
                    ..at
                }
            } else if ix + 1 < ps.len() {
                Position {
                    block: ps[ix + 1].editor_id(),
                    offset: 0,
                }
            } else {
                at
            }
        } else if at.offset > 0 {
            Position {
                offset: previous_boundary(&text, at.offset),
                ..at
            }
        } else if ix > 0 {
            Position {
                block: ps[ix - 1].editor_id(),
                offset: ps[ix - 1].text_len(),
            }
        } else {
            at
        }
    }

    pub fn format(&mut self, a: Position, b: Position, bold: bool) {
        let ranges: Vec<_> = self
            .paragraphs()
            .iter()
            .filter_map(|p| {
                self.selection_in(p.editor_id(), a, b)
                    .filter(|r| !r.is_empty())
                    .map(|r| (p.editor_id(), r))
            })
            .collect();
        let enabled = !ranges.iter().all(|(id, r)| {
            let p = self
                .paragraphs()
                .into_iter()
                .find(|p| p.editor_id() == *id)
                .unwrap();
            slice(p, r.clone()).iter().all(|n| {
                n.marks.iter().any(|(r, m)| {
                    r.start == 0 && r.end == n.text.len() && if bold { m.bold } else { m.italic }
                })
            })
        });
        for (id, r) in ranges {
            let p = find_mut(&mut self.blocks, id).unwrap();
            let mut nodes = slice(p, 0..r.start);
            for mut node in slice(p, r.clone()) {
                for (_, mark) in &mut node.marks {
                    if bold {
                        mark.bold = false;
                    } else {
                        mark.italic = false;
                    }
                }
                let mut mark = TextMark::default();
                if bold {
                    mark.bold = enabled;
                } else {
                    mark.italic = enabled;
                }
                node.marks.push((0..node.text.len(), mark));
                nodes.push(node);
            }
            nodes.extend(slice(p, r.end..p.text_len()));
            p.children = nodes;
        }
    }

    pub fn heading(&mut self, id: u64, level: u8) {
        fn convert(nodes: &mut [BlockNode], id: u64, level: u8) {
            for node in nodes {
                if let Some(p) = paragraph(node).filter(|p| p.editor_id() == id) {
                    let p = p.clone();
                    *node = if level == 0 {
                        BlockNode::Paragraph(p)
                    } else {
                        BlockNode::Heading {
                            level,
                            children: p,
                            span: None,
                        }
                    };
                    return;
                }
                if let Some(nodes) = children_mut(node) {
                    convert(nodes, id, level);
                }
            }
        }
        convert(&mut self.blocks, id, level.min(6));
    }

    pub fn list(&mut self, id: u64) {
        if self.unlist(id) {
            return;
        }
        self.make_list(id, false);
    }

    pub fn make_list(&mut self, id: u64, ordered: bool) {
        if let Some(ix) = self
            .blocks
            .iter()
            .position(|n| paragraph(n).is_some_and(|p| p.editor_id() == id))
        {
            let node = self.blocks.remove(ix);
            self.blocks.insert(
                ix,
                BlockNode::List {
                    children: vec![BlockNode::ListItem {
                        children: vec![node],
                        spread: false,
                        checked: None,
                        span: None,
                    }],
                    ordered,
                    span: None,
                },
            );
        }
    }

    // Reuse the list item's existing task state and Markdown serialization.
    pub fn make_task(&mut self, id: u64, checked: bool, require_list: bool) -> bool {
        if self.list_item_path(id).is_none() {
            if require_list {
                return false;
            }
            self.make_list(id, false);
        }
        let Some((path, ix)) = self.list_item_path(id) else {
            return false;
        };
        let BlockNode::List {
            children: items, ..
        } = node_at_mut(&mut self.blocks, &path)
        else {
            return false;
        };
        let BlockNode::ListItem { checked: task, .. } = &mut items[ix] else {
            return false;
        };
        *task = Some(checked);
        true
    }

    pub fn unlist(&mut self, id: u64) -> bool {
        fn unwrap(nodes: &mut Vec<BlockNode>, id: u64) -> bool {
            for ix in 0..nodes.len() {
                if let BlockNode::List {
                    children: items,
                    ordered,
                    ..
                } = &nodes[ix]
                {
                    if let Some(item_ix) = items.iter().position(|item| {
                        children(item).is_some_and(|nodes| {
                            nodes
                                .first()
                                .and_then(paragraph)
                                .is_some_and(|p| p.editor_id() == id)
                        })
                    }) {
                        let mut replacement = Vec::new();
                        if item_ix > 0 {
                            replacement.push(BlockNode::List {
                                children: items[..item_ix].to_vec(),
                                ordered: *ordered,
                                span: None,
                            });
                        }
                        replacement.extend(children(&items[item_ix]).unwrap().clone());
                        if item_ix + 1 < items.len() {
                            replacement.push(BlockNode::List {
                                children: items[item_ix + 1..].to_vec(),
                                ordered: *ordered,
                                span: None,
                            });
                        }
                        nodes.splice(ix..ix + 1, replacement);
                        return true;
                    }
                }
                if let Some(children) = children_mut(&mut nodes[ix]) {
                    if unwrap(children, id) {
                        return true;
                    }
                }
            }
            false
        }
        unwrap(&mut self.blocks, id)
    }

    // Locate the list and item by structure, keeping paragraph identities unchanged.
    fn list_item_path(&self, id: u64) -> Option<(Vec<usize>, usize)> {
        fn find(
            nodes: &[BlockNode],
            id: u64,
            path: &mut Vec<usize>,
        ) -> Option<(Vec<usize>, usize)> {
            for (ix, node) in nodes.iter().enumerate() {
                path.push(ix);
                if let BlockNode::List {
                    children: items, ..
                } = node
                {
                    if let Some(item) = items.iter().position(|item| {
                        children(item)
                            .and_then(|nodes| nodes.first())
                            .and_then(paragraph)
                            .is_some_and(|p| p.editor_id() == id)
                    }) {
                        return Some((path.clone(), item));
                    }
                }
                if let Some(nested) = children(node) {
                    if let Some(found) = find(nested, id, path) {
                        return Some(found);
                    }
                }
                path.pop();
            }
            None
        }
        find(&self.blocks, id, &mut Vec::new())
    }

    pub fn indent_list(&mut self, id: u64) -> bool {
        let Some((path, ix)) = self.list_item_path(id) else {
            return false;
        };
        if ix == 0 {
            return false;
        }
        let BlockNode::List {
            children: items,
            ordered,
            ..
        } = node_at_mut(&mut self.blocks, &path)
        else {
            return false;
        };
        let ordered = *ordered;
        let item = items.remove(ix);
        let nested = children_mut(&mut items[ix - 1]).unwrap();
        if let Some(BlockNode::List {
            children: siblings,
            ordered: sibling_ordered,
            ..
        }) = nested.last_mut()
        {
            if *sibling_ordered == ordered {
                siblings.push(item);
                return true;
            }
        }
        nested.push(BlockNode::List {
            children: vec![item],
            ordered,
            span: None,
        });
        true
    }

    pub fn outdent_list(&mut self, id: u64) -> bool {
        let Some((path, ix)) = self.list_item_path(id) else {
            return false;
        };
        // A nested list is a child of an item in an outer list.
        if path.len() < 3
            || !matches!(
                node_at_mut(&mut self.blocks, &path[..path.len() - 2]),
                BlockNode::List { .. }
            )
        {
            return self.unlist(id);
        }
        let (mut item, trailing, ordered, empty) = {
            let BlockNode::List {
                children: items,
                ordered,
                ..
            } = node_at_mut(&mut self.blocks, &path)
            else {
                return false;
            };
            let trailing = items.split_off(ix + 1);
            let item = items.remove(ix);
            (item, trailing, *ordered, items.is_empty())
        };
        if !trailing.is_empty() {
            children_mut(&mut item).unwrap().push(BlockNode::List {
                children: trailing,
                ordered,
                span: None,
            });
        }
        if empty {
            let parent = node_at_mut(&mut self.blocks, &path[..path.len() - 1]);
            children_mut(parent).unwrap().remove(*path.last().unwrap());
        }
        let parent_ix = path[path.len() - 2];
        let BlockNode::List {
            children: items, ..
        } = node_at_mut(&mut self.blocks, &path[..path.len() - 2])
        else {
            unreachable!()
        };
        items.insert(parent_ix + 1, item);
        true
    }

    pub fn move_block(&mut self, id: u64, down: bool) {
        let Some(ix) = self.blocks.iter().position(|node| {
            let mut ps = Vec::new();
            visit(std::slice::from_ref(node), &mut ps);
            ps.iter().any(|p| p.editor_id() == id)
        }) else {
            return;
        };
        let other = if down { ix + 1 } else { ix.saturating_sub(1) };
        if other < self.blocks.len() {
            self.blocks.swap(ix, other);
        }
    }
}

fn node_at_mut<'a>(nodes: &'a mut [BlockNode], path: &[usize]) -> &'a mut BlockNode {
    let node = &mut nodes[path[0]];
    if path.len() == 1 {
        node
    } else {
        node_at_mut(children_mut(node).unwrap(), &path[1..])
    }
}

pub(super) fn previous_boundary(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(ix, _)| ix)
        .take_while(|ix| *ix < offset)
        .last()
        .unwrap_or(0)
}

pub(super) fn next_boundary(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(ix, _)| ix)
        .find(|ix| *ix > offset)
        .unwrap_or(text.len())
}
