//! Retained scrolling capability and its two honest render surfaces.
//!
//! `Scroll` is explicitly an adapter wrapper for `ScrollableElement` behavior;
//! that extension-only trait is not registered as a constructor. `Scrollbar`
//! materializes the real native base element and shares the same retained
//! `gpui::ScrollHandle` with `Scroll`.

mod scroll;

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    scroll::register(registry)
}

#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use scroll::test_probe;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_bounded_to_capability_and_real_surfaces() {
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
            ["ScrollbarHandle"]
        );
        assert_eq!(
            frozen
                .descriptors()
                .map(|item| item.name())
                .collect::<Vec<_>>(),
            ["Scroll", "Scrollbar"]
        );
    }
}
