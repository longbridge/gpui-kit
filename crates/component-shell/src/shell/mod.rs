//! Component-family registrations.

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    spinner::register(registry)?;
    separator::register(registry)?;
    skeleton::register(registry)?;
    chat::register(registry)?;
    controls::register(registry)?;
    delegate_collections::register(registry)?;
    delegate_combobox::register(registry)?;
    delegate_select::register(registry)?;
    data_table::register(registry)?;
    display::register(registry)?;
    compound::register(registry)?;
    typed_compound::register(registry)?;
    lifecycle::register(registry)?;
    collections::register(registry)?;
    command::register(registry)?;
    window_effects::register(registry)?;
    overlays::register(registry)?;
    retained_forms::register(registry)?;
    layout::register(registry)?;
    media::register(registry)?;
    scroll::register(registry)?;
    settings::register(registry)?;
    structured::register(registry)?;
    navigation::register(registry)?;
    basic::register(registry)?;
    chart::register(registry)?;
    carousel::register(registry)?;
    Ok(())
}

mod support;
mod typed_child;

mod basic;
mod carousel;
mod chart;
mod chat;
mod collections;
mod command;
mod compound;
mod controls;
mod data_table;
mod delegate_collections;
mod delegate_combobox;
mod delegate_select;
mod display;
mod layout;
mod lifecycle;
mod media;
mod navigation;
mod overlays;
mod retained_forms;
mod scroll;
mod separator;
mod settings;
mod skeleton;
mod spinner;
mod structured;
mod typed_compound;
mod window_effects;
