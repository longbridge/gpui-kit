use std::{
    cell::RefCell,
    ops::{Range, RangeInclusive},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use gpui::{HighlightStyle, SharedString, Task};
use gpui_base::input::{
    EditorState, FoldRange, HighlightStyleResolver, InputEdit as BaseInputEdit, InputHighlighter,
    InputHighlighterFactory, RopeExt as _,
};
use ropey::{LineType, Rope};
use tree_sitter::{InputEdit, ParseOptions, Point};

use super::{
    LanguageRegistry, SyntaxHighlighter,
    highlighter::{INJECTION_PARSE_TIMEOUT, parse_input_bytes},
};

pub(crate) fn input_highlighter_factory() -> InputHighlighterFactory {
    Rc::new(|language| {
        LanguageRegistry::singleton().has_parser(language).then(|| {
            Box::new(TreeSitterInputHighlighter::new(language)) as Box<dyn InputHighlighter>
        })
    })
}

const SYNC_PARSE_TIMEOUT: Duration = Duration::from_millis(2);
const SYNC_PARSE_MAX_BYTES: usize = 256 * 1024;
const PARSE_DEBOUNCE: Duration = Duration::from_millis(150);

struct TreeSitterInputHighlighter {
    inner: Rc<RefCell<SyntaxHighlighter>>,
    parse_task: Rc<RefCell<Option<Task<()>>>>,
}

impl TreeSitterInputHighlighter {
    fn new(language: &str) -> Self {
        Self {
            inner: Rc::new(RefCell::new(SyntaxHighlighter::new(language))),
            parse_task: Rc::new(RefCell::new(None)),
        }
    }

    /// Settle after the edits reached the tree: drop a pending background
    /// parse when the synchronous one `completed`, or schedule one for `text`.
    fn finish_update(
        &mut self,
        completed: bool,
        text: &Rope,
        folding: bool,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<EditorState>,
    ) {
        if completed {
            self.parse_task.borrow_mut().take();
            return;
        }

        let highlighter = self.inner.clone();
        let parse_task = self.parse_task.clone();
        let language = highlighter.borrow().language().clone();
        let old_tree = highlighter.borrow().tree().cloned();
        let injection_data = highlighter.borrow().injection_parse_data();
        let text = text.clone();
        let text_for_apply = text.clone();
        let cancel = Arc::new(AtomicBool::new(false));

        let task = cx.spawn_in(window, async move |entity, cx| {
            struct CancelOnDrop(Arc<AtomicBool>);
            impl Drop for CancelOnDrop {
                fn drop(&mut self) {
                    self.0.store(true, Ordering::Relaxed);
                }
            }
            let _cancel_guard = CancelOnDrop(cancel.clone());
            cx.background_executor().timer(PARSE_DEBOUNCE).await;

            let parse_cancel = cancel.clone();
            let result = cx
                .background_executor()
                .spawn(async move {
                    let (mut parser, grammar) =
                        LanguageRegistry::singleton().parser(&language).ok()?;
                    parser.set_language(&grammar).ok()?;
                    let mut progress = |_: &tree_sitter::ParseState| {
                        if parse_cancel.load(Ordering::Relaxed) {
                            std::ops::ControlFlow::Break(())
                        } else {
                            std::ops::ControlFlow::Continue(())
                        }
                    };
                    let options = ParseOptions::new().progress_callback(&mut progress);
                    let tree = parser.parse_with_options(
                        &mut |offset, _| parse_input_bytes(&text, offset),
                        old_tree.as_ref(),
                        Some(options),
                    )?;
                    if parse_cancel.load(Ordering::Relaxed) {
                        return None;
                    }
                    // `cancel` only reaches the host parse, so each injection layer
                    // keeps its fixed budget here, as before.
                    let injections = injection_data.map_or_else(Default::default, |data| {
                        SyntaxHighlighter::compute_injection_layers(
                            data,
                            &tree,
                            &text,
                            Some(INJECTION_PARSE_TIMEOUT),
                        )
                    });
                    let folds = if folding {
                        extract_fold_ranges(&tree, &text)
                    } else {
                        Vec::new()
                    };
                    Some((tree, injections, folds))
                })
                .await;

            if let Some((tree, injections, folds)) = result {
                highlighter
                    .borrow_mut()
                    .apply_background_tree(tree, &text_for_apply, injections);
                let _ = entity.update(cx, |state, cx| {
                    state.apply_highlighter_fold_candidates(folds, cx);
                });
            }
        });
        parse_task.borrow_mut().replace(task);
    }
}

impl SyntaxHighlighter {
    pub(crate) fn update_input(
        &mut self,
        edit: Option<BaseInputEdit>,
        text: &Rope,
        timeout: Option<Duration>,
    ) -> bool {
        self.update(edit.map(to_tree_sitter_edit), text, timeout)
    }
}

impl InputHighlighter for TreeSitterInputHighlighter {
    fn language(&self) -> SharedString {
        self.inner.borrow().language().clone()
    }

    fn update(
        &mut self,
        edit: Option<BaseInputEdit>,
        text: &Rope,
        folding: bool,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<EditorState>,
    ) {
        let edit = edit.map(to_tree_sitter_edit);
        let completed = {
            let mut highlighter = self.inner.borrow_mut();
            if text.len() > SYNC_PARSE_MAX_BYTES {
                highlighter.edit_tree(edit, text);
                false
            } else {
                highlighter.update(edit, text, Some(SYNC_PARSE_TIMEOUT))
            }
        };
        self.finish_update(completed, text, folding, window, cx);
    }

    fn update_batch(
        &mut self,
        edits: &[(BaseInputEdit, Rope)],
        folding: bool,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<EditorState>,
    ) {
        let Some((_, text)) = edits.last() else {
            return;
        };
        let edits: Vec<InputEdit> = edits
            .iter()
            .map(|(edit, _)| to_tree_sitter_edit(*edit))
            .collect();
        let completed = {
            let mut highlighter = self.inner.borrow_mut();
            if text.len() > SYNC_PARSE_MAX_BYTES {
                for edit in edits {
                    highlighter.edit_tree(Some(edit), text);
                }
                false
            } else {
                highlighter.update_edits(&edits, text, Some(SYNC_PARSE_TIMEOUT))
            }
        };
        self.finish_update(completed, text, folding, window, cx);
    }

    fn styles(
        &self,
        range: &Range<usize>,
        resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        self.inner.borrow().styles(range, resolver)
    }
    fn fold_ranges(&self, text: &Rope) -> Vec<FoldRange> {
        self.inner
            .borrow()
            .tree()
            .map(|tree| extract_fold_ranges(tree, text))
            .unwrap_or_default()
    }

    fn fold_ranges_for_edit(&self, range: Range<usize>, text: &Rope) -> Vec<FoldRange> {
        // `adjust_folds_for_edit` drops every candidate whose lines touch the
        // edit, so the re-extraction covers those lines, not just its bytes.
        let start_row = text.offset_to_point(range.start).row;
        let end_row = text.offset_to_point(range.end.min(text.len())).row;
        self.inner
            .borrow()
            .tree()
            .map(|tree| extract_fold_ranges_in_rows(tree, text, start_row..=end_row))
            .unwrap_or_default()
    }
}

fn to_tree_sitter_edit(edit: BaseInputEdit) -> InputEdit {
    InputEdit {
        start_byte: edit.start_byte,
        old_end_byte: edit.old_end_byte,
        new_end_byte: edit.new_end_byte,
        start_position: Point::new(edit.start_position.row, edit.start_position.column),
        old_end_position: Point::new(edit.old_end_position.row, edit.old_end_position.column),
        new_end_position: Point::new(edit.new_end_position.row, edit.new_end_position.column),
    }
}

fn extract_fold_ranges(tree: &tree_sitter::Tree, text: &Rope) -> Vec<FoldRange> {
    extract_fold_ranges_in_rows(tree, text, 0..=usize::MAX)
}

/// Fold ranges of the nodes whose rows intersect `rows`. A fold can end on the row after
/// its node (see [`fold_range_for_node`]), so that row counts as one of the node's.
fn extract_fold_ranges_in_rows(
    tree: &tree_sitter::Tree,
    text: &Rope,
    rows: RangeInclusive<usize>,
) -> Vec<FoldRange> {
    fn collect(
        node: tree_sitter::Node,
        text: &Rope,
        rows: &RangeInclusive<usize>,
        ranges: &mut Vec<FoldRange>,
    ) {
        let start = node.start_position().row;
        let end = node.end_position().row;
        // A node on one row has no child that spans two rows.
        if end == start || end + 1 < *rows.start() || start > *rows.end() {
            return;
        }
        if let Some(range) = fold_range_for_node(node, text) {
            ranges.push(range);
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            collect(child, text, rows, ranges);
        }
    }

    let root = tree.root_node();
    let mut ranges = Vec::new();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        collect(child, text, &rows, &mut ranges);
    }
    ranges.sort_by_key(|range| range.start_line);
    ranges.dedup_by_key(|range| range.start_line);
    ranges
}

