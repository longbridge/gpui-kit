use gpui::{
    App, Hsla, ImageSource, InteractiveElement, Interactivity, IntoElement, ParentElement as _,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, div, prelude::FluentBuilder,
};
use gpui_base::{Avatar as BaseAvatar, AvatarFallback, AvatarImage};

use crate::{
    ActiveTheme, Colorize, Icon, IconName, Sizable, Size, StyledExt, ThemeStyled as _,
    avatar::{AvatarSized as _, avatar_size},
};

/// User avatar element.
///
/// We can use [`Sizable`] trait to set the size of the avatar (see also: [`avatar_size`] about the size in pixels).
#[derive(IntoElement)]
pub struct Avatar {
    base: BaseAvatar,
    style: StyleRefinement,
    src: Option<ImageSource>,
    name: Option<SharedString>,
    short_name: SharedString,
    placeholder: Icon,
    size: Size,
}

impl Avatar {
    pub fn new() -> Self {
        Self {
            base: BaseAvatar::new(),
            style: StyleRefinement::default(),
            src: None,
            name: None,
            short_name: SharedString::default(),
            placeholder: Icon::new(IconName::User),
            size: Size::Medium,
        }
    }

    /// Set to use image source for the avatar.
    pub fn src(mut self, source: impl Into<ImageSource>) -> Self {
        self.src = Some(source.into());
        self
    }

    /// Set name of the avatar user, if `src` is none, will use this name as placeholder.
    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        let name: SharedString = name.into();
        let short: SharedString = extract_text_initials(&name).into();

        self.name = Some(name);
        self.short_name = short;
        self
    }

    /// Set placeholder icon, default: [`IconName::User`]
    pub fn placeholder(mut self, icon: impl Into<Icon>) -> Self {
        self.placeholder = icon.into();
        self
    }
}

impl Sizable for Avatar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Avatar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl InteractiveElement for Avatar {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for Avatar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let corner_radii = self.style.corner_radii.clone();
        let mut inner_style = StyleRefinement::default();
        inner_style.corner_radii = corner_radii;

        const COLOR_COUNT: u64 = 360 / 15;
        fn default_color(ix: u64, cx: &mut App) -> Hsla {
            let h = (ix * 15).clamp(0, 360) as f32;
            cx.theme().blue.hue(h / 360.0)
        }

        const BG_OPACITY: f32 = 0.2;

        let fallback = AvatarFallback::new()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .rounded_full_style(cx)
            .overflow_hidden()
            .when(self.name.is_none(), |this| {
                this.text_size(avatar_size(self.size) * 0.6)
                    .child(self.placeholder)
            })
            .when(self.name.is_some(), |this| {
                let color_ix = gpui::hash(&self.short_name) % COLOR_COUNT;
                let color = default_color(color_ix, cx);
                this.bg(color.opacity(BG_OPACITY))
                    .text_color(color)
                    .child(div().avatar_text_size(self.size).child(self.short_name))
            })
            .refine_style(&inner_style);

        self.base
            .size(avatar_size(self.size))
            .flex_shrink_0()
            .rounded_full_style(cx)
            .overflow_hidden()
            .bg(cx.theme().tokens.secondary)
            .text_color(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .fallback(fallback)
            .when_some(self.src, |this, src| {
                this.image(
                    AvatarImage::new(src)
                        .size_full()
                        .rounded_full_style(cx)
                        .refine_style(&inner_style),
                )
            })
            .refine_style(&self.style)
    }
}

fn extract_text_initials(text: &str) -> String {
    let mut result = text
        .split(" ")
        .flat_map(|word| word.chars().next().map(|c| c.to_string()))
        .take(2)
        .collect::<Vec<String>>()
        .join("");

    if result.len() == 1 {
        result = text.chars().take(2).collect::<String>();
    }

    result.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avatar_text_initials() {
        assert_eq!(extract_text_initials(&"Jason Lee"), "JL".to_string());
        assert_eq!(extract_text_initials(&"Foo Bar Dar"), "FB".to_string());
        assert_eq!(extract_text_initials(&"huacnlee"), "HU".to_string());
    }

    #[gpui::test]
    fn test_avatar_builder(_cx: &mut gpui::TestAppContext) {
        let avatar = Avatar::new()
            .name("Jason Lee")
            .placeholder(Icon::new(IconName::User))
            .large();

        assert_eq!(avatar.name, Some(SharedString::from("Jason Lee")));
        assert_eq!(avatar.short_name, SharedString::from("JL"));
        assert_eq!(avatar.size, Size::Large);
    }
}
