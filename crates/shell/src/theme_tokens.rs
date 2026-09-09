//! Script token names resolved from gpui-base's active semantic theme.

use std::cell::RefCell;

use gpui::{App, Hsla, Pixels};
use gpui_base::{ColorTokens, RadiusTokens, SemanticThemeTokens, SpacingTokens, Theme};

use crate::scope::with_current_app;

thread_local! {
    static CACHED: RefCell<CachedTheme> = const { RefCell::new(CachedTheme {
        key: None,
        revision: 0,
    }) };
}

#[derive(Clone, PartialEq)]
pub(crate) struct ThemeSnapshotKey {
    pub(crate) tokens: SemanticThemeTokens,
    pub(crate) appearance: gpui_base::ThemeAppearance,
}

struct CachedTheme {
    key: Option<ThemeSnapshotKey>,
    revision: u32,
}

pub(crate) fn sync(cx: &App) -> ThemeSnapshotKey {
    let theme = Theme::global(cx);
    let key = ThemeSnapshotKey {
        tokens: theme.tokens,
        appearance: theme.appearance,
    };
    CACHED.with(|cached| {
        let mut cached = cached.borrow_mut();
        if cached.key.as_ref() != Some(&key) {
            cached.key = Some(key.clone());
            cached.revision = cached.revision.wrapping_add(1);
        }
    });
    key
}

/// Lends the active palette to `read`, from the same two places in the same
/// order [`sync`] keeps them in.
///
/// Lends rather than hands over. `SemanticThemeTokens` is around six hundred
/// bytes and carries two `SharedString`s, and `Theme::global` copies the whole
/// theme around it — while a description resolves a token for every colour,
/// spacing and radius it names, and materializing that description resolves
/// every one of them again. A copy per lookup made `bg("surface")` one of the
/// most expensive calls a script could make.
fn with_tokens<R>(read: impl FnOnce(&SemanticThemeTokens) -> R) -> Option<R> {
    // A scoped `App` is authoritative, so it is asked first — and asked
    // whether it has a theme before `read` is handed to it, because `read` can
    // only be given away once and the cache is still to try.
    if with_current_app(|cx| cx.has_global::<Theme>()) == Some(true) {
        return with_current_app(|cx| cx.try_global::<Theme>().map(|theme| read(&theme.tokens)))
            .flatten();
    }
    // Materializing a description runs outside every call scope, so this is
    // the only palette that path can see.
    CACHED.with(|cached| cached.borrow().key.as_ref().map(|key| read(&key.tokens)))
}

pub(crate) fn snapshot() -> ThemeSnapshotKey {
    CACHED.with(|cached| {
        cached
            .borrow()
            .key
            .clone()
            .unwrap_or_else(|| ThemeSnapshotKey {
                tokens: SemanticThemeTokens::default(),
                appearance: gpui_base::ThemeAppearance::default(),
            })
    })
}

pub(crate) fn revision() -> u32 {
    CACHED.with(|cached| cached.borrow().revision)
}

pub(crate) fn token_color(name: &str) -> Option<Hsla> {
    with_tokens(|tokens| resolve_color(&tokens.colors, name)).flatten()
}

pub(crate) const COLOR_TOKEN_NAMES: &[&str] = &[
    "background",
    "foreground",
    "surface",
    "surface_foreground",
    "primary",
    "primary_foreground",
    "secondary",
    "secondary_foreground",
    "muted",
    "muted_foreground",
    "accent",
    "accent_foreground",
    "destructive",
    "destructive_foreground",
    "border",
    "input",
    "ring",
    "selection",
];

pub(crate) const SPACING_TOKEN_NAMES: &[&str] = &["xxs", "xs", "sm", "md", "lg", "xl", "xxl"];
pub(crate) const RADIUS_TOKEN_NAMES: &[&str] = &["none", "sm", "md", "lg", "xl", "full"];

pub(crate) fn color_token_names() -> &'static [&'static str] {
    COLOR_TOKEN_NAMES
}
pub(crate) fn spacing_token_names() -> &'static [&'static str] {
    SPACING_TOKEN_NAMES
}
pub(crate) fn radius_token_names() -> &'static [&'static str] {
    RADIUS_TOKEN_NAMES
}

pub(crate) fn resolve_color(colors: &ColorTokens, name: &str) -> Option<Hsla> {
    Some(match name {
        "background" => colors.background,
        "foreground" => colors.foreground,
        "surface" => colors.surface,
        "surface_foreground" => colors.surface_foreground,
        "primary" => colors.primary,
        "primary_foreground" => colors.primary_foreground,
        "secondary" => colors.secondary,
        "secondary_foreground" => colors.secondary_foreground,
        "muted" => colors.muted,
        "muted_foreground" => colors.muted_foreground,
        "accent" => colors.accent,
        "accent_foreground" => colors.accent_foreground,
        "destructive" => colors.destructive,
        "destructive_foreground" => colors.destructive_foreground,
        "border" => colors.border,
        "input" => colors.input,
        "ring" => colors.ring,
        "selection" => colors.selection,
        _ => return None,
    })
}

pub(crate) fn resolve_spacing(spacing: &SpacingTokens, name: &str) -> Option<Pixels> {
    Some(match name {
        "xxs" => spacing.xxs,
        "xs" => spacing.xs,
        "sm" => spacing.sm,
        "md" => spacing.md,
        "lg" => spacing.lg,
        "xl" => spacing.xl,
        "xxl" => spacing.xxl,
        _ => return None,
    })
}

pub(crate) fn resolve_radius(radius: &RadiusTokens, name: &str) -> Option<Pixels> {
    Some(match name {
        "none" => radius.none,
        "sm" => radius.sm,
        "md" => radius.md,
        "lg" => radius.lg,
        "xl" => radius.xl,
        "full" => radius.full,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field_names<T: serde::Serialize>(value: &T) -> Vec<String> {
        let mut names: Vec<_> = serde_json::to_value(value)
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        names.sort();
        names
    }

    fn sorted(names: &[&str]) -> Vec<String> {
        let mut names: Vec<_> = names.iter().map(ToString::to_string).collect();
        names.sort();
        names
    }

    #[test]
    fn token_names_match_gpui_base_fields() {
        assert_eq!(
            field_names(&ColorTokens::default()),
            sorted(color_token_names())
        );
        assert_eq!(
            field_names(&SpacingTokens::default()),
            sorted(spacing_token_names())
        );
        assert_eq!(
            field_names(&RadiusTokens::default()),
            sorted(radius_token_names())
        );
    }

    #[test]
    fn unknown_names_are_rejected() {
        assert!(resolve_color(&ColorTokens::default(), "backgrund").is_none());
        assert!(resolve_spacing(&SpacingTokens::default(), "xxxl").is_none());
        assert!(resolve_radius(&RadiusTokens::default(), "rounded").is_none());
    }
}
