use std::{ops::Range, rc::Rc, sync::Arc};

use gpui::{AbsoluteLength, AnyElement, Context, HighlightStyle, Hsla, SharedString, Window, px};
use ropey::Rope;

use super::{EditorState, FoldRange, GutterMarker, InputEdit};
use crate::SemanticThemeTokens;

/// Resolves semantic highlight names into renderable GPUI styles.
///
/// Base deliberately knows nothing about a concrete syntax theme. UI crates and
/// applications can provide any resolver, independently of their parser.
pub trait HighlightStyleResolver: Send + Sync {
    fn style(&self, name: &str) -> Option<HighlightStyle>;
}

#[derive(Default)]
struct NoHighlightStyles;

impl HighlightStyleResolver for NoHighlightStyles {
    fn style(&self, _: &str) -> Option<HighlightStyle> {
        None
    }
}

/// Parser-independent syntax highlighting seam consumed by the Base editor.
///
/// Implementations own parsing, incremental state, and language-specific
/// behavior. Base only asks for styled ranges and fold candidates.
pub trait InputHighlighter {
    fn language(&self) -> SharedString;

    fn update(
        &mut self,
        edit: Option<InputEdit>,
        text: &Rope,
        folding: bool,
        window: &mut Window,
        cx: &mut Context<EditorState>,
    );

    /// Apply several edits made as one change, such as typing with multiple
    /// cursors. Each entry is an edit with the text as it stood right after
    /// it, in the order the edits were applied.
    ///
    /// The default hands each edit to [`Self::update`] in turn. Override it to
    /// reparse once for the whole change.
    fn update_batch(
        &mut self,
        edits: &[(InputEdit, Rope)],
        folding: bool,
        window: &mut Window,
        cx: &mut Context<EditorState>,
    ) {
        for (edit, text) in edits {
            self.update(Some(*edit), text, folding, window, cx);
        }
    }

    /// Return ordered, non-overlapping style runs that fully cover `range`.
    /// Use [`HighlightStyle::default`] for text without a semantic style.
    fn styles(
        &self,
        range: &Range<usize>,
        resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)>;

    fn fold_ranges(&self, text: &Rope) -> Vec<FoldRange>;

    fn fold_ranges_for_edit(&self, range: Range<usize>, text: &Rope) -> Vec<FoldRange> {
        let _ = range;
        self.fold_ranges(text)
    }
}

pub type InputHighlighterFactory = Rc<dyn Fn(&str) -> Option<Box<dyn InputHighlighter>>>;
pub type SharedHighlightStyleResolver = Arc<dyn HighlightStyleResolver>;
pub type FoldIconRenderer = Rc<dyn Fn(usize, bool) -> AnyElement>;
pub type FoldPlaceholderRenderer = Rc<dyn Fn(usize) -> AnyElement>;
/// Renders a [`GutterMarker`] within the size supplied by [`InputEditorStyle`].
pub type GutterMarkerRenderer = Rc<dyn Fn(&GutterMarker) -> AnyElement>;

/// Where in the syntax tree an offset sits, for editing decisions.
///
/// Parser-independent: `gpui-component` answers from tree-sitter, apps may
/// answer heuristically. `None` (no provider installed) means `Code`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxContext {
    Code,
    String,
    Comment,
}

/// Answers syntax context for editing decisions (pairing, skip, indent).
///
/// Created per editor by the application's [`super::LanguageProvider`].
/// Base never imports a parser; implementations live in UI crates or apps.
pub trait SyntaxContextProvider {
    fn context_at(&self, text: &Rope, offset: usize) -> SyntaxContext;
}

#[derive(Clone, Copy, Default)]
pub struct DiagnosticColors {
    pub error: Hsla,
    pub warning: Hsla,
    pub info: Hsla,
    pub hint: Hsla,
}

