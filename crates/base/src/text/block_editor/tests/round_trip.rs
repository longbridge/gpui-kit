//! Mark-loss cases from main's `Paragraph::to_markdown`, through import and
//! the editor serializer.

use markdown::mdast::Node;

use super::*;
use crate::text::block_editor::{inline::InlineText, transaction::TransactionBuilder};
use crate::text::node::TextMark;
use crate::text::{MarkdownNode, markdown_ext::TestInlinePlugin};

fn round_trip(source: &str) -> String {
    model(source).source()
}

#[test]
fn bold_italic_round_trips() {
    assert_eq!(round_trip("a ***x*** b"), "a ***x*** b");
}

#[test]
fn nested_italic_in_bold_round_trips() {
    assert_eq!(round_trip("a **b*c*d** e"), "a **b*c*d** e");
}

#[test]
fn underline_round_trips() {
    assert_eq!(round_trip("a <u>x</u> b"), "a <u>x</u> b");
}

#[test]
fn highlight_round_trips_as_mark() {
    assert_eq!(round_trip("a <mark>x</mark> b"), "a <mark>x</mark> b");
}

#[test]
fn highlight_color_survives() {
    let source = r##"<mark color="#ff000059">x</mark>"##;
    assert_eq!(round_trip(source), source);
}

#[test]
fn whitespace_hoisted_out_of_emphasis() {
    // Build runs `a `{bold} + `b` through the model.
    let mut m = model("");
    m.run(EditorCommand::ToggleMark(MarkKind::Bold)).unwrap();
    m.insert_text("a ", EditOrigin::Typing).unwrap();
    m.run(EditorCommand::ToggleMark(MarkKind::Bold)).unwrap();
    m.insert_text("b", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "**a** b");
    assert_eq!(round_trip(&m.source()), "**a** b");
}

#[test]
fn literal_asterisk_escaped() {
    let mut m = model("");
    m.insert_text("a * b", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "a \\* b");
    let reparsed = model(&m.source());
    assert_eq!(
        reparsed.document().leaves().next().unwrap().text(),
        Some("a * b")
    );
}

#[test]
fn custom_node_round_trips() {
    let extensions = MarkdownExtensions::default().plugin(
        TestInlinePlugin::new("math").parse_with(|node, _| match node {
            Node::InlineMath(math) => Some(MarkdownNode::new("math", ()).text(math.value.clone())),
            _ => None,
        }),
    );
    let m = BlockEditorModel::from_markdown("$x$", extensions).unwrap();
    let leaf = m.document().leaves().next().unwrap().id();
    assert_eq!(m.doc().inline(leaf).unwrap().atoms().len(), 1);
    assert_eq!(m.source(), "$x$");
}

#[test]
fn list_continuation_indented() {
    assert_eq!(round_trip("- a\n\n  b"), "- a\n\n  b");
    assert_eq!(round_trip("10. a\n\n    b"), "10. a\n\n    b");
}

#[test]
fn table_cells_keep_marks() {
    let source = "| **a** | <u>b</u> |\n| :-- | --: |\n| *c* | d |";
    assert_eq!(round_trip(source), source);
}

#[test]
fn every_block_kind_round_trips() {
    for source in [
        "# H1\n\n###### H6",
        "> quote\n>\n> - item",
        "1. one\n2. two",
        "3. three\n4. four",
        "- [ ] todo\n- [x] done",
        "```rust\nfn main() {}\n```",
        "````\n```\n````",
        "---",
        "[id]: https://example.com \"Title\"",
        "a [link](https://example.com) b",
        "![alt](https://example.com/a.png)",
        "`code` and ~~strike~~",
    ] {
        assert_eq!(round_trip(source), source, "{source}");
    }
}

// Source export is a data-integrity boundary, not just a serializer fixpoint.
#[test]
fn list_items_ending_in_hyphens_keep_their_content() {
    for source in [
        "- a-\n- b",
        "7. a-\n8. b",
        "- [ ] a-\n- [x] b-",
        "- parent-\n  - child-",
        "> - quoted-",
    ] {
        assert_eq!(round_trip(source), source, "{source}");
    }
}

#[test]
fn code_literals_survive_import_export_not_just_a_fixpoint() {
    for (source, literal) in [
        ("`` `tick` ``", "`tick`"),
        ("`  padded  `", " padded "),
        ("`   `", "   "),
        ("```text\nline\n\n\n```", "line\n\n"),
    ] {
        let initial = model(source);
        let first = initial
            .document()
            .leaves()
            .next()
            .unwrap()
            .text()
            .unwrap()
            .to_owned();
        assert_eq!(first, literal, "import: {source}");
        let exported = initial.source();
        let reparsed = model(&exported);
        assert_eq!(
            reparsed.document().leaves().next().unwrap().text(),
            Some(literal),
            "export: {source} => {exported}"
        );
    }
}

