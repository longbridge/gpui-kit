use std::sync::Arc;

use gpui::{HighlightStyle, Hsla, Pixels, Rems, StyleRefinement, px, rems};

use crate::ColorTokens;

/// TextViewStyle used to customize the style for [`super::TextView`].
///
/// The fields are private because this type crosses the `gpui-base` seam:
/// build one with the `with_*` methods and read it back through the accessors
/// of the same name, so a later field is an additive change rather than a
/// breaking one.
#[derive(Clone)]
pub struct TextViewStyle {
    foreground: Hsla,
    muted_foreground: Hsla,
    link: Hsla,
    selection: Hsla,
    code_background: Hsla,
    border: Hsla,
    paragraph_gap: Rems,
    heading_base_font_size: Pixels,
    heading_font_size: Option<Arc<dyn Fn(u8, Pixels) -> Pixels + Send + Sync + 'static>>,
    code_block: StyleRefinement,
    table: StyleRefinement,
    table_head: StyleRefinement,
    table_cell: StyleRefinement,
    inline_code: HighlightStyle,
    is_dark: bool,
}

impl PartialEq for TextViewStyle {
    fn eq(&self, other: &Self) -> bool {
        self.paragraph_gap == other.paragraph_gap
            && self.foreground == other.foreground
            && self.muted_foreground == other.muted_foreground
            && self.link == other.link
            && self.selection == other.selection
            && self.code_background == other.code_background
            && self.border == other.border
            && self.heading_base_font_size == other.heading_base_font_size
            && match (&self.heading_font_size, &other.heading_font_size) {
                (Some(left), Some(right)) => (1..=6).all(|level| {
                    left(level, self.heading_base_font_size)
                        == right(level, other.heading_base_font_size)
                }),
                (None, None) => true,
                _ => false,
            }
            && self.code_block == other.code_block
            && self.table == other.table
            && self.table_head == other.table_head
            && self.table_cell == other.table_cell
            && self.inline_code == other.inline_code
            && self.is_dark == other.is_dark
    }
}

impl Default for TextViewStyle {
    fn default() -> Self {
        Self::from_colors(&ColorTokens::light(), false)
    }
}

impl TextViewStyle {
    /// Derives rich-text colors from Base semantic theme tokens.
    pub fn from_theme(theme: &crate::Theme) -> Self {
        Self::from_colors(
            &theme.tokens.colors,
            theme.appearance == crate::ThemeAppearance::Dark,
        )
    }

    /// Derives rich-text colors from one palette.
    ///
    /// Rich text needs a handful of roles the palette does not name directly —
    /// a code background, a link color — so they are mapped here once instead
    /// of at every call site.
    fn from_colors(colors: &ColorTokens, is_dark: bool) -> Self {
        Self {
            foreground: colors.foreground,
            muted_foreground: colors.muted_foreground,
            link: colors.primary,
            selection: colors.selection,
            code_background: colors.accent,
            border: colors.border,
            paragraph_gap: rems(1.),
            heading_base_font_size: px(14.),
            heading_font_size: None,
            code_block: StyleRefinement::default(),
            table: StyleRefinement::default(),
            table_head: StyleRefinement::default(),
            table_cell: StyleRefinement::default(),
            inline_code: HighlightStyle {
                background_color: Some(colors.accent),
                ..Default::default()
            },
            is_dark,
        }
    }

    /// Sets the default body-text color.
    pub fn with_foreground(mut self, color: Hsla) -> Self {
        self.foreground = color;
        self
    }

    /// Sets the secondary text color.
    pub fn with_muted_foreground(mut self, color: Hsla) -> Self {
        self.muted_foreground = color;
        self
    }

    /// Sets the link text color.
    pub fn with_link(mut self, color: Hsla) -> Self {
        self.link = color;
        self
    }

    /// Sets the background painted behind selected text.
    ///
    /// Selection quads are painted under the glyphs, so this is normally a
    /// translucent wash rather than a solid fill.
    pub fn with_selection(mut self, color: Hsla) -> Self {
        self.selection = color;
        self
    }

    /// Sets the background of fenced code blocks and table header rows.
    pub fn with_code_background(mut self, color: Hsla) -> Self {
        self.code_background = color;
        self
    }

