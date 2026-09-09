use gpui_kit::component::{
    ActiveTheme as _, IconName, Theme, ThemeConfig, ThemeMode, ThemeRegistry,
    command::{CommandEntry, CommandGroup, CommandItem},
    scroll::ScrollbarMode,
};
use gpui_kit::*;
use serde::{Deserialize, Serialize};

use crate::AppState;

#[cfg(target_family = "wasm")]
use crate::embedded_themes;

const STATE_FILE: &str = "target/state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct State {
    theme: SharedString,
    #[serde(default)]
    radius: Option<f32>,
    #[serde(alias = "scrollbar_show")]
    scrollbar_mode: Option<ScrollbarMode>,
    #[serde(default)]
    show_fps_monitor: Option<bool>,
}

fn apply_theme_config(theme_config: std::rc::Rc<ThemeConfig>, cx: &mut App) {
    let mode = theme_config.mode;
    Theme::global_mut(cx).apply_config(&theme_config);
    Theme::change(mode, None, cx);
}

impl Default for State {
    fn default() -> Self {
        Self {
            theme: "Default Light".into(),
            radius: None,
            scrollbar_mode: None,
            show_fps_monitor: None,
        }
    }
}

#[cfg(not(target_family = "wasm"))]
fn save_state(cx: &mut App) {
    if AppState::global(cx).previewing_theme {
        return;
    }

    let state = State {
        theme: cx.theme().theme_name().clone(),
        radius: Some(cx.theme().radius.as_f32()),
        scrollbar_mode: Some(cx.theme().scrollbar_mode),
        show_fps_monitor: Some(AppState::global(cx).show_fps_monitor),
    };

    if let Ok(json) = serde_json::to_string_pretty(&state) {
        // Ignore write errors - if STATE_FILE doesn't exist or can't be written, do nothing
        let _ = std::fs::write(STATE_FILE, json);
    }
}

pub(crate) fn begin_theme_preview(cx: &mut App) {
    AppState::global_mut(cx).previewing_theme = true;
}

pub(crate) fn finish_theme_preview(cx: &mut App) {
    AppState::global_mut(cx).previewing_theme = false;
}

fn apply_persisted_radius(state: &State, cx: &mut App) {
    let Some(radius) = state.radius else {
        return;
    };

    let theme = Theme::global_mut(cx);
    theme.radius = gpui_kit::px(radius);
    theme.radius_lg = if radius > 0. {
        gpui_kit::px(radius + 2.)
    } else {
        gpui_kit::px(0.)
    };
    Theme::sync_base(cx);
}

pub fn init(cx: &mut App) {
    #[cfg(target_family = "wasm")]
    {
        tracing::info!("Loading embedded themes for WASM...");
        let embedded = embedded_themes::embedded_themes();
        let registry = ThemeRegistry::global_mut(cx);

        for (name, content) in embedded {
            if let Err(e) = registry.load_themes_from_str(content) {
                tracing::error!("Failed to load embedded theme {}: {}", name, e);
            } else {
                tracing::info!("Loaded embedded theme: {}", name);
            }
        }
    }

    let state = if cfg!(not(target_family = "wasm")) {
        let json = std::fs::read_to_string(STATE_FILE).unwrap_or(String::default());
        serde_json::from_str::<State>(&json).unwrap_or_default()
    } else {
        State::default()
    };

    #[cfg(not(target_family = "wasm"))]
    let watched_state = state.clone();
    #[cfg(not(target_family = "wasm"))]
    if let Err(err) =
        ThemeRegistry::watch_dir(std::path::PathBuf::from("./themes"), cx, move |cx| {
            if let Some(theme) = ThemeRegistry::global(cx)
                .themes()
                .get(&watched_state.theme)
                .cloned()
            {
                apply_theme_config(theme, cx);
                apply_persisted_radius(&watched_state, cx);
            }
        })
    {
        tracing::error!("Failed to watch themes directory: {}", err);
    }

    if let Some(scrollbar_mode) = state.scrollbar_mode {
        Theme::set_scrollbar_mode(scrollbar_mode, cx);
    }
    apply_persisted_radius(&state, cx);
    if let Some(show_fps_monitor) = state.show_fps_monitor {
        AppState::global_mut(cx).show_fps_monitor = show_fps_monitor;
    }
    cx.refresh_windows();

    // Both globals carry persisted settings, so either changing writes the file.
    #[cfg(not(target_family = "wasm"))]
    {
        cx.observe_global::<Theme>(save_state).detach();
        cx.observe_global::<AppState>(save_state).detach();
    }

    cx.on_action(|switch: &SwitchTheme, cx| {
        let theme_name = switch.0.clone();
        if let Some(theme_config) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
            apply_theme_config(theme_config, cx);
        }
        cx.refresh_windows();
    });
    cx.on_action(|switch: &SwitchThemeMode, cx| {
        let mode = switch.0;
        Theme::change(mode, None, cx);
        cx.refresh_windows();
    });
}