/// Application-owned colors and highlight resolver consumed by editor painting.
///
/// Start from [`Default`]. Configure gutter markers through its builders;
/// existing fields retain their direct-access API.
#[derive(Clone)]
#[non_exhaustive]
pub struct InputEditorStyle {
    pub foreground: Hsla,
    pub muted_foreground: Hsla,
    pub background: Hsla,
    pub border: Hsla,
    pub selection: Hsla,
    pub caret: Hsla,
    pub diagnostics: DiagnosticColors,
    pub highlight_styles: SharedHighlightStyleResolver,
    pub editor_invisible: Option<Hsla>,
    pub editor_active_line: Option<Hsla>,
    pub editor_gutter_background: Option<Hsla>,
    pub fold_icon_renderer: Option<FoldIconRenderer>,
    /// Renders line decoration gutter markers; without one, markers are not painted.
    gutter_marker_renderer: Option<GutterMarkerRenderer>,
    gutter_marker_size: AbsoluteLength,
    gutter_marker_gap: AbsoluteLength,
}

impl InputEditorStyle {
    /// Set the gutter marker renderer, or `None` to stop rendering markers.
    pub fn with_gutter_marker_renderer(mut self, renderer: Option<GutterMarkerRenderer>) -> Self {
        self.gutter_marker_renderer = renderer;
        self
    }

    /// The gutter marker renderer, if one has been supplied.
    pub fn gutter_marker_renderer(&self) -> Option<&GutterMarkerRenderer> {
        self.gutter_marker_renderer.as_ref()
    }

    /// Set the square marker size. The presentation layer supplies its scale.
    ///
    /// Defaults to zero; configure this together with the renderer.
    pub fn with_gutter_marker_size(mut self, size: impl Into<AbsoluteLength>) -> Self {
        self.gutter_marker_size = size.into();
        self
    }

    /// The square marker size, resolved against the window's rem during layout.
    pub fn gutter_marker_size(&self) -> AbsoluteLength {
        self.gutter_marker_size
    }

    /// Set the gap between markers and line numbers. Defaults to zero.
    pub fn with_gutter_marker_gap(mut self, gap: impl Into<AbsoluteLength>) -> Self {
        self.gutter_marker_gap = gap.into();
        self
    }

    /// The gap between markers and line numbers.
    pub fn gutter_marker_gap(&self) -> AbsoluteLength {
        self.gutter_marker_gap
    }

    /// Fills in every colour that was left unset, from the active palette.
    ///
    /// `Hsla::default()` is fully transparent, and every colour on `Default` is
    /// that — so an input nothing projected onto painted its glyphs, its caret
    /// and its selection in nothing at all. Transparent is not a colour anyone
    /// means for ink, which is what makes it usable as "unset" here.
    ///
    /// This is resolution, not assignment: whatever a consumer did project is
    /// kept exactly. `crates/component` projects the whole style on every render and
    /// never reaches this; a consumer that projects once at construction gets
    /// the palette that is current now rather than the one that happened to be
    /// installed when the state was built.
    pub fn resolved(&self, tokens: &SemanticThemeTokens) -> Self {
        let colors = &tokens.colors;
        let unset = |value: Hsla| value.a == 0.;
        let or = |value: Hsla, fallback: Hsla| if unset(value) { fallback } else { value };

        let foreground = or(self.foreground, colors.foreground);
        let mut selection = self.selection;
        if unset(selection) {
            selection = colors.accent;
            // A selection must not hide the glyphs it selects.
            selection.a = 0.4;
        }

        Self {
            foreground,
            muted_foreground: or(self.muted_foreground, colors.muted_foreground),
            background: or(self.background, colors.surface),
            border: or(self.border, colors.border),
            selection,
            caret: or(self.caret, foreground),
            ..self.clone()
        }
    }
}

