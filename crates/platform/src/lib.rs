//! Shared backend selection for GPUI Kit's gpui_platform interfaces.
//! `gpui-fast` selects GPUI Fast for every consumer of this facade.

// The platform trait must be the same type exposed by the shared core.
pub use gpui::Platform;

#[cfg(feature = "gpui-fast")]
pub use fast::*;
#[cfg(not(feature = "gpui-fast"))]
pub use gpui_platform_upstream::*;
