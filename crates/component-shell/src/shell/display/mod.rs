//! Stateless display and content component registrations.

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    alert::register(registry)?;
    breadcrumb::register(registry)?;
    clipboard::register(registry)?;
    group_box::register(registry)?;
    rating::register(registry)?;
    status_bar::register(registry)?;
    toolbar::register(registry)?;
    Ok(())
}

mod alert;
mod breadcrumb;
mod clipboard;
mod common;
mod group_box;
mod rating;
mod status_bar;
mod toolbar;

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_shell::COMPONENT_REGISTRY_API_VERSION;

    #[test]
    fn registers_the_display_catalog_with_documented_callables() {
        let mut registry = ComponentRegistry::new(
            COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let descriptors = frozen.descriptors().collect::<Vec<_>>();

        assert_eq!(
            descriptors
                .iter()
                .map(|entry| entry.name())
                .collect::<Vec<_>>(),
            [
                "Alert",
                "Breadcrumb",
                "Clipboard",
                "GroupBox",
                "Rating",
                "StatusBar",
                "Toolbar"
            ]
        );
        assert!(
            descriptors
                .iter()
                .all(|entry| entry.documentation().is_some())
        );
        assert!(
            descriptors
                .iter()
                .flat_map(|entry| entry.methods())
                .all(|method| { method.documentation().is_some() })
        );
    }
}
