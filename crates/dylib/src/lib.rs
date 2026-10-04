//! Forces dynamic linking of the GPUI Kit engine.
//!
//! Dynamic linking causes GPUI, GPUI Base, GPUI Component, and platform backends
//! to be compiled as a shared dynamic library (`.so` / `.dylib` / `.dll`).
//! This drastically improves iterative linking times during application development.
//!
//! # Usage
//!
//! Enable the `dynlib` feature on `gpui-kit`:
//!
//! ```toml
//! [dependencies]
//! gpui-kit = { version = "0.7.0", features = ["dynlib"] }
//! ```
//!
//! Or via a dev-only feature in your `Cargo.toml`:
//!
//! ```toml
//! [features]
//! dev = ["gpui-kit/dynlib"]
//! ```
//!
//! And run:
//!
//! ```bash
//! cargo run --features dev
//! ```

// Force linking of the underlying engine crates into the shared dynamic library.
// These imports cannot be removed without preventing the dynamic library from
// exporting the required GPUI and component symbols.
#[allow(unused_imports, clippy::single_component_path_imports)]
use gpui as _;

#[allow(unused_imports, clippy::single_component_path_imports)]
use gpui_base as _;

#[cfg(feature = "component")]
#[allow(unused_imports, clippy::single_component_path_imports)]
use gpui_component as _;

#[cfg(feature = "assets")]
#[allow(unused_imports, clippy::single_component_path_imports)]
use gpui_kit_assets as _;

#[cfg(not(any(target_os = "ios", target_os = "android")))]
#[allow(unused_imports, clippy::single_component_path_imports)]
use gpui_platform as _;

// The dynamic library retains GPUI's io-surface symbols even when the
// application does not use them. Resolve them at this link boundary.
#[cfg(target_os = "macos")]
#[link(name = "IOSurface", kind = "framework")]
unsafe extern "C" {}
