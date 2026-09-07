//! GPUI Shell compatibility facade for LLRT dependencies naming `rquickjs`.
//!
//! The workspace root patches crates.io `rquickjs` to this local package.
//! Its `upstream` dependency must use exactly the same source and revision as
//! the shell's `quickjs-jit` and `quickjs-jit-runtime` dependencies so LLRT and
//! the shell share Rust types and a single VM. All APIs are re-exported without
//! runtime conversion; this application-owned facade is not published.

#![no_std]

pub use upstream::*;
