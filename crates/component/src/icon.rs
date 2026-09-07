use std::sync::Arc;

use crate::{ActiveTheme, Sizable, Size};
use gpui::{
    AnyElement, App, AppContext, Context, Entity, Hsla, IntoElement, Pixels, Radians, Render,
    RenderOnce, SharedString, StyleRefinement, Styled, Svg, Transformation, Window,
    prelude::FluentBuilder as _, svg,
};
use gpui_component_macros::icon_named;

/// Types implementing this trait can automatically be converted to [`Icon`].
///
/// This allows you to implement a custom version of [`IconName`] that functions as a drop-in
/// replacement for other UI components.
pub trait IconNamed {
    /// Returns the embedded path of the icon.
    fn path(self) -> SharedString;
}

impl<T: IconNamed> From<T> for Icon {
    fn from(value: T) -> Self {
        Icon::build(value)
    }
}

// Generate `IconName` from the icons that `gpui-kit-assets` ships.
// The `$VAR` form resolves to the absolute path published by the assets
// crate's `build.rs` (via cargo's `links` mechanism) and re-exported by
// our own `build.rs`. See `gpui_component_macros::icon_named!`'s doc
// comment for the full mechanism.
icon_named!(IconName, "$GPUI_KIT_DEFAULT_ICONS_DIR");

impl IconName {
    /// Return the icon as a Entity<Icon>
    pub fn view(self, cx: &mut App) -> Entity<Icon> {
        Icon::build(self).view(cx)
    }
}

impl From<IconName> for AnyElement {
    fn from(val: IconName) -> Self {
        Icon::build(val).into_any_element()
    }
}

impl RenderOnce for IconName {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        Icon::build(self)
    }
}

#[derive(Clone)]
pub(crate) enum IconSource {
    Path(SharedString),
    Data(Arc<[u8]>),
}

#[derive(Clone, IntoElement)]
pub struct Icon {
    style: StyleRefinement,
    source: IconSource,
    text_color: Option<Hsla>,
    size: Option<Size>,
    transformation: Option<Transformation>,
}

impl Default for Icon {
    fn default() -> Self {
        Self {
            style: StyleRefinement::default(),
            source: IconSource::Path("".into()),
            text_color: None,
            size: None,
            transformation: None,
        }
    }
}

impl Icon {
    pub fn new(icon: impl Into<Icon>) -> Self {
        icon.into()
    }

    fn build(name: impl IconNamed) -> Self {
        Self::default().path(name.path())
    }

    /// Set the icon path of the Assets bundle
    ///
    /// For example: `icons/foo.svg`
    /// Replaces any previously set path or SVG data.
    pub fn path(mut self, path: impl Into<SharedString>) -> Self {
        self.source = IconSource::Path(path.into());
        self
    }

    /// Set raw SVG bytes without registering an asset path.
    ///
    /// Copies the bytes into shared storage; the input need not be static.
    /// Cloning the icon shares those bytes. Replaces any previously set path or data.
    /// Parsing and rendering follow GPUI's SVG behavior.
    ///
    /// ```
    /// use gpui_component::Icon;
    ///
    /// let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
    ///     <path d="M4 12h16" stroke="currentColor"/>
    /// </svg>"#;
    /// let icon = Icon::default().data(bytes);
    /// ```
    pub fn data(mut self, data: &[u8]) -> Self {
        self.source = IconSource::Data(Arc::from(data));
        self
    }

    #[cfg(any(target_os = "macos", target_os = "windows", test))]
    pub(crate) fn source_ref(&self) -> &IconSource {
        &self.source
    }

    /// Create a new view for the icon
    pub fn view(self, cx: &mut App) -> Entity<Icon> {
        cx.new(|_| self)
    }

    /// Set the SVG transformation, replacing any previous transformation or rotation.
    pub fn transform(mut self, transformation: gpui::Transformation) -> Self {
        self.transformation = Some(transformation);
        self
    }

    pub fn empty() -> Self {
        Self::default()
    }

