//! Seeded property tests (no `proptest` dev-dependency on main) and the
//! fixture fixpoint.

use super::*;

/// xorshift64*: small, deterministic, dependency-free.
pub(super) struct Rng(pub(super) u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
}

const WORDS: &[&str] = &["alpha", "beta", "gamma", "delta", "x", "y2", "zeta"];

fn inline(rng: &mut Rng) -> String {
    let count = 1 + rng.below(5);
    (0..count)
        .map(|_| {
            let word = WORDS[rng.below(WORDS.len())];
            match rng.below(8) {
                0 => format!("**{word}**"),
                1 => format!("*{word}*"),
                2 => format!("~~{word}~~"),
                3 => format!("<u>{word}</u>"),
                4 => format!("`{word}`"),
                5 => format!("***{word}***"),
                _ => word.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn list(rng: &mut Rng, depth: usize, indent: &str) -> String {
    let ordered = rng.chance(40);
    let items = 1 + rng.below(3);
    let mut out = Vec::new();
    for ix in 0..items {
        let marker = if ordered {
            format!("{}. ", ix + 1)
        } else {
            "- ".to_string()
        };
        let mut item = format!("{indent}{marker}{}", inline(rng));
        if depth < 3 && rng.chance(30) {
            let nested_indent = format!("{indent}{}", " ".repeat(marker.len()));
            item.push('\n');
            item.push_str(&list(rng, depth + 1, &nested_indent));
        }
        out.push(item);
    }
    out.join("\n")
}

pub(super) fn document(rng: &mut Rng) -> String {
    let count = 1 + rng.below(6);
    (0..count)
        .map(|_| match rng.below(6) {
            0 => format!("{} {}", "#".repeat(1 + rng.below(6)), inline(rng)),
            1 => list(rng, 1, ""),
            2 => format!("> {}", inline(rng)),
            3 => format!("```rs\n{}\n```", WORDS[rng.below(WORDS.len())]),
            _ => inline(rng),
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Structure and content up to ids. Subtrees with no content are skipped:
/// empty paragraphs (and lists of them) have no Markdown form.
fn signature(m: &BlockEditorModel) -> String {
    walk(m.document().root(), m, 0)
}

fn walk(b: BlockRef<'_>, m: &BlockEditorModel, depth: usize) -> String {
    {
        let mut line = format!("{}{:?}", " ".repeat(depth), b.kind());
        let mut content = false;
        if let Some(inline) = m.doc().inline(b.id()) {
            // Per character; whitespace ignores marks, because hoisting moves
            // edge whitespace out of a mark group, and Markdown drops leading
            // and trailing whitespace of a paragraph (canonicalization).
            let mut tokens = Vec::new();
            for (range, marks, _) in inline.pieces() {
                for ch in inline.text()[range].chars() {
                    tokens.push((!ch.is_whitespace()).then(|| {
                        format!(
                            " {ch:?}{}{}{}{}{}",
                            marks.bold as u8,
                            marks.italic as u8,
                            marks.strikethrough as u8,
                            marks.underline as u8,
                            marks.code as u8
                        )
                    }));
                }
            }
            let start = tokens
                .iter()
                .position(Option::is_some)
                .unwrap_or(tokens.len());
            let end = tokens
                .iter()
                .rposition(Option::is_some)
                .map_or(start, |ix| ix + 1);
            for token in &tokens[start..end] {
                line.push_str(token.as_deref().unwrap_or(" _"));
            }
            content = start < end || b.kind() == BlockKind::TableCell;
        } else if let Some(text) = b.text() {
            // Trailing empty lines of a code block have no Markdown form
            // (documented lossiness).
            let text = if b.kind() == BlockKind::CodeBlock {
                text.trim_end_matches('\n')
            } else {
                text
            };
            line.push_str(&format!(" {text:?}"));
            content = true;
        } else if b.children().next().is_none() && b.kind() != BlockKind::Root {
            content = true;
        }
        let children: String = b
            .children()
            .map(|child| walk(child, m, depth + 1))
            .collect();
        if !content && children.is_empty() && b.kind() != BlockKind::Root {
            return String::new();
        }
        format!("{line}\n{children}")
    }
}

/// Two same-type lists separated only by empty (or blank) paragraphs: Markdown has no
/// form for the gap, and main's parser merges the lists (documented).
fn has_list_gap(m: &BlockEditorModel) -> bool {
    fn gap(b: BlockRef<'_>, m: &BlockEditorModel) -> bool {
        let children: Vec<_> = b
            .children()
            .filter(|c| !walk(*c, m, 0).is_empty())
            .collect();
        let found = children.len() < b.children().count()
            && children.windows(2).any(|w| {
                matches!(
                    (w[0].kind(), w[1].kind()),
                    (BlockKind::List { ordered: a }, BlockKind::List { ordered: b }) if a == b
                )
            });
        found || b.children().any(|c| gap(c, m))
    }
    gap(m.document().root(), m)
}

fn random_caret(m: &mut BlockEditorModel, rng: &mut Rng) {
    let leaves: Vec<_> = m.document().leaves().collect();
    let leaf = leaves[rng.below(leaves.len())];
    let (id, len) = (leaf.id(), m.doc().leaf_len(leaf.id()).unwrap());
    let a = rng.below(len + 1);
    let b = if rng.chance(30) {
        rng.below(len + 1)
    } else {
        a
    };
    let _ = m.set_selection(BlockSelection::Text {
        anchor: BlockPosition::new(id, a),
        head: BlockPosition::new(id, b),
    });
}

/// One random edit. Errors (e.g. typing into a code block) are fine; panics
/// and broken invariants are not.
fn random_edit(m: &mut BlockEditorModel, rng: &mut Rng) {
    random_caret(m, rng);
    let _ = match rng.below(12) {
        0 | 1 => m.insert_text(WORDS[rng.below(WORDS.len())], EditOrigin::Typing),
        2 => m.insert_text(" ", EditOrigin::Typing),
        3 => m.delete(false),
        4 => m.delete(true),
        5 => m.split_block(),
        6 => m
            .run(EditorCommand::ToggleMark(
                [
                    MarkKind::Bold,
                    MarkKind::Italic,
                    MarkKind::Strikethrough,
                    MarkKind::Underline,
                ][rng.below(4)],
            ))
            .map(|_| ()),
        7 => m
            .run(EditorCommand::ToggleList {
                ordered: rng.chance(50),
            })
            .map(|_| ()),
        8 => m.run(EditorCommand::Indent).map(|_| ()),
        9 => m.run(EditorCommand::Outdent).map(|_| ()),
        10 => m
            .run(EditorCommand::MoveBlocks {
                down: rng.chance(50),
            })
            .map(|_| ()),
        _ => {
            // A cross-block selection, then delete it.
            let leaves: Vec<_> = m.document().leaves().map(|b| b.id()).collect();
            let a = leaves[rng.below(leaves.len())];
            let b = leaves[rng.below(leaves.len())];
            let _ = m.set_selection(BlockSelection::Text {
                anchor: BlockPosition::new(a, 0),
                head: BlockPosition::new(b, 0),
            });
            m.run(EditorCommand::DeleteSelection).map(|_| ())
        }
    };
}

#[test]
fn prop_normalized_after_every_step() {
    for seed in 1..=150u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let mut m = model(&document(&mut rng));
        for step in 0..60 {
            random_edit(&mut m, &mut rng);
            if let Err(err) = m.doc().check_invariants() {
                panic!("seed {seed} step {step}: {err}\n{}", m.source());
            }
        }
    }
}

#[test]
fn prop_round_trip() {
    for seed in 1..=200u64 {
        let mut rng = Rng(seed.wrapping_mul(0x2545_f491_4f6c_dd1d));
        let source = document(&mut rng);
        let first = model(&source);
        let s1 = first.source();
        let second = model(&s1);
        assert_eq!(
            signature(&first),
            signature(&second),
            "seed {seed}\nsource:\n{source}\nserialized:\n{s1}"
        );
        assert_eq!(s1, second.source(), "seed {seed}: not a fixpoint");
    }
}

#[test]
fn prop_round_trip_after_edits() {
    for seed in 1..=150u64 {
        let mut rng = Rng(seed.wrapping_mul(0x5851_f42d_4c95_7f2d));
        let mut m = model(&document(&mut rng));
        for _ in 0..30 {
            random_edit(&mut m, &mut rng);
        }
        let s1 = m.source();
        let reparsed = model(&s1);
        if !has_list_gap(&m) {
            assert_eq!(signature(&m), signature(&reparsed), "seed {seed}\n{s1}");
        }
        // Edits can leave list tightness (`spread`) that the parser reads
        // back differently, so allow one canonicalization pass, then require
        // a fixpoint.
        let s2 = reparsed.source();
        assert_eq!(model(&s2).source(), s2, "seed {seed}: not a fixpoint");
    }
}

#[test]
fn prop_undo_redo_identity() {
    for seed in 1..=150u64 {
        let mut rng = Rng(seed.wrapping_mul(0x1405_7b7e_f767_814f));
        let mut m = model(&document(&mut rng));
        let leaf_ids =
            |m: &BlockEditorModel| m.document().leaves().map(|b| b.id()).collect::<Vec<_>>();
        let mut states = vec![(m.source(), leaf_ids(&m))];
        for _ in 0..25 {
            random_edit(&mut m, &mut rng);
            // One undo entry per edit, so the states line up with history.
            m.break_coalescing();
            if m.can_undo() && states.last().map(|s| &s.0) != Some(&m.source()) {
                states.push((m.source(), leaf_ids(&m)));
            }
        }
        // Only edits that changed the source were pushed; replay the history
        // and compare whenever the source matches a recorded state.
        let mut undone = vec![(m.source(), leaf_ids(&m))];
        while m.undo() {
            undone.push((m.source(), leaf_ids(&m)));
            assert_eq!(m.doc().check_invariants(), Ok(()), "seed {seed}");
        }
        assert_eq!(undone.last(), states.first(), "seed {seed}: undo all");
        let mut redone = vec![(m.source(), leaf_ids(&m))];
        while m.redo() {
            redone.push((m.source(), leaf_ids(&m)));
        }
        undone.reverse();
        assert_eq!(redone, undone, "seed {seed}: redo all");
    }
}

#[test]
fn fixture_test_md_is_fixpoint() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/fixtures/test.md"
    );
    let source = std::fs::read_to_string(path).unwrap();
    let s1 = model(&source).source();
    let s2 = model(&s1).source();
    assert_eq!(s1, s2);
}
