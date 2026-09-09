use crate::{
    highlighter::HighlightTheme, list::ListSettings, notification::NotificationSettings,
    scroll::ScrollbarMode, sheet::SheetSettings,
};
use gpui::{App, Global, Hsla, IsZero as _, Pixels, SharedString, Window, WindowAppearance, px};
pub use gpui_base::{
    ColorTokens, RadiusTokens, SemanticThemeTokens, ShadowTokens, SpacingTokens, TextStyleToken,
    TypographyTokens,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

mod color;
mod mono_font;
mod motion;
mod registry;
mod schema;
mod theme_color;

pub use color::*;
pub use motion::*;
pub use registry::*;
pub use schema::*;
pub use theme_color::*;

pub fn init(cx: &mut App) {
    registry::init(cx);

    // Ensure theme is loaded directly on startup for WASM compatibility
    Theme::change(ThemeMode::Light, None, cx);
    Theme::sync_scrollbar_appearance(cx);
}

pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    #[inline(always)]
    fn theme(&self) -> &Theme {
        Theme::global(self)
    }
}

fn default_true() -> bool {
    true
}

/// The radius that rounds a shape as far as its own size allows, giving a
/// circle or a pill. Any value past half the shorter side is clamped by the
/// renderer, so this is simply "as round as it goes".
const RADIUS_FULL: Pixels = px(9999.);

/// How long the scrollbar stays visible after the last scroll, drag, or hover.
const SCROLLBAR_IDLE: Duration = Duration::from_secs(2);
/// How long the scrollbar takes to appear.
const SCROLLBAR_ENTER: Duration = Duration::from_millis(300);
/// How long the scrollbar takes to fade away once the idle hold expires.
const SCROLLBAR_EXIT: Duration = Duration::from_millis(500);
/// How long the thumb takes to reach its hovered or resting width.
const SCROLLBAR_EXPAND: Duration = Duration::from_millis(300);

/// The scrollbar motion this design system projects onto Base.
///
/// Scrolling and track hover reveal a scrollbar by fading it in place. In hover
/// mode, pointing at the thumb slides it in from the nearest edge as it fades.
fn scrollbar_motion(mode: ScrollbarMode) -> gpui_base::ScrollbarMotion {
    gpui_base::ScrollbarMotion::default()
        .with_idle(SCROLLBAR_IDLE)
        .with_enter(SCROLLBAR_ENTER)
        .with_exit(SCROLLBAR_EXIT)
        .with_expand(SCROLLBAR_EXPAND)
        .with_entrance(gpui_base::ScrollbarEntrance::Fade)
        .with_thumb_hover_entrance(match mode {
            ScrollbarMode::Scrolling | ScrollbarMode::Always => gpui_base::ScrollbarEntrance::Fade,
            ScrollbarMode::Hover => gpui_base::ScrollbarEntrance::SlideAndFade,
        })
}

