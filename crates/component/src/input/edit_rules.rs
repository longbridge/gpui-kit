use gpui_base::input::{BracketPair, EditRules};

/// Default editing rules for a language name.
///
/// Unknown languages get [`EditRules::default`]. Plain text gets no pairs or
/// triggers: prose and single-line fields should not auto-close quotes.
/// Matching is case-insensitive on the name as passed to `.language()`.
pub fn language_rules(language: &str) -> EditRules {
    if is_plain_text(language) {
        return EditRules {
            pairs: Vec::new(),
            indent_triggers: Vec::new(),
            auto_close: true,
            smart_indent: true,
        };
    }
    EditRules::default()
}

/// Whether `language` names unstyled plain text.
fn is_plain_text(language: &str) -> bool {
    matches!(
        language.to_lowercase().as_str(),
        "text" | "plain" | "plaintext"
    )
}

/// Bracket pairs shared by C-like languages, for tables built elsewhere.
pub fn c_style_pairs() -> Vec<BracketPair> {
    vec![
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_languages_get_default_pairs() {
        for language in ["rust", "json", "html", "python", "rs", "yml"] {
            let rules = language_rules(language);
            assert_eq!(rules.pairs.len(), 5, "{language}");
            assert!(rules.auto_close, "{language}");
            assert!(rules.smart_indent, "{language}");
            assert!(rules.indent_triggers.contains(&'{'), "{language}");
        }
    }

    #[test]
    fn plain_text_gets_no_pairs_or_triggers() {
        for language in ["text", "plain", "plaintext", "TEXT"] {
            let rules = language_rules(language);
            assert!(rules.pairs.is_empty(), "{language}");
            assert!(rules.indent_triggers.is_empty(), "{language}");
        }
    }

    #[test]
    fn unknown_language_falls_back_to_default() {
        let rules = language_rules("not-a-language");
        assert_eq!(rules, EditRules::default());
    }
}
