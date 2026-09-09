use crate::input::InputModeKind;
use std::rc::Rc;
use std::{cell::RefCell, ops::Range};

use gpui::{Context, SharedString, Window};
use ropey::Rope;

use super::DisplayMap;
use crate::input::{
    DiagnosticSet, InputEdit, InputHighlighter, InputHighlighterFactory, LanguageConfig,
    LanguageConfigs, RopeExt as _, TabSize,
};

/// What changed, handed to the syntax highlighter.
pub(crate) struct HighlighterUpdate<'a> {
    pub(super) selected_range: &'a Range<usize>,
    pub(super) old_text: &'a Rope,
    pub(super) new_text: &'a Rope,
    pub(super) change_text: &'a str,
    pub(super) force: bool,
}

/// How the input lays its text out: rows, growth, and the code-editor extras.
///
/// This does not say what *kind* of input this is — that is fixed at the type
/// level by [`crate::input::InputModeKind`], and asking it here is what used to
/// let the two disagree.
#[derive(Clone)]
pub(crate) enum LayoutMode {
    /// A plain text input mode.
    PlainText { tab: TabSize, rows: usize },
    /// An auto grow input mode.
    AutoGrow {
        rows: usize,
        min_rows: usize,
        max_rows: usize,
    },
    /// A code editor input mode.
    CodeEditor {
        tab: TabSize,
        rows: usize,
        /// Show line number
        line_number: bool,
        language: SharedString,
        indent_guides: bool,
        folding: bool,
        highlighter: Rc<RefCell<Option<Box<dyn InputHighlighter>>>>,
        highlighter_factory: Option<InputHighlighterFactory>,
        diagnostics: DiagnosticSet,
        /// Effective editing configuration for the current language.
        language_config: Box<LanguageConfig>,
        language_configs: LanguageConfigs,
        /// Automatic delimiter ranges, adjusted with document edits.
        auto_closed_pairs: super::undo_manager::AutoClosedPairs,
        auto_close: bool,
        smart_indent: bool,
    },
}

impl Default for LayoutMode {
    fn default() -> Self {
        LayoutMode::plain_text()
    }
}

#[allow(unused)]
impl LayoutMode {
    /// Create a plain input mode with default settings.
    pub(super) fn plain_text() -> Self {
        LayoutMode::PlainText {
            tab: TabSize::default(),
            rows: 1,
        }
    }

    /// Create a code editor input mode with default settings.
    ///
    /// Starts with no language; the state sets one through its own builder.
    pub(super) fn code_editor() -> Self {
        LayoutMode::CodeEditor {
            rows: 2,
            tab: TabSize::default(),
            language: SharedString::default(),
            highlighter: Rc::new(RefCell::new(None)),
            highlighter_factory: None,
            line_number: true,
            indent_guides: true,
            folding: true,
            diagnostics: DiagnosticSet::new(&Rope::new()),
            language_config: Box::default(),
            language_configs: LanguageConfigs::default(),
            auto_closed_pairs: Vec::new(),
            auto_close: true,
            smart_indent: true,
        }
    }

    /// Create an auto grow input mode with given min and max rows.
    pub(super) fn auto_grow(min_rows: usize, max_rows: usize) -> Self {
        LayoutMode::AutoGrow {
            rows: min_rows,
            min_rows,
            max_rows,
        }
    }

    /// Return true if this layout is a code editor with folding enabled.
    #[inline]
    pub(crate) fn is_folding(&self) -> bool {
        if cfg!(target_family = "wasm") {
            return false;
        }

        matches!(self, LayoutMode::CodeEditor { folding: true, .. })
    }

    pub(super) fn language_config(&self) -> Option<&LanguageConfig> {
        match self {
            Self::CodeEditor {
                language_config, ..
            } => Some(language_config),
            _ => None,
        }
    }

    pub(super) fn set_language_configs(&mut self, configurations: LanguageConfigs) {
        if let Self::CodeEditor {
            language_configs, ..
        } = self
        {
            *language_configs = configurations;
        }
        self.reload_language_config();
    }

    pub(super) fn reload_language_config(&mut self) {
        if let Self::CodeEditor {
            language,
            language_config,
            language_configs,
            ..
        } = self
        {
            **language_config = language_configs.get(language);
        }
    }

    pub(super) fn is_auto_close(&self) -> bool {
        matches!(
            self,
            Self::CodeEditor {
                auto_close: true,
                ..
            }
        )
    }

    pub(super) fn is_smart_indent(&self) -> bool {
        matches!(
            self,
            Self::CodeEditor {
                smart_indent: true,
                ..
            }
        )
    }

    pub(super) fn set_auto_close(&mut self, enabled: bool) {
        if let Self::CodeEditor { auto_close, .. } = self {
            *auto_close = enabled;
        }
    }

    pub(super) fn set_smart_indent(&mut self, enabled: bool) {
        if let Self::CodeEditor { smart_indent, .. } = self {
            *smart_indent = enabled;
        }
    }

    #[cfg(test)]
    pub(super) fn set_language_config(&mut self, configuration: LanguageConfig) {
        if let Self::CodeEditor {
            language_config, ..
        } = self
        {
            **language_config = configuration;
        }
    }