/// The global theme configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Theme {
    pub colors: ThemeColor,
    /// Component-specific resolved tokens retained for legacy compatibility.
    ///
    /// New application-owned presentation should use [`Self::semantic_tokens`]
    /// rather than extending this legacy surface.
    #[serde(default)]
    pub tokens: ThemeTokens,
    pub highlight_theme: Arc<HighlightTheme>,
    pub light_theme: Rc<ThemeConfig>,
    pub dark_theme: Rc<ThemeConfig>,

    pub mode: ThemeMode,
    /// The font family for the application, default is `.SystemUIFont`.
    pub font_family: SharedString,
    /// The base font size for the application, default is 16px.
    pub font_size: Pixels,
    /// The monospace font family for the application.
    ///
    /// Defaults to:
    ///
    /// - macOS: `Menlo`
    /// - Windows: `Consolas`
    /// - Linux: `DejaVu Sans Mono`
    ///
    /// When that default is not installed, [`Theme::change`] swaps it for the
    /// first installed alternative (`Monaco`, `Cascadia Mono`, `Noto Sans Mono`
    /// and the like) and finally `.SystemUIFont`, so a missing font cannot
    /// crash text layout. A family set explicitly is used as-is.
    pub mono_font_family: SharedString,
    /// The monospace font size for the application, default is 13px.
    pub mono_font_size: Pixels,
    /// Radius for the general elements.
    pub radius: Pixels,
    /// Radius for the large elements, e.g.: Dialog, Notification border radius.
    pub radius_lg: Pixels,
    pub shadow: bool,
    /// Whether focused controls draw a ring outside their border, default true.
    ///
    /// The ring is painted outside the element, so any ancestor that clips its
    /// content will cut it off. An application whose layout clips heavily can
    /// turn it off here: focused controls then show only their tinted border,
    /// which costs no space and cannot be clipped.
    #[serde(default = "default_true")]
    pub focus_ring: bool,
    pub transparent: Hsla,
    /// Show the scrollbar mode, default: Scrolling
    #[serde(alias = "scrollbar_show")]
    pub scrollbar_mode: ScrollbarMode,
    /// The notification setting.
    #[serde(skip)]
    pub notification: NotificationSettings,
    /// Tile grid size, default is 4px.
    pub tile_grid_size: Pixels,
    /// The shadow of the tile panel.
    pub tile_shadow: bool,
    /// The border radius of the tile panel, default is 0px.
    pub tile_radius: Pixels,
    /// The list settings.
    pub list: ListSettings,
    /// The sheet settings.
    pub sheet: SheetSettings,
    /// Semantic motion policy for styled components.
    #[serde(skip)]
    pub motion: MotionTokens,
}

impl Default for Theme {
    fn default() -> Self {
        Self::from(&ThemeColor::default())
    }
}

impl Deref for Theme {
    type Target = ThemeColor;

    fn deref(&self) -> &Self::Target {
        &self.colors
    }
}

impl DerefMut for Theme {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.colors
    }
}

impl Global for Theme {}

