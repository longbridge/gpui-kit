use std::rc::Rc;

#[cfg(feature = "tree-sitter")]
use gpui_base::input::Rope;
use gpui_base::input::SyntaxContextProvider;

/// A syntax-context provider for `language`, or `None` when no grammar is
/// compiled in (unknown language, plain text, or feature disabled).
#[cfg(feature = "tree-sitter")]
pub(crate) fn syntax_context_provider(language: &str) -> Option<Rc<dyn SyntaxContextProvider>> {
    use crate::highlighter::LanguageRegistry;

    let config = LanguageRegistry::singleton().language(language)?;
    let grammar = config.language?;
    TreeSitterSyntaxContext::new(grammar)
        .map(|provider| Rc::new(provider) as Rc<dyn SyntaxContextProvider>)
}

/// No tree-sitter support: no provider, engine falls back to heuristics.
#[cfg(not(feature = "tree-sitter"))]
pub(crate) fn syntax_context_provider(_language: &str) -> Option<Rc<dyn SyntaxContextProvider>> {
    None
}

/// Tree-sitter backed syntax context for editing decisions.
///
/// Parses incrementally (previous tree reused) and classifies an offset by
/// walking the named node and its ancestors for `string` / `comment` kinds.
/// Generic across grammars: no per-language queries needed for this coarse
/// classification.
#[cfg(feature = "tree-sitter")]
struct TreeSitterSyntaxContext {
    parser: std::cell::RefCell<tree_sitter::Parser>,
    tree: std::cell::RefCell<Option<tree_sitter::Tree>>,
}

#[cfg(feature = "tree-sitter")]
impl TreeSitterSyntaxContext {
    /// Synchronous parse budget per query; mirrors the highlighter's sync path.
    const PARSE_BUDGET: std::time::Duration = std::time::Duration::from_millis(5);

    fn new(language: tree_sitter::Language) -> Option<Self> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language).ok()?;
        Some(Self {
            parser: std::cell::RefCell::new(parser),
            tree: std::cell::RefCell::new(None),
        })
    }

    fn classify(kind: &str) -> Option<gpui_base::input::SyntaxContext> {
        let lower = kind.to_lowercase();
        if lower.contains("comment") {
            Some(gpui_base::input::SyntaxContext::Comment)
        } else if lower.contains("string") {
            Some(gpui_base::input::SyntaxContext::String)
        } else {
            None
        }
    }
}

#[cfg(feature = "tree-sitter")]
impl SyntaxContextProvider for TreeSitterSyntaxContext {
    fn context_at(&self, text: &Rope, offset: usize) -> gpui_base::input::SyntaxContext {
        use std::ops::ControlFlow;

        let source = text.to_string();
        let offset = offset.min(source.len());
        let start = std::time::Instant::now();
        let mut progress = |_: &tree_sitter::ParseState| -> ControlFlow<()> {
            if start.elapsed() > Self::PARSE_BUDGET {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        };
        let options = tree_sitter::ParseOptions::new().progress_callback(&mut progress);
        let mut parser = self.parser.borrow_mut();
        let new_tree = parser.parse_with_options(
            &mut |byte_offset, _| {
                if byte_offset >= source.len() {
                    ""
                } else {
                    &source[byte_offset..]
                }
            },
            self.tree.borrow().as_ref(),
            Some(options),
        );
        let Some(tree) = new_tree else {
            return gpui_base::input::SyntaxContext::Code;
        };
        let mut node = tree.root_node().descendant_for_byte_range(offset, offset);
        // At the very end the range may match nothing; try the last byte.
        if node.is_none() && offset > 0 {
            node = tree
                .root_node()
                .descendant_for_byte_range(offset - 1, offset - 1);
        }
        let mut current = node;
        while let Some(n) = current {
            if n.is_named() {
                if let Some(context) = Self::classify(n.kind()) {
                    *self.tree.borrow_mut() = Some(tree);
                    return context;
                }
            }
            current = n.parent();
        }
        *self.tree.borrow_mut() = Some(tree);
        gpui_base::input::SyntaxContext::Code
    }
}

#[cfg(all(test, feature = "tree-sitter"))]
mod tests {
    use super::*;

    fn json_provider() -> Rc<dyn SyntaxContextProvider> {
        syntax_context_provider("json").expect("json grammar must be compiled in")
    }

    #[test]
    fn json_string_content_is_string_context() {
        use gpui_base::input::{Rope, SyntaxContext};

        let provider = json_provider();
        // {"key": "value"} — offset inside "value".
        let text = Rope::from_str(r#"{"key": "value"}"#);
        assert_eq!(
            provider.context_at(&text, 10),
            SyntaxContext::String,
            "inside string literal"
        );
    }

    #[test]
    fn json_punctuation_is_code_context() {
        use gpui_base::input::{Rope, SyntaxContext};

        let provider = json_provider();
        let text = Rope::from_str(r#"{"key": "value"}"#);
        assert_eq!(
            provider.context_at(&text, 0),
            SyntaxContext::Code,
            "opening brace"
        );
        assert_eq!(
            provider.context_at(&text, 7),
            SyntaxContext::Code,
            "colon between pairs"
        );
    }

    #[test]
    fn unknown_language_has_no_provider() {
        assert!(syntax_context_provider("not-a-language").is_none());
    }
}