    pub(super) fn auto_closed_pairs(&self) -> &[(Range<usize>, Range<usize>)] {
        match self {
            Self::CodeEditor {
                auto_closed_pairs, ..
            } => auto_closed_pairs,
            _ => &[],
        }
    }

    pub(super) fn restore_auto_closed_pairs(
        &mut self,
        pairs: super::undo_manager::AutoClosedPairs,
    ) {
        if let Self::CodeEditor {
            auto_closed_pairs, ..
        } = self
        {
            *auto_closed_pairs = pairs;
        }
    }

    pub(super) fn track_auto_closed_pair(&mut self, open: Range<usize>, close: Range<usize>) {
        if let Self::CodeEditor {
            auto_closed_pairs, ..
        } = self
        {
            auto_closed_pairs.push((open, close));
        }
    }

    pub(super) fn adjust_auto_closed_pair(&mut self, edit: &Range<usize>, new_len: usize) {
        let Self::CodeEditor {
            auto_closed_pairs, ..
        } = self
        else {
            return;
        };
        let delta = new_len as isize - edit.len() as isize;
        let shift = |range: &mut Range<usize>| {
            range.start = range.start.saturating_add_signed(delta);
            range.end = range.end.saturating_add_signed(delta);
        };
        auto_closed_pairs.retain_mut(|(open, close)| {
            if edit.end <= open.start {
                shift(open);
                shift(close);
            } else if edit.start >= open.end && edit.end <= close.start {
                shift(close);
            } else if edit.start < close.end {
                return false;
            }
            true
        });
    }

    #[inline]
    pub(super) fn is_auto_grow(&self) -> bool {
        matches!(self, LayoutMode::AutoGrow { .. })
    }

    pub(super) fn set_rows(&mut self, new_rows: usize) {
        match self {
            LayoutMode::PlainText { rows, .. } => {
                *rows = new_rows;
            }
            LayoutMode::CodeEditor { rows, .. } => {
                *rows = new_rows;
            }
            LayoutMode::AutoGrow {
                rows,
                min_rows,
                max_rows,
            } => {
                *rows = new_rows.clamp(*min_rows, *max_rows);
            }
        }
    }

    /// Grow the row count to fit the content.
    ///
    /// Callers gate this on the input being multi-line; a single-line field
    /// keeps its one row.
    pub(super) fn update_auto_grow(&mut self, display_map: &DisplayMap) {
        let wrapped_lines = display_map.wrap_row_count();
        self.set_rows(wrapped_lines);
    }

    /// At least 1 row be return.
    pub(super) fn rows(&self) -> usize {
        match self {
            LayoutMode::PlainText { rows, .. } => *rows,
            LayoutMode::CodeEditor { rows, .. } => *rows,
            LayoutMode::AutoGrow { rows, .. } => *rows,
        }
        .max(1)
    }

    /// At least 1 row be return.
    #[allow(unused)]
    pub(super) fn min_rows(&self) -> usize {
        match self {
            LayoutMode::AutoGrow { min_rows, .. } => *min_rows,
            _ => 1,
        }
        .max(1)
    }

    #[allow(unused)]
    pub(super) fn max_rows(&self) -> usize {
        match self {
            LayoutMode::AutoGrow { max_rows, .. } => *max_rows,
            _ => usize::MAX,
        }
    }

    /// Return false if the mode is not [`LayoutMode::CodeEditor`].
    #[inline]
    pub(super) fn line_number(&self) -> bool {
        match self {
            LayoutMode::CodeEditor { line_number, .. } => *line_number,
            _ => false,
        }
    }

    /// Update the syntax highlighter with new text.
    ///
    pub(crate) fn update_highlighter<M: InputModeKind>(
        &mut self,
        update: HighlighterUpdate<'_>,
        window: &mut Window,
        cx: &mut Context<crate::input::InputBaseState<M>>,
    ) {
        match &self {
            LayoutMode::CodeEditor {
                language,
                highlighter,
                highlighter_factory,
                folding,
                ..
            } => {
                if !update.force && highlighter.borrow().is_some() {
                    return;
                }

                let mut highlighter_ref = highlighter.borrow_mut();
                if highlighter_ref.is_none() {
                    let Some(factory) = highlighter_factory else {
                        return;
                    };
                    *highlighter_ref = factory(language);
                }

                if highlighter_ref.is_none() {
                    return;
                }
                drop(highlighter_ref);

                let edit = replacement_input_edit(
                    update.old_text,
                    update.new_text,
                    update.selected_range,
                    update.change_text,
                );
                M::drive_highlighter(highlighter, edit, update.new_text, *folding, window, cx);
            }
            _ => {}
        }
    }

    #[allow(unused)]
    pub(super) fn diagnostics(&self) -> Option<&DiagnosticSet> {
        match self {
            LayoutMode::CodeEditor { diagnostics, .. } => Some(diagnostics),
            _ => None,
        }
    }