impl Theme {
    /// Returns the global theme reference
    #[inline(always)]
    pub fn global(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    /// Returns the global theme mutable reference
    ///
    /// Changes to fields the Base layer mirrors — the radius, the colors, the
    /// fonts — reach the scrollbar and resize handles only once
    /// [`Theme::sync_base`] runs.
    #[inline(always)]
    pub fn global_mut(cx: &mut App) -> &mut Theme {
        cx.global_mut::<Theme>()
    }

    /// Returns true if the theme is dark.
    #[inline(always)]
    pub fn is_dark(&self) -> bool {
        self.mode.is_dark()
    }

    /// Returns the current theme name.
    pub fn theme_name(&self) -> &SharedString {
        if self.is_dark() {
            &self.dark_theme.name
        } else {
            &self.light_theme.name
        }
    }

    /// Sync the theme with the system appearance
    pub fn sync_system_appearance(window: Option<&mut Window>, cx: &mut App) {
        // Better use window.appearance() for avoid error on Linux.
        // https://github.com/longbridge/gpui-kit/issues/104
        let appearance = window
            .as_ref()
            .map(|window| window.appearance())
            .unwrap_or_else(|| cx.window_appearance());

        Self::change(appearance, window, cx);
    }

    /// Sync the Scrollbar showing behavior with the system
    pub fn sync_scrollbar_appearance(cx: &mut App) {
        let mode = if cx.should_auto_hide_scrollbars() {
            ScrollbarMode::Scrolling
        } else {
            ScrollbarMode::Hover
        };
        Self::set_scrollbar_mode(mode, cx);
    }

    /// Changes the scrollbar display mode and synchronizes the Base projection.
    pub fn set_scrollbar_mode(mode: ScrollbarMode, cx: &mut App) {
        Theme::global_mut(cx).scrollbar_mode = mode;
        let base_theme = gpui_base::Theme::global_mut(cx);
        base_theme.scrollbar = base_theme
            .scrollbar
            .clone()
            .with_mode(mode)
            .with_motion(scrollbar_motion(mode));
    }

    /// Change the theme mode.
    pub fn change(mode: impl Into<ThemeMode>, window: Option<&mut Window>, cx: &mut App) {
        let mode = mode.into();
        if !cx.has_global::<Theme>() {
            let mut theme = Theme::default();
            theme.light_theme = ThemeRegistry::global(cx).default_light_theme().clone();
            theme.dark_theme = ThemeRegistry::global(cx).default_dark_theme().clone();
            cx.set_global(theme);
        }

        {
            let theme = cx.global_mut::<Theme>();
            theme.mode = mode;
            if mode.is_dark() {
                theme.apply_config(&theme.dark_theme.clone());
            } else {
                theme.apply_config(&theme.light_theme.clone());
            }
        }
        mono_font::resolve_default_mono_font(cx);
        let theme = cx.global::<Theme>().clone();

        let base_theme = theme.base_theme();
        cx.set_global(base_theme);
        crate::text::install_text_view_defaults(&theme, cx);

        if let Some(window) = window {
            window.refresh();
        }
    }

    /// This theme projected onto the Base layer, which owns the scrollbar and
    /// resize handles and reads the semantic tokens.
    fn base_theme(&self) -> gpui_base::Theme {
        gpui_base::Theme {
            appearance: if self.mode.is_dark() {
                gpui_base::ThemeAppearance::Dark
            } else {
                gpui_base::ThemeAppearance::Light
            },
            tokens: self.semantic_tokens(),
            scrollbar: gpui_base::ScrollbarTheme::new()
                .with_mode(self.scrollbar_mode)
                .with_motion(scrollbar_motion(self.scrollbar_mode))
                .with_styles(
                    gpui_base::ScrollbarStyles::default()
                        .track(|style| style.bg(self.scrollbar))
                        .track_hover(|style| style.bg(self.scrollbar))
                        .track_active(|style| style.bg(self.scrollbar).border_color(self.border))
                        .thumb(|style| style.bg(self.tokens.scrollbar_thumb).radius(self.radius))
                        .thumb_hover(|style| {
                            style
                                .bg(self.tokens.scrollbar_thumb_hover)
                                .radius(self.radius)
                        })
                        .thumb_active(|style| {
                            style
                                .bg(self.tokens.scrollbar_thumb_hover)
                                .radius(self.radius)
                        }),
                ),
            resizable: gpui_base::ResizableTheme {
                handle: Some(self.border),
                active_handle: Some(self.drag_border),
            },
        }
    }

    /// Push the current theme down to the Base layer.
    ///
    /// The Base layer holds its own copy of the theme — the semantic tokens
    /// plus the scrollbar and resize-handle styles — because it paints those
    /// without going through `gpui-component`. [`Theme::change`] refreshes that
    /// copy, but writing to the theme's public fields directly does not, so a
    /// scrollbar keeps painting with the radius and colors it was last given.
    ///
    /// Call this after mutating the theme through [`Theme::global_mut`]:
    ///
    /// ```ignore
    /// Theme::global_mut(cx).radius = px(0.);
    /// Theme::sync_base(cx);
    /// ```
    ///
    /// It rebuilds the Base theme from scratch, so any style written straight
    /// onto the Base global is replaced — the same thing [`Theme::change`]
    /// does.
    pub fn sync_base(cx: &mut App) {
        let theme = Theme::global(cx).clone();
        let base_theme = theme.base_theme();
        cx.set_global(base_theme);
        crate::text::install_text_view_defaults(&theme, cx);
    }

    /// Get the input background color.
    ///
    /// For dark, use a transparent color mixed with the input border: `cx.theme().input`,
    /// otherwise use the `cx.theme().background` color.
    #[inline]
    pub fn input_background(&self) -> Hsla {
        if self.is_dark() {
            self.input.mix_oklab(self.transparent, 0.3)
        } else {
            self.background
        }
    }

    /// Get the editor background color, if not set, use the input background color.
    #[inline]
    pub(crate) fn editor_background(&self) -> Hsla {
        self.highlight_theme
            .style
            .editor_background
            .unwrap_or_else(|| self.input_background())
    }

    /// Returns a snapshot of the semantic design tokens represented by this
    /// theme. The snapshot is computed from the legacy public fields so direct
    /// mutations of those fields are reflected immediately.
    pub fn semantic_tokens(&self) -> SemanticThemeTokens {
        SemanticThemeTokens {
            colors: self.color_tokens(),
            radius: self.radius_tokens(),
            spacing: self.spacing_tokens(),
            typography: self.typography_tokens(),
            shadow: self.shadow_tokens(),
        }
    }

    /// Returns the styled layer's semantic motion policy.
    pub fn motion_tokens(&self) -> &MotionTokens {
        &self.motion
    }

    pub fn color_tokens(&self) -> ColorTokens {
        ColorTokens {
            background: self.background,
            foreground: self.foreground,
            surface: self.popover,
            surface_foreground: self.popover_foreground,
            primary: self.primary,
            primary_foreground: self.primary_foreground,
            secondary: self.secondary,
            secondary_foreground: self.secondary_foreground,
            muted: self.muted,
            muted_foreground: self.muted_foreground,
            accent: self.accent,
            accent_foreground: self.accent_foreground,
            destructive: self.danger,
            destructive_foreground: self.danger_foreground,
            border: self.border,
            input: self.input,
            ring: self.ring,
            selection: self.selection,
        }
    }

    /// The radius of a shape that reads as a circle or a pill — an avatar, a
    /// slider thumb, a badge dot, a pill tab.
    ///
    /// A theme whose [`Theme::radius`] is zero squares these off too, so one
    /// setting governs the whole UI instead of leaving a handful of
    /// permanently round elements behind. Use it in place of
    /// [`gpui::Styled::rounded_full`], or reach for
    /// [`crate::ThemeStyled::rounded_full_style`] when styling an element.
    pub fn radius_full(&self) -> Pixels {
        if self.radius.is_zero() {
            px(0.)
        } else {
            RADIUS_FULL
        }
    }

    /// Returns the next surface radius above the existing `xl` theme tier.
    ///
    /// Larger surface tiers derive from the same application-controlled base
    /// radius, so adjusting or squaring the theme updates every tier together.
    pub fn radius_2xl(&self) -> Pixels {
        self.radius * 2.5
    }

    /// Returns the surface radius above [`Self::radius_2xl`].
    pub fn radius_3xl(&self) -> Pixels {
        self.radius * 3.
    }

    /// Returns the surface radius above [`Self::radius_3xl`].
    pub fn radius_4xl(&self) -> Pixels {
        self.radius * 3.5
    }

    pub fn radius_tokens(&self) -> RadiusTokens {
        RadiusTokens {
            none: px(0.),
            sm: self.radius / 2.,
            md: self.radius,
            lg: self.radius_lg,
            xl: self.radius * 2.,
            full: self.radius_full(),
        }
    }

    pub fn spacing_tokens(&self) -> SpacingTokens {
        SpacingTokens::default()
    }

    pub fn typography_tokens(&self) -> TypographyTokens {
        let mut tokens = TypographyTokens::default();
        tokens.sans = self.font_family.clone();
        tokens.mono = self.mono_font_family.clone();
        tokens.md.size = self.font_size;
        tokens.mono_md.size = self.mono_font_size;
        tokens
    }

    pub fn shadow_tokens(&self) -> ShadowTokens {
        if self.shadow {
            ShadowTokens::elevations(self.transparent.alpha(0.18))
        } else {
            ShadowTokens::default()
        }
    }

    /// Applies the subset of semantic tokens representable by the legacy
    /// theme. Scale-only spacing and elevation details have no legacy storage;
    /// legacy components therefore keep their existing behavior.
    pub fn apply_semantic_tokens(&mut self, tokens: &SemanticThemeTokens) {
        let colors = tokens.colors;
        self.background = colors.background;
        self.foreground = colors.foreground;
        self.popover = colors.surface;
        self.popover_foreground = colors.surface_foreground;
        self.primary = colors.primary;
        self.primary_foreground = colors.primary_foreground;
        self.secondary = colors.secondary;
        self.secondary_foreground = colors.secondary_foreground;
        self.muted = colors.muted;
        self.muted_foreground = colors.muted_foreground;
        self.accent = colors.accent;
        self.accent_foreground = colors.accent_foreground;
        self.danger = colors.destructive;
        self.danger_foreground = colors.destructive_foreground;
        self.border = colors.border;
        self.input = colors.input;
        self.ring = colors.ring;

        self.tokens.background = colors.background.into();
        self.tokens.popover = colors.surface.into();
        self.tokens.primary = colors.primary.into();
        self.tokens.secondary = colors.secondary.into();
        self.tokens.muted = colors.muted.into();
        self.tokens.accent = colors.accent.into();
        self.tokens.danger = colors.destructive.into();

        self.radius = tokens.radius.md;
        self.radius_lg = tokens.radius.lg;
        self.font_family = tokens.typography.sans.clone();
        self.mono_font_family = tokens.typography.mono.clone();
        self.font_size = tokens.typography.md.size;
        self.mono_font_size = tokens.typography.mono_md.size;
        self.shadow = !tokens.shadow.sm.is_empty()
            || !tokens.shadow.md.is_empty()
            || !tokens.shadow.lg.is_empty();
    }

    /// Resolves a standalone semantic configuration over the current legacy
    /// theme without mutating either value.
    pub fn resolve_semantic_config(&self, config: &SemanticThemeConfig) -> SemanticThemeTokens {
        let mut tokens = self.semantic_tokens();
        config.apply_to(&mut tokens);
        tokens
    }

    /// Applies the legacy-representable part of a standalone semantic config
    /// and returns the complete resolved snapshot for application-owned UI.
    pub fn apply_semantic_config(&mut self, config: &SemanticThemeConfig) -> SemanticThemeTokens {
        let tokens = self.resolve_semantic_config(config);
        self.apply_semantic_tokens(&tokens);
        tokens
    }

    /// Parses and applies a standalone `{ "tokens": ... }` semantic theme file.
    pub fn apply_semantic_config_str(
        &mut self,
        content: &str,
    ) -> anyhow::Result<SemanticThemeTokens> {
        let config = serde_json::from_str::<SemanticThemeConfigFile>(content)?;
        Ok(self.apply_semantic_config(&config.tokens))
    }
}

#[cfg(test)]
mod semantic_token_tests {
    use gpui::{Hsla, IsZero as _, px};

