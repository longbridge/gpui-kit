//! GPUI's `gpui_platform`, re-exported from the implementation `gpui-kit-backend` picks.

#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub use gpui_kit_backend::gpui_platform::*;
