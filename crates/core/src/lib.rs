//! Shared backend selection for GPUI Kit's gpui interfaces.
//! `gpui-fast` selects GPUI Fast for every consumer of this facade.

#[cfg(feature = "gpui-fast")]
pub use fast::*;
#[cfg(not(feature = "gpui-fast"))]
pub use gpui_upstream::*;

// Keep the facade-aware macros so applications need only gpui-kit, including
// selective imports and #[gpui_kit::test], whichever engine is selected.
#[cfg(all(feature = "gpui-fast", any(feature = "inspector", debug_assertions)))]
pub use gpui_macros::derive_inspector_reflection;
#[cfg(feature = "gpui-fast")]
pub use gpui_macros::{
    Action, AppContext, IntoElement, Render, VisualContext, bench, border_style_methods,
    box_shadow_style_methods, cursor_style_methods, margin_style_methods, overflow_style_methods,
    padding_style_methods, position_style_methods, property_test, register_action, style_helpers,
    test, visibility_style_methods,
};