    use super::{RADIUS_FULL, Theme};

    #[test]
    fn semantic_colors_are_a_live_projection_of_legacy_fields() {
        let mut theme = Theme::default();
        let primary = Hsla::default().alpha(0.42);
        theme.primary = primary;

        assert_eq!(theme.color_tokens().primary, primary);
        assert_eq!(theme.semantic_tokens().colors.primary, primary);
    }

    #[test]
    fn applying_semantic_tokens_only_updates_generic_legacy_colors() {
        let mut theme = Theme::default();
        let component_color = theme.button_primary;
        let mut tokens = theme.semantic_tokens();
        tokens.colors.primary = Hsla::default().alpha(0.25);
        tokens.colors.destructive = Hsla::default().alpha(0.75);
        tokens.radius.md = px(10.);

        theme.apply_semantic_tokens(&tokens);

        assert_eq!(theme.primary, tokens.colors.primary);
        assert_eq!(theme.tokens.primary.color, tokens.colors.primary);
        assert_eq!(theme.danger, tokens.colors.destructive);
        assert_eq!(theme.radius, px(10.));
        assert_eq!(theme.button_primary, component_color);
    }

    #[test]
    fn square_themes_square_off_pills_and_circles() {
        let mut theme = Theme::default();
        assert_eq!(theme.radius_full(), RADIUS_FULL);
        assert_eq!(theme.radius_tokens().full, RADIUS_FULL);

        // An application asking for square corners gets them everywhere, not
        // just on the elements whose radius happens to come from `radius`.
        theme.radius = px(0.);
        assert_eq!(theme.radius_full(), px(0.));
        assert_eq!(theme.radius_tokens().full, px(0.));
    }

