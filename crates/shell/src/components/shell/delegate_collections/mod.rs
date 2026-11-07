mod list;

#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use list::test_probe;

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    list::register(registry)
}
