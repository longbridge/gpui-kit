//! Text and dropdown-button surfaces.
//!
//! What this adapter does *not* register, and why, is recorded once in
//! `component-inventory.json`, which a test checks against the public exports.
//! A prose list beside the source would be a second answer to the same
//! question, and the one nothing checks.

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    text::register(registry)?;
    dropdown_button::register(registry)?;
    Ok(())
}

mod dropdown_button;
mod text;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_only_the_two_closed_renderable_surfaces() {
        let mut registry = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        assert_eq!(
            frozen
                .descriptors()
                .map(|descriptor| descriptor.name())
                .collect::<Vec<_>>(),
            ["Text", "DropdownButton"]
        );
        assert!(frozen.descriptors().all(|descriptor| {
            descriptor.documentation().is_some()
                && descriptor
                    .methods()
                    .iter()
                    .all(|method| method.documentation().is_some())
        }));
    }
}
