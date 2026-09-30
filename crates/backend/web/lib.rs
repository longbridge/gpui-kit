//! GPUI's `gpui_web`, re-exported from the implementation `gpui-kit-backend` picks.

#[cfg(target_family = "wasm")]
pub use gpui_kit_backend::gpui_web::*;