    #[test]
    fn larger_surface_radii_follow_the_theme_radius() {
        let mut theme = Theme::default();
        assert!(theme.radius_tokens().xl < theme.radius_2xl());
        assert!(theme.radius_2xl() < theme.radius_3xl());
        assert!(theme.radius_3xl() < theme.radius_4xl());

        theme.radius = px(10.);
        assert_eq!(theme.radius_2xl(), px(25.));
        assert_eq!(theme.radius_3xl(), px(30.));
        assert_eq!(theme.radius_4xl(), px(35.));

        theme.radius = px(0.);
        assert_eq!(theme.radius_2xl(), px(0.));
        assert_eq!(theme.radius_3xl(), px(0.));
        assert_eq!(theme.radius_4xl(), px(0.));
    }

    #[test]
    fn base_projection_carries_a_square_radius_to_the_scrollbar() {
        let mut theme = Theme::default();
        assert!(!theme.base_theme().tokens.radius.md.is_zero());

        // The scrollbar paints from the Base layer's copy of the theme, so a
        // square theme has to reach it or the thumb stays a pill.
        theme.radius = px(0.);
        assert!(theme.base_theme().tokens.radius.md.is_zero());
    }

    #[test]
    fn disabled_legacy_shadows_project_to_empty_elevations() {
        let mut theme = Theme::default();
        theme.shadow = false;

        let shadows = theme.shadow_tokens();
        assert!(shadows.sm.is_empty());
        assert!(shadows.md.is_empty());
        assert!(shadows.lg.is_empty());
    }
}

impl From<&ThemeColor> for Theme {
    fn from(colors: &ThemeColor) -> Self {
        Theme {
            mode: ThemeMode::default(),
            transparent: Hsla::transparent_black(),
            font_family: ".SystemUIFont".into(),
            font_size: px(16.),
            mono_font_family: mono_font::default_mono_font_family(),
            mono_font_size: px(13.),
            radius: px(6.),
            radius_lg: px(8.),
            shadow: true,
            focus_ring: true,
            scrollbar_mode: ScrollbarMode::default(),
            notification: NotificationSettings::default(),
            tile_grid_size: px(8.),
            tile_shadow: true,
            tile_radius: px(0.),
            list: ListSettings::default(),
            colors: *colors,
            tokens: ThemeTokens::from(colors),
            light_theme: Rc::new(ThemeConfig::default()),
            dark_theme: Rc::new(ThemeConfig::default()),
            highlight_theme: HighlightTheme::default_light(),
            sheet: SheetSettings::default(),
            motion: MotionTokens::default(),
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    #[inline(always)]
    pub fn is_dark(&self) -> bool {
        matches!(self, Self::Dark)
    }

    /// Return lower_case theme name: `light`, `dark`.
    pub fn name(&self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }
}

impl From<WindowAppearance> for ThemeMode {
    fn from(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::Dark,
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::Light,
        }
    }
}

#[cfg(test)]
mod base_theme_projection_tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn base_theme_tracks_initialization_and_mode_changes(cx: &mut TestAppContext) {
        cx.update(|cx| {
            init(cx);
            assert_styled_projection(cx);

            Theme::change(ThemeMode::Dark, None, cx);
            assert_styled_projection(cx);

            Theme::set_scrollbar_mode(ScrollbarMode::Always, cx);
            assert_eq!(Theme::global(cx).scrollbar_mode, ScrollbarMode::Always);
            assert_eq!(
                gpui_base::Theme::global(cx).scrollbar.mode(),
                gpui_base::ScrollbarMode::Always
            );
            assert_styled_projection(cx);
        });
    }

