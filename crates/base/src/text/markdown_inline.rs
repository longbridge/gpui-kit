use std::sync::Arc;

use gpui::{AnyView, App, FontWeight, Hsla, Image, Pixels, Size, TextStyle, Window};

type HoverCardBuilder = dyn Fn(&mut Window, &mut App) -> AnyView + Send + Sync;

#[derive(Clone, Default)]
pub(crate) struct InlineAppearance {
    pub color: Option<Hsla>,
    pub background: Option<Hsla>,
    pub hover_background: Option<Hsla>,
    pub font_weight: Option<FontWeight>,
    pub padding_x: Pixels,
    pub radius: Pixels,
}

/// Geometry of a read-only inline object, in logical pixels at the current font size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkdownInlineMetrics {
    pub size: Size<Pixels>,
    /// Distance from the top edge to the alphabetic baseline.
    pub baseline: Pixels,
}

impl MarkdownInlineMetrics {
    pub fn new(size: Size<Pixels>, baseline: Pixels) -> Self {
        Self { size, baseline }
    }

    pub(crate) fn is_valid(self) -> bool {
        let width = f32::from(self.size.width);
        let height = f32::from(self.size.height);
        let baseline = f32::from(self.baseline);
        width.is_finite()
            && height.is_finite()
            && baseline.is_finite()
            && width > 0.
            && height > 0.
            && baseline >= 0.
            && baseline <= height
    }
}

/// Static inline content. TextView owns all hit testing and selection.
///
/// Inline content is limited to text and images. Optional read-only hover
/// cards render separately from the inline layout.
#[derive(Clone)]
pub struct MarkdownInlinePresentation {
    pub(crate) image: Option<(Arc<Image>, MarkdownInlineMetrics)>,
    pub(crate) hover_card: Option<Arc<HoverCardBuilder>>,
    pub(crate) appearance: InlineAppearance,
}

impl MarkdownInlinePresentation {
    /// Present a prepared image (including SVG) with explicit baseline metrics.
    /// Invalid metrics or unavailable images fall back to the node's plain text.
    pub fn image(image: Arc<Image>, metrics: MarkdownInlineMetrics) -> Self {
        Self {
            image: Some((image, metrics)),
            hover_card: None,
            appearance: InlineAppearance::default(),
        }
    }

    /// Display the node's plain text as a measured, atomic inline object.
    pub fn text() -> Self {
        Self {
            image: None,
            hover_card: None,
            appearance: InlineAppearance::default(),
        }
    }

    /// Set the atomic text's foreground color.
    pub fn text_color(mut self, color: Hsla) -> Self {
        self.appearance.color = Some(color);
        self
    }

    /// Set the atomic text's weight, used for both measurement and painting.
    pub fn font_weight(mut self, weight: FontWeight) -> Self {
        self.appearance.font_weight = Some(weight);
        self
    }

    /// Set a background behind the entire object.
    pub fn background(mut self, color: Hsla) -> Self {
        self.appearance.background = Some(color);
        self
    }

    /// Set the background while the pointer is over the object.
    pub fn hover_background(mut self, color: Hsla) -> Self {
        self.appearance.hover_background = Some(color);
        self
    }

    /// Add horizontal padding to atomic text, included in wrapping and selection.
    pub fn padding_x(mut self, padding: Pixels) -> Self {
        self.appearance.padding_x = if f32::from(padding).is_finite() {
            padding.max(Pixels::ZERO)
        } else {
            Pixels::ZERO
        };
        self
    }

    /// Round the object's background corners.
    pub fn rounded(mut self, radius: Pixels) -> Self {
        self.appearance.radius = if f32::from(radius).is_finite() {
            radius.max(Pixels::ZERO)
        } else {
            Pixels::ZERO
        };
        self
    }

    /// Show read-only content centered horizontally on the whole inline object.
    ///
    /// The builder runs on hover, outside inline layout. The card must not
    /// contain focusable controls; selection and copying stay with TextView.
    pub fn hover_card<F>(mut self, build: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> AnyView + Send + Sync + 'static,
    {
        self.hover_card = Some(Arc::new(build));
        self
    }
}

/// Current layout inputs for an inline renderer. Recomputed when layout changes.
#[derive(Clone)]
pub struct MarkdownInlineRenderContext {
    pub text_style: TextStyle,
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub rem_size: Pixels,
    pub available_width: Option<Pixels>,
}
