//! Stateless visual-control bindings for `gpui-shell`.

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    action::register(registry)?;
    display::register(registry)?;
    text::register(registry)?;
    Ok(())
}

mod action;
mod display;
mod support;
mod text;

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_shell::COMPONENT_REGISTRY_API_VERSION;

    #[test]
    fn controls_register_the_supported_public_exports() {
        let mut registry = ComponentRegistry::new(
            COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let exports = frozen
            .descriptors()
            .flat_map(|descriptor| descriptor.constructors().iter())
            .map(|constructor| constructor.export())
            .collect::<Vec<_>>();

        assert_eq!(
            exports,
            [
                "Button", "Checkbox", "Switch", "Toggle", "Badge", "Tag", "Label", "Link", "Kbd"
            ]
        );
    }

    #[test]
    fn every_control_uses_closed_argument_schemas_and_documents_its_surface() {
        let mut registry = ComponentRegistry::new(
            COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();

        for descriptor in frozen.descriptors() {
            assert!(descriptor.documentation().is_some());
            assert!(
                descriptor
                    .methods()
                    .iter()
                    .all(|method| method.documentation().is_some())
            );
        }
    }
}
