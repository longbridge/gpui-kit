use std::{
    collections::HashMap,
    ops::Range,
    sync::{Arc, Mutex},
};

use gpui::{
    AnyElement, App, DefiniteLength, Div, ElementId, FontStyle, FontWeight, HighlightStyle, Hsla,
    Image, ImageFormat, InteractiveElement as _, IntoElement, Length, ObjectFit, Overflow,
    ParentElement, Pixels, ScrollHandle, SharedString, SharedUri, StatefulInteractiveElement,
    StyleRefinement, Styled, StyledImage as _, WhiteSpace, Window, div, img,
    prelude::FluentBuilder as _, px, relative, rems,
};
use markdown::mdast;

use crate::{
    StyledExt, h_flex,
    scrollable_mask::horizontal_scroll_area,
    text::{
        CodeBlockActionsFn, CodeBlockHighlighterFn, LinkClickHandlerFn, MarkdownExtensions,
        MarkdownNode, TableActionsFn,
        document::NodeRenderOptions,
        inline::{
            Inline, InlineHighlight, InlineState, combine_highlights, text_runs, text_size_ranges,
        },
        inline_flow::{InlineFlow, InlineFlowItem, slice_ranges},
        text_view::handle_link_click,
    },
    theme::ActiveTheme as _,
    v_flex,
};

use super::{
    SelectionFormat, TextViewStyle,
    utils::{image_source, list_item_prefix},
};

const CHECK_SVG_LIGHT: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none"><path d="m3.25 8.25 3 3 6.5-7" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>"#;
const CHECK_SVG_DARK: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none"><path d="m3.25 8.25 3 3 6.5-7" stroke="black" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>"#;

/// The block-level nodes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BlockNode {
    /// Something like a Div container in HTML.
    Root {
        children: Vec<BlockNode>,
        span: Option<Span>,
    },
    Paragraph(Paragraph),
    Heading {
        level: u8,
        children: Paragraph,
        span: Option<Span>,
    },
    Blockquote {
        children: Vec<BlockNode>,
        span: Option<Span>,
    },
    List {
        /// Only contains ListItem, others will be ignored
        children: Vec<BlockNode>,
        ordered: bool,
        span: Option<Span>,
    },
    ListItem {
        children: Vec<BlockNode>,
        spread: bool,
        /// Whether the list item is checked, if None, it's not a checkbox
        checked: Option<bool>,
        span: Option<Span>,
    },
    CodeBlock(CodeBlock),
    /// A custom Markdown node produced by [`MarkdownExtensions`].
    Custom(MarkdownNode),
    Table(Table),
    Break {
        html: bool,
        span: Option<Span>,
    },
    HorizontalRule {
        span: Option<Span>,
    },
    /// Use for to_markdown get raw definition
    Definition {
        identifier: SharedString,
        url: SharedString,
        title: Option<SharedString>,
        span: Option<Span>,
    },
    Unknown,
}

#[derive(Clone, Copy)]
enum BlockTextKind {
    All,
    Selected,
    /// Like `Selected`, but reconstructs Markdown source for the selection
    /// instead of the rendered plain text.
    SelectedSource,
}

impl BlockNode {
    pub(super) fn is_list_item(&self) -> bool {
        matches!(self, Self::ListItem { .. })
    }

    /// Combine all children, omitting the empt parent nodes.
    pub(super) fn compact(self) -> BlockNode {
        match self {
            Self::Root { mut children, .. } if children.len() == 1 => children.remove(0).compact(),
            _ => self,
        }
    }

    /// Get the span of the node.
    pub(crate) fn span(&self) -> Option<Span> {
        match self {
            BlockNode::Root { span, .. } => *span,
            BlockNode::Paragraph(paragraph) => paragraph.span,
            BlockNode::Heading { span, .. } => *span,
            BlockNode::Blockquote { span, .. } => *span,
            BlockNode::List { span, .. } => *span,
            BlockNode::ListItem { span, .. } => *span,
            BlockNode::CodeBlock(code_block) => code_block.span,
            BlockNode::Custom(el) => el.span,
            BlockNode::Table(table) => table.span,
            BlockNode::Break { span, .. } => *span,
            BlockNode::HorizontalRule { span, .. } => *span,
            BlockNode::Definition { span, .. } => *span,
            BlockNode::Unknown { .. } => None,
        }
    }

    pub(super) fn text(&self) -> String {
        self.text_by_kind(BlockTextKind::All)
    }

    /// The selected text within this block, in `format`.
    ///
    /// [`SelectionFormat::Source`] reconstructs the Markdown source of the
    /// selection instead of the rendered text.
    pub(super) fn selected_text(&self, format: SelectionFormat) -> String {
        self.text_by_kind(match format {
            SelectionFormat::Plain => BlockTextKind::Selected,
            SelectionFormat::Source => BlockTextKind::SelectedSource,
        })
    }

