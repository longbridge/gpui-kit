//! Typed Tree binding plus explicit delegate-heavy collection deferrals.
//!
//! List/DataTable need mutable generic delegates and lazy row/cell element
//! renderers. Select/Combobox/SearchableList additionally need value lookup,
//! async search and retained selection subscriptions. VirtualList needs an
//! owning `Entity<V: Render>`, visible-range callbacks returning elements, and
//! a measurement budget. None of those capabilities exists in the shell yet,
//! so only the native Tree surface that fits typed children is registered.

pub(super) use super::support::{bool_method, require_child};

pub(super) use super::typed_child::{Carrier, take};

mod tree;
use gpui_shell::{ComponentRegistry, RegistryError};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use tree::test_probe;
pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    tree::register(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_is_only_honest_tree_surface() {
        let mut r = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut r).unwrap();
        assert_eq!(
            r.freeze()
                .unwrap()
                .descriptors()
                .map(|d| d.name())
                .collect::<Vec<_>>(),
            ["TreeItem", "Tree"]
        );
    }
}
