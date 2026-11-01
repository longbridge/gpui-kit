//! Retained textarea and typed resizable-layout bindings.
//!
//! `Scrollbar` is intentionally not registered: its public constructor requires
//! a concrete `ScrollbarHandle`, and the shell does not expose scroll handles as
//! retained entities. `ScrollableElement` is a Rust extension trait rather than
//! a component constructor, so registering `Scroll` would fabricate an API.

pub(super) use super::support::{Empty, bool_method};

pub(super) use super::typed_child::{Carrier, take};

mod resizable;
mod textarea;

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    textarea::register(registry)?;
    resizable::register(registry)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_only_real_constructible_layout_surfaces() {
        let mut registry = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        assert_eq!(
            frozen
                .states()
                .map(|state| state.export())
                .collect::<Vec<_>>(),
            ["TextareaState"]
        );
        assert_eq!(
            frozen
                .descriptors()
                .map(|item| item.name())
                .collect::<Vec<_>>(),
            ["Textarea", "ResizablePanel", "Resizable"]
        );
    }
}
