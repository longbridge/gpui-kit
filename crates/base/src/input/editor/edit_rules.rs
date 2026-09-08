/// A bracket or quote pair for automatic closing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BracketPair {
    /// The typed character, e.g. `(`.
    pub open: char,
    /// The inserted or skipped character, e.g. `)`.
    pub close: char,
}

/// Editing rules for automatic brackets and indentation.
///
/// Language-independent configuration consumed by the shared editing engine.
/// The engine never names a bracket or language; `gpui-component` supplies
/// per-language tables through [`crate::input::EditorState`] builders.
#[derive(Debug, Clone, PartialEq)]
pub struct EditRules {
    /// Pairs inserted or skipped by auto-close, in priority order.
    pub pairs: Vec<BracketPair>,
    /// Line-end characters that add one indent level on Enter.
    pub indent_triggers: Vec<char>,
    /// Insert or skip matching closers when typing openers.
    pub auto_close: bool,
    /// Add one indent level after lines ending with an indent trigger.
    pub smart_indent: bool,
}

impl Default for EditRules {
    fn default() -> Self {
        Self {
            pairs: vec![
                BracketPair {
                    open: '(',
                    close: ')',
                },
                BracketPair {
                    open: '[',
                    close: ']',
                },
                BracketPair {
                    open: '{',
                    close: '}',
                },
                BracketPair {
                    open: '"',
                    close: '"',
                },
                BracketPair {
                    open: '\'',
                    close: '\'',
                },
            ],
            indent_triggers: vec!['{', '(', '[', ':'],
            auto_close: true,
            smart_indent: true,
        }
    }
}

impl EditRules {
    /// The closing counterpart for an opening bracket or quote, if any.
    pub fn matching_close(&self, opener: char) -> Option<char> {
        self.pairs
            .iter()
            .find(|pair| pair.open == opener)
            .map(|pair| pair.close)
    }

    /// Whether `typed` is a recognized closer.
    pub fn is_closer(&self, typed: char) -> bool {
        self.pairs.iter().any(|pair| pair.close == typed)
    }

    /// Whether `line` (already trimmed at the end) opens a new indent level.
    pub fn opens_indent(&self, trimmed_line_end: &str) -> bool {
        trimmed_line_end
            .chars()
            .next_back()
            .is_some_and(|c| self.indent_triggers.contains(&c))
    }
}