    /// Rotate the icon by the given angle
    ///
    /// Replaces any previous transformation or rotation.
    pub fn rotate(mut self, radians: impl Into<Radians>) -> Self {
        self.transformation = Some(Transformation::rotate(radians));
        self
    }

    fn into_svg(self, text_size: Pixels, fallback_color: Hsla) -> Svg {
        let text_color = self.text_color.unwrap_or(fallback_color);
        let has_base_size = self.style.size.width.is_some() || self.style.size.height.is_some();

        svg()
            .map(|mut this| {
                *this.style() = self.style;
                this
            })
            .flex_shrink_0()
            .text_color(text_color)
            .when(!has_base_size, |this| this.size(text_size))
            .when_some(self.size, |this, size| match size {
                Size::Size(px) => this.size(px),
                Size::XSmall => this.size_3(),
                Size::Small => this.size_3p5(),
                Size::Medium => this.size_4(),
                Size::Large => this.size_6(),
            })
            .map(|this| match self.source {
                IconSource::Path(path) => this.path(path),
                IconSource::Data(data) => this.data(&data),
            })
            .when_some(self.transformation, |this, transformation| {
                this.with_transformation(transformation)
            })
    }
}

impl Styled for Icon {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }

    fn text_color(mut self, color: impl Into<Hsla>) -> Self {
        self.text_color = Some(color.into());
        self
    }
}

impl Sizable for Icon {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let text_size = window.text_style().font_size.to_pixels(window.rem_size());
        self.into_svg(text_size, window.text_style().color)
    }
}

impl From<Icon> for AnyElement {
    fn from(val: Icon) -> Self {
        val.into_any_element()
    }
}

impl Render for Icon {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text_size = window.text_style().font_size.to_pixels(window.rem_size());
        self.clone().into_svg(text_size, cx.theme().foreground)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{px, size};

    const SVG: &[u8] = include_bytes!("../../assets/assets/icons/arrow-up.svg");

    #[test]
    fn test_icon_builder_preserves_owned_data_and_transform_on_clone() {
        let transformation = Transformation::scale(size(0.5, 0.5))
            .with_rotation(gpui::radians(std::f32::consts::FRAC_PI_2));
        let icon = {
            let bytes = SVG.to_vec();
            Icon::default()
                .data(&bytes)
                .large()
                .text_color(gpui::red())
                .transform(transformation)
        };
        let cloned = icon.clone();
        let (IconSource::Data(original), IconSource::Data(copy)) = (&icon.source, &cloned.source)
        else {
            panic!("cloning must preserve the data source");
        };
        assert_eq!(copy.as_ref(), SVG);
        assert!(Arc::ptr_eq(original, copy));
        assert_eq!(cloned.transformation, Some(transformation));
        assert_eq!(cloned.size, icon.size);
        assert_eq!(cloned.text_color, icon.text_color);

        let mut svg = cloned.into_svg(px(12.), gpui::blue());
        assert_eq!(svg.style().text.color, Some(gpui::red()));
        assert_eq!(svg.style().size.width, Some(gpui::rems(1.5).into()));

        let rotated = icon.rotate(gpui::radians(std::f32::consts::PI)).clone();
        assert_eq!(
            rotated.transformation,
            Some(Transformation::rotate(gpui::radians(std::f32::consts::PI)))
        );
    }

    #[test]
    fn test_icon_source_builders_replace_previous_source() {
        let icon = Icon::new(IconName::Search).data(SVG);
        assert!(matches!(icon.source_ref(), IconSource::Data(bytes) if bytes.as_ref() == SVG));

        let icon = icon.path("icons/replacement.svg");
        assert!(
            matches!(icon.source_ref(), IconSource::Path(path) if path == "icons/replacement.svg")
        );

        let icon = icon.data(SVG).data(b"replacement");
        assert!(
            matches!(icon.source_ref(), IconSource::Data(bytes) if bytes.as_ref() == b"replacement")
        );

        let icon = icon.path("");
        assert!(matches!(icon.source_ref(), IconSource::Path(path) if path.is_empty()));
    }
}
