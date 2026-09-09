use gpui::App;
use std::ops::Deref;

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
pub mod dock;
pub mod form;
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
pub mod radio;
pub mod rating;
/// Backwards-compatible resizable component paths.
pub mod resizable {
    pub use super::{
        ResizablePanel, ResizablePanelEvent, ResizablePanelGroup, ResizableState, h_resizable,
        resizable_panel, v_resizable,
    };
}
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
pub mod spinner;
pub mod status_bar;
pub mod stepper;
pub mod switch;
pub mod tab;
pub mod table;
pub mod tag;
pub mod text;
pub mod theme;
pub mod tooltip;
pub mod tree;

pub use crate::Disableable;
pub use element_ext::*;
pub use global_state::GlobalState;
pub use gpui_base::animation;
pub(crate) use gpui_base::measurement_enabled as measure_enable;
#[doc(hidden)]
pub(crate) use gpui_base::resize_handle;
pub use gpui_base::{
    AxisExt, Edges, FocusTrapElement, InteractiveElementExt, LengthExt, Measure, OngoingScrollExt,
    Placement, Side, measure, measure_if,
};
pub use gpui_base::{
    ResizablePanel, ResizablePanelEvent, ResizablePanelGroup, ResizableState, h_resizable,
    resizable_panel, v_resizable,
};
pub use gpui_component_macros::icon_named;
pub use icon::*;
pub use index_path::IndexPath;
pub use input::{Rope, RopeExt, RopeLines};
#[cfg(any(feature = "inspector", debug_assertions))]
pub use inspector::*;
pub use root::Root;
pub use styled::*;
pub use theme::*;
pub use time::{calendar, date_picker};
pub use title_bar::*;
pub use virtual_list::{VirtualList, VirtualListScrollHandle, h_virtual_list, v_virtual_list};
pub use window_border::{WindowBorder, window_border, window_paddings};
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
    date_picker::init(cx);
    dock::init(cx);
    sheet::init(cx);
    list::init(cx);
    command::init(cx);
    carousel::init(cx);
    notification::init(cx);
    popover::init(cx);
    menu::init(cx);
    table::init(cx);
    tooltip::init(cx);
}

#[inline]
pub fn locale() -> impl Deref<Target = str> {
    rust_i18n::locale()
}

#[inline]
pub fn set_locale(locale: &str) {
    rust_i18n::set_locale(locale)
}
