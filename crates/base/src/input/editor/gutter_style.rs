use gpui::{Hsla, Pixels, SharedString, TextStyle};

/// Optional line-number presentation supplied by the application or UI layer.
///
/// Unspecified properties inherit the editor's defaults. Keeping them optional
/// lets a per-editor override follow later theme changes for every other property.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GutterStyle {
    gap: Option<Pixels>,
    line_number_color: Option<Hsla>,
    active_line_number_color: Option<Hsla>,
    line_number_font_family: Option<SharedString>,
    line_number_font_size: Option<Pixels>,
}

impl GutterStyle {
    /// Sets the gap before the body text, after any folding controls.
    pub fn with_gap(mut self, gap: Pixels) -> Self {
        self.gap = Some(gap);
        self
    }

    /// Sets the color of line numbers outside the current line.
    pub fn with_line_number_color(mut self, color: Hsla) -> Self {
        self.line_number_color = Some(color);
        self
    }

    /// Sets the color of the current line number.
    pub fn with_active_line_number_color(mut self, color: Hsla) -> Self {
        self.active_line_number_color = Some(color);
        self
    }

    /// Sets the line-number font independently of the body font.
    pub fn with_line_number_font_family(mut self, family: impl Into<SharedString>) -> Self {
        self.line_number_font_family = Some(family.into());
        self
    }

    /// Sets the line-number font size without changing the body row height.
    pub fn with_line_number_font_size(mut self, size: Pixels) -> Self {
        self.line_number_font_size = Some(size);
        self
    }

    /// Returns the explicitly specified gap.
    pub fn get_gap(&self) -> Option<Pixels> {
        self.gap
    }

    /// Returns the explicitly specified normal line-number color.
    pub fn get_line_number_color(&self) -> Option<Hsla> {
        self.line_number_color
    }

    /// Returns the explicitly specified current-line number color.
    pub fn get_active_line_number_color(&self) -> Option<Hsla> {
        self.active_line_number_color
    }

    /// Returns the explicitly specified line-number font family.
    pub fn get_line_number_font_family(&self) -> Option<&SharedString> {
        self.line_number_font_family.as_ref()
    }

    /// Returns the explicitly specified line-number font size.
    pub fn get_line_number_font_size(&self) -> Option<Pixels> {
        self.line_number_font_size
    }

    /// Applies only explicitly specified properties, including transparent colors.
    pub fn apply_overrides(&mut self, overrides: &Self) {
        self.gap = overrides.gap.or(self.gap);
        self.line_number_color = overrides.line_number_color.or(self.line_number_color);
        self.active_line_number_color = overrides
            .active_line_number_color
            .or(self.active_line_number_color);
        if let Some(family) = &overrides.line_number_font_family {
            self.line_number_font_family = Some(family.clone());
        }
        self.line_number_font_size = overrides
            .line_number_font_size
            .or(self.line_number_font_size);
    }

    pub(crate) fn resolve_gap(&self) -> Pixels {
        self.gap.unwrap_or(super::element::LINE_NUMBER_RIGHT_MARGIN)
    }

    pub(crate) fn resolve_text_style(&self, body: &TextStyle) -> TextStyle {
        let mut style = body.clone();
        if let Some(family) = &self.line_number_font_family {
            style.font_family = family.clone();
        }
        if let Some(size) = self.line_number_font_size {
            style.font_size = size.into();
        }
        style
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;

    #[test]
    fn test_gutter_style_builder_and_partial_overrides() {
        let normal = gpui::rgb(0x808080).into();
        let active = gpui::rgb(0xeeeeee).into();
        let mut style = GutterStyle::default()
            .with_gap(px(12.))
            .with_line_number_color(normal)
            .with_active_line_number_color(active)
            .with_line_number_font_family("Number Font")
            .with_line_number_font_size(px(12.));
        let defaults = style.clone();

        style.apply_overrides(&GutterStyle::default());
        assert_eq!(style, defaults);

        // Zero spacing and transparent ink are explicit values, so treating
        // either as an unset sentinel would lose an application's override.
        style.apply_overrides(
            &GutterStyle::default()
                .with_gap(px(0.))
                .with_line_number_color(Hsla::transparent_black())
                .with_line_number_font_size(px(10.)),
        );
        assert_eq!(style.get_gap(), Some(px(0.)));
        assert_eq!(style.get_line_number_color(), Some(Hsla::transparent_black()));
        assert_eq!(style.get_active_line_number_color(), Some(active));
        assert_eq!(style.get_line_number_font_family().unwrap().as_ref(), "Number Font");
        assert_eq!(style.get_line_number_font_size(), Some(px(10.)));

        style.apply_overrides(&GutterStyle::default().with_active_line_number_color(normal));
        assert_eq!(style.get_active_line_number_color(), Some(normal));
        assert_eq!(style.get_line_number_color(), Some(Hsla::transparent_black()));
    }

    #[test]
    fn gutter_typography_inherits_unspecified_properties_without_changing_the_body() {
        let mut body = TextStyle::default();
        body.font_family = "Body Font".into();
        body.font_size = px(16.).into();
        let original = body.clone();

        assert_eq!(GutterStyle::default().resolve_text_style(&body), original);
        assert_eq!(GutterStyle::default().resolve_gap(), px(6.));

        let family_only = GutterStyle::default()
            .with_line_number_font_family("Number Font")
            .resolve_text_style(&body);
        assert_eq!(family_only.font_family.as_ref(), "Number Font");
        assert_eq!(family_only.font_size, body.font_size);
        assert_eq!(family_only.line_height, body.line_height);

        let size_only = GutterStyle::default()
            .with_line_number_font_size(px(12.))
            .resolve_text_style(&body);
        assert_eq!(size_only.font_family, body.font_family);
        assert_eq!(size_only.font_size.to_pixels(px(16.)), px(12.));
        assert_eq!(size_only.line_height, body.line_height);
        assert_eq!(body, original);
    }
}
