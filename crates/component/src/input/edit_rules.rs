use gpui_base::input::{BracketPair, EditRules};

/// Default editing rules for a language name.
///
/// Accepts names case-insensitively (`"Rust"`, `"rs"` aliases are resolved by
/// the caller through `Language::from_name` where available; this table keys
/// on canonical lowercase names). Unknown languages get [`EditRules::default`].
/// Plain text gets no pairs or triggers: prose and single-line fields should
/// not auto-close quotes.
pub fn language_rules(language: &str) -> EditRules {
    match language.to_lowercase().as_str() {
        "text" | "plain" | "plaintext" => EditRules {
            pairs: Vec::new(),
            indent_triggers: Vec::new(),
            auto_close: true,
            smart_indent: true,
        },
        // JSON has no single-quoted strings and `:` never opens a block
        // (`"key": value` must not indent).
        "json" | "jsonc" => EditRules {
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
            ],
            indent_triggers: vec!['{', '(', '['],
            auto_close: true,
            smart_indent: true,
        },
        // Rust: `::` paths and labels mean `:` never opens a block.
        // Quotes pair (char literals); lifetimes fall back to the word guard.
        "rust" | "rs" => EditRules {
            pairs: c_style_pairs(),
            indent_triggers: vec!['{', '(', '['],
            auto_close: true,
            smart_indent: true,
        },
        // HTML: `:` never opens a block (e.g. inline `style="color: red"`).
        // `<`/`>` are intentionally not pairs: angle brackets in text
        // would misfire far more often than tags benefit.
        "html" | "htm" | "xml" | "svg" | "vue" | "svelte" | "astro" => EditRules {
            pairs: c_style_pairs(),
            indent_triggers: vec!['{', '(', '['],
            auto_close: true,
            smart_indent: true,
        },
        // Python: `:` opens blocks; standard pairs.
        "python" | "py" | "pyi" => EditRules::default(),
        _ => EditRules::default(),
    }
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
        for language in ["rust", "python", "rs", "javascript", "go"] {
            let rules = language_rules(language);
            assert!(!rules.pairs.is_empty(), "{language}");
            assert!(rules.auto_close, "{language}");
            assert!(rules.smart_indent, "{language}");
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
    fn json_has_no_single_quotes_or_colon_trigger() {
        let rules = language_rules("json");
        assert!(
            rules.pairs.iter().all(|p| p.open != '\''),
            "no single quotes"
        );
        assert!(
            !rules.indent_triggers.contains(&':'),
            "colon must not indent JSON"
        );
        assert!(rules.indent_triggers.contains(&'{'), "brace still indents");
    }

    #[test]
    fn rust_has_no_colon_trigger() {
        let rules = language_rules("rust");
        assert!(
            !rules.indent_triggers.contains(&':'),
            "colon must not indent Rust (`::` paths)"
        );
        assert!(rules.indent_triggers.contains(&'{'), "brace still indents");
    }

    #[test]
    fn python_keeps_colon_trigger() {
        let rules = language_rules("python");
        assert!(
            rules.indent_triggers.contains(&':'),
            "colon must indent Python blocks"
        );
    }

    #[test]
    fn unknown_language_falls_back_to_default() {
        let rules = language_rules("not-a-language");
        assert_eq!(rules, EditRules::default());
    }
}