    #[gpui::test]
    fn scrollbar_motion_is_owned_here_and_projected_onto_base(cx: &mut TestAppContext) {
        cx.update(|cx| {
            init(cx);

            // Base itself ships none of this timing.
            let bare = gpui_base::ScrollbarMotion::default();
            assert_eq!(bare.enter(), Duration::ZERO);
            assert_eq!(bare.exit(), Duration::ZERO);
            assert_eq!(bare.expand(), Duration::ZERO);

            Theme::set_scrollbar_mode(ScrollbarMode::Scrolling, cx);
            let motion = gpui_base::Theme::global(cx).scrollbar.motion();
            assert_eq!(motion.idle(), SCROLLBAR_IDLE);
            assert_eq!(motion.enter(), SCROLLBAR_ENTER);
            assert_eq!(motion.exit(), SCROLLBAR_EXIT);
            assert_eq!(motion.expand(), SCROLLBAR_EXPAND);
            assert_eq!(
                motion.entrance(),
                gpui_base::ScrollbarEntrance::Fade,
                "scroll-revealed scrollbars fade in without sliding"
            );

            Theme::set_scrollbar_mode(ScrollbarMode::Hover, cx);
            let motion = gpui_base::Theme::global(cx).scrollbar.motion();
            assert_eq!(motion.entrance(), gpui_base::ScrollbarEntrance::Fade);
            assert_eq!(
                motion.thumb_hover_entrance(),
                gpui_base::ScrollbarEntrance::SlideAndFade
            );
        });
    }

