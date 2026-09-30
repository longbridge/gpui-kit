//! GPUI's `reqwest_client`, re-exported from the implementation `gpui-kit-backend` picks.

#[cfg(not(target_family = "wasm"))]
pub use gpui_kit_backend::reqwest_client::*;
