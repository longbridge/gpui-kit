use gpui::{Hsla, SharedString};

use crate::text::node::{LinkMark, TextMark};

/// A kind of inline mark, keyed on exactly `TextMark`'s attributes (D12).
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MarkKind {
    Bold,
    Italic,
    Strikethrough,
    Underline,
    Code,
    Highlight,
    Link,
}

impl MarkKind {
    pub(crate) fn is_set(self, mark: &TextMark) -> bool {
        match self {
            Self::Bold => mark.bold,
            Self::Italic => mark.italic,
            Self::Strikethrough => mark.strikethrough,
            Self::Underline => mark.underline,
            Self::Code => mark.code,
            Self::Highlight => mark.highlight.is_some(),
            Self::Link => mark.link.is_some(),
        }
    }

    /// Whether typing at the trailing edge of this mark extends it (D6).
    pub(crate) fn extends_at_boundary(self) -> bool {
        !matches!(self, Self::Link | Self::Code)
    }
}

/// Read-only view of the marks at a position. `TextMark` stays internal.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct MarkSet(pub(crate) TextMark);

impl MarkSet {
    pub fn has(&self, kind: MarkKind) -> bool {
        kind.is_set(&self.0)
    }

    pub fn highlight(&self) -> Option<Hsla> {
        self.0.highlight
    }

    pub fn link_url(&self) -> Option<&SharedString> {
        self.0.link.as_ref().map(|link| &link.url)
    }

    pub fn is_empty(&self) -> bool {
        self.0 == TextMark::default()
    }
}

/// Whether a mark covers a whole range, none of it, or part of it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarkState {
    Off,
    On,
    Mixed,
}

/// A change to apply over a range. Builder with private fields.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct MarkChange {
    set: Vec<MarkKind>,
    clear: Vec<MarkKind>,
    link: Option<LinkMark>,
    highlight: Option<Hsla>,
}

/// The highlight colour `<mark>` gets without a colour attribute.
pub(crate) fn default_highlight() -> Hsla {
    gpui::rgb(0xfef08a).into()
}

impl MarkChange {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(mut self, kind: MarkKind) -> Self {
        self.clear.retain(|k| *k != kind);
        self.set.push(kind);
        self
    }

    pub fn clear(mut self, kind: MarkKind) -> Self {
        self.set.retain(|k| *k != kind);
        self.clear.push(kind);
        self
    }

    pub fn with_link(mut self, url: impl Into<SharedString>, title: Option<SharedString>) -> Self {
        self.link = Some(LinkMark {
            url: url.into(),
            identifier: None,
            title,
        });
        self.set(MarkKind::Link)
    }

    pub fn with_highlight(mut self, color: Hsla) -> Self {
        self.highlight = Some(color);
        self.set(MarkKind::Highlight)
    }

    pub(crate) fn apply(&self, mark: &TextMark) -> TextMark {
        let mut out = self.apply_raw(mark);
        // Inline code holds literal text: only a link may wrap it, so other
        // marks never apply inside code, and making text code drops them.
        if out.code {
            out.bold = false;
            out.italic = false;
            out.strikethrough = false;
            out.underline = false;
            out.highlight = None;
        }
        out
    }

    fn apply_raw(&self, mark: &TextMark) -> TextMark {
        let mut out = mark.clone();
        for kind in &self.clear {
            match kind {
                MarkKind::Bold => out.bold = false,
                MarkKind::Italic => out.italic = false,
                MarkKind::Strikethrough => out.strikethrough = false,
                MarkKind::Underline => out.underline = false,
                MarkKind::Code => out.code = false,
                MarkKind::Highlight => out.highlight = None,
                MarkKind::Link => out.link = None,
            }
        }
        for kind in &self.set {
            match kind {
                MarkKind::Bold => out.bold = true,
                MarkKind::Italic => out.italic = true,
                MarkKind::Strikethrough => out.strikethrough = true,
                MarkKind::Underline => out.underline = true,
                MarkKind::Code => out.code = true,
                MarkKind::Highlight => {
                    out.highlight = Some(self.highlight.unwrap_or_else(default_highlight))
                }
                MarkKind::Link => {
                    if let Some(link) = &self.link {
                        out.link = Some(link.clone());
                    }
                }
            }
        }
        out
    }
}