    #[test]
    fn default_motion_tokens_form_a_coherent_semantic_scale() {
        let theme = Theme::default();
        let motion = theme.motion_tokens();

        assert_eq!(motion.duration_instant, Duration::ZERO);
        assert!(motion.duration_fast < motion.duration_normal);
        assert!(motion.duration_normal < motion.duration_slow);
        assert!(motion.distance_short.0 < motion.distance_medium.0);
        assert_eq!(motion.easing_enter.sample(0.0), 0.0);
        assert_eq!(motion.easing_enter.sample(1.0), 1.0);
    }

    fn assert_styled_projection(cx: &App) {
        let theme = Theme::global(cx);
        let base = gpui_base::Theme::global(cx);

        assert_eq!(base.tokens, theme.semantic_tokens());
        assert_eq!(base.scrollbar.mode(), theme.scrollbar_mode);
        assert_eq!(
            base.scrollbar.motion(),
            scrollbar_motion(theme.scrollbar_mode)
        );
        assert_eq!(base.resizable.handle, Some(theme.border));
        assert_eq!(base.resizable.active_handle, Some(theme.drag_border));
    }

    #[gpui::test]
    fn default_component_palettes_match_base_light_and_dark_tokens(cx: &mut gpui::TestAppContext) {
        fn assert_close(left: ColorTokens, right: ColorTokens) {
            macro_rules! color {
                ($field:ident) => {
                    assert!(
                        (left.$field.h - right.$field.h).abs() < 1e-6
                            && (left.$field.s - right.$field.s).abs() < 1e-6
                            && (left.$field.l - right.$field.l).abs() < 1e-6
                            && (left.$field.a - right.$field.a).abs() < 1e-6,
                        "{} differs: {:?} != {:?}",
                        stringify!($field),
                        left.$field,
                        right.$field
                    );
                };
            }
            color!(background);
            color!(foreground);
            color!(surface);
            color!(surface_foreground);
            color!(primary);
            color!(primary_foreground);
            color!(secondary);
            color!(secondary_foreground);
            color!(muted);
            color!(muted_foreground);
            color!(accent);
            color!(accent_foreground);
            color!(destructive);
            color!(destructive_foreground);
            color!(border);
            color!(input);
            color!(ring);
            color!(selection);
        }

        cx.update(crate::init);
        cx.update(|cx| {
            assert_close(Theme::global(cx).color_tokens(), ColorTokens::light());
        });

        cx.update(|cx| Theme::change(ThemeMode::Dark, None, cx));
        cx.update(|cx| {
            assert_close(Theme::global(cx).color_tokens(), ColorTokens::dark());
        });
    }
}