    pub(super) fn diagnostics_mut(&mut self) -> Option<&mut DiagnosticSet> {
        match self {
            LayoutMode::CodeEditor { diagnostics, .. } => Some(diagnostics),
            _ => None,
        }
    }

    /// Get a reference to the highlighter (if available)
    pub(super) fn highlighter(&self) -> Option<&Rc<RefCell<Option<Box<dyn InputHighlighter>>>>> {
        match self {
            LayoutMode::CodeEditor { highlighter, .. } => Some(highlighter),
            _ => None,
        }
    }

    pub(super) fn set_highlighter_factory(&mut self, factory: InputHighlighterFactory) {
        if let LayoutMode::CodeEditor {
            highlighter_factory,
            highlighter,
            ..
        } = self
        {
            *highlighter_factory = Some(factory);
            *highlighter.borrow_mut() = None;
        }
    }

    pub(super) fn ensure_highlighter_factory(&mut self, factory: InputHighlighterFactory) {
        if let LayoutMode::CodeEditor {
            highlighter_factory,
            ..
        } = self
        {
            if highlighter_factory.is_none() {
                *highlighter_factory = Some(factory);
            }
        }
    }
}

/// Builds the tree-sitter edit for a text replacement.
///
/// Byte offsets and positions for `start`/`old_end` come from `old_text`;
/// `new_end` byte/position come from the post-edit `text`.
fn replacement_input_edit(
    old_text: &Rope,
    new_text: &Rope,
    selected_range: &Range<usize>,
    change_text: &str,
) -> InputEdit {
    let start_byte = selected_range.start.min(old_text.len());
    let old_end_byte = selected_range.end.min(old_text.len()).max(start_byte);
    let new_end_byte = (start_byte + change_text.len()).min(new_text.len());

    InputEdit {
        start_byte,
        old_end_byte,
        new_end_byte,
        start_position: old_text.offset_to_point(start_byte),
        old_end_position: old_text.offset_to_point(old_end_byte),
        new_end_position: new_text.offset_to_point(new_end_byte),
    }
}

#[cfg(test)]
mod tests {
    use ropey::Rope;

    use super::replacement_input_edit;
    use crate::input::{DiagnosticSet, LanguageConfigs, Point, TabSize, mode::LayoutMode};

    #[test]
    fn test_replacement_input_edit_backspace_at_end_uses_old_range() {
        let old_text = Rope::from_str("-=");
        let text = Rope::from_str("-");
        let edit = replacement_input_edit(&old_text, &text, &(1..2), "");

        assert_eq!(edit.start_byte, 1);
        assert_eq!(edit.old_end_byte, 2);
        assert_eq!(edit.new_end_byte, 1);
        assert_eq!(edit.start_position, Point::new(0, 1));
        assert_eq!(edit.old_end_position, Point::new(0, 2));
        assert_eq!(edit.new_end_position, Point::new(0, 1));
    }

    #[test]
    fn test_code_editor() {
        let mode = LayoutMode::code_editor();
        assert_eq!(mode.line_number(), true);
        assert_eq!(mode.has_indent_guides(), true);
        assert_eq!(mode.max_rows(), usize::MAX);
        assert_eq!(mode.min_rows(), 1);
        assert_eq!(mode.is_folding(), true);

        let mode = LayoutMode::CodeEditor {
            line_number: false,
            indent_guides: false,
            folding: false,
            rows: 0,
            tab: Default::default(),
            language: "rust".into(),
            highlighter: Default::default(),
            highlighter_factory: None,
            diagnostics: DiagnosticSet::new(&Rope::new()),
            language_config: Box::default(),
            language_configs: LanguageConfigs::default(),
            auto_closed_pairs: Vec::new(),
            auto_close: false,
            smart_indent: false,
        };
        assert_eq!(mode.line_number(), false);
        assert_eq!(mode.has_indent_guides(), false);
        assert_eq!(mode.min_rows(), 1);
        assert_eq!(mode.is_folding(), false);
        assert_eq!(mode.is_auto_close(), false);
        assert_eq!(mode.is_smart_indent(), false);
    }

    #[test]
    fn test_plain() {
        let mode = LayoutMode::PlainText {
            tab: TabSize::default(),
            rows: 5,
        };
        assert_eq!(mode.line_number(), false);
        assert_eq!(mode.rows(), 5);
        assert_eq!(mode.max_rows(), usize::MAX);
        assert_eq!(mode.min_rows(), 1);

        let mode = LayoutMode::plain_text();
        assert_eq!(mode.line_number(), false);
        assert_eq!(mode.rows(), 1);
        assert_eq!(mode.min_rows(), 1);
    }

    #[test]
    fn test_auto_grow() {
        let mut mode = LayoutMode::auto_grow(2, 5);
        assert_eq!(mode.line_number(), false);
        assert_eq!(mode.rows(), 2);
        assert_eq!(mode.max_rows(), 5);
        assert_eq!(mode.min_rows(), 2);

        mode.set_rows(4);
        assert_eq!(mode.rows(), 4);

        mode.set_rows(1);
        assert_eq!(mode.rows(), 2);

        mode.set_rows(10);
        assert_eq!(mode.rows(), 5);
    }
}