#[test]
fn literal_and_hard_break_corpus_preserves_text() {
    for literal in [
        "literal * _ [ref] <tag> &amp; \\ ` $5 | 雪🦀",
        "- not a list\n# not a heading\n10. not ordered",
        "first\nsecond",
    ] {
        let mut initial = model("");
        let id = initial.document().leaves().next().unwrap().id();
        let mut tx = TransactionBuilder::new(&mut initial.doc);
        tx.replace_text(id, 0..0, InlineText::plain(literal, TextMark::default()))
            .unwrap();
        drop(tx);
        let exported = initial.source();
        let reparsed = model(&exported);
        assert_eq!(
            reparsed.document().leaves().next().unwrap().text(),
            Some(literal),
            "{literal} => {exported}"
        );
    }
}

#[test]
fn references_and_footnotes_retain_targets_and_source() {
    let source = "a [link][target]\n\n[target]: https://example.com/path \"Title\"";
    let exported = round_trip(source);
    assert!(exported.contains("https://example.com/path"));
    let reparsed = model(&exported);
    let leaf = reparsed.document().leaves().next().unwrap().id();
    assert!(
        reparsed
            .doc
            .inline(leaf)
            .unwrap()
            .pieces()
            .any(|(_, mark, _)| mark
                .link
                .as_ref()
                .is_some_and(|link| link.url == "https://example.com/path"
                    && link.title.as_deref() == Some("Title")))
    );
    for source in [
        "text [^note]\n\n[^note]: footnote *body*",
        "text [^note]\n\n[^note]: first\n\n    second paragraph",
    ] {
        let exported = round_trip(source);
        assert_eq!(
            exported, source,
            "footnote/reference source must not become styled prose"
        );
    }
}

#[test]
fn unresolved_references_remain_literal_text() {
    let literal = "unresolved [link][missing] and [^missing]";
    let exported = round_trip(literal);
    let reparsed = model(&exported);
    assert_eq!(
        reparsed.document().leaves().next().unwrap().text(),
        Some(literal)
    );
}

#[test]
fn terminal_composer_whitespace_is_editable_content() {
    for source in ["Review: ", "word  ", "word\t", "  "] {
        let mut initial = model(source);
        assert_eq!(initial.source(), source);
        initial.select_end().unwrap();
        let mut range = initial.capture_caret().unwrap();
        initial.replace_captured(&mut range, "next").unwrap();
        assert_eq!(
            initial.document().leaves().next().unwrap().text(),
            Some(format!("{source}next").as_str())
        );
    }
}

#[test]
fn unsupported_html_is_source_not_silently_discarded_content() {
    for source in [
        "<div data-custom=\"literal\">keep this</div>",
        "<script>never execute this</script>",
        "a <kbd>key</kbd> b",
    ] {
        let exported = round_trip(source);
        assert_eq!(exported, source);
        assert_eq!(round_trip(&exported), source);
    }
}

#[test]
fn terminal_hard_break_and_table_br_are_semantic_content() {
    let mut editor = model("last");
    editor.select_end().unwrap();
    let id = editor.document().leaves().next().unwrap().id();
    let mut tx = TransactionBuilder::new(&mut editor.doc);
    tx.replace_text(id, 4..4, InlineText::plain("\n", TextMark::default()))
        .unwrap();
    drop(tx);
    let exported = editor.source_snapshot();
    let reparsed = model(exported.text());
    assert_eq!(
        reparsed.document().leaves().next().unwrap().text(),
        Some("last\n")
    );
    for source in [
        "| first<br>second |\n| -- |\n| last<br> |",
        "**bold<br>**",
        "<br>",
    ] {
        let initial = model(source);
        let exported = initial.source();
        let reparsed = model(&exported);
        let expected = initial
            .document()
            .leaves()
            .map(|leaf| leaf.text().map(str::to_owned))
            .collect::<Vec<_>>();
        assert_eq!(
            reparsed
                .document()
                .leaves()
                .map(|leaf| leaf.text().map(str::to_owned))
                .collect::<Vec<_>>(),
            expected,
            "{source} => {exported}"
        );
        assert_eq!(reparsed.source(), exported);
    }
}

#[test]
fn nested_footnote_source_does_not_double_container_prefixes() {
    for source in [
        "> text [^n]\n>\n> [^n]: first\n>     second",
        "> text [^n]\n>\n> [^n]: first\n>\n>     second paragraph",
        "- text [^n]\n\n  [^n]: first\n      second",
        "> - text [^n]\n>\n>   [^n]: first\n>       second",
    ] {
        let exported = round_trip(source);
        assert_eq!(exported, source);
        assert_eq!(round_trip(&exported), exported);
    }
}

