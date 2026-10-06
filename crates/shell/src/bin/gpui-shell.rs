//! The shipped `gpui-shell` command.
//!
//! The command itself lives in [`gpui_shell::host`] so that an embedding
//! binary can run exactly the same parsing, capabilities, assets, theming,
//! checking, and diagnostics under its own name and component catalog. This
//! file is only the entry point, with the empty catalog the bare runtime ships.

#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;
#[cfg(feature = "gpui-fast")]
extern crate gpui_platform_fast as gpui_platform;

fn main() {
    gpui_shell::host::main_with_components(gpui_shell::FrozenComponentRegistry::default());
}