    /// Sets the color of borders and horizontal rules.
    pub fn with_border(mut self, color: Hsla) -> Self {
        self.border = color;
        self
    }

    /// Sets the gap between paragraphs. Defaults to 1 rem.
    pub fn with_paragraph_gap(mut self, gap: Rems) -> Self {
        self.paragraph_gap = gap;
        self
    }

    /// Sets the base font size headings are derived from. Defaults to 14px.
    pub fn with_heading_base_font_size(mut self, size: Pixels) -> Self {
        self.heading_base_font_size = size;
        self
    }

    /// Sets the function that resolves a heading's font size.
    ///
    /// The first parameter is the heading level (1-6), the second is
    /// [`Self::heading_base_font_size`].
    pub fn with_heading_font_size<F>(mut self, f: F) -> Self
    where
        F: Fn(u8, Pixels) -> Pixels + Send + Sync + 'static,
    {
        self.heading_font_size = Some(Arc::new(f));
        self
    }

    /// Sets the style refinement for code blocks.
    pub fn with_code_block(mut self, style: StyleRefinement) -> Self {
        self.code_block = style;
        self
    }

    /// Sets the highlight style for inline code spans.
    ///
    /// When `background_color` is `None`, the neutral code background is used,
    /// which keeps [`TextViewStyle::default`] usable without a theme.
    pub fn with_inline_code(mut self, style: HighlightStyle) -> Self {
        self.inline_code = style;
        self
    }

    /// Sets the style refinement for the table container (the bordered wrapper
    /// in wrap mode, the scroll viewport in horizontal-scroll mode).
    ///
    /// Set `overflow_x: scroll` on the refinement for adaptive table layout:
    /// columns fit their content when space allows, shrink (wrapping cell
    /// text) down to a per-column floor when the frame is narrower, and below
    /// that the table scrolls horizontally instead of squeezing further.
    pub fn with_table(mut self, style: StyleRefinement) -> Self {
        self.table = style;
        self
    }

    /// Sets the style refinement for the header row (the first row) of a
    /// table, applied on top of the header background and foreground.
    pub fn with_table_head(mut self, style: StyleRefinement) -> Self {
        self.table_head = style;
        self
    }

    /// Sets the style refinement for each table cell.
    ///
    /// With the scroll table layout, `white_space: nowrap` here keeps cells on
    /// a single line — columns then never shrink and the table scrolls as soon
    /// as the content is wider than the frame.
    pub fn with_table_cell(mut self, style: StyleRefinement) -> Self {
        self.table_cell = style;
        self
    }

    /// Sets whether content-specific assets should use their dark variant.
    pub fn with_dark(mut self, is_dark: bool) -> Self {
        self.is_dark = is_dark;
        self
    }

    /// The default body-text color.
    pub fn foreground(&self) -> Hsla {
        self.foreground
    }

    /// The secondary text color.
    pub fn muted_foreground(&self) -> Hsla {
        self.muted_foreground
    }

    /// The link text color.
    pub fn link(&self) -> Hsla {
        self.link
    }

    /// The background painted behind selected text.
    pub fn selection(&self) -> Hsla {
        self.selection
    }

    /// The background of fenced code blocks and table header rows.
    pub fn code_background(&self) -> Hsla {
        self.code_background
    }

    /// The color of borders and horizontal rules.
    pub fn border(&self) -> Hsla {
        self.border
    }

    /// The gap between paragraphs.
    pub fn paragraph_gap(&self) -> Rems {
        self.paragraph_gap
    }

    /// The base font size headings are derived from.
    pub fn heading_base_font_size(&self) -> Pixels {
        self.heading_base_font_size
    }

    /// The size this style gives a heading of `level`, when it resolves
    /// heading sizes itself.
    ///
    /// `None` means the caller keeps whatever size it had already derived from
    /// [`Self::heading_base_font_size`].
    pub fn heading_font_size(&self, level: u8) -> Option<Pixels> {
        self.heading_font_size
            .as_ref()
            .map(|f| f(level, self.heading_base_font_size))
    }

    /// The style refinement for code blocks.
    pub fn code_block(&self) -> &StyleRefinement {
        &self.code_block
    }

    /// The style refinement for the table container.
    pub fn table(&self) -> &StyleRefinement {
        &self.table
    }