/// The fold of a multi-row node. The node's last row goes with the fold (hidden), unless it
/// closes a bracket that the first row opens (`FROM (` … `) t;`: the closer stays visible, as
/// VS Code and Zed do) or more code follows the node on that row (`) t; SELECT 1`).
fn fold_range_for_node(node: tree_sitter::Node, text: &Rope) -> Option<FoldRange> {
    let start = node.start_position().row;
    let end = node.end_position().row;
    let start_line = text.get_line(start, LineType::LF)?.to_string();
    let end_line = text.get_line(end, LineType::LF)?.to_string();
    let line_start = text.line_to_byte_idx(end, LineType::LF);
    let rest = end_line.get(node.end_byte().checked_sub(line_start)?..)?;
    if closes_opened_bracket(&start_line, &end_line) || has_code_after(rest) {
        (end - start >= 2).then(|| FoldRange::new(start, end))
    } else {
        Some(FoldRange::new(start, end + 1))
    }
}

/// The first row ends with an opening bracket and the last row starts with its closer.
fn closes_opened_bracket(first_row: &str, last_row: &str) -> bool {
    let Some(open) = first_row.trim_end().chars().last() else {
        return false;
    };
    let close = match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        _ => return false,
    };
    last_row.trim_start().starts_with(close)
}