impl Default for InputEditorStyle {
    fn default() -> Self {
        Self {
            foreground: Hsla::default(),
            muted_foreground: Hsla::default(),
            background: Hsla::default(),
            border: Hsla::default(),
            selection: Hsla::default(),
            caret: Hsla::default(),
            diagnostics: DiagnosticColors::default(),
            highlight_styles: Arc::new(NoHighlightStyles),
            editor_invisible: None,
            editor_active_line: None,
            editor_gutter_background: None,
            fold_icon_renderer: None,
            gutter_marker_renderer: None,
            gutter_marker_size: px(0.).into(),
            gutter_marker_gap: px(0.).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{IntoElement as _, hsla, px, rems};

    use super::{GutterMarkerRenderer, InputEditorStyle};
    use crate::SemanticThemeTokens;
    use std::rc::Rc;

    fn dark() -> SemanticThemeTokens {
        let mut tokens = SemanticThemeTokens::default();
        tokens.colors.foreground = hsla(0., 0., 0.98, 1.0);
        tokens.colors.muted_foreground = hsla(0., 0., 0.64, 1.0);
        tokens.colors.surface = hsla(0., 0., 0.04, 1.0);
        tokens.colors.border = hsla(0., 0., 0.15, 1.0);
        tokens.colors.accent = hsla(0.6, 0.5, 0.5, 1.0);
        tokens
    }

    #[test]
    fn test_input_editor_style_builder() {
        let renderer: GutterMarkerRenderer = Rc::new(|_| gpui::Empty.into_any_element());
        let style = InputEditorStyle::default()
            .with_gutter_marker_renderer(Some(renderer.clone()))
            .with_gutter_marker_size(rems(0.75))
            .with_gutter_marker_gap(rems(0.25));
        // Palette resolution and cloning must preserve the presentation seam.
        let resolved = style.clone().resolved(&dark());
        assert!(Rc::ptr_eq(
            resolved.gutter_marker_renderer().unwrap(),
            &renderer
        ));
        assert_eq!(resolved.gutter_marker_size().to_pixels(px(32.)), px(24.));
        assert_eq!(resolved.gutter_marker_gap().to_pixels(px(32.)), px(8.));
        assert!(
            resolved
                .with_gutter_marker_renderer(None)
                .gutter_marker_renderer()
                .is_none()
        );
    }

    #[test]
    fn an_unprojected_style_takes_its_ink_from_the_palette() {
        let tokens = dark();
        let resolved = InputEditorStyle::default().resolved(&tokens);

        assert_eq!(resolved.foreground, tokens.colors.foreground);
        assert_eq!(resolved.caret, tokens.colors.foreground);
        assert_eq!(resolved.muted_foreground, tokens.colors.muted_foreground);
        assert_eq!(resolved.background, tokens.colors.surface);
        assert_eq!(resolved.border, tokens.colors.border);
        // The point of the change: every one of these was transparent, so an
        // input nothing projected onto painted its text in nothing at all.
        for colour in [
            resolved.foreground,
            resolved.caret,
            resolved.muted_foreground,
            resolved.selection,
        ] {
            assert!(colour.a > 0., "{colour:?} is still invisible");
        }
    }

    #[test]
    fn a_selection_stays_translucent_enough_to_read_through() {
        let resolved = InputEditorStyle::default().resolved(&dark());
        assert_eq!(resolved.selection.a, 0.4);
    }

    #[test]
    fn projected_colours_are_kept_verbatim() {
        let chosen = hsla(0.3, 0.4, 0.5, 1.0);
        let style = InputEditorStyle {
            foreground: chosen,
            caret: chosen,
            ..Default::default()
        };
        let resolved = style.resolved(&dark());

        assert_eq!(resolved.foreground, chosen);
        assert_eq!(resolved.caret, chosen);
        // And what was not projected still comes from the palette.
        assert_eq!(resolved.border, dark().colors.border);
    }

    #[test]
    fn resolution_never_consumes_its_own_output() {
        // The projected style is kept verbatim precisely so that this holds:
        // resolving against a second palette must follow it, not stay on the
        // first. Resolving in place would have frozen after one pass.
        let projected = InputEditorStyle::default();
        let first = projected.resolved(&dark());

        let mut light = SemanticThemeTokens::default();
        light.colors.foreground = hsla(0., 0., 0.04, 1.0);
        let second = projected.resolved(&light);

        assert_ne!(first.foreground, second.foreground);
        assert_eq!(second.foreground, light.colors.foreground);
    }
}
