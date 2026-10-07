//! Renderable media bindings with closed resource and retained-state contracts.
//!
//! `Chart` is intentionally deferred: the native surface is a family of generic
//! chart types whose accessor closures require a typed row carrier that the shell
//! does not expose. `Plot` is a Rust paint/prepaint trait, not a constructor.

pub(super) use super::support::bool_method;

mod diff;
mod editor;
mod image;

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    image::register(registry)?;
    editor::register(registry)?;
    diff::register(registry)?;
    Ok(())
}
