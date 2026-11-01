//! Concrete lifecycle-adjacent surfaces that can be mounted as ordinary elements.

pub(super) use super::support::{bool_method, reject_style};

pub(super) use super::typed_child::{Carrier, take};

mod menu;
mod tooltip;

#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use tooltip::test_probe;

use gpui_shell::{ComponentRegistry, RegistryError};

pub(crate) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    tooltip::register(registry)?;
    menu::register(registry)
}
