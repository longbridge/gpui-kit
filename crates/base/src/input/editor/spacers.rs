use std::{ops::Range, rc::Rc};

use gpui::{App, Context, Hsla};

use super::{EditorMode, InputBaseState};

/// Blank rows an editor shows above or below a buffer line.
///
/// A spacer row holds no text: it has no line number, the caret never stands
/// on it, and line decorations, selections and search matches pass it by.
/// Two editors shown side by side pad their shorter lines with spacers, so that
/// lines paired across them start level however each one wraps.
#[derive(Clone, Debug, PartialEq)]
pub struct Spacer {
    line: usize,
    rows: usize,
    above: bool,
    background: Option<Hsla>,
}

impl Spacer {
    /// Create `rows` blank rows below a zero-based buffer line, after its last
    /// soft-wrapped row.
    pub fn below(line: usize, rows: usize) -> Self {
        Self {
            line,
            rows,
            above: false,
            background: None,
        }
    }

    /// Create `rows` blank rows above a zero-based buffer line, before its first
    /// row.
    pub fn above(line: usize, rows: usize) -> Self {
        Self {
            above: true,
            ..Self::below(line, rows)
        }
    }

    /// The zero-based buffer line the rows stand beside.
    pub fn line(&self) -> usize {
        self.line
    }

    /// How many blank rows there are.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Whether the rows stand above the line rather than below it.
    pub fn is_above(&self) -> bool {
        self.above
    }

    /// The color the rows are filled with, if any.
    pub fn background(&self) -> Option<Hsla> {
        self.background
    }

    /// Fill the rows with `color`, gutter included.
    pub fn with_background(mut self, color: Hsla) -> Self {
        self.background = Some(color);
        self
    }
}

/// Supplies the spacers of an editor.
///
/// Unlike line decorations, spacers move every row below them, so the editor
/// asks for those of every line at once and keeps the answer: when the provider
/// is set, after an edit, after the text is wrapped again or a fold changes,
/// and when [`InputBaseState::refresh_spacers`] is called. A spacer on a line
/// hidden by a fold, or past the last line, is not shown.
pub trait SpacerProvider {
    /// Return the spacers of the buffer lines within `lines`.
    fn spacers(&self, lines: Range<usize>, cx: &App) -> Vec<Spacer>;
}

impl InputBaseState<EditorMode> {
    /// Set the provider of this editor's spacers, or `None` to show none.
    ///
    /// The provider is asked during the next layout, after the text has been
    /// wrapped to the editor's width, so it can read how many rows each line
    /// takes from [`InputBaseState::display_map`] — of this editor and of
    /// another one it pads against.
    pub fn set_spacer_provider(
        &mut self,
        provider: Option<Rc<dyn SpacerProvider>>,
        cx: &mut Context<Self>,
    ) {
        self.extras.spacer_provider = provider;
        self.spacers_asked = None;
        cx.notify();
    }

    /// Ask the spacer provider again during the next layout.
    ///
    /// The editor asks by itself after its own rows change; call this when
    /// something the answer depends on changes elsewhere, such as the rows of
    /// another editor this one is padded against.
    pub fn refresh_spacers(&mut self, cx: &mut Context<Self>) {
        self.spacers_asked = None;
        cx.notify();
    }
}
