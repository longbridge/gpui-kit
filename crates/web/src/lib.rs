#![cfg(target_family = "wasm")]

//! Shared backend selection for GPUI Kit's gpui_web interfaces.
//! `gpui-fast` selects GPUI Fast for every consumer of this facade.

#[cfg(feature = "gpui-fast")]
pub use fast::*;
#[cfg(not(feature = "gpui-fast"))]
pub use gpui_web_upstream::*;