    /// The style refinement for table header rows.
    pub fn table_head(&self) -> &StyleRefinement {
        &self.table_head
    }

    /// The style refinement for table cells.
    pub fn table_cell(&self) -> &StyleRefinement {
        &self.table_cell
    }

    /// The highlight style for inline code, before the code-background
    /// fallback in [`Self::inline_code_highlight`] applies.
    pub fn inline_code(&self) -> HighlightStyle {
        self.inline_code
    }

    /// Whether content-specific assets should use their dark variant.
    pub fn is_dark(&self) -> bool {
        self.is_dark
    }

    /// Returns the [`HighlightStyle`] to use for inline code, falling back to
    /// the code background when no custom background was supplied.
    pub(crate) fn inline_code_highlight(&self) -> HighlightStyle {
        let mut style = self.inline_code;
        if style.background_color.is_none() {
            style.background_color = Some(self.code_background);
        }
        style
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_layout_fingerprint_covers_callback_table_and_theme_fields() {
        let base = TextViewStyle::default();
        let heading = base.clone().with_heading_font_size(|_, size| size);
        assert!(heading == base.clone().with_heading_font_size(|_, size| size));
        assert!(heading != base.clone().with_heading_font_size(|_, size| size * 2.));

        let mut table = StyleRefinement::default();
        table.text.white_space = Some(gpui::WhiteSpace::Nowrap);
        assert!(base != base.clone().with_table_cell(table));

        assert!(base != base.clone().with_dark(true));
    }

    #[test]
    fn cloning_preserves_the_same_heading_callback_fingerprint() {
        let style = TextViewStyle::default().with_heading_font_size(|_, size| size);
        assert!(style == style.clone());
    }

    #[test]
    fn default_style_is_readable_without_an_application_theme() {
        let style = TextViewStyle::default();

        assert_eq!(style.foreground().a, 1.0);
        assert_eq!(style.link().a, 1.0);
        assert!(style.selection().a > 0.0);
        assert!(style.inline_code().background_color.is_some());
        assert!(style.code_background().a > 0.0);
        assert!(style.border().a > 0.0);
        assert_eq!(style.code_block().corner_radii.top_left, None);
        assert_eq!(style.code_block().corner_radii.top_right, None);
        assert_eq!(style.code_block().corner_radii.bottom_left, None);
        assert_eq!(style.code_block().corner_radii.bottom_right, None);
    }

    #[test]
    fn heading_font_size_resolves_through_the_installed_callback() {
        let style = TextViewStyle::default();
        assert_eq!(style.heading_font_size(1), None);

        let style = style.with_heading_font_size(|level, base| base * (7. - level as f32));
        assert_eq!(style.heading_font_size(1), Some(px(14.) * 6.));
        assert_eq!(style.heading_font_size(6), Some(px(14.)));
    }

    #[test]
    fn inline_code_falls_back_to_the_code_background() {
        let style = TextViewStyle::default()
            .with_code_background(gpui::rgb(0x123456).into())
            .with_inline_code(HighlightStyle::default());

        assert_eq!(
            style.inline_code_highlight().background_color,
            Some(gpui::rgb(0x123456).into())
        );
    }

    #[test]
    fn from_theme_maps_base_semantic_tokens() {
        let mut theme = crate::Theme::default();
        theme.tokens.colors.foreground = gpui::rgb(0x112233).into();
        theme.tokens.colors.muted_foreground = gpui::rgb(0x445566).into();
        theme.tokens.colors.primary = gpui::rgb(0x3366ff).into();
        theme.tokens.colors.accent = gpui::rgb(0xddeeff).into();
        theme.tokens.colors.border = gpui::rgb(0x778899).into();
        theme.tokens.colors.selection = gpui::rgb(0x55a0fc).into();

        let style = TextViewStyle::from_theme(&theme);
        assert_eq!(style.foreground(), theme.tokens.colors.foreground);
        assert_eq!(style.link(), theme.tokens.colors.primary);
        assert_eq!(style.selection(), theme.tokens.colors.selection);
        assert_eq!(style.code_background(), theme.tokens.colors.accent);
        assert_eq!(style.border(), theme.tokens.colors.border);
    }
}
