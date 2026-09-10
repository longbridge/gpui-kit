use std::sync::Arc;

use gpui::{AnyView, App, Image, Pixels, Size, TextStyle, Window};

type HoverCardBuilder = dyn Fn(&mut Window, &mut App) -> AnyView + Send + Sync;

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
}

impl MarkdownInlinePresentation {
    /// Present a prepared image (including SVG) with explicit baseline metrics.
    /// Invalid metrics or unavailable images fall back to the node's plain text.
    pub fn image(image: Arc<Image>, metrics: MarkdownInlineMetrics) -> Self {
        Self {
            image: Some((image, metrics)),
            hover_card: None,
        }
    }

    /// Display the node's plain text as a measured, atomic inline object.
    pub fn text() -> Self {
        Self {
            image: None,
            hover_card: None,
        }
    }

    /// Show supplementary, read-only content in GPUI's hoverable tooltip layer.
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
