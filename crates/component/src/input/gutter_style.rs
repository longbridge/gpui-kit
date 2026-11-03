use gpui::{
    AbsoluteLength, App, Font, Hsla, Pixels, ShapedLine, SharedString, TextRun, TextStyle, Window,
    px,
};
use gpui_base::input::EditorLineNumberRenderer;

use crate::ActiveTheme as _;

/// Optional line-number presentation supplied by the application or UI layer.
///
/// Unspecified properties inherit the editor's defaults. Keeping them optional
/// lets a per-editor override follow later theme changes for every other property.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GutterStyle {
    gap: Option<AbsoluteLength>,
    line_number_color: Option<Hsla>,
    active_line_number_color: Option<Hsla>,
    line_number_font_family: Option<SharedString>,
    line_number_font_size: Option<AbsoluteLength>,
}

impl EditorLineNumberRenderer for GutterStyle {
    fn measure_line_numbers(
        &self,
        digit_count: usize,
        body_style: &TextStyle,
        window: &mut Window,
        _: &App,
    ) -> (Pixels, Pixels) {
        let gap = self.resolve_gap(window.rem_size());
        // Hidden line numbers need spacing but no font resolution or glyph metrics.
        if digit_count == 0 {
            return (px(0.), gap);
        }
        let style = self.resolve_text_style(body_style);
        let font_size = style.font_size.to_pixels(window.rem_size());
        let font_id = window.text_system().resolve_font(&style.font());
        // A font's punctuation does not bound its digits, and proportional
        // digits can differ from each other. Every supported digit must fit.
        let width = "0123456789"
            .chars()
            .map(|digit| window.text_system().layout_width(font_id, font_size, digit))
            .fold(px(0.), Pixels::max)
            * digit_count;
        (width, gap)
    }

    fn shape_line_number(
        &self,
        number: usize,
        is_active: bool,
        body_style: &TextStyle,
        window: &mut Window,
        cx: &App,
    ) -> ShapedLine {
        let color = if is_active {
            self.active_line_number_color
                .unwrap_or(cx.theme().foreground)
        } else {
            self.line_number_color
                .unwrap_or(cx.theme().muted_foreground)
        };
        // Base aligns shaped advances, so padding would distort the label width.
        let label: SharedString = number.to_string().into();
        // Build only the shaping inputs; cloning TextStyle for every visible
        // line would also copy unrelated layout settings and shared handles.
        let run = TextRun {
            len: label.len(),
            font: Font {
                family: self
                    .line_number_font_family
                    .as_ref()
                    .unwrap_or(&body_style.font_family)
                    .clone(),
                features: body_style.font_features.clone(),
                fallbacks: body_style.font_fallbacks.clone(),
                weight: body_style.font_weight,
                style: body_style.font_style,
            },
            color,
            background_color: body_style.background_color,
            underline: body_style.underline,
            strikethrough: body_style.strikethrough,
        };
        window.text_system().shape_line(
            label,
            self.line_number_font_size
                .unwrap_or(body_style.font_size)
                .to_pixels(window.rem_size()),
            &[run],
            None,
        )
    }
}

