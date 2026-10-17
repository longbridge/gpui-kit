//! Markdown input rules (D13). A rule fires on a trigger typed into a text
//! leaf and rewrites what was typed; each firing is its own undo step, so
//! one undo restores the literal text. The set is configurable with
//! [`BlockEditorState::with_input_rules`](super::BlockEditorState::with_input_rules).

/// One input rule. Construct the built-in rules with the associated
/// functions, or take them all with [`InputRule::markdown_defaults`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct InputRule(pub(crate) RuleKind);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum RuleKind {
    Heading,
    BulletList,
    OrderedList,
    TaskItem,
    Blockquote,
    CodeFence,
    HorizontalRule,
    InlineMarks,
    Link,
    Table,
}

impl InputRule {
    /// `# ` … `###### ` at the start of a paragraph: a heading.
    pub const fn heading() -> Self {
        Self(RuleKind::Heading)
    }
    /// `- `, `* `, `+ ` at the start of a paragraph: a bullet list.
    pub const fn bullet_list() -> Self {
        Self(RuleKind::BulletList)
    }
    /// `1. ` (any number) at the start of a paragraph: an ordered list.
    pub const fn ordered_list() -> Self {
        Self(RuleKind::OrderedList)
    }
    /// `[ ] ` / `[x] ` at the start of a paragraph or list item (so also
    /// `- [ ] `): a task item, unchecked / checked.
    pub const fn task_item() -> Self {
        Self(RuleKind::TaskItem)
    }
    /// `> ` at the start of a paragraph: a blockquote.
    pub const fn blockquote() -> Self {
        Self(RuleKind::Blockquote)
    }
    /// ```` ``` ```` / ```` ```lang ```` then Enter or space: a code block.
    pub const fn code_fence() -> Self {
        Self(RuleKind::CodeFence)
    }
    /// `---` then Enter in an otherwise empty paragraph: a horizontal rule.
    pub const fn horizontal_rule() -> Self {
        Self(RuleKind::HorizontalRule)
    }
    /// `` `x` ``, `**x**`, `*x*`, `~~x~~` on the closing delimiter.
    pub const fn inline_marks() -> Self {
        Self(RuleKind::InlineMarks)
    }
    /// `[text](url)` on the closing `)`: `text` with a link mark.
    pub const fn link() -> Self {
        Self(RuleKind::Link)
    }

    /// `| a | b |` then Enter at the end of a paragraph: a table whose
    /// header row holds those cells, plus one empty body row.
    pub const fn table() -> Self {
        Self(RuleKind::Table)
    }

    /// Every built-in rule.
    pub fn markdown_defaults() -> Vec<Self> {
        vec![
            Self::heading(),
            Self::bullet_list(),
            Self::ordered_list(),
            Self::task_item(),
            Self::blockquote(),
            Self::code_fence(),
            Self::horizontal_rule(),
            Self::inline_marks(),
            Self::link(),
            Self::table(),
        ]
    }
}
