//! Picks the GPUI implementation gpui-kit builds on.
//!
//! By default every GPUI crate comes from the `gpui-pre-*` snapshots on
//! crates.io. The `gpui-fast` feature builds them from
//! [gpui-fast](https://github.com/longbridge/gpui-fast) instead. gpui-kit's
//! crates never name this crate: they depend on the selectors next to it
//! (`gpui`, `gpui_platform`, ...), each of which glob re-exports one module
//! below, so the choice made here reaches every crate at once.
//!
//! Both implementations are always resolved, and with `gpui-fast` on, the
//! snapshots still build but go unused: a crate lower in the graph cannot turn
//! a feature off, so gpui-fast wins whenever anything asks for it.

pub use implementation::*;

#[cfg(not(feature = "gpui-fast"))]
mod implementation {
    pub use gpui_pre as gpui;
    pub use gpui_pre_macros as gpui_macros;
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    pub use gpui_pre_platform as gpui_platform;
    #[cfg(not(target_family = "wasm"))]
    pub use gpui_pre_reqwest_client as reqwest_client;
    pub use gpui_pre_sum_tree as sum_tree;
    #[cfg(target_family = "wasm")]
    pub use gpui_pre_web as gpui_web;
}

#[cfg(feature = "gpui-fast")]
mod implementation {
    pub use gpui_fast as gpui;
    pub use gpui_fast_macros as gpui_macros;
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    pub use gpui_fast_platform as gpui_platform;
    #[cfg(not(target_family = "wasm"))]
    pub use gpui_fast_reqwest_client as reqwest_client;
    pub use gpui_fast_sum_tree as sum_tree;
    #[cfg(target_family = "wasm")]
    pub use gpui_fast_web as gpui_web;
}
