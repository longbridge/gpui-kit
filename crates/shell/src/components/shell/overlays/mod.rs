//! Registrations for overlay components that can be expressed as elements.
//!
//! Popover and HoverCard consume repeatable slot factories because their
//! native content builders run after the registered materialization request.
//! DropdownMenu is intentionally narrower: closed label/callback item specs
//! build the real native popup menu without exposing a delegate or menu IR.
//!
//! Dialog, AlertDialog, and Sheet remain absent. Their only public native APIs
//! are command-style `WindowExt::open_*` mutations; invoking those while a
//! component is materialized would repeat the effect on every render. They
//! require a keyed event/window-effect seam, not merely scoped render
//! authority. Tooltip also remains absent: its public `Tooltip` builds an
//! `AnyView`, while the trait that attaches managed hover lifecycle to a
//! trigger is crate-private, so an adapter cannot compose a real trigger.

use gpui_shell::{ComponentRegistry, RegistryError};

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    hover_card::register(registry)?;
    popover::register(registry)?;
    dropdown_menu::register(registry)
}

mod dropdown_menu;
mod hover_card;
mod popover;
