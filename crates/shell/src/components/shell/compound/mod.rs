//! Compound components that can be represented without retaining typed child state.

mod avatar;
mod collapsible;
mod common;
mod pagination;
mod progress;
mod radio;

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    avatar::register(registry)?;
    collapsible::register(registry)?;
    pagination::register(registry)?;
    progress::register(registry)?;
    radio::register(registry)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use gpui_shell::{ArgumentSchema, COMPONENT_REGISTRY_API_VERSION};

    use super::*;

    #[test]
    fn registers_only_the_honestly_materializable_compound_batch() {
        let mut registry = ComponentRegistry::new(
            COMPONENT_REGISTRY_API_VERSION,
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
            ["Avatar", "Collapsible", "Pagination", "Progress", "Radio"]
        );
        assert!(
            frozen
                .descriptors()
                .flat_map(|descriptor| descriptor.methods().iter())
                .all(|method| method.documentation().is_some())
        );
    }

    #[test]
    fn numeric_and_controlled_arguments_have_closed_schemas() {
        let mut registry = ComponentRegistry::new(
            COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let pagination = frozen
            .descriptors()
            .find(|descriptor| descriptor.name() == "Pagination")
            .unwrap();

        // By name, not by position: what this asserts is that the page number
        // is a number, and a method added anywhere in the list should not be
        // able to fail it.
        let schema_of = |descriptor: &gpui_shell::ComponentDescriptor, method: &str| {
            descriptor
                .methods()
                .iter()
                .find(|candidate| candidate.name() == method)
                .unwrap_or_else(|| panic!("`{method}` is registered"))
                .arguments()[0]
                .schema()
                .clone()
        };
        assert_eq!(
            schema_of(pagination, "current_page"),
            ArgumentSchema::Number
        );
        let radio = frozen
            .descriptors()
            .find(|descriptor| descriptor.name() == "Radio")
            .unwrap();
        assert_eq!(schema_of(radio, "checked"), ArgumentSchema::Boolean);
    }
}