#[test]
fn code_fence_info_metadata_keeps_filename_hints_and_editable_body() {
    let source = "```rust file=src/main.rs\nfn main() {}\n```";
    let mut editor = model(source);
    assert_eq!(editor.source(), source);
    let leaf = editor.document().leaves().next().unwrap();
    assert_eq!(leaf.text(), Some("fn main() {}"));
    assert_eq!(leaf.kind(), BlockKind::CodeBlock);
    assert!(
        matches!(editor.doc().kind(leaf.id()), Some(crate::text::block_editor::kind::BlockKindData::CodeBlock { lang: Some(lang), meta: Some(meta) }) if lang == "rust" && meta == "file=src/main.rs")
    );
    editor.select_end().unwrap();
    let mut capture = editor.capture_caret().unwrap();
    editor.replace_captured(&mut capture, " // edited").unwrap();
    assert!(editor.source().starts_with("```rust file=src/main.rs\n"));
    assert!(editor.source().contains("fn main() {} // edited"));
}

#[test]
fn list_export_restore_typing_keeps_terminal_word_separators() {
    for source in ["- word ", "8. word\t", "- parent\n  - word ", "> - word "] {
        let exported = round_trip(source);
        assert_eq!(exported, source);
        let mut restored = model(&exported);
        restored.select_end().unwrap();
        let mut capture = restored.capture_caret().unwrap();
        restored.replace_captured(&mut capture, "next").unwrap();
        let last = restored.document().leaves().last().unwrap();
        assert!(matches!(last.text(), Some("word next" | "word\tnext")));
    }
}

#[test]
fn draft_export_restore_keeps_trailing_paragraph_separation() {
    let mut editor = model("first");
    editor.select_end().unwrap();
    editor.split_block().unwrap();
    let exported = editor.source_snapshot();
    for source in [exported.text().as_ref(), "first\n\n"] {
        let mut restored = model(source);
        restored.select_end().unwrap();
        let mut capture = restored.capture_caret().unwrap();
        restored.replace_captured(&mut capture, "next").unwrap();
        assert_eq!(restored.source(), "first\n\nnext");
    }
}

#[test]
fn destination_and_title_writers_preserve_link_reference_and_image_targets() {
    use crate::text::block_editor::inline::InlineAtom;
    fn targets(editor: &BlockEditorModel) -> Vec<(String, Option<String>)> {
        editor
            .document()
            .leaves()
            .flat_map(|leaf| {
                editor
                    .doc()
                    .inline(leaf.id())
                    .into_iter()
                    .flat_map(|text| text.pieces())
                    .filter_map(|(_, mark, atom)| {
                        if let Some(InlineAtom::Image(image)) = atom {
                            Some((
                                image.url.to_string(),
                                image.title.as_ref().map(ToString::to_string),
                            ))
                        } else {
                            mark.link.as_ref().map(|link| {
                                (
                                    link.url.to_string(),
                                    link.title.as_ref().map(ToString::to_string),
                                )
                            })
                        }
                    })
            })
            .collect()
    }
    for source in [
        r#"[x](<https://example.test/a)b> "say \"hi\"")"#,
        r#"[x][target]

[target]: <https://example.test/a)b> "say \"hi\"""#,
        r#"![x](<https://example.test/a)b> "say \"hi\"")"#,
    ] {
        let initial = model(source);
        let expected = vec![(
            "https://example.test/a)b".into(),
            Some(r#"say "hi""#.into()),
        )];
        assert_eq!(
            targets(&initial),
            expected,
            "corpus must import the intended target/title"
        );
        let exported = initial.source();
        assert_eq!(
            targets(&model(&exported)),
            expected,
            "{source} => {exported}"
        );
    }
}

#[test]
fn code_owned_line_endings_are_not_exterior_draft_trivia() {
    use crate::text::block_editor::kind::BlockKindData;
    let mut unfinished = model("```rs\ncode\n\n");
    unfinished.select_end().unwrap();
    assert_eq!(unfinished.document().leaves().count(), 1);
    let id = unfinished.caret().unwrap().block;
    assert!(matches!(
        unfinished.doc().kind(id),
        Some(BlockKindData::CodeBlock { .. })
    ));
    let mut capture = unfinished.capture_caret().unwrap();
    unfinished.replace_captured(&mut capture, "next").unwrap();
    assert_eq!(unfinished.doc().leaf_text(id), Some("code\nnext"));
    let mut closed = model("```rs\ncode\n```\n\n");
    closed.select_end().unwrap();
    let id = closed.caret().unwrap().block;
    assert!(matches!(
        closed.doc().kind(id),
        Some(BlockKindData::Paragraph)
    ));
    let mut capture = closed.capture_caret().unwrap();
    closed.replace_captured(&mut capture, "next").unwrap();
    assert!(closed.source().ends_with("```\n\nnext"));
}