/// Text after a node on its last row is code, not just separators or a trailing comment.
fn has_code_after(rest: &str) -> bool {
    let rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == ';' || c == ',');
    !(rest.is_empty() || rest.starts_with("--") || rest.starts_with("//") || rest.starts_with('#'))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fold ranges of `source` parsed as `language`, as `(start_line, end_line)`.
    fn fold_spans(language: &str, source: &str) -> Vec<(usize, usize)> {
        let text = Rope::from(source);
        let mut highlighter = SyntaxHighlighter::new(language);
        highlighter.update(None, &text, None);
        let tree = highlighter
            .tree()
            .expect("the highlighter parsed the source");
        extract_fold_ranges(tree, &text)
            .iter()
            .map(|range| (range.start_line, range.end_line))
            .collect()
    }

    #[test]
    fn statement_without_brackets_hides_its_last_line() {
        // `WHERE id = 1;` is the last row of the statement and of the WHERE clause,
        // so both folds hide it.
        assert_eq!(
            fold_spans("sql", "SELECT id\nFROM users\nWHERE id = 1;\n"),
            [(0, 3), (1, 3)]
        );
    }

    #[test]
    fn statement_ending_on_the_last_text_line_still_folds() {
        assert_eq!(
            fold_spans("sql", "SELECT 1\nFROM t\nWHERE a = 1"),
            [(0, 3), (1, 3)]
        );
    }

    #[test]
    fn closing_bracket_line_stays_visible() {
        assert_eq!(
            fold_spans("json", "{\n  \"a\": 1,\n  \"b\": 2\n}"),
            [(0, 3)]
        );
        assert_eq!(
            fold_spans("rust", "fn main() {\n    let x = 1;\n}\n"),
            [(0, 2)]
        );
    }

    #[test]
    fn code_after_the_node_on_its_last_line_stays_visible() {
        // The statement ends with `;` on row 2, and `SELECT 3` follows it there.
        assert_eq!(
            fold_spans("sql", "SELECT 1\nFROM t\nWHERE a = 1; SELECT 3\n"),
            [(0, 2)]
        );
    }

    #[test]
    fn separators_and_comments_after_the_node_do_not_count_as_code() {
        assert_eq!(
            fold_spans("sql", "SELECT id\nFROM users\nWHERE id = 1; -- done\n"),
            [(0, 3), (1, 3)]
        );
    }

    #[test]
    fn two_line_node_folds_its_last_line() {
        assert_eq!(fold_spans("sql", "SELECT id\nFROM users;\n"), [(0, 2)]);
    }

    #[test]
    fn one_line_node_is_not_a_fold() {
        assert!(fold_spans("sql", "SELECT 1;\n").is_empty());
    }
}