    fn text_by_kind(&self, kind: BlockTextKind) -> String {
        let mut text = String::new();
        match self {
            BlockNode::Root { children, .. } => {
                let block_text = Self::children_text(children, kind);
                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::Paragraph(paragraph) => {
                let block_text = match kind {
                    BlockTextKind::All => paragraph.text(),
                    BlockTextKind::Selected => paragraph.selected_text(),
                    BlockTextKind::SelectedSource => paragraph.selected_source(),
                };
                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::Heading {
                level, children, ..
            } => {
                let block_text = match kind {
                    BlockTextKind::All => children.text(),
                    BlockTextKind::Selected => children.selected_text(),
                    BlockTextKind::SelectedSource => children.selected_source(),
                };
                if !block_text.is_empty() {
                    // In source mode, prefix the heading marker so a selected
                    // heading round-trips as Markdown (e.g. `## Title`).
                    if matches!(kind, BlockTextKind::SelectedSource) {
                        text.push_str(&"#".repeat(*level as usize));
                        text.push(' ');
                    }
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::List {
                children, ordered, ..
            } => {
                if matches!(kind, BlockTextKind::SelectedSource) {
                    // Reconstruct the list source, indenting nested lists and
                    // restoring list markers and task-list checkboxes.
                    text.push_str(&list_selected_source(children, *ordered, ""));
                } else {
                    text.push_str(&Self::children_text(children, kind));
                }
            }
            BlockNode::ListItem { children, .. } => {
                text.push_str(&Self::children_text(children, kind));
            }
            BlockNode::Blockquote { children, .. } => {
                let block_text = Self::children_text(children, kind);

                if !block_text.is_empty() {
                    if matches!(kind, BlockTextKind::SelectedSource) {
                        // Prefix every line with `> ` so a selected blockquote
                        // round-trips as Markdown.
                        let quoted = block_text
                            .trim_end_matches('\n')
                            .lines()
                            .map(|line| {
                                if line.is_empty() {
                                    ">".to_string()
                                } else {
                                    format!("> {}", line)
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        text.push_str(&quoted);
                    } else {
                        text.push_str(&block_text);
                    }
                    text.push('\n');
                }
            }
            BlockNode::Table(table) => {
                if matches!(kind, BlockTextKind::SelectedSource) {
                    let block_text = table_selected_source(table);
                    if !block_text.is_empty() {
                        text.push_str(&block_text);
                        text.push('\n');
                    }
                } else {
                    let mut block_text = String::new();
                    for row in table.children.iter() {
                        let mut row_texts = vec![];
                        for cell in row.children.iter() {
                            row_texts.push(match kind {
                                BlockTextKind::All => cell.children.text(),
                                // Source is handled above; only Selected reaches here.
                                _ => cell.children.selected_text(),
                            });
                        }
                        if !row_texts.is_empty() {
                            block_text.push_str(&row_texts.join(" "));
                            block_text.push('\n');
                        }
                    }

                    if !block_text.is_empty() {
                        text.push_str(&block_text);
                        text.push('\n');
                    }
                }
            }
            BlockNode::CodeBlock(code_block) => {
                let block_text = match kind {
                    BlockTextKind::All => code_block.text(),
                    BlockTextKind::Selected => code_block.selected_text(),
                    BlockTextKind::SelectedSource => code_block.selected_source(),
                };
                if !block_text.is_empty() {
                    text.push_str(&block_text);
                    text.push('\n');
                }
            }
            BlockNode::Custom(node) => {
                if let BlockTextKind::All = kind {
                    let content = node.as_text();
                    if !content.is_empty() {
                        text.push_str(content);
                        text.push('\n');
                    }
                }
            }
            BlockNode::Definition { .. }
            | BlockNode::Break { .. }
            | BlockNode::HorizontalRule { .. }
            | BlockNode::Unknown { .. } => {}
        }

        text
    }

    fn children_text(children: &[BlockNode], kind: BlockTextKind) -> String {
        let mut text = String::new();
        for child in children.iter() {
            text.push_str(&child.text_by_kind(kind));
        }

        text
    }

    /// Synchronously clear the selection stored in every inline state.
    ///
    /// Mirrors the [`selected_text`](Self::selected_text) traversal so the
    /// selection can be cleared without relying on a repaint.
    /// Whether this block carries a selection, even an empty one.
    ///
    /// A block only learns its selection when it is painted, so this doubles as
    /// "this block was on screen while the selection was made". An empty
    /// selection is the caret left by the press that started the drag, which is
    /// why it counts (see [`ParsedDocument::selected_text`]).
    pub(super) fn has_selection(&self) -> bool {
        match self {
            BlockNode::Root { children, .. }
            | BlockNode::Blockquote { children, .. }
            | BlockNode::List { children, .. }
            | BlockNode::ListItem { children, .. } => {
                children.iter().any(|child| child.has_selection())
            }
            BlockNode::Paragraph(paragraph) => paragraph.has_selection(),
            BlockNode::Heading { children, .. } => children.has_selection(),
            BlockNode::Table(table) => table.children.iter().any(|row| {
                row.children
                    .iter()
                    .any(|cell| cell.children.has_selection())
            }),
            BlockNode::CodeBlock(code_block) => code_block.has_selection(),
            BlockNode::Custom { .. }
            | BlockNode::Definition { .. }
            | BlockNode::Break { .. }
            | BlockNode::HorizontalRule { .. }
            | BlockNode::Unknown { .. } => false,
        }
    }

    pub(super) fn clear_selection(&self) {
        match self {
            BlockNode::Root { children, .. }
            | BlockNode::Blockquote { children, .. }
            | BlockNode::List { children, .. }
            | BlockNode::ListItem { children, .. } => {
                for child in children.iter() {
                    child.clear_selection();
                }
            }
            BlockNode::Paragraph(paragraph) => paragraph.clear_selection(),
            BlockNode::Heading { children, .. } => children.clear_selection(),
            BlockNode::Table(table) => {
                for row in table.children.iter() {
                    for cell in row.children.iter() {
                        cell.children.clear_selection();
                    }
                }
            }
            BlockNode::CodeBlock(code_block) => code_block.clear_selection(),
            BlockNode::Custom { .. }
            | BlockNode::Definition { .. }
            | BlockNode::Break { .. }
            | BlockNode::HorizontalRule { .. }
            | BlockNode::Unknown { .. } => {}
        }
    }
}

#[allow(unused)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LinkMark {
    pub url: SharedString,
    /// Optional identifier for footnotes.
    pub identifier: Option<SharedString>,
    pub title: Option<SharedString>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TextMark {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub underline: bool,
    pub code: bool,
    /// Highlight (`<mark>`) the text with this background color.
    ///
    /// `None` means the text is not highlighted.
    pub highlight: Option<Hsla>,
    pub link: Option<LinkMark>,
}

impl TextMark {
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    pub fn code(mut self) -> Self {
        self.code = true;
        self
    }

    /// Mark the text as highlighted (`<mark>`) with the given background color.
    pub fn highlight(mut self, color: Hsla) -> Self {
        self.highlight = Some(color);
        self
    }

    pub fn link(mut self, link: impl Into<LinkMark>) -> Self {
        self.link = Some(link.into());
        self
    }

    pub fn merge(&mut self, other: TextMark) {
        self.bold |= other.bold;
        self.italic |= other.italic;
        self.strikethrough |= other.strikethrough;
        self.underline |= other.underline;
        self.code |= other.code;
        if other.highlight.is_some() {
            self.highlight = other.highlight;
        }
        if let Some(link) = other.link {
            self.link = Some(link);
        }
    }
}

/// The bytes
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl From<Span> for ElementId {
    fn from(value: Span) -> Self {
        ElementId::Name(format!("md-{}:{}", value.start, value.end).into())
    }
}

#[allow(unused)]
#[derive(Debug, Default, Clone)]
pub struct ImageNode {
    pub url: SharedUri,
    pub link: Option<LinkMark>,
    pub title: Option<SharedString>,
    pub alt: Option<SharedString>,
    pub width: Option<DefiniteLength>,
    pub height: Option<DefiniteLength>,
}

impl ImageNode {
    pub fn title(&self) -> String {
        self.title
            .clone()
            .unwrap_or_else(|| self.alt.clone().unwrap_or_default())
            .to_string()
    }
}

impl PartialEq for ImageNode {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
            && self.link == other.link
            && self.title == other.title
            && self.alt == other.alt
            && self.width == other.width
            && self.height == other.height
    }
}

#[derive(Default, Clone, Debug)]
pub(crate) struct InlineNode {
    /// The text content.
    pub(crate) text: SharedString,
    pub(crate) image: Option<ImageNode>,
    /// The text styles, each tuple contains the range of the text and the style.
    pub(crate) marks: Vec<(Range<usize>, TextMark)>,

    state: Arc<Mutex<InlineState>>,
}

impl PartialEq for InlineNode {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text && self.image == other.image && self.marks == other.marks
    }
}

/// Wrap `text` with the Markdown syntax implied by `mark`.
///
/// This mirrors the per-mark formatting in [`Paragraph::to_markdown`] but
/// operates on an already-sliced run, so it can reconstruct the Markdown
/// source for a *partial* text selection. Applied inside-out (innermost markup
/// first) so nested emphasis like `**_x_**` round-trips.
pub(crate) fn wrap_with_mark(text: &str, mark: &TextMark) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut out = text.to_string();
    if mark.code {
        out = format!("`{}`", out);
    }
    if mark.italic {
        out = format!("*{}*", out);
    }
    if mark.bold {
        out = format!("**{}**", out);
    }
    if mark.strikethrough {
        out = format!("~~{}~~", out);
    }
    if mark.underline {
        // Markdown has no underline syntax, and `__` reads as bold to most
        // parsers, so fall back to the inline HTML `<u>` parses from.
        out = format!("<u>{}</u>", out);
    }
    if mark.highlight.is_some() {
        out = format!("=={}==", out);
    }
    if let Some(link) = &mark.link {
        out = match &link.title {
            Some(title) => format!("[{}]({} \"{}\")", out, link.url, title),
            None => format!("[{}]({})", out, link.url),
        };
    }
    out
}

/// How a selection covers one rendered run, so the caller can tell whether it
/// continues into an adjacent inline image.
#[derive(Default)]
struct RunSelection {
    emitted: bool,
    at_start: bool,
    at_end: bool,
}

/// Emit the selected part of one rendered run, preceded by the images the
/// selection has run into. `run` holds the run's children with their offset
/// into the run's concatenated text.
fn emit_run(
    state: &Arc<Mutex<InlineState>>,
    run: &[(usize, &InlineNode)],
    pending_images: &mut Vec<String>,
    out: &mut String,
) -> RunSelection {
    let mut selected = RunSelection::default();
    let Ok(state) = state.lock() else {
        return selected;
    };
    let Some(selection) = &state.selection else {
        return selected;
    };
    if selection.start >= selection.end {
        return selected;
    }

    selected.at_start = selection.start == 0;
    selected.at_end = selection.end >= state.text.len();

    for (start, child) in run {
        let end = start + child.text.len();
        let lo = selection.start.max(*start);
        let hi = selection.end.min(end);
        if lo >= hi {
            continue;
        }

        if !selected.emitted {
            if selected.at_start {
                out.push_str(&pending_images.join(""));
            }
            pending_images.clear();
        }
        selected.emitted = true;

        out.push_str(&reconstruct_markdown(
            &child.text,
            &child.marks,
            (lo - start)..(hi - start),
        ));
    }

    selected
}

/// The Markdown source for an inline image, e.g. `![alt](url "title")`.
fn image_markdown(image: &ImageNode) -> String {
    let alt = image.alt.clone().unwrap_or_default();
    let title = image
        .title
        .clone()
        .map_or(String::new(), |title| format!(" \"{}\"", title));
    format!("![{}]({}{})", alt, image.url, title)
}

/// Reconstruct the Markdown source for the `selection` sub-range of a text run
/// carrying `marks`.
///
/// `selection` is a byte range into `text`. For each mark that overlaps the
/// selection, the overlapping slice is wrapped in the mark's Markdown syntax
/// (see [`wrap_with_mark`]); slices not covered by any mark are emitted
/// verbatim. This lets a rendered-offset selection be copied back as Markdown
/// source (e.g. selecting inside a `**bold**` run yields `**bold**`).
pub(crate) fn reconstruct_markdown(
    text: &str,
    marks: &[(Range<usize>, TextMark)],
    selection: Range<usize>,
) -> String {
    let start = selection.start.min(text.len());
    let end = selection.end.min(text.len());
    if start >= end {
        return String::new();
    }

    let mut out = String::new();
    let mut cursor = start;
    // Marks are stored in ascending, non-overlapping order by the parser.
    for (range, mark) in marks.iter() {
        let seg_start = range.start.max(start);
        let seg_end = range.end.min(end);
        if seg_start >= seg_end {
            continue;
        }
        // Emit any unmarked text before this mark verbatim.
        if cursor < seg_start {
            out.push_str(&text[cursor..seg_start]);
        }
        out.push_str(&wrap_with_mark(&text[seg_start..seg_end], mark));
        cursor = seg_end;
    }
    // Trailing unmarked text.
    if cursor < end {
        out.push_str(&text[cursor..end]);
    }
    out
}

/// Reconstruct the Markdown source of the selected cells of `table`.
///
/// Cells emit their own selected source; rows are piped (`| a | b |`) and the
/// delimiter/alignment row is inserted after the first row, so a selected
/// table round-trips as a Markdown table. Returns an empty string when no cell
/// is selected.
fn table_selected_source(table: &Table) -> String {
    let cell_source = |cell: &TableCell| cell.children.selected_source().replace('\n', " ");

    let any_selected = table.children.iter().any(|row| {
        row.children
            .iter()
            .any(|cell| !cell_source(cell).trim().is_empty())
    });
    if !any_selected {
        return String::new();
    }

    let mut lines: Vec<String> = Vec::new();
    for (row_ix, row) in table.children.iter().enumerate() {
        let cells: Vec<String> = row
            .children
            .iter()
            .map(|cell| cell_source(cell).trim().to_string())
            .collect();
        lines.push(format!("| {} |", cells.join(" | ")));

        // The Markdown delimiter row carries the column alignments and must
        // follow the header row.
        if row_ix == 0 {
            let aligns: Vec<String> = (0..row.children.len())
                .map(|ix| {
                    match table.column_align(ix) {
                        ColumnumnAlign::Left => ":--",
                        ColumnumnAlign::Center => ":-:",
                        ColumnumnAlign::Right => "--:",
                    }
                    .to_string()
                })
                .collect();
            lines.push(format!("| {} |", aligns.join(" | ")));
        }
    }

    lines.join("\n")
}

/// Reconstruct the Markdown source of the selected items of a list.
///
/// Restores the list marker (`- ` / `N. `) and task-list checkbox (`[x] ` /
/// `[ ] `) of each item, and recurses into nested lists with a deeper `indent`
/// so nesting is preserved. `indent` is the leading whitespace for this level;
/// nested levels are indented by the width of the parent marker so continuation
/// and sub-list lines align under the item text. Items with no selected content
/// are skipped but still consume an ordered number, so the remaining items keep
/// their original numbering.
fn list_selected_source(children: &[BlockNode], ordered: bool, indent: &str) -> String {
    let mut out = String::new();
    let mut item_ix = 0usize;

    for child in children {
        let BlockNode::ListItem {
            children: item_children,
            checked,
            ..
        } = child
        else {
            continue;
        };

        let marker = if ordered {
            format!("{}. ", item_ix + 1)
        } else {
            "- ".to_string()
        };
        let checkbox = match checked {
            Some(true) => "[x] ",
            Some(false) => "[ ] ",
            None => "",
        };
        let child_indent = format!("{}{}", indent, " ".repeat(marker.len()));

        // Split the item into its own content and any nested lists, so the
        // nested lists can be indented under the content.
        let mut content = String::new();
        let mut nested = String::new();
        for sub in item_children {
            if let BlockNode::List {
                children: sub_children,
                ordered: sub_ordered,
                ..
            } = sub
            {
                nested.push_str(&list_selected_source(
                    sub_children,
                    *sub_ordered,
                    &child_indent,
                ));
            } else {
                content.push_str(&sub.text_by_kind(BlockTextKind::SelectedSource));
            }
        }
        let content = content.trim_end_matches('\n');

        if content.is_empty() && nested.is_empty() {
            item_ix += 1;
            continue;
        }

        if content.is_empty() {
            // An item whose only selected content is a nested list.
            out.push_str(indent);
            out.push_str(&marker);
            out.push_str(checkbox.trim_end());
            out.push('\n');
        } else {
            // The first line carries the marker and checkbox; continuation
            // lines are indented to align under the item text.
            let mut lines = content.lines();
            if let Some(first) = lines.next() {
                out.push_str(indent);
                out.push_str(&marker);
                out.push_str(checkbox);
                out.push_str(first);
                out.push('\n');
            }
            for line in lines {
                out.push_str(&child_indent);
                out.push_str(line);
                out.push('\n');
            }
        }
        out.push_str(&nested);
        item_ix += 1;
    }

    out
}

impl InlineNode {
    pub(crate) fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            image: None,
            marks: vec![],
            state: Arc::new(Mutex::new(InlineState::default())),
        }
    }

    pub(crate) fn image(image: ImageNode) -> Self {
        let mut this = Self::new("");
        this.image = Some(image);
        this
    }

    pub(crate) fn marks(mut self, marks: Vec<(Range<usize>, TextMark)>) -> Self {
        self.marks = marks;
        self
    }
}

/// The paragraph element, contains multiple text nodes.
///
/// Unlike other Element, this is cloneable, because it is used in the Node AST.
/// We are keep the selection state inside this AST Nodes.
#[derive(Debug, Clone, Default)]
pub(crate) struct Paragraph {
    pub(super) span: Option<Span>,
    pub(super) children: Vec<InlineNode>,
    /// The link references in this paragraph, used for reference links.
    ///
    /// The key is the identifier, the value is the url.
    pub(super) link_refs: HashMap<SharedString, SharedString>,

    pub(crate) state: Arc<Mutex<InlineState>>,
}

impl PartialEq for Paragraph {
    fn eq(&self, other: &Self) -> bool {
        self.span == other.span
            && self.children == other.children
            && self.link_refs == other.link_refs
    }
}

impl Paragraph {
    pub(crate) fn new(text: String) -> Self {
        Self {
            span: None,
            children: vec![InlineNode::new(&text)],
            link_refs: HashMap::new(),
            state: Arc::new(Mutex::new(InlineState::default())),
        }
    }

    pub(super) fn selected_text(&self) -> String {
        let mut text = String::new();

        for c in self.children.iter() {
            let Ok(state) = c.state.lock() else {
                continue;
            };
            if let Some(selection) = &state.selection {
                text.push_str(&state.text[selection.start..selection.end]);
            }
        }

        if let Ok(state) = self.state.lock()
            && let Some(selection) = &state.selection
        {
            text.push_str(&state.text[selection.start..selection.end]);
        }

        text
    }

    /// Reconstruct the Markdown source for the current selection.
    ///
    /// Mirrors [`selected_text`](Self::selected_text), but emits Markdown
    /// instead of the rendered text, using each inline node's `marks` (see
    /// [`reconstruct_markdown`]).
    ///
    /// Selection offsets index an `InlineState.text`, and one such state spans
    /// *several* children: [`Paragraph::render`] concatenates children until it
    /// hits an inline image, stores that run in the image child's state, then
    /// starts over; whatever follows the last image is stored in `self.state`.
    /// So walk the children in the same runs and map each selected byte back to
    /// the child it was rendered from — mapping against a single child's text
    /// would attribute the same offsets to children in other runs.
    ///
    /// An image has no selection of its own, so it is emitted when the
    /// selection runs into it: reaching the end of the run before it, and
    /// starting at the beginning of the run after it. A paragraph that begins
    /// or ends with an image has no run on that side, which counts as reaching
    /// it.
    pub(super) fn selected_source(&self) -> String {
        let mut source = String::new();
        let mut pending_images: Vec<String> = Vec::new();
        let mut run: Vec<(usize, &InlineNode)> = Vec::new();
        let mut offset = 0;
        let mut enters_image = true;

        for child in self.children.iter() {
            let Some(image) = &child.image else {
                run.push((offset, child));
                offset += child.text.len();
                continue;
            };

            // The run before an image is stored in that image's own state.
            let run_before = !run.is_empty();
            let selected = emit_run(&child.state, &run, &mut pending_images, &mut source);
            if run_before {
                enters_image = selected.emitted && selected.at_end;
            }
            if enters_image {
                pending_images.push(image_markdown(image));
            } else {
                pending_images.clear();
            }

            run.clear();
            offset = 0;
        }

        let trailing = emit_run(&self.state, &run, &mut pending_images, &mut source);
        // Trailing images have no run after them to flush them.
        if !trailing.emitted && enters_image && !source.is_empty() {
            source.push_str(&pending_images.join(""));
        }

        source
    }

    pub(super) fn text(&self) -> String {
        let mut text = String::new();
        for node in self.children.iter() {
            text.push_str(&node.text);
        }
        text
    }

    /// Synchronously clear the selection stored in every inline state.
    ///
    /// Mirrors the [`selected_text`](Self::selected_text) traversal.
    pub(super) fn has_selection(&self) -> bool {
        self.children
            .iter()
            .any(|c| c.state.lock().is_ok_and(|state| state.selection.is_some()))
            || self
                .state
                .lock()
                .is_ok_and(|state| state.selection.is_some())
    }

    pub(super) fn clear_selection(&self) {
        for c in self.children.iter() {
            if let Ok(mut state) = c.state.lock() {
                state.selection = None;
            }
        }

        if let Ok(mut state) = self.state.lock() {
            state.selection = None;
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Table {
    pub(crate) children: Vec<TableRow>,
    pub(crate) column_aligns: Vec<ColumnumnAlign>,
    pub(crate) span: Option<Span>,
}

/// Plain snapshot of a rendered Markdown table, passed to the
/// [`crate::text::TextView::table_actions`] hook.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TableData {
    /// First table row (header cells) as plain text.
    pub headers: Vec<String>,
    /// Rows after the header as plain text cells. May be ragged while
    /// a table is still streaming in.
    pub rows: Vec<Vec<String>>,
    /// The table serialized back to GFM pipe-table Markdown, alignments kept.
    pub markdown: String,
    /// Byte range of the table in the Markdown source, for callers that need
    /// to map the table back to the document.
    ///
    /// Not needed to keep element ids apart: the actions row is wrapped in its
    /// own identified element, so plain ids like `"copy"` are already scoped
    /// per table.
    pub span: Option<Range<usize>>,
}

impl Table {
    pub(crate) fn column_align(&self, index: usize) -> ColumnumnAlign {
        self.column_aligns.get(index).copied().unwrap_or_default()
    }

    /// Serialize the table back to GFM pipe-table Markdown (`| a | b |`),
    /// preserving column alignments. Cell newlines collapse to spaces and
    /// `|` is escaped so rows stay intact.
    ///
    /// Mirrors [`table_selected_source`], which does the same for the selected
    /// cells only.
    pub(crate) fn to_markdown(&self) -> String {
        let mut lines: Vec<String> = Vec::with_capacity(self.children.len() + 1);

        for (row_ix, row) in self.children.iter().enumerate() {
            let cells: Vec<String> = row
                .children
                .iter()
                .map(|cell| {
                    cell.children
                        .to_markdown()
                        .trim()
                        .replace('\n', " ")
                        .replace('|', "\\|")
                })
                .collect();
            lines.push(format!("| {} |", cells.join(" | ")));

            // The Markdown delimiter row carries the column alignments and must
            // follow the header row.
            if row_ix == 0 {
                let aligns: Vec<String> = (0..row.children.len())
                    .map(|ix| {
                        match self.column_align(ix) {
                            ColumnumnAlign::Left => ":--",
                            ColumnumnAlign::Center => ":-:",
                            ColumnumnAlign::Right => "--:",
                        }
                        .to_string()
                    })
                    .collect();
                lines.push(format!("| {} |", aligns.join(" | ")));
            }
        }

        lines.join("\n")
    }

    /// Snapshot of this table for the [`crate::text::TextView::table_actions`]
    /// hook.
    pub(crate) fn table_data(&self) -> TableData {
        let row_text = |row: &TableRow| {
            row.children
                .iter()
                .map(|cell| cell.children.text().trim().to_string())
                .collect::<Vec<_>>()
        };

        TableData {
            headers: self.children.first().map(row_text).unwrap_or_default(),
            rows: self.children.iter().skip(1).map(row_text).collect(),
            markdown: self.to_markdown(),
            span: self.span.map(|span| span.start..span.end),
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub(crate) enum ColumnumnAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl From<mdast::AlignKind> for ColumnumnAlign {
    fn from(value: mdast::AlignKind) -> Self {
        match value {
            mdast::AlignKind::None => ColumnumnAlign::Left,
            mdast::AlignKind::Left => ColumnumnAlign::Left,
            mdast::AlignKind::Center => ColumnumnAlign::Center,
            mdast::AlignKind::Right => ColumnumnAlign::Right,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TableRow {
    pub children: Vec<TableCell>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TableCell {
    pub children: Paragraph,
    pub width: Option<DefiniteLength>,
}

impl Paragraph {
    pub(crate) fn take(&mut self) -> Paragraph {
        std::mem::replace(
            self,
            Paragraph {
                span: None,
                children: vec![],
                link_refs: Default::default(),
                state: Arc::new(Mutex::new(InlineState::default())),
            },
        )
    }

    pub(crate) fn is_image(&self) -> bool {
        false
    }

    pub(crate) fn set_span(&mut self, span: Span) {
        self.span = Some(span);
    }

    pub(crate) fn push_str(&mut self, text: &str) {
        self.children.push(
            InlineNode::new(text.to_string()).marks(vec![(0..text.len(), TextMark::default())]),
        );
    }

    pub(crate) fn push(&mut self, text: InlineNode) {
        self.children.push(text);
    }

    pub(crate) fn push_image(&mut self, image: ImageNode) {
        self.children.push(InlineNode::image(image));
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.children.is_empty()
            || self
                .children
                .iter()
                .all(|node| node.text.is_empty() && node.image.is_none())
    }

    /// Return length of children text.
    pub(crate) fn text_len(&self) -> usize {
        self.children
            .iter()
            .map(|node| node.text.len())
            .sum::<usize>()
    }

    pub(crate) fn merge(&mut self, other: Self) {
        self.children.extend(other.children);
    }
}

#[derive(Debug, Clone)]
pub struct CodeBlock {
    lang: Option<SharedString>,
    state: Arc<Mutex<InlineState>>,
    highlight_cache: Arc<Mutex<Option<CachedCodeBlockHighlights>>>,
    pub span: Option<Span>,
}

struct CachedCodeBlockHighlights {
    highlighter: Arc<CodeBlockHighlighterFn>,
    styles: Vec<(Range<usize>, HighlightStyle)>,
}

impl std::fmt::Debug for CachedCodeBlockHighlights {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedCodeBlockHighlights")
            .field("styles", &self.styles)
            .finish_non_exhaustive()
    }
}

impl PartialEq for CodeBlock {
    fn eq(&self, other: &Self) -> bool {
        self.lang == other.lang && self.code() == other.code() && self.span == other.span
    }
}

impl CodeBlock {
    /// Get the language of the code block.
    pub fn lang(&self) -> Option<SharedString> {
        self.lang.clone()
    }

    /// Get the code content of the code block.
    pub fn code(&self) -> SharedString {
        self.state
            .lock()
            .map(|state| state.text.clone())
            .unwrap_or_default()
    }

    /// Builds a code block that is not tied to a parsed document.
    ///
    /// [`crate::TextView::code_block_highlighter`] hands a `&CodeBlock` to the
    /// highlighter it is given, so anyone writing one needs a way to build a
    /// block to exercise it against.
    pub fn from_code(code: impl Into<SharedString>, lang: Option<impl Into<SharedString>>) -> Self {
        Self::new(code.into(), lang.map(Into::into), None::<Span>)
    }

    pub(crate) fn new(
        code: SharedString,
        lang: Option<SharedString>,
        span: Option<impl Into<Span>>,
    ) -> Self {
        let state = Arc::new(Mutex::new(InlineState::default()));
        if let Ok(mut state) = state.lock() {
            state.set_text(code);
        }

        Self {
            lang,
            state,
            highlight_cache: Arc::new(Mutex::new(None)),
            span: span.map(|s| s.into()),
        }
    }

    fn highlighted_styles(
        &self,
        highlighter: &Arc<CodeBlockHighlighterFn>,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        if let Ok(cache) = self.highlight_cache.lock()
            && let Some(cache) = cache.as_ref()
            && Arc::ptr_eq(&cache.highlighter, highlighter)
        {
            return cache.styles.clone();
        }

        let code_len = self.code().len();
        let styles = highlighter(self)
            .into_iter()
            .filter(|(range, _)| range.start <= range.end && range.end <= code_len)
            .collect::<Vec<_>>();
        if let Ok(mut cache) = self.highlight_cache.lock() {
            *cache = Some(CachedCodeBlockHighlights {
                highlighter: highlighter.clone(),
                styles: styles.clone(),
            });
        }
        styles
    }

    pub(super) fn selected_text(&self) -> String {
        let mut text = String::new();
        if let Ok(state) = self.state.lock()
            && let Some(selection) = &state.selection
        {
            text.push_str(&state.text[selection.start..selection.end]);
        }
        text
    }

    /// Markdown source for the current selection.
    ///
    /// The selected code is wrapped in a fenced code block carrying the block's
    /// language, so a selected code block round-trips as Markdown (e.g.
    /// ```` ```rust\n…\n``` ````) instead of pasting as bare, unfenced text.
    /// A partial selection is still emitted as a valid fenced block.
    pub(super) fn selected_source(&self) -> String {
        let code = self.selected_text();
        if code.is_empty() {
            return String::new();
        }
        let lang = self.lang.clone().unwrap_or_default();
        // Trim trailing newlines so the closing fence sits on its own line
        // directly after the last code line (no blank line before it).
        let code = code.trim_end_matches('\n');
        format!("```{}\n{}\n```", lang, code)
    }

    pub(super) fn text(&self) -> String {
        self.state
            .lock()
            .map(|state| state.text.to_string())
            .unwrap_or_default()
    }

    /// Synchronously clear the selection stored in the inline state.
    ///
    /// Mirrors the [`selected_text`](Self::selected_text) traversal.
    pub(super) fn has_selection(&self) -> bool {
        self.state
            .lock()
            .is_ok_and(|state| state.selection.is_some())
    }

    pub(super) fn clear_selection(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.selection = None;
        }
    }

    fn render(
        &self,
        options: &NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let style = &node_cx.style;

        div()
            .w_full()
            .min_w_0()
            .when(!options.is_last, |this| this.pb(style.paragraph_gap()))
            .child(
                div()
                    .id(("codeblock", options.ix))
                    .w_full()
                    .min_w_0()
                    .p_3()
                    .bg(style.code_background())
                    .font_family(cx.theme().tokens.typography.mono.clone())
                    .text_size(cx.theme().tokens.typography.mono_md.size)
                    .relative()
                    .refine_style(&style.code_block())
                    .child(Inline::new(
                        "code",
                        self.state.clone(),
                        vec![],
                        node_cx
                            .code_block_highlighter
                            .as_ref()
                            .map(|highlighter| self.highlighted_styles(highlighter))
                            .unwrap_or_default()
                            .into_iter()
                            .map(|(range, style)| (range, InlineHighlight::from(style)))
                            .collect(),
                        node_cx.link_click_handler.clone(),
                    ))
                    .when_some(node_cx.code_block_actions.clone(), |this, actions| {
                        this.child(
                            div()
                                .id("actions")
                                .absolute()
                                .top_2()
                                .right_2()
                                .bg(style.code_background())
                                .rounded(cx.theme().tokens.radius.md)
                                .child(actions(&self, window, cx)),
                        )
                    }),
            )
            .into_any_element()
    }
}

/// A context for rendering nodes, contains link references.
#[derive(Default, Clone)]
pub(crate) struct NodeContext {
    /// The byte offset of the node in the original markdown text.
    /// Used for incremental updates.
    pub(crate) offset: usize,
    pub(crate) link_refs: HashMap<SharedString, LinkMark>,
    pub(crate) style: TextViewStyle,
    pub(crate) code_block_actions: Option<Arc<CodeBlockActionsFn>>,
    pub(crate) code_block_highlighter: Option<Arc<CodeBlockHighlighterFn>>,
    pub(crate) table_actions: Option<Arc<TableActionsFn>>,
    pub(crate) link_click_handler: Option<Arc<LinkClickHandlerFn>>,
    pub(crate) markdown_extensions: Arc<MarkdownExtensions>,
}

impl NodeContext {
    pub(super) fn add_ref(&mut self, identifier: SharedString, link: LinkMark) {
        self.link_refs.insert(identifier, link);
    }
}

impl PartialEq for NodeContext {
    fn eq(&self, other: &Self) -> bool {
        self.link_refs == other.link_refs && self.style == other.style
        // Note: code_block_actions, table_actions and markdown_extensions are
        // intentionally not compared (closures can't be compared)
    }
}

/// The highlight a text mark renders with. The link decoration is applied by
/// the caller, which also has to record the link range.
fn mark_highlight(mark: &TextMark, node_cx: &NodeContext, cx: &App) -> InlineHighlight {
    let mut highlight = HighlightStyle::default();
    if mark.bold {
        highlight.font_weight = Some(FontWeight::BOLD);
    }
    if mark.italic {
        highlight.font_style = Some(FontStyle::Italic);
    }
    if mark.strikethrough {
        highlight.strikethrough = Some(gpui::StrikethroughStyle {
            thickness: gpui::px(1.),
            ..Default::default()
        });
    }
    if mark.underline {
        highlight.underline = Some(gpui::UnderlineStyle {
            thickness: gpui::px(1.),
            ..Default::default()
        });
    }
    let mut font_family = None;
    if mark.code {
        highlight = highlight.highlight(node_cx.style.inline_code_highlight());
        font_family = Some(cx.theme().tokens.typography.mono.clone());
    }
    if let Some(color) = mark.highlight {
        highlight.background_color = Some(color);
    }
    InlineHighlight {
        style: highlight,
        font_family,
        font_size_scale: mark.code.then_some(0.875),
    }
}

impl Paragraph {
    /// The highlights over [`Self::text`], for measuring the paragraph with
    /// the runs it renders with. Link colors are left out: they do not move
    /// glyphs.
    fn inline_highlights(
        &self,
        node_cx: &NodeContext,
        cx: &App,
    ) -> Vec<(Range<usize>, InlineHighlight)> {
        let mut highlights = vec![];
        let mut offset = 0;
        for inline_node in &self.children {
            let node_highlights = inline_node
                .marks
                .iter()
                .map(|(range, mark)| {
                    (
                        (offset + range.start)..(offset + range.end),
                        mark_highlight(mark, node_cx, cx),
                    )
                })
                .collect::<Vec<_>>();
            highlights = combine_highlights(highlights, node_highlights);
            offset += inline_node.text.len();
        }
        highlights
    }

    fn render(&self, node_cx: &NodeContext, _window: &mut Window, cx: &mut App) -> AnyElement {
        let span = self.span;
        let children = &self.children;

        if self.should_render_inline_flow() {
            return InlineFlow::new(
                span.unwrap_or_default(),
                self.inline_flow_items(node_cx, cx),
                node_cx.link_click_handler.clone(),
            )
            .into_any_element();
        }

        let mut child_nodes: Vec<AnyElement> = vec![];

        let mut text = String::new();
        let mut highlights: Vec<(Range<usize>, InlineHighlight)> = vec![];
        let mut links: Vec<(Range<usize>, LinkMark)> = vec![];
        let mut offset = 0;

        let mut ix = 0;
        for inline_node in children {
            let text_len = inline_node.text.len();
            text.push_str(&inline_node.text);

            if let Some(image) = &inline_node.image {
                if text.len() > 0 {
                    if let Ok(mut state) = inline_node.state.lock() {
                        state.set_text(text.clone().into());
                    }
                    child_nodes.push(
                        Inline::new(
                            ix,
                            inline_node.state.clone(),
                            links.clone(),
                            highlights.clone(),
                            node_cx.link_click_handler.clone(),
                        )
                        .into_any_element(),
                    );
                }
                let link_click_handler = node_cx.link_click_handler.clone();
                child_nodes.push(
                    img(image_source(&image.url))
                        .id(ix)
                        .object_fit(ObjectFit::Contain)
                        .max_w(relative(1.))
                        .when_some(image.width, |this, width| this.w(width))
                        .when_some(image.link.clone(), |this, link| {
                            let link_click_handler = link_click_handler.clone();
                            let aux_link = link.clone();
                            let aux_link_click_handler = link_click_handler.clone();
                            this.cursor_pointer()
                                .on_click(move |event, window, cx| {
                                    crate::TextSelection::end(window, cx);
                                    cx.stop_propagation();
                                    handle_link_click(
                                        &link_click_handler,
                                        link.url.clone(),
                                        event.clone(),
                                        window,
                                        cx,
                                    );
                                })
                                .on_aux_click(move |event, window, cx| {
                                    crate::TextSelection::end(window, cx);
                                    cx.stop_propagation();
                                    handle_link_click(
                                        &aux_link_click_handler,
                                        aux_link.url.clone(),
                                        event.clone(),
                                        window,
                                        cx,
                                    );
                                })
                        })
                        .into_any_element(),
                );

                text.clear();
                links.clear();
                highlights.clear();
                offset = 0;
            } else {
                let mut node_highlights = vec![];
                for (range, style) in &inline_node.marks {
                    let inner_range = (offset + range.start)..(offset + range.end);
                    let mut highlight = mark_highlight(style, node_cx, cx);

                    if let Some(mut link_mark) = style.link.clone() {
                        highlight.style.color = Some(node_cx.style.link());
                        highlight.style.underline = Some(gpui::UnderlineStyle {
                            thickness: gpui::px(1.),
                            ..Default::default()
                        });

                        // convert link references, replace link
                        if let Some(identifier) = link_mark.identifier.as_ref() {
                            if let Some(mark) = node_cx.link_refs.get(identifier) {
                                link_mark = mark.clone();
                            }
                        }

                        links.push((inner_range.clone(), link_mark));
                    }

                    node_highlights.push((inner_range, highlight));
                }

                highlights = combine_highlights(highlights, node_highlights);
                offset += text_len;
            }
            ix += 1;
        }

        // Add the last text node
        if text.len() > 0 {
            if let Ok(mut state) = self.state.lock() {
                state.set_text(text.into());
            }
            child_nodes.push(
                Inline::new(
                    ix,
                    self.state.clone(),
                    links,
                    highlights,
                    node_cx.link_click_handler.clone(),
                )
                .into_any_element(),
            );
        }

        div()
            .id(span.unwrap_or_default())
            .children(child_nodes)
            .into_any_element()
    }

    fn should_render_inline_flow(&self) -> bool {
        let has_image = self.children.iter().any(|child| child.image.is_some());
        let has_text = self.children.iter().any(|child| !child.text.is_empty());
        (has_image && has_text)
            || self
                .children
                .iter()
                .any(|child| child.marks.iter().any(|(_, mark)| mark.code))
    }

    fn inline_flow_items(&self, node_cx: &NodeContext, cx: &mut App) -> Vec<InlineFlowItem> {
        let mut items = Vec::new();
        let mut text = String::new();
        let mut highlights: Vec<(Range<usize>, InlineHighlight)> = vec![];
        let mut links: Vec<(Range<usize>, LinkMark)> = vec![];
        let mut offset = 0;

        for inline_node in &self.children {
            let text_len = inline_node.text.len();
            text.push_str(&inline_node.text);

            if let Some(image) = &inline_node.image {
                if !text.is_empty() {
                    if let Ok(mut state) = inline_node.state.lock() {
                        state.set_text(text.clone().into());
                    }
                    items.push(InlineFlowItem::Text {
                        state: inline_node.state.clone(),
                        text: text.clone().into(),
                        links: links.clone(),
                        highlights: highlights.clone(),
                    });
                }

                items.push(InlineFlowItem::Image {
                    url: image.url.clone(),
                    link: image.link.clone(),
                    title: image.title(),
                    width: image.width,
                    height: image.height,
                });

                text.clear();
                links.clear();
                highlights.clear();
                offset = 0;
            } else {
                let mut node_highlights = vec![];
                for (range, style) in &inline_node.marks {
                    let inner_range = (offset + range.start)..(offset + range.end);
                    let mut highlight = mark_highlight(style, node_cx, cx);

                    if let Some(mut link_mark) = style.link.clone() {
                        highlight.style.color = Some(node_cx.style.link());
                        highlight.style.underline = Some(gpui::UnderlineStyle {
                            thickness: gpui::px(1.),
                            ..Default::default()
                        });

                        if let Some(identifier) = link_mark.identifier.as_ref()
                            && let Some(mark) = node_cx.link_refs.get(identifier)
                        {
                            link_mark = mark.clone();
                        }

                        links.push((inner_range.clone(), link_mark));
                    }

                    node_highlights.push((inner_range, highlight));
                }

                highlights = combine_highlights(highlights, node_highlights);
                offset += text_len;
            }
        }

        if !text.is_empty() {
            if let Ok(mut state) = self.state.lock() {
                state.set_text(text.clone().into());
            }
            items.push(InlineFlowItem::Text {
                state: self.state.clone(),
                text: text.into(),
                links,
                highlights,
            });
        }

        items
    }
}

const CELL_PAD_PX: f32 = 16.0; // px_2 horizontal padding
const CELL_MIN_PX: f32 = 48.0;
const CELL_BORDER_PX: f32 = 1.0; // border_r_1 drawn by every column but the last

/// The max-content width of every table column: the widest cell line,
/// shaped with the runs the cell renders with, plus the cell's padding and
/// border. Never capped: a cap would clip overflowing text *and* leave it
/// outside the scrollable width, making it unreachable.
fn measure_table_columns(
    table: &Table,
    col_count: usize,
    node_cx: &NodeContext,
    window: &mut Window,
    cx: &App,
) -> Vec<f32> {
    let text_style = window.text_style();
    let font_size = text_style.font_size.to_pixels(window.rem_size());
    let mut col_w = vec![CELL_MIN_PX; col_count];
    for row in table.children.iter() {
        for (ix, cell) in row.children.iter().enumerate() {
            let Some(slot) = col_w.get_mut(ix) else {
                continue;
            };
            let text = cell.children.text();
            let highlights = cell.children.inline_highlights(node_cx, cx);
            let mut w = 0.0_f32;
            let mut line_start = 0;
            for line in text.split('\n') {
                let start = line_start + (line.len() - line.trim_start().len());
                let line_end = line_start + line.len();
                line_start = line_end + 1;
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let end = start + line.len();
                let line_highlights = highlights
                    .iter()
                    .filter_map(|(range, highlight)| {
                        let clipped = range.start.max(start)..range.end.min(end);
                        (clipped.start < clipped.end).then(|| {
                            (
                                clipped.start - start..clipped.end - start,
                                highlight.clone(),
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                let mut line_w = gpui::Pixels::ZERO;
                for (range, scale) in text_size_ranges(line.len(), &line_highlights) {
                    let highlights = slice_ranges(
                        &line_highlights,
                        range.start,
                        range.end,
                        |range, highlight| (range, highlight.clone()),
                    );
                    if highlights.iter().any(|(_, h)| h.font_size_scale.is_some()) {
                        line_w += px(crate::text::inline_flow::INLINE_CODE_PADDING * 2.);
                    }
                    let runs = text_runs(range.len(), &text_style, &highlights);
                    line_w += window
                        .text_system()
                        .layout_line(&line[range], font_size * scale, &runs, None)
                        .width;
                }
                w = w.max(f32::from(line_w));
            }
            // Border-box widths, so the padding and border the cell draws
            // must leave the measured text its full width.
            let border = if ix + 1 < col_count {
                CELL_BORDER_PX
            } else {
                0.
            };
            *slot = slot.max(w + CELL_PAD_PX + border);
        }
    }
    col_w
}

impl Paragraph {
    fn to_markdown(&self) -> String {
        let mut text = self
            .children
            .iter()
            .map(|text_node| {
                let mut text = text_node.text.to_string();
                for (range, style) in &text_node.marks {
                    if style.bold {
                        text = format!("**{}**", &text_node.text[range.clone()]);
                    }
                    if style.italic {
                        text = format!("*{}*", &text_node.text[range.clone()]);
                    }
                    if style.strikethrough {
                        text = format!("~~{}~~", &text_node.text[range.clone()]);
                    }
                    if style.code {
                        text = format!("`{}`", &text_node.text[range.clone()]);
                    }
                    if style.highlight.is_some() {
                        text = format!("=={}==", &text_node.text[range.clone()]);
                    }
                    if let Some(link) = &style.link {
                        text = format!("[{}]({})", &text_node.text[range.clone()], link.url);
                    }
                }

                if let Some(image) = &text_node.image {
                    let alt = image.alt.clone().unwrap_or_default();
                    let title = image
                        .title
                        .clone()
                        .map_or(String::new(), |t| format!(" \"{}\"", t));
                    text.push_str(&format!("![{}]({}{})", alt, image.url, title))
                }

                text
            })
            .collect::<Vec<_>>()
            .join("");

        text.push_str("\n\n");
        text
    }
}

impl BlockNode {
    /// Converts the node to markdown format.
    ///
    /// This is used to generate markdown for test.
    #[allow(dead_code)]
    pub(crate) fn to_markdown(&self) -> String {
        match self {
            BlockNode::Root { children, .. } => children
                .iter()
                .map(|child| child.to_markdown())
                .collect::<Vec<_>>()
                .join("\n\n"),
            BlockNode::Paragraph(paragraph) => paragraph.to_markdown(),
            BlockNode::Heading {
                level, children, ..
            } => {
                let hashes = "#".repeat(*level as usize);
                format!("{} {}", hashes, children.to_markdown())
            }
            BlockNode::Blockquote { children, .. } => {
                let content = children
                    .iter()
                    .map(|child| child.to_markdown())
                    .collect::<Vec<_>>()
                    .join("\n\n");

                content
                    .lines()
                    .map(|line| format!("> {}", line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            BlockNode::List {
                children, ordered, ..
            } => children
                .iter()
                .enumerate()
                .map(|(i, child)| {
                    let prefix = if *ordered {
                        format!("{}. ", i + 1)
                    } else {
                        "- ".to_string()
                    };
                    format!("{}{}", prefix, child.to_markdown())
                })
                .collect::<Vec<_>>()
                .join("\n"),
            BlockNode::ListItem {
                children, checked, ..
            } => {
                let checkbox = if let Some(checked) = checked {
                    if *checked { "[x] " } else { "[ ] " }
                } else {
                    ""
                };
                format!(
                    "{}{}",
                    checkbox,
                    children
                        .iter()
                        .map(|child| child.to_markdown())
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
            BlockNode::CodeBlock(code_block) => {
                format!(
                    "```{}\n{}\n```",
                    code_block.lang.clone().unwrap_or_default(),
                    code_block.code()
                )
            }
            BlockNode::Table(table) => table.to_markdown(),
            BlockNode::Break { html, .. } => {
                if *html {
                    "<br>".to_string()
                } else {
                    "\n".to_string()
                }
            }
            BlockNode::HorizontalRule { .. } => "---".to_string(),
            BlockNode::Custom(node) => node.to_markdown(),
            BlockNode::Definition {
                identifier,
                url,
                title,
                ..
            } => {
                if let Some(title) = title {
                    format!("[{}]: {} \"{}\"", identifier, url, title)
                } else {
                    format!("[{}]: {}", identifier, url)
                }
            }
            BlockNode::Unknown { .. } => "".to_string(),
        }
        .trim()
        .to_string()
    }
}

impl BlockNode {
    fn render_list_item_row(
        content: AnyElement,
        ix: usize,
        options: NodeRenderOptions,
        checked: Option<bool>,
        style: &TextViewStyle,
        line_height: Pixels,
    ) -> Div {
        h_flex()
            .w_full()
            .flex_1()
            .min_w_0()
            .relative()
            .items_start()
            .content_start()
            .when(!options.todo && checked.is_none(), |this| {
                this.child(list_item_prefix(ix, options.ordered, options.depth))
            })
            .when_some(checked, |this, checked| {
                // Todo list checkbox
                let check_svg = if style.is_dark() {
                    CHECK_SVG_DARK
                } else {
                    CHECK_SVG_LIGHT
                };
                this.child(
                    div()
                        .flex()
                        .mr_1p5()
                        .h(line_height)
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .flex()
                                .size(rems(0.875))
                                .items_center()
                                .justify_center()
                                .border_1()
                                .border_color(style.foreground())
                                .when(checked, |this| {
                                    this.bg(style.foreground()).child(
                                        img(Arc::new(Image::from_bytes(
                                            ImageFormat::Svg,
                                            check_svg.to_vec(),
                                        )))
                                        .size(rems(0.625)),
                                    )
                                }),
                        ),
                )
            })
            .child(div().flex_1().min_w_0().overflow_hidden().child(content))
    }

    fn render_list_item(
        item: &BlockNode,
        ix: usize,
        options: NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        match item {
            BlockNode::ListItem {
                children,
                spread,
                checked,
                ..
            } => v_flex()
                .id(("li", options.ix))
                .w_full()
                .min_w_0()
                .when(*spread, |this| this.child(div()))
                .children({
                    let mut items: Vec<Div> = Vec::with_capacity(children.len());

                    for (child_ix, child) in children.iter().enumerate() {
                        match child {
                            BlockNode::Paragraph { .. } => {
                                let last_not_list = child_ix > 0
                                    && !matches!(children[child_ix - 1], BlockNode::List { .. });

                                let text = child.render_block(
                                    NodeRenderOptions {
                                        depth: options.depth + 1,
                                        todo: checked.is_some(),
                                        is_last: true,
                                        ..options
                                    },
                                    node_cx,
                                    window,
                                    cx,
                                );

                                // Continuation paragraph — stack vertically below
                                // the previous row, indented to align with the text
                                // column (past bullet/number prefix).
                                if last_not_list {
                                    if let Some(preceding_row) = items.pop() {
                                        items.push(
                                            v_flex().child(preceding_row).child(
                                                div()
                                                    .w_full()
                                                    .pl(rems(1.))
                                                    .overflow_hidden()
                                                    .child(text),
                                            ),
                                        );
                                        continue;
                                    }
                                }

                                items.push(Self::render_list_item_row(
                                    text,
                                    ix,
                                    options,
                                    *checked,
                                    &node_cx.style,
                                    window.line_height(),
                                ));
                            }
                            BlockNode::List { .. } => {
                                items.push(div().ml(rems(1.)).child(child.render_block(
                                    NodeRenderOptions {
                                        depth: options.depth + 1,
                                        todo: checked.is_some(),
                                        is_last: true,
                                        ..options
                                    },
                                    node_cx,
                                    window,
                                    cx,
                                )));
                            }
                            BlockNode::Root { .. }
                            | BlockNode::Heading { .. }
                            | BlockNode::Blockquote { .. }
                            | BlockNode::CodeBlock(_)
                            | BlockNode::Custom(_)
                            | BlockNode::Table(_)
                            | BlockNode::HorizontalRule { .. } => {
                                let block = child.render_block(
                                    NodeRenderOptions {
                                        depth: options.depth + 1,
                                        todo: checked.is_some(),
                                        is_last: true,
                                        ..options
                                    },
                                    node_cx,
                                    window,
                                    cx,
                                );

                                if child_ix == 0 {
                                    items.push(Self::render_list_item_row(
                                        block,
                                        ix,
                                        options,
                                        *checked,
                                        &node_cx.style,
                                        window.line_height(),
                                    ));
                                } else {
                                    // Indent continuation blocks to align with a
                                    // nested sub-list (`ml(rems(1.))`) and with
                                    // continuation paragraphs.
                                    items.push(
                                        div()
                                            .w_full()
                                            .min_w_0()
                                            .pl(rems(1.))
                                            .overflow_hidden()
                                            .child(block),
                                    );
                                }
                            }
                            BlockNode::ListItem { .. }
                            | BlockNode::Break { .. }
                            | BlockNode::Definition { .. }
                            | BlockNode::Unknown => {}
                        }
                    }
                    items
                })
                .into_any_element(),
            _ => div().into_any_element(),
        }
    }

    /// Render a Markdown table. Dispatches to a horizontally scrollable layout
    /// when `style.table` opts in with overflow-x: scroll, otherwise to the
    /// default layout that fits the container width and wraps cell content.
    fn render_table(
        item: &BlockNode,
        options: &NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        const DEFAULT_LENGTH: usize = 5;

        let table = match item {
            BlockNode::Table(table) => table,
            _ => return div().into_any_element(),
        };

        // Per-column max text length (in chars), used to proportion the columns
        // in the default (wrap) layout.
        let mut col_lens: Vec<usize> = vec![];
        for row in table.children.iter() {
            for (ix, cell) in row.children.iter().enumerate() {
                if col_lens.len() <= ix {
                    col_lens.push(DEFAULT_LENGTH);
                }
                col_lens[ix] = col_lens[ix].max(cell.children.text_len());
            }
        }

        // Scroll mode is opted in via `style.table` overflow-x: scroll.
        if matches!(node_cx.style.table().overflow.x, Some(Overflow::Scroll)) {
            Self::render_scroll_table(table, col_lens.len(), options, node_cx, window, cx)
        } else {
            Self::render_wrap_table(table, &col_lens, options, node_cx, window, cx)
        }
    }

    /// Horizontally scrollable table layout (opt-in via `style.table`
    /// overflow-x: scroll).
    ///
    /// Column widths come from the **measured** shaped text of each cell (the
    /// widest per column across all rows), so columns line up and fit their
    /// content exactly — char-count heuristics are inaccurate on proportional
    /// fonts. The layout adapts to the frame like CSS auto table layout:
    ///
    /// - Wider than the content: cells `flex_grow` proportionally to fill.
    /// - Narrower: columns shrink and their text wraps, but not below a
    ///   per-column floor.
    /// - Narrower than the floors: the table keeps the floor widths and
    ///   scrolls horizontally, so no content ever becomes unreachable.
    ///
    /// `white_space: nowrap` on `style.table_cell` composes like in CSS: the
    /// refinement keeps cell text on a single line, and the floors are raised
    /// to the full content widths so the single-line columns never shrink —
    /// the table scrolls as soon as the content is wider than the frame.
    fn render_scroll_table(
        table: &Table,
        col_count: usize,
        options: &NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        // Shrinking columns stop (and the table starts to scroll) at a floor
        // scaled to their content: roughly the width at which the text wraps
        // to `CELL_WRAP_MAX_LINES` lines, clamped between the two bounds so
        // moderate columns can still wrap meaningfully while one huge column
        // cannot push the scroll threshold arbitrarily high.
        const CELL_WRAP_MAX_LINES: f32 = 2.0;
        const CELL_WRAP_MIN_PX: f32 = 160.0;
        const CELL_WRAP_MAX_PX: f32 = 480.0;
        const TABLE_BORDER_PX: f32 = 2.0; // the track's border_1, left + right

        let col_w = measure_table_columns(table, col_count, node_cx, window, cx);
        let style = &node_cx.style;
        // Nowrap cells (via the `table_cell` refinement, which cascades to
        // the cell text) must never shrink below their single-line content,
        // so their floor is the content width itself.
        let nowrap = style.table_cell().text.white_space == Some(WhiteSpace::Nowrap);
        let col_min_w: Vec<f32> = if nowrap {
            col_w.clone()
        } else {
            col_w
                .iter()
                .map(|w| {
                    (w / CELL_WRAP_MAX_LINES)
                        .clamp(CELL_WRAP_MIN_PX, CELL_WRAP_MAX_PX)
                        .min(*w)
                })
                .collect()
        };
        let min_total_w: f32 = col_min_w.iter().sum::<f32>() + TABLE_BORDER_PX;

        let table_scroll_key = if let Some(span) = table.span {
            SharedString::from(format!(
                "{}-table-scroll-{}:{}",
                window.current_view(),
                span.start,
                span.end
            ))
        } else {
            SharedString::from(format!(
                "{}-table-scroll-{}",
                window.current_view(),
                options.ix
            ))
        };
        let scroll_handle = window
            .use_keyed_state(table_scroll_key, cx, |_, _| ScrollHandle::default())
            .read(cx)
            .clone();
        let row_count = table.children.len();
        let mut rows = Vec::with_capacity(row_count);
        for (row_ix, row) in table.children.iter().enumerate() {
            let mut cells = Vec::with_capacity(row.children.len());
            for (ix, cell) in row.children.iter().enumerate() {
                let align = table.column_align(ix);
                let is_last_col = ix == row.children.len() - 1;
                let width = col_w.get(ix).copied().unwrap_or(CELL_MIN_PX);
                let min_width = col_min_w.get(ix).copied().unwrap_or(CELL_MIN_PX);
                cells.push(
                    div()
                        .id(("cell", ix))
                        // Measured max-content width is the flex-basis;
                        // `flex_grow` (proportional to it) distributes extra
                        // space so a narrow table still fills the frame, while
                        // shrinking is clamped at `min_w` — the flex engine
                        // squeezes columns (their text wraps) down to the
                        // floors before the track starts to scroll.
                        .flex_basis(px(width))
                        .flex_grow(width)
                        .flex_shrink(1.)
                        .min_w(px(min_width))
                        .overflow_hidden()
                        .when(align == ColumnumnAlign::Center, |this| this.text_center())
                        .when(align == ColumnumnAlign::Right, |this| this.text_right())
                        .px_2()
                        .py_1()
                        .when(!is_last_col, |this| {
                            this.border_r_1().border_color(style.border())
                        })
                        .refine_style(&style.table_cell())
                        .child(cell.children.render(node_cx, window, cx)),
                );
            }
            rows.push(
                div()
                    .id("row")
                    .w_full()
                    .when(row_ix < row_count - 1, |this| this.border_b_1())
                    .border_color(style.border())
                    .flex()
                    .flex_row()
                    // The first row is the header, as everywhere else that
                    // reads a table (`table_data`, `to_markdown`). The
                    // refinement comes last so it can override the defaults.
                    .when(row_ix == 0, |this| {
                        this.bg(style.code_background())
                            .text_color(style.foreground())
                            .refine_style(&style.table_head())
                    })
                    .children(cells),
            );
        }

        div()
            .pb(rems(1.))
            .w_full()
            .child(
                // Scroll viewport owns the visible frame, including any
                // caller-provided radius. Keeping the border here makes the
                // rounded frame stable while the wider row track moves below it.
                //
                // `horizontal_scroll_area` clips with `overflow_hidden` and
                // delegates the wheel to a sibling `ScrollableMask`, so the
                // gesture is locked to its starting axis and a horizontal swipe
                // is consumed before an ancestor scroller (`gpui::list` under
                // `TextView::scrollable`) can take its vertical component.
                horizontal_scroll_area(
                    ("table", options.ix),
                    &scroll_handle,
                    &StyleRefinement::default()
                        .bg(cx.theme().tokens.colors.surface)
                        .border_1()
                        .border_color(style.border())
                        .refine_style(style.table()),
                    // Row track sized to `max(viewport, column floors)`:
                    // `min_w_full` fills the frame while the columns can still
                    // shrink-to-fit (their text wrapping), the definite
                    // `w(min_total_w)` keeps the floors once they are reached,
                    // letting the track exceed the viewport and scroll.
                    div().min_w_full().w(px(min_total_w)).children(rows),
                ),
            )
            // Custom actions row (e.g. copy / download) rendered below the
            // table. The hook's element spans full width; alignment is up to
            // the caller (e.g. `h_flex().justify_end()`). The gap keeps hover
            // backgrounds of the action buttons off the table border, and the
            // id scopes the caller's element ids per table, so plain ids like
            // `"copy"` don't collide across tables (same as code blocks).
            .children(node_cx.table_actions.clone().map(|f| {
                div().id(("table-actions", options.ix)).mt_1().child(f(
                    &table.table_data(),
                    window,
                    cx,
                ))
            }))
            .into_any_element()
    }

    /// Default table layout: a flex grid whose columns are proportioned by
    /// content length and shrink to fit the container width (cell text wraps).
    fn render_wrap_table(
        table: &Table,
        col_lens: &[usize],
        options: &NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        const MAX_LENGTH: usize = 150;

        let style = &node_cx.style;
        let row_count = table.children.len();
        let mut rows = Vec::with_capacity(row_count);
        for (row_ix, row) in table.children.iter().enumerate() {
            let mut cells = Vec::with_capacity(row.children.len());
            for (ix, cell) in row.children.iter().enumerate() {
                let align = table.column_align(ix);
                let is_last_col = ix == row.children.len() - 1;
                let len = col_lens
                    .get(ix)
                    .copied()
                    .unwrap_or(MAX_LENGTH)
                    .min(MAX_LENGTH);

                cells.push(
                    div()
                        .id(("cell", ix))
                        .overflow_hidden()
                        .when(align == ColumnumnAlign::Center, |this| this.text_center())
                        .when(align == ColumnumnAlign::Right, |this| this.text_right())
                        .min_w_16()
                        .w(Length::Definite(relative(len as f32)))
                        .px_2()
                        .py_1()
                        .when(!is_last_col, |this| {
                            this.border_r_1().border_color(style.border())
                        })
                        .refine_style(&style.table_cell())
                        .child(cell.children.render(node_cx, window, cx)),
                );
            }

            rows.push(
                div()
                    .id("row")
                    .w_full()
                    .when(row_ix < row_count - 1, |this| this.border_b_1())
                    .border_color(style.border())
                    .flex()
                    .flex_row()
                    // The first row is the header, as everywhere else that
                    // reads a table (`table_data`, `to_markdown`). The
                    // refinement comes last so it can override the defaults.
                    .when(row_ix == 0, |this| {
                        this.bg(style.code_background())
                            .text_color(style.foreground())
                            .refine_style(&style.table_head())
                    })
                    .children(cells),
            );
        }

        div()
            .pb(rems(1.))
            .w_full()
            .child(
                div()
                    .id(("table", options.ix))
                    .w_full()
                    .bg(cx.theme().tokens.colors.surface)
                    .border_1()
                    .border_color(style.border())
                    .overflow_hidden()
                    .children(rows)
                    .refine_style(&style.table()),
            )
            // Custom actions row (e.g. copy / download) rendered below the
            // table. The hook's element spans full width; alignment is up to
            // the caller (e.g. `h_flex().justify_end()`). The gap keeps hover
            // backgrounds of the action buttons off the table border, and the
            // id scopes the caller's element ids per table, so plain ids like
            // `"copy"` don't collide across tables (same as code blocks).
            .children(node_cx.table_actions.clone().map(|f| {
                div().id(("table-actions", options.ix)).mt_1().child(f(
                    &table.table_data(),
                    window,
                    cx,
                ))
            }))
            .into_any_element()
    }

    pub(crate) fn render_block(
        &self,
        options: NodeRenderOptions,
        node_cx: &NodeContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let ix = options.ix;
        let mb = if options.in_list || options.is_last {
            rems(0.)
        } else {
            node_cx.style.paragraph_gap()
        };

        match self {
            BlockNode::Root { children, .. } => div()
                .id(("div", ix))
                .children(children.into_iter().enumerate().map(move |(ix, node)| {
                    node.render_block(NodeRenderOptions { ix, ..options }, node_cx, window, cx)
                }))
                .into_any_element(),
            BlockNode::Paragraph(paragraph) => div()
                .id(("p", ix))
                .pb(mb)
                .child(paragraph.render(node_cx, window, cx))
                .into_any_element(),
            BlockNode::Heading {
                level, children, ..
            } => {
                let (text_size, font_weight) = match level {
                    1 => (rems(2.), FontWeight::BOLD),
                    2 => (rems(1.5), FontWeight::SEMIBOLD),
                    3 => (rems(1.25), FontWeight::SEMIBOLD),
                    4 => (rems(1.125), FontWeight::SEMIBOLD),
                    5 => (rems(1.), FontWeight::SEMIBOLD),
                    6 => (rems(1.), FontWeight::MEDIUM),
                    _ => (rems(1.), FontWeight::NORMAL),
                };

                let mut text_size = text_size.to_pixels(node_cx.style.heading_base_font_size());
                if let Some(size) = node_cx.style.heading_font_size(*level) {
                    text_size = size;
                }

                div()
                    .id(SharedString::from(format!("h{}-{}", level, ix)))
                    .pb(rems(0.3))
                    .whitespace_normal()
                    .text_size(text_size)
                    .font_weight(font_weight)
                    .child(children.render(node_cx, window, cx))
                    .into_any_element()
            }
            BlockNode::Blockquote { children, .. } => div()
                .w_full()
                .pb(mb)
                .child(
                    div()
                        .id(("blockquote", ix))
                        .w_full()
                        .text_color(node_cx.style.muted_foreground())
                        .border_l_3()
                        .border_color(node_cx.style.border())
                        .px_4()
                        .children({
                            let children_len = children.len();
                            children.into_iter().enumerate().map(move |(index, c)| {
                                let is_last = index == children_len - 1;
                                c.render_block(options.is_last(is_last), node_cx, window, cx)
                            })
                        }),
                )
                .into_any_element(),
            BlockNode::List {
                children, ordered, ..
            } => v_flex()
                .id((if *ordered { "ol" } else { "ul" }, ix))
                .w_full()
                .min_w_0()
                .pb(mb)
                .children({
                    let mut items = Vec::with_capacity(children.len());
                    let mut item_index = 0;
                    for (ix, item) in children.into_iter().enumerate() {
                        let is_item = item.is_list_item();

                        items.push(Self::render_list_item(
                            item,
                            item_index,
                            NodeRenderOptions {
                                ix,
                                ordered: *ordered,
                                ..options
                            },
                            node_cx,
                            window,
                            cx,
                        ));

                        if is_item {
                            item_index += 1;
                        }
                    }
                    items
                })
                .into_any_element(),
            BlockNode::CodeBlock(code_block) => code_block.render(&options, node_cx, window, cx),
            BlockNode::Custom(node) => {
                let inner = match node_cx.markdown_extensions.render_block(node, window, cx) {
                    Some(rendered) => rendered,
                    None => div().child(node.as_text().to_string()).into_any_element(),
                };

                div().pb(mb).child(inner).into_any_element()
            }
            BlockNode::Table { .. } => {
                Self::render_table(self, &options, node_cx, window, cx).into_any_element()
            }
            BlockNode::HorizontalRule { .. } => div()
                .pb(mb)
                .child(
                    div()
                        .id("horizontal-rule")
                        .bg(node_cx.style.border())
                        .h(px(2.)),
                )
                .into_any_element(),
            BlockNode::Break { .. } => div().id("break").into_any_element(),
            BlockNode::Unknown { .. } | BlockNode::Definition { .. } => div().into_any_element(),
            _ => {
                if cfg!(debug_assertions) {
                    tracing::warn!("unknown implementation: {:?}", self);
                }

                div().into_any_element()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Table columns are sized from shaped text, so a column of inline code
    /// has to be measured in the code family. Measured in the body font, the
    /// wide-mono test font makes `col_w` come out at half the rendered width.
    #[test]
    fn table_column_of_inline_code_cells_fits_the_mono_width() {
        use crate::text::inline::test_fonts::{MONO, WideMonoTextSystem};
        use gpui::{Empty, TestApp};

        let code = "method_name()";
        let mut paragraph = Paragraph::default();
        paragraph
            .push(InlineNode::new(code).marks(vec![(0..code.len(), TextMark::default().code())]));
        let table = Table {
            children: vec![TableRow {
                children: vec![TableCell {
                    children: paragraph,
                    width: None,
                }],
            }],
            column_aligns: vec![],
            span: None,
        };
        let node_cx = NodeContext::default();

        let mut app = TestApp::with_text_system(Arc::new(WideMonoTextSystem));
        let mut window = app.open_window(|_, _| Empty);
        let (col_w, font_size) = window.update(|_, window, cx| {
            let mut theme = crate::Theme::default();
            theme.tokens.typography.mono = MONO.into();
            cx.set_global(theme);
            let font_size = window.text_style().font_size.to_pixels(window.rem_size());
            (
                measure_table_columns(&table, 1, &node_cx, window, cx),
                font_size,
            )
        });

        let mono_w = f32::from(WideMonoTextSystem::width_of(code, MONO, font_size * 0.875));
        assert!(
            (col_w[0]
                - (mono_w + CELL_PAD_PX + crate::text::inline_flow::INLINE_CODE_PADDING * 2.))
                .abs()
                < 0.01,
            "col_w {} must fit the mono width {} plus padding {}",
            col_w[0],
            mono_w,
            CELL_PAD_PX
        );
    }

    #[test]
    fn code_block_highlights_are_cached_by_highlighter_identity() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_highlighter = calls.clone();
        let highlighter: Arc<CodeBlockHighlighterFn> = Arc::new(move |_| {
            calls_for_highlighter.fetch_add(1, Ordering::Relaxed);
            Vec::new()
        });
        let block = CodeBlock::new("fn main() {}".into(), Some("rust".into()), None::<Span>);

        block.highlighted_styles(&highlighter);
        block.highlighted_styles(&highlighter);
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        let replacement: Arc<CodeBlockHighlighterFn> = Arc::new(|_| Vec::new());
        block.highlighted_styles(&replacement);
        assert!(Arc::ptr_eq(
            &block
                .highlight_cache
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .highlighter,
            &replacement
        ));
    }

    #[test]
    fn a_new_highlighter_replaces_styles_instead_of_reusing_the_cache() {
        // Swapping the highlighter is how a theme change reaches a code block:
        // the parsed document is untouched, so the styles must come from the
        // new highlighter rather than from the styles cached for the old one.
        let light: Arc<CodeBlockHighlighterFn> = Arc::new(|_| {
            vec![(
                0..2,
                HighlightStyle {
                    color: Some(gpui::rgb(0x0000ff).into()),
                    ..Default::default()
                },
            )]
        });
        let dark: Arc<CodeBlockHighlighterFn> = Arc::new(|_| {
            vec![(
                0..2,
                HighlightStyle {
                    color: Some(gpui::rgb(0xffff00).into()),
                    ..Default::default()
                },
            )]
        });
        let block = CodeBlock::from_code("42", Some("json"));

        let light_styles = block.highlighted_styles(&light);
        let dark_styles = block.highlighted_styles(&dark);

        assert_eq!(light_styles[0].1.color, Some(gpui::rgb(0x0000ff).into()));
        assert_eq!(dark_styles[0].1.color, Some(gpui::rgb(0xffff00).into()));
        assert_eq!(block.code(), "42", "the document must survive the swap");
    }

    #[test]
    fn reconstruct_markdown_wraps_marked_runs() {
        // "bold" fully covered by a bold mark.
        let marks = vec![(0..4, TextMark::default().bold())];
        assert_eq!(reconstruct_markdown("bold", &marks, 0..4), "**bold**");
        // Partial selection inside the bold run still wraps the slice.
        assert_eq!(reconstruct_markdown("bold", &marks, 1..3), "**ol**");
    }

    #[test]
    fn reconstruct_markdown_emits_unmarked_text_verbatim() {
        // "a b c": plain, code, plain across three runs concatenated.
        let text = "a b c";
        let marks = vec![(2..3, TextMark::default().code())];
        assert_eq!(reconstruct_markdown(text, &marks, 0..5), "a `b` c");
        // Selecting only the plain tail.
        assert_eq!(reconstruct_markdown(text, &marks, 3..5), " c");
    }

    #[test]
    fn reconstruct_markdown_handles_code_italic_strike_link() {
        assert_eq!(
            reconstruct_markdown("x", &[(0..1, TextMark::default().code())], 0..1),
            "`x`"
        );
        assert_eq!(
            reconstruct_markdown("x", &[(0..1, TextMark::default().italic())], 0..1),
            "*x*"
        );
        assert_eq!(
            reconstruct_markdown("x", &[(0..1, TextMark::default().strikethrough())], 0..1),
            "~~x~~"
        );
        let link = TextMark::default().link(LinkMark {
            url: "https://example.com".into(),
            ..Default::default()
        });
        assert_eq!(
            reconstruct_markdown("x", &[(0..1, link)], 0..1),
            "[x](https://example.com)"
        );
    }

    #[test]
    fn reconstruct_markdown_nested_bold_italic() {
        // A single run marked both bold and italic (as produced by `**_x_**`).
        let mark = TextMark::default().bold().italic();
        // Inner (italic) is applied first, then bold: `***x***`.
        assert_eq!(reconstruct_markdown("x", &[(0..1, mark)], 0..1), "***x***");
    }

    /// Build a paragraph whose combined `state.text` is the concatenation of
    /// its children (mirroring `Paragraph::render`), then set the paragraph
    /// selection so `selected_source` can be exercised without a real paint.
    fn paragraph_with_children(children: Vec<InlineNode>) -> Paragraph {
        let combined: String = children.iter().map(|c| c.text.to_string()).collect();
        let paragraph = Paragraph {
            span: None,
            children,
            link_refs: HashMap::new(),
            state: Arc::new(Mutex::new(InlineState::default())),
        };
        if let Ok(mut state) = paragraph.state.lock() {
            state.set_text(combined.into());
        }
        paragraph
    }

    fn set_paragraph_selection(paragraph: &Paragraph, range: Range<usize>) {
        if let Ok(mut state) = paragraph.state.lock() {
            state.selection = Some(range.into());
        }
    }

    #[test]
    fn paragraph_selected_source_maps_partial_selection_across_runs() {
        // "This has **bold** text." rendered as ["This has ", "bold", " text."].
        let children = vec![
            InlineNode::new("This has ").marks(vec![(0..9, TextMark::default())]),
            InlineNode::new("bold").marks(vec![(0..4, TextMark::default().bold())]),
            InlineNode::new(" text.").marks(vec![(0..6, TextMark::default())]),
        ];
        let paragraph = paragraph_with_children(children);

        // Select the whole paragraph: "This has bold text." -> source with **.
        set_paragraph_selection(&paragraph, 0..(9 + 4 + 6));
        assert_eq!(paragraph.selected_source(), "This has **bold** text.");

        // Select only across the boundary "has **bold** te".
        // Rendered offsets: "has " starts at 5, "bold" at 9..13, " te" 13..16.
        set_paragraph_selection(&paragraph, 5..16);
        assert_eq!(paragraph.selected_source(), "has **bold** te");

        // Select entirely inside the bold run -> still wrapped.
        set_paragraph_selection(&paragraph, 10..12);
        assert_eq!(paragraph.selected_source(), "**ol**");
    }

    #[test]
    fn paragraph_selected_source_matches_text_when_no_marks() {
        let children =
            vec![InlineNode::new("plain words").marks(vec![(0..11, TextMark::default())])];
        let paragraph = paragraph_with_children(children);
        set_paragraph_selection(&paragraph, 0..11);
        assert_eq!(paragraph.selected_source(), "plain words");
        assert_eq!(paragraph.selected_text(), "plain words");
    }

    fn selected_paragraph(text: &str) -> Paragraph {
        let len = text.len();
        let paragraph = paragraph_with_children(vec![
            InlineNode::new(text).marks(vec![(0..len, TextMark::default())]),
        ]);
        set_paragraph_selection(&paragraph, 0..len);
        paragraph
    }

    #[test]
    fn heading_selected_source_prefixes_hashes() {
        let heading = BlockNode::Heading {
            level: 2,
            children: selected_paragraph("Title"),
            span: None,
        };
        assert_eq!(heading.selected_text(SelectionFormat::Source), "## Title\n");
        // Rendered text keeps no marker.
        assert_eq!(heading.selected_text(SelectionFormat::Plain), "Title\n");
    }

    #[test]
    fn unordered_list_selected_source_prefixes_dash() {
        let list = BlockNode::List {
            ordered: false,
            span: None,
            children: vec![
                BlockNode::ListItem {
                    children: vec![BlockNode::Paragraph(selected_paragraph("one"))],
                    spread: false,
                    checked: None,
                    span: None,
                },
                BlockNode::ListItem {
                    children: vec![BlockNode::Paragraph(selected_paragraph("two"))],
                    spread: false,
                    checked: None,
                    span: None,
                },
            ],
        };
        assert_eq!(
            list.selected_text(SelectionFormat::Source),
            "- one\n- two\n"
        );
    }

    #[test]
    fn ordered_list_selected_source_prefixes_numbers() {
        let list = BlockNode::List {
            ordered: true,
            span: None,
            children: vec![
                BlockNode::ListItem {
                    children: vec![BlockNode::Paragraph(selected_paragraph("first"))],
                    spread: false,
                    checked: None,
                    span: None,
                },
                BlockNode::ListItem {
                    children: vec![BlockNode::Paragraph(selected_paragraph("second"))],
                    spread: false,
                    checked: None,
                    span: None,
                },
            ],
        };
        assert_eq!(
            list.selected_text(SelectionFormat::Source),
            "1. first\n2. second\n"
        );
    }

    #[test]
    fn nested_list_selected_source_indents_sublists() {
        // - one
        //   - nested
        // - two
        let nested = BlockNode::List {
            ordered: false,
            span: None,
            children: vec![BlockNode::ListItem {
                children: vec![BlockNode::Paragraph(selected_paragraph("nested"))],
                spread: false,
                checked: None,
                span: None,
            }],
        };
        let list = BlockNode::List {
            ordered: false,
            span: None,
            children: vec![
                BlockNode::ListItem {
                    children: vec![BlockNode::Paragraph(selected_paragraph("one")), nested],
                    spread: false,
                    checked: None,
                    span: None,
                },
                BlockNode::ListItem {
                    children: vec![BlockNode::Paragraph(selected_paragraph("two"))],
                    spread: false,
                    checked: None,
                    span: None,
                },
            ],
        };
        assert_eq!(
            list.selected_text(SelectionFormat::Source),
            "- one\n  - nested\n- two\n"
        );
    }

    #[test]
    fn task_list_selected_source_restores_checkboxes() {
        let list = BlockNode::List {
            ordered: false,
            span: None,
            children: vec![
                BlockNode::ListItem {
                    children: vec![BlockNode::Paragraph(selected_paragraph("done"))],
                    spread: false,
                    checked: Some(true),
                    span: None,
                },
                BlockNode::ListItem {
                    children: vec![BlockNode::Paragraph(selected_paragraph("todo"))],
                    spread: false,
                    checked: Some(false),
                    span: None,
                },
            ],
        };
        assert_eq!(
            list.selected_text(SelectionFormat::Source),
            "- [x] done\n- [ ] todo\n"
        );
    }

    #[test]
    fn blockquote_selected_source_prefixes_gt() {
        let quote = BlockNode::Blockquote {
            span: None,
            children: vec![BlockNode::Paragraph(selected_paragraph("quoted text"))],
        };
        assert_eq!(
            quote.selected_text(SelectionFormat::Source),
            "> quoted text\n"
        );
    }

    #[test]
    fn table_selected_source_pipes_cells_with_alignment_row() {
        let cell = |text: &str| TableCell {
            children: selected_paragraph(text),
            width: None,
        };
        let table = Table {
            children: vec![
                TableRow {
                    children: vec![cell("Name"), cell("Age")],
                },
                TableRow {
                    children: vec![cell("Alice"), cell("30")],
                },
            ],
            column_aligns: vec![ColumnumnAlign::Left, ColumnumnAlign::Right],
            span: None,
        };
        let block = BlockNode::Table(table);
        assert_eq!(
            block.selected_text(SelectionFormat::Source),
            "| Name | Age |\n| :-- | --: |\n| Alice | 30 |\n"
        );
    }

    /// A cell holding plain text, as `Table::to_markdown` and
    /// `Table::table_data` see it (neither needs a selection).
    fn plain_cell(text: &str) -> TableCell {
        TableCell {
            children: Paragraph::new(text.to_string()),
            width: None,
        }
    }

    fn table_of(rows: Vec<Vec<TableCell>>, column_aligns: Vec<ColumnumnAlign>) -> Table {
        Table {
            children: rows
                .into_iter()
                .map(|children| TableRow { children })
                .collect(),
            column_aligns,
            span: None,
        }
    }

    #[test]
    fn table_to_markdown_pipes_cells_with_alignment_row() {
        let table = table_of(
            vec![
                vec![plain_cell("Name"), plain_cell("Age"), plain_cell("Score")],
                vec![plain_cell("Alice"), plain_cell("30"), plain_cell("9.5")],
            ],
            vec![
                ColumnumnAlign::Left,
                ColumnumnAlign::Center,
                ColumnumnAlign::Right,
            ],
        );

        assert_eq!(
            table.to_markdown(),
            "| Name | Age | Score |\n| :-- | :-: | --: |\n| Alice | 30 | 9.5 |"
        );
        // The block arm delegates to it.
        assert_eq!(
            BlockNode::Table(table.clone()).to_markdown(),
            table.to_markdown()
        );
    }

    #[test]
    fn table_to_markdown_keeps_outer_pipes_for_a_single_column() {
        let table = table_of(
            vec![vec![plain_cell("Symbol")], vec![plain_cell("TSLA.US")]],
            vec![ColumnumnAlign::Left],
        );

        assert_eq!(table.to_markdown(), "| Symbol |\n| :-- |\n| TSLA.US |");
    }

    #[test]
    fn table_to_markdown_escapes_pipes_and_keeps_inline_marks() {
        let bold = TableCell {
            children: paragraph_with_children(vec![
                InlineNode::new("bold").marks(vec![(0..4, TextMark::default().bold())]),
            ]),
            width: None,
        };
        let table = table_of(
            vec![
                vec![plain_cell("a | b"), plain_cell("plain")],
                vec![plain_cell("c"), bold],
            ],
            vec![ColumnumnAlign::Left, ColumnumnAlign::Left],
        );

        assert_eq!(
            table.to_markdown(),
            "| a \\| b | plain |\n| :-- | :-- |\n| c | **bold** |"
        );
    }

    #[test]
    fn table_data_snapshots_plain_cells_and_markdown() {
        let mut table = table_of(
            vec![
                vec![plain_cell("  Name  "), plain_cell("Age")],
                vec![plain_cell("Alice"), plain_cell("30")],
            ],
            vec![ColumnumnAlign::Left, ColumnumnAlign::Right],
        );
        table.span = Some(Span { start: 4, end: 42 });

        let data = table.table_data();
        assert_eq!(data.headers, vec!["Name", "Age"]);
        assert_eq!(data.rows, vec![vec!["Alice", "30"]]);
        assert_eq!(data.markdown, table.to_markdown());
        assert_eq!(data.span, Some(4..42));
    }

    #[test]
    fn table_data_handles_tables_without_rows() {
        // Header only: still a valid table, with no data rows.
        let header_only = table_of(
            vec![vec![plain_cell("Name"), plain_cell("Age")]],
            vec![ColumnumnAlign::Left, ColumnumnAlign::Left],
        );
        let data = header_only.table_data();
        assert_eq!(data.headers, vec!["Name", "Age"]);
        assert!(data.rows.is_empty());
        assert_eq!(data.markdown, "| Name | Age |\n| :-- | :-- |");

        // No rows at all (a table still streaming in): an empty snapshot.
        assert_eq!(Table::default().table_data(), TableData::default());
    }

    fn image_paragraph(alt: &str, url: &str) -> Paragraph {
        let image = ImageNode {
            url: url.into(),
            alt: Some(alt.into()),
            ..Default::default()
        };
        Paragraph {
            span: None,
            children: vec![InlineNode::image(image)],
            link_refs: HashMap::new(),
            state: Arc::new(Mutex::new(InlineState::default())),
        }
    }

    /// Every mark round-trips, including the two Markdown has no plain syntax
    /// for.
    #[test]
    fn marks_round_trip_through_reconstruction() {
        let wrap = |mark: TextMark| reconstruct_markdown("x", &[(0..1, mark)], 0..1);

        assert_eq!(wrap(TextMark::default().bold()), "**x**");
        assert_eq!(wrap(TextMark::default().italic()), "*x*");
        assert_eq!(wrap(TextMark::default().code()), "`x`");
        assert_eq!(wrap(TextMark::default().strikethrough()), "~~x~~");
        assert_eq!(
            wrap(TextMark::default().highlight(gpui::rgb(0xfef08a).into())),
            "==x=="
        );
        // No Markdown syntax for underline, so it keeps the tag it came from.
        assert_eq!(wrap(TextMark::default().underline()), "<u>x</u>");

        // A link keeps its title, which Markdown carries after the URL.
        assert_eq!(
            wrap(TextMark::default().link(LinkMark {
                url: "https://example.com".into(),
                title: Some("Tip".into()),
                ..Default::default()
            })),
            "[x](https://example.com \"Tip\")"
        );
    }

    /// A block the selection covers whole comes straight from the source, so it
    /// keeps what the author wrote instead of a normalized reconstruction.
    #[test]
    fn document_selected_source_slices_covered_blocks_from_the_source() {
        use crate::text::document::ParsedDocument;

        // `_italic_`, the `3.` start and the column padding all survive only
        // because the block is copied, not rebuilt.
        let source = "start\n\n3. _one_\n4. two\n\n---\n\nend";
        let list = "3. _one_\n4. two";
        let list_start = source.find(list).unwrap();
        let rule_start = source.find("---").unwrap();

        let document = ParsedDocument {
            source: source.into(),
            blocks: vec![
                BlockNode::Paragraph(selected_paragraph("start")),
                BlockNode::List {
                    ordered: true,
                    children: vec![],
                    span: Some(Span {
                        start: list_start,
                        end: list_start + list.len(),
                    }),
                },
                BlockNode::HorizontalRule {
                    span: Some(Span {
                        start: rule_start,
                        end: rule_start + 3,
                    }),
                },
                BlockNode::Paragraph(selected_paragraph("end")),
            ]
            .into(),
        };

        assert_eq!(
            document.selected_text(SelectionFormat::Source, None),
            "start\n\n3. _one_\n4. two\n\n---\n\nend"
        );
    }

    #[test]
    fn document_selected_source_includes_enclosed_image() {
        use crate::text::document::ParsedDocument;

        // A standalone image between two selected paragraphs is covered by the
        // selection, so it is copied whole even though it holds no selection of
        // its own — straight out of the source the parser located it in.
        let source = "before\n\n![alt](https://example.com/i.png)\n\nafter";
        let image_markdown = "![alt](https://example.com/i.png)";
        let start = source.find(image_markdown).unwrap();
        let mut image = image_paragraph("alt", "https://example.com/i.png");
        image.span = Some(Span {
            start,
            end: start + image_markdown.len(),
        });

        let document = ParsedDocument {
            source: source.into(),
            blocks: vec![
                BlockNode::Paragraph(selected_paragraph("before")),
                BlockNode::Paragraph(image),
                BlockNode::Paragraph(selected_paragraph("after")),
            ]
            .into(),
        };
        assert_eq!(
            document.selected_text(SelectionFormat::Source, None),
            "before\n\n![alt](https://example.com/i.png)\n\nafter"
        );
    }

    #[test]
    fn document_selected_source_drops_unenclosed_image() {
        use crate::text::document::ParsedDocument;

        // An image after the only selected block, with nothing selected after
        // it, is not enclosed and is dropped.
        let document = ParsedDocument {
            source: String::new().into(),
            blocks: vec![
                BlockNode::Paragraph(selected_paragraph("before")),
                BlockNode::Paragraph(image_paragraph("alt", "u")),
            ]
            .into(),
        };
        assert_eq!(
            document.selected_text(SelectionFormat::Source, None),
            "before"
        );
    }

    fn selected_code_block(code: &str, lang: Option<&str>) -> BlockNode {
        let block = CodeBlock::new(
            code.to_string().into(),
            lang.map(|l| l.to_string().into()),
            None::<Span>,
        );
        if let Ok(mut state) = block.state.lock() {
            let len = state.text.len();
            state.selection = Some((0..len).into());
        }
        BlockNode::CodeBlock(block)
    }

    #[test]
    fn code_block_selected_source_wraps_in_fence_with_lang() {
        let block = selected_code_block("let x = 1;\n", Some("rust"));
        let code = block.selected_text(SelectionFormat::Plain);
        let code_trimmed = code.trim_end_matches('\n');
        // The source wraps the (trailing-newline-trimmed) selected code in a
        // fenced block carrying the language; the block arm adds one trailing
        // newline.
        assert_eq!(
            block.selected_text(SelectionFormat::Source),
            format!("```rust\n{}\n```\n", code_trimmed)
        );
        assert!(
            block
                .selected_text(SelectionFormat::Source)
                .starts_with("```rust\n")
        );
        assert!(
            block
                .selected_text(SelectionFormat::Source)
                .trim_end()
                .ends_with("\n```")
        );
    }

    #[test]
    fn code_block_selected_source_without_lang() {
        let block = selected_code_block("plain\n", None);
        let code_trimmed = block.selected_text(SelectionFormat::Plain);
        let code_trimmed = code_trimmed.trim_end_matches('\n');
        assert_eq!(
            block.selected_text(SelectionFormat::Source),
            format!("```\n{}\n```\n", code_trimmed)
        );
    }

    #[test]
    fn document_selected_source_joins_blocks_with_blank_line() {
        use crate::text::document::ParsedDocument;

        // A heading, a paragraph, and a two-item ordered list, each fully
        // selected. Top-level blocks must be separated by a blank line so the
        // copied Markdown re-renders with the same structure.
        let document = ParsedDocument {
            source: String::new().into(),
            blocks: vec![
                BlockNode::Heading {
                    level: 1,
                    children: selected_paragraph("Title"),
                    span: None,
                },
                BlockNode::Paragraph(selected_paragraph("A paragraph.")),
                selected_code_block("let x = 1;\n", Some("rust")),
                BlockNode::List {
                    ordered: true,
                    span: None,
                    children: vec![
                        BlockNode::ListItem {
                            children: vec![BlockNode::Paragraph(selected_paragraph("one"))],
                            spread: false,
                            checked: None,
                            span: None,
                        },
                        BlockNode::ListItem {
                            children: vec![BlockNode::Paragraph(selected_paragraph("two"))],
                            spread: false,
                            checked: None,
                            span: None,
                        },
                    ],
                },
            ]
            .into(),
        };

        assert_eq!(
            document.selected_text(SelectionFormat::Source, None),
            "# Title\n\nA paragraph.\n\n```rust\nlet x = 1;\n```\n\n1. one\n2. two"
        );
    }

    #[test]
    fn code_block_equality_includes_code_content() {
        let first = CodeBlock::new("let value = 1;".into(), Some("rust".into()), None::<Span>);
        let second = CodeBlock::new("let value = 2;".into(), Some("rust".into()), None::<Span>);

        assert_ne!(first, second);
    }
}
