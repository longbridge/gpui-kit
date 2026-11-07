//! The shipped `gpui-shell` command.
//!
//! The command itself lives in [`gpui_shell::host`] so that an embedding
//! binary can run exactly the same parsing, capabilities, assets, theming,
//! checking, and diagnostics under its own name and component catalog. This
//! file is only the entry point: with the `components` feature it carries the
//! `gpui-component` catalog, without it the empty catalog of the bare runtime.

#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;
#[cfg(feature = "gpui-fast")]
extern crate gpui_fast_platform as gpui_platform;

fn main() {
    #[cfg(feature = "components")]
    let components = match gpui_shell::components::components() {
        Ok(components) => components,
        Err(error) => {
            eprintln!("gpui-shell: {error:#}");
            std::process::exit(1);
        }
    };
    #[cfg(not(feature = "components"))]
    let components = gpui_shell::FrozenComponentRegistry::default();

    gpui_shell::host::main_with_components(components);
}
