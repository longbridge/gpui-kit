use gpui_base::input::{
    AutoClosingPair, BracketPair, IndentationRules, LanguageConfig, SyntaxContext,
};
use regex::Regex;
use std::sync::LazyLock;

pub(crate) fn init(cx: &mut gpui::App) {
    for language in [
        "text",
        "plain",
        "plaintext",
        "json",
        "jsonc",
        "python",
        "py",
        "pyi",
    ] {
        gpui_base::input::set_language_config(language, default_language_config(language), cx);
    }
}

/// Language editing defaults. Editor preferences are stored separately on EditorState.
/// Unknown languages use structural bracket indentation; only Python treats a colon
/// as a block opener. Plain text has no structural or automatic pairs.
fn default_language_config(language: &str) -> LanguageConfig {
    static PYTHON_INDENT: LazyLock<IndentationRules> = LazyLock::new(|| {
        IndentationRules::new(
            Regex::new(r"[\{\(\[:]\s*$").unwrap(),
            Regex::new(r"^\s*[\}\)\]]").unwrap(),
        )
    });
    match language.to_lowercase().as_str() {
        "text" | "plain" | "plaintext" => LanguageConfig::default()
            .brackets([])
            .auto_closing_pairs([]),
        "json" | "jsonc" => LanguageConfig::default()
            .brackets([BracketPair::new("{", "}"), BracketPair::new("[", "]")])
            .auto_closing_pairs(
                [
                    AutoClosingPair::new("{", "}"),
                    AutoClosingPair::new("[", "]"),
                    AutoClosingPair::new("\"", "\""),
                ]
                .into_iter()
                .map(|p| p.not_in([SyntaxContext::String, SyntaxContext::Comment])),
            ),
        "python" | "py" | "pyi" => {
            LanguageConfig::default().indentation_rules(PYTHON_INDENT.clone())
        }
        _ => LanguageConfig::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_has_no_editing_rules() {
        let rules = default_language_config("text");
        assert!(rules.brackets.is_empty());
        assert!(rules.auto_closing_pairs.unwrap().is_empty());
        assert!(rules.indentation_rules.is_none());
    }

    #[test]
    fn json_only_closes_json_delimiters() {
        let rules = default_language_config("json");
        let pairs = rules.auto_closing_pairs.unwrap();
        assert_eq!(pairs.len(), 3);
        assert!(
            pairs
                .iter()
                .all(|p| p.open.as_ref() != "'" && p.open.as_ref() != "(")
        );
    }

    #[test]
    fn colon_indentation_is_language_specific() {
        assert!(
            default_language_config("python")
                .indentation_rules
                .unwrap()
                .increase_indent_pattern
                .unwrap()
                .is_match("if enabled:")
        );
        for language in ["rust", "javascript", "json", "unknown"] {
            assert!(
                default_language_config(language)
                    .indentation_rules
                    .is_none()
            );
        }
    }
}
