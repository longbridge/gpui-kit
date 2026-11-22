#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;

use gpui::App;
use std::ops::Deref;
use std::sync::LazyLock;

mod component_traits;
mod element_ext;
pub mod global_state;
mod icon;
mod index_path;
#[cfg(any(feature = "inspector", debug_assertions))]
mod inspector;
mod root;
mod sizing;
mod styled;
mod time;
mod title_bar;
mod touch_selection;
mod virtual_list;
mod window_border;
mod window_ext;

pub(crate) mod actions {
    pub use gpui_base::actions::*;
}

pub mod accordion;
pub mod alert;
pub mod attachment;
pub mod avatar;
pub mod badge;
pub mod breadcrumb;
pub mod bubble;
pub mod button;
pub mod carousel;
pub mod chart;
pub mod checkbox;
pub mod clipboard;
pub mod collapsible;
pub mod color_picker;
pub mod combobox;
pub mod command;
pub mod description_list;
pub mod dialog;
pub mod diff;
pub mod dock;
pub mod empty;
pub mod form;
pub mod grid_layout;
pub mod group_box;
pub mod highlighter;
pub mod history;
pub mod hover_card;
pub mod input;
pub mod kbd;
pub mod label;
pub mod link;
pub mod list;
pub mod marker;
pub mod menu;
pub mod message;
pub mod message_scroller;
pub mod native_menu;
pub mod notification;
pub mod pagination;
pub mod plot;
pub mod popover;
pub mod progress;
pub mod questionnaire;
pub mod radio;
pub mod rating;
pub mod resizable;
pub mod scroll;
pub mod searchable_list;
pub mod select;
pub mod separator;
pub mod setting;
pub mod sheet;
pub mod shimmer;
pub mod sidebar;
pub mod skeleton;
pub mod slider;
pub mod speech;
pub mod spinner;
pub mod status_bar;
pub mod stepper;
pub mod switch;
pub mod tab;
pub mod table;
pub mod tag;
pub mod text;
pub mod theme;
pub mod toolbar;
pub mod tooltip;
pub mod tree;

pub use crate::Disableable;
pub use element_ext::*;
pub use global_state::GlobalState;
pub use gpui_base::Root;
pub use gpui_base::animation;
pub use gpui_base::{
    AxisExt, Edges, FocusTrapElement, InteractiveElementExt, LengthExt, Measure, OngoingScrollExt,
    Placement, Side, measure, measure_if,
};
pub use gpui_component_macros::icon_named;
pub use icon::*;
pub use index_path::IndexPath;
pub use input::{Rope, RopeExt, RopeLines};
#[cfg(any(feature = "inspector", debug_assertions))]
pub use inspector::*;
pub use resizable::{
    ResizablePanel, ResizablePanelEvent, ResizablePanelGroup, ResizableState, h_resizable,
    resizable_panel, resize_handle, resize_handle_appearance, v_resizable,
};
pub use styled::*;
pub use theme::*;
pub use time::{calendar, date_picker, time_field};
pub use title_bar::*;
pub use virtual_list::{VirtualList, VirtualListScrollHandle, h_virtual_list, v_virtual_list};
pub use window_border::{WindowBorder, window_border, window_corner_radii, window_paddings};
pub use window_ext::WindowExt;

rust_i18n::i18n!("locales", fallback = "en");

/// Initialize the components.
///
/// You must initialize the components at your application's entry point.
pub fn init(cx: &mut App) {
    theme::init(cx);
    global_state::init(cx);
    #[cfg(any(feature = "inspector", debug_assertions))]
    inspector::init(cx);
    root::init(cx);
    gpui_base::init(cx);
    input::init(cx);
    diff::init(cx);
    date_picker::init(cx);
    dock::init(cx);
    sheet::init(cx);
    list::init(cx);
    command::init(cx);
    carousel::init(cx);
    notification::init(cx);
    popover::init(cx);
    questionnaire::init(cx);
    menu::init(cx);
    table::init(cx);
    tooltip::init(cx);

    warn_if_component_icons_missing(cx);
}

/// The icon used as a canary for "is an asset source that carries the component
/// icons installed at all". `icons/check.svg` is part of the default
/// [`assets::Assets`](gpui_kit_assets) bundle and is drawn by `Checkbox`, menus
/// and more, so if it does not resolve, neither will the other component icons.
fn canary_icon_path() -> gpui::SharedString {
    use gpui_kit_assets::IconNamed as _;
    crate::IconName::Check.path()
}

/// Whether `source` can supply gpui-kit's component icons.
fn asset_source_has_component_icons(source: &dyn gpui::AssetSource) -> bool {
    matches!(source.load(&canary_icon_path()), Ok(Some(_)))
}

/// Warn, at most once per process, when no asset source that can supply the
/// component icons is installed.
///
/// The usual cause is a missing `.with_assets(gpui_kit::assets::Assets)` on the
/// application (see the `Icons` section of the crate README). Without it every
/// icon component renders as blank space and nothing is logged, which is one of
/// the hardest mistakes to diagnose; this surfaces it. It also catches an asset
/// source that embeds a subset not covering the component icons.
fn warn_if_component_icons_missing(cx: &App) {
    if asset_source_has_component_icons(cx.asset_source().as_ref()) {
        return;
    }
    static WARNED: std::sync::Once = std::sync::Once::new();
    WARNED.call_once(|| {
        tracing::warn!(
            "gpui-kit: no asset source resolves `{}`, so icon components will render \
             blank. Pass one with \
             `gpui_kit::application().with_assets(gpui_kit::assets::Assets)`, or ship your \
             own icons named per `IconName`.",
            canary_icon_path()
        );
    });
}

#[inline]
pub fn locale() -> impl Deref<Target = str> {
    rust_i18n::locale()
}

#[inline]
pub fn set_locale(locale: &str) {
    rust_i18n::set_locale(locale)
}

/// Whether measurement logging is enabled, read once per process.
///
/// Measurement is a startup debug flag, and the table queries it for every
/// rendered cell, so cache it instead of reading the environment each call.
#[inline]
pub(crate) fn measure_enable() -> bool {
    static MEASURE_ENABLED: LazyLock<bool> = LazyLock::new(gpui_base::measurement_enabled);
    *MEASURE_ENABLED
}

/// Draws a frame that re-renders every view. GPUI Fast replays retained
/// subtrees when nothing was invalidated, so a bare `window.draw` in a test
/// would skip `render`/`paint` and any side-channel recording they do.
#[cfg(test)]
pub(crate) fn test_redraw(window: &mut gpui::Window, cx: &mut gpui::App) {
    window.refresh();
    window.draw(cx).clear(cx);
}

#[cfg(test)]
mod icon_asset_warning_tests {
    use super::{asset_source_has_component_icons, canary_icon_path};
    use std::borrow::Cow;

    #[test]
    fn the_default_unit_asset_source_is_detected_as_missing_icons() {
        // `()` is GPUI's asset source when `.with_assets` is never called; it
        // resolves nothing, so icon components would render blank.
        assert!(!asset_source_has_component_icons(&()));
    }

    #[test]
    fn a_source_serving_the_canary_icon_is_detected_as_present() {
        struct Stub;
        impl gpui::AssetSource for Stub {
            fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
                Ok((path == canary_icon_path().as_ref()).then(|| Cow::Borrowed(&b"<svg/>"[..])))
            }
            fn list(&self, _path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
                Ok(Vec::new())
            }
        }
        assert!(asset_source_has_component_icons(&Stub));
    }
}
