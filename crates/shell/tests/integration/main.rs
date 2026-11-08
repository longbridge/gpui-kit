//! The shell's integration tests, in one binary.
//!
//! Each file under `tests/` is a binary of its own, linked with the runtime
//! and every component -- thirty of them -- so they are modules of this one.
//! What a file keeps for itself (the counter that names its scratch
//! directories, what its probes observed) is still its own: a `static` in a
//! module is that module's, and the directories carry the file's prefix.

// Loaded once for the binary, which every file that used to load its own copy
// now `use`s: clippy refuses a file loaded as a module twice.
#[path = "../../src/components/shell/support.rs"]
mod support;
#[path = "../../src/components/shell/typed_child.rs"]
mod typed_child;

mod basic_public_host;
mod chart_host;
mod chat_host;
mod check;
mod cli;
mod collections_host;
mod command_host;
mod component_registry;
mod controls_host;
mod data_table_host;
mod delegate_collections_host;
mod delegate_combobox_host;
mod delegate_select_host;
mod empty_host;
mod inline_tokens_host;
mod input_group_host;
mod inventory;
mod layout_host;
mod lazy_overlay_host;
mod lifecycle_host;
mod media_public_host;
mod navigation_host;
mod otp_host;
mod overlay_host;
mod public_host;
mod questionnaire_host;
mod scroll_host;
mod settings_public_host;
mod state_release_host;
mod story_gallery_host;
mod structured_host;
mod window_effects_host;