#[derive(Action, Clone, PartialEq)]
#[action(namespace = themes, no_json)]
pub(crate) struct SwitchTheme(pub(crate) SharedString);

#[derive(Action, Clone, PartialEq)]
#[action(namespace = themes, no_json)]
pub(crate) struct SwitchThemeMode(pub(crate) ThemeMode);

actions!(
    themes,
    [
        /// Opens the Command palette to pick a theme.
        SelectTheme
    ]
);

/// Apply the named theme without going through [`SwitchTheme`].
///
/// The palette calls this as the highlight moves, so the theme under the cursor
/// is previewed on the whole window; cancelling puts the previous one back.
pub(crate) fn apply_theme(name: &SharedString, cx: &mut App) {
    let Some(theme_config) = ThemeRegistry::global(cx).themes().get(name).cloned() else {
        return;
    };

    apply_theme_config(theme_config, cx);
    cx.refresh_windows();
}

/// The palette entries for the [`SelectTheme`] Command palette: every
/// registered theme, grouped by mode, with the active one checked.
pub(crate) fn theme_entries(cx: &App) -> Vec<CommandEntry> {
    let active_name = cx.theme().theme_name().clone();
    let themes = ThemeRegistry::global(cx).sorted_themes();

    [(ThemeMode::Light, "Light"), (ThemeMode::Dark, "Dark")]
        .into_iter()
        .map(|(mode, heading)| {
            CommandGroup::new()
                .label(heading)
                .items(
                    themes
                        .iter()
                        .filter(|theme| theme.mode == mode)
                        .map(|theme| theme_item(theme, &active_name)),
                )
                .into()
        })
        .collect()
}

pub(crate) fn theme_name_at(
    index: gpui_kit::component::IndexPath,
    cx: &App,
) -> Option<SharedString> {
    let mode = [ThemeMode::Light, ThemeMode::Dark].get(index.section)?;
    ThemeRegistry::global(cx)
        .sorted_themes()
        .into_iter()
        .filter(|theme| theme.mode == *mode)
        .nth(index.row)
        .map(|theme| theme.name.clone())
}

fn theme_item(theme: &ThemeConfig, active_name: &SharedString) -> CommandItem {
    let name = theme.name.clone();

    CommandItem::new()
        .label(name.clone())
        .icon(IconName::Palette)
        .checked(&name == active_name)
        .keywords([theme.mode.name()])
        .action(Box::new(SwitchTheme(name)))
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    // `super::*` also carries GPUI's `test` attribute; keep the built-in one for `#[test]`.
    use core::prelude::v1::test;
    use gpui_kit::component::ThemeConfig;
    use std::rc::Rc;

    #[gpui_kit::test]
    fn applying_custom_theme_updates_base_component_colors(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let config = Rc::new(
            serde_json::from_value::<ThemeConfig>(serde_json::json!({
                "name": "Custom Dark",
                "mode": "dark",
                "colors": {
                    "popover.background": "#102030",
                    "popover.foreground": "#f0e0d0",
                    "border": "#405060",
                    "drag_border": "#708090",
                    "ring": "#a0b0c0"
                }
            }))
            .unwrap(),
        );

        cx.update(|cx| apply_theme_config(config, cx));

        cx.read(|cx| {
            let theme = cx.theme();
            let base = gpui_kit::base::Theme::global(cx);
            assert_eq!(base.tokens.colors.surface, theme.popover);
            assert_eq!(
                base.tokens.colors.surface_foreground,
                theme.popover_foreground
            );
            assert_eq!(base.resizable.handle, Some(theme.border));
            assert_eq!(base.resizable.active_handle, Some(theme.drag_border));
            assert_eq!(base.tokens.colors.ring, theme.ring);
        });
    }

    #[test]
    fn state_serializes_the_selected_radius() {
        let state = State {
            theme: "Default Light".into(),
            radius: Some(4.),
            scrollbar_mode: None,
            show_fps_monitor: None,
        };

        let json = serde_json::to_value(state).unwrap();

        assert_eq!(json["radius"], 4.0);
    }

    #[gpui_kit::test]
    fn persisted_radius_overrides_the_theme_radius(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let config = Rc::new(
            serde_json::from_value::<ThemeConfig>(serde_json::json!({
                "name": "Rounded Theme",
                "mode": "light",
                "radius": 8,
                "radius.lg": 10
            }))
            .unwrap(),
        );
        let state = State {
            theme: "Rounded Theme".into(),
            radius: Some(4.),
            scrollbar_mode: None,
            show_fps_monitor: None,
        };

        cx.update(|cx| {
            apply_theme_config(config, cx);
            apply_persisted_radius(&state, cx);
        });

        cx.read(|cx| {
            assert_eq!(cx.theme().radius, px(4.));
            assert_eq!(cx.theme().radius_lg, px(6.));
        });
    }
}
