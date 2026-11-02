//! Retained command palette bindings.
//!
//! Application menus remain deferred because they are global lifecycle state:
//! the host lacks mounted-application-scoped install/reload/removal for GPUI
//! menus. Command row/header/footer builders use owned repeatable factories;
//! native empty content remains deferred because the shell has no named
//! `empty(element)` slot route.

pub(super) use super::support::{bool_method, reject_style, require_child};

pub(super) use super::typed_child::{Carrier, take};

mod command;
mod native_menu;

#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use command::test_probe as command_probe;
use gpui_shell::{ComponentRegistry, RegistryError};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use native_menu::test_probe;

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    command::register(registry)?;
    native_menu::register(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_the_retained_command_family_only() {
        let mut registry = ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        register(&mut registry).unwrap();
        assert_eq!(
            registry
                .freeze()
                .unwrap()
                .descriptors()
                .map(|item| item.name())
                .collect::<Vec<_>>(),
            [
                "CommandItem",
                "CommandGroup",
                "CommandSeparator",
                "Command",
                "NativeMenuItem",
                "NativeMenuSeparator",
                "NativeMenuTrigger"
            ]
        );
    }
}