impl GutterStyle {
    /// Sets the gap before the body text, after any folding controls.
    ///
    /// Accepts rems that follow window zoom or pixels for a fixed gap.
    pub fn with_gap(mut self, gap: impl Into<AbsoluteLength>) -> Self {
        self.gap = Some(gap.into());
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
    ///
    /// Accepts rems that follow window zoom or pixels for a fixed font size.
    pub fn with_line_number_font_size(mut self, size: impl Into<AbsoluteLength>) -> Self {
        self.line_number_font_size = Some(size.into());
        self
    }

    /// Returns the explicitly specified gap.
    pub fn gap(&self) -> Option<AbsoluteLength> {
        self.gap
    }

    /// Returns the explicitly specified normal line-number color.
    pub fn line_number_color(&self) -> Option<Hsla> {
        self.line_number_color
    }

    /// Returns the explicitly specified current-line number color.
    pub fn active_line_number_color(&self) -> Option<Hsla> {
        self.active_line_number_color
    }

    /// Returns the explicitly specified line-number font family.
    pub fn line_number_font_family(&self) -> Option<&SharedString> {
        self.line_number_font_family.as_ref()
    }

    /// Returns the explicitly specified line-number font size.
    pub fn line_number_font_size(&self) -> Option<AbsoluteLength> {
        self.line_number_font_size
    }

    /// Applies only explicitly specified properties, including transparent colors.
    pub(crate) fn apply_overrides(&mut self, overrides: Self) {
        self.gap = overrides.gap.or(self.gap);
        self.line_number_color = overrides.line_number_color.or(self.line_number_color);
        self.active_line_number_color = overrides
            .active_line_number_color
            .or(self.active_line_number_color);
        if let Some(family) = overrides.line_number_font_family {
            self.line_number_font_family = Some(family);
        }
        self.line_number_font_size = overrides
            .line_number_font_size
            .or(self.line_number_font_size);
    }

    pub(crate) fn resolve_gap(&self, rem_size: Pixels) -> Pixels {
        // Preserve each length's unit so GPUI controls pixel and rem behavior.
        self.gap.unwrap_or(px(6.).into()).to_pixels(rem_size)
    }

    pub(crate) fn resolve_text_style(&self, body: &TextStyle) -> TextStyle {
        let mut style = body.clone();
        if let Some(family) = &self.line_number_font_family {
            style.font_family = family.clone();
        }
        if let Some(size) = self.line_number_font_size {
            style.font_size = size;
        }
        style
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Styled as _, div, px, rems, rgb};

    #[test]
    fn test_gutter_style_builder_and_partial_overrides() {
        let normal = rgb(0x808080).into();
        let active = rgb(0xeeeeee).into();
        let mut style = GutterStyle::default()
            .with_gap(px(12.))
            .with_line_number_color(normal)
            .with_active_line_number_color(active)
            .with_line_number_font_family("Number Font")
            .with_line_number_font_size(px(12.));
        let defaults = style.clone();

        style.apply_overrides(GutterStyle::default());
        assert_eq!(style, defaults);

        // Zero spacing and transparent ink are explicit values, so treating
        // either as an unset sentinel would lose an application's override.
        style.apply_overrides(
            GutterStyle::default()
                .with_gap(px(0.))
                .with_line_number_color(Hsla::transparent_black())
                .with_line_number_font_size(px(10.)),
        );
        assert_eq!(style.gap(), Some(px(0.).into()));
        assert_eq!(style.line_number_color(), Some(Hsla::transparent_black()));
        assert_eq!(style.active_line_number_color(), Some(active));
        assert_eq!(
            style.line_number_font_family().unwrap().as_ref(),
            "Number Font"
        );
        assert_eq!(style.line_number_font_size(), Some(px(10.).into()));

        style.apply_overrides(GutterStyle::default().with_active_line_number_color(normal));
        assert_eq!(style.active_line_number_color(), Some(normal));
        assert_eq!(style.line_number_color(), Some(Hsla::transparent_black()));
    }

    #[test]
    fn gutter_typography_inherits_unspecified_properties_without_changing_the_body() {
        let mut body = TextStyle::default();
        body.font_family = "Body Font".into();
        body.font_size = px(16.).into();
        let original = body.clone();

        assert_eq!(GutterStyle::default().resolve_text_style(&body), original);
        assert_eq!(GutterStyle::default().resolve_gap(px(16.)), px(6.));

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

    #[test]
    fn gutter_dimensions_follow_zoom_without_changing_configured_values() {
        let style = GutterStyle::default()
            .with_gap(rems(0.75))
            .with_line_number_font_size(rems(0.75));
        let fixed_style = GutterStyle::default()
            .with_gap(px(12.))
            .with_line_number_font_size(px(12.));
        let mut body = TextStyle::default();
        body.font_size = rems(1.).into();
        let resolved = style.resolve_text_style(&body);
        let reference_gap = div().gap_3().style().gap.width.unwrap();

        for (rem_size, expected_size) in [(12., 9.), (16., 12.), (24., 18.), (32., 24.), (16., 12.)]
        {
            assert_eq!(style.resolve_gap(px(rem_size)), px(expected_size));
            assert_eq!(fixed_style.resolve_gap(px(rem_size)), px(12.));
            assert_eq!(
                fixed_style
                    .resolve_text_style(&body)
                    .font_size
                    .to_pixels(px(rem_size)),
                px(12.)
            );
            assert_eq!(
                style.resolve_gap(px(rem_size)),
                reference_gap.to_pixels(px(0.).into(), px(rem_size))
            );
            assert_eq!(
                resolved.font_size.to_pixels(px(rem_size)),
                px(expected_size)
            );
            assert_eq!(GutterStyle::default().resolve_gap(px(rem_size)), px(6.));
            assert_eq!(
                style.clone().with_gap(px(0.)).resolve_gap(px(rem_size)),
                px(0.)
            );
            assert_eq!(GutterStyle::default().resolve_text_style(&body), body);
        }
        assert_eq!(style.gap(), Some(rems(0.75).into()));
        assert_eq!(style.line_number_font_size(), Some(rems(0.75).into()));
    }
}
