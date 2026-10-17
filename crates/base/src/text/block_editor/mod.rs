//! Block-based WYSIWYG Markdown editor (spike for the design in
//! [longbridge/gpui-kit#3049](https://github.com/longbridge/gpui-kit/pull/3049);
//! see `docs/design/block-editor.md`). Behind the off-by-default
//! `block-editor` feature (`gpui-kit` forwards it to Base and Component).
//! Component provides a themed content-sized viewport over this behavior.
//!
//! - [`BlockEditorModel`]: the headless model. It has stable [`BlockId`]s,
//!   validated [`BlockPosition`]s, normalized inline mark runs, invertible
//!   steps grouped into transactions, undo coalescing, commands on nested
//!   blocks ([`EditorCommand`]), and a Markdown serializer.
//! - [`BlockEditorState`]: the entity that owns a model, focus, IME and
//!   key bindings, and emits [`BlockEditorEvent`]s.
//! - [`BlockEditor`]: the view. It renders through gpui-kit's own text
//!   renderer (a direct model-to-node projection) and culls off-screen blocks.
//!
//! ```ignore
//! let scroll = ScrollHandle::new();
//! let editor = cx.new(|cx| {
//!     let mut state = BlockEditorState::new(window, cx).with_source("# Hello");
//!     // Optional: keep the caret in view inside your scroll container.
//!     state.set_scroll_handle(scroll.clone());
//!     state
//! });
//! cx.subscribe(&editor, |_, editor, event: &BlockEditorEvent, cx| {
//!     if let BlockEditorEvent::Changed { .. } = event {
//!         let markdown = editor.read(cx).source();
//!     }
//! });
//! // in render:
//! div().id("doc").overflow_y_scroll().track_scroll(&scroll)
//!     .child(BlockEditor::new(&editor))
//! ```
//!
//! Keys: Enter, Backspace/Delete, arrows/Home/End (+Shift), Cmd/Ctrl+A, B, I,
//! E, Z, Shift+Z, C, X, V, and Shift+V (plain paste); Tab/Shift+Tab indent
//! list items. Input rules: `# `..`###### `, `- `, `* `, `+ `, `1. `.
//! The scroll handle is optional: without it, blocks outside the visible
//! area are still culled, but moving the caret never scrolls (a no-op).

mod command;
mod cull;
mod direct;
mod document;
mod editor;
mod element;
mod frame;
mod host;
mod id;
mod import;
mod inline;
mod input_rule;
mod kind;
mod marks;
mod order;
mod position;
mod projection;
mod selection;
mod serialize;
mod state;
mod step;
mod style;
pub(crate) mod table;
mod transaction;
mod undo;

#[cfg(test)]
mod tests;

pub use command::{CommandError, EditorCommand};
pub use document::{BlockRef, DocumentRef};
pub use editor::BlockEditorModel;
pub use element::BlockEditor;
pub use input_rule::InputRule;
pub use state::{BlockEditorEvent, BlockEditorState, ClipboardPolicy, PasteRequest, SubmitPolicy};
pub use style::BlockEditorStyle;
pub use table::TableEdit;

pub(crate) fn init(cx: &mut gpui::App) {
    state::init(cx);
}
pub use id::BlockId;
pub use kind::BlockKind;
pub use marks::{MarkChange, MarkKind, MarkSet, MarkState};
pub use position::{Affinity, BlockPosition, PositionError};
pub use selection::{BlockSelection, SelectionError};
pub use transaction::{EditOrigin, Transaction};

pub use host::{CapturedRange, CapturedRangeError, SourceSnapshot};

pub use super::node::ImagePolicy;
