//! Questionnaire behavior: answers, validation, navigation, focus and shortcuts.
//!
//! The state machine lives here so an application can replace the visual
//! language without reimplementing it. `gpui-component` owns the skin.

mod state;
mod types;

pub use state::*;
pub use types::*;
