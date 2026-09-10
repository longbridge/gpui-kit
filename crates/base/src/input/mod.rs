//! Text input: the shared editing engine and the three states built on it.
//!
//! Nothing here should be `pub` unless it is reachable from outside the crate.
//! A `pub` on an item behind a private module says something the module path
//! contradicts, and it hides the real API surface from anyone reading it.
#![warn(unreachable_pub)]

use gpui::App;

/// Character used by masked editor modes.
pub(crate) const MASK_CHAR: char = '•';

#[path = "editor/auto_close.rs"]
mod auto_close;
mod base;
#[path = "base/blink_cursor.rs"]
pub(crate) mod blink_cursor;
#[path = "base/change.rs"]
mod change;
#[path = "base/cursor.rs"]
mod cursor;
#[path = "editor/decorations.rs"]
mod decorations;
#[path = "editor/diagnostics.rs"]
mod diagnostics;
#[path = "editor/display_map/mod.rs"]
mod display_map;
mod editor;
#[path = "base/element.rs"]
mod element;
#[path = "editor/highlighting.rs"]
mod highlighting;
#[path = "editor/indent.rs"]
mod indent;
mod input;
#[path = "base/kind.rs"]
mod kind;
#[path = "editor/language.rs"]
mod language;
#[path = "editor/language_config.rs"]
pub mod language_config;
#[path = "base/layout.rs"]
mod layout;
#[path = "editor/lsp/mod.rs"]
mod lsp;
#[path = "base/mask_pattern.rs"]
mod mask_pattern;
#[path = "base/mode.rs"]
mod mode;
#[path = "base/movement.rs"]
mod movement;
#[path = "base/native.rs"]
mod native;
#[path = "base/rope_ext.rs"]
mod rope_ext;
#[path = "editor/search.rs"]
mod search;
#[path = "base/selection.rs"]
mod selection;
#[path = "base/state.rs"]
mod state;
mod textarea;
#[path = "base/undo_manager.rs"]
mod undo_manager;

pub(crate) fn init(cx: &mut App) {
    state::init(cx);
}

pub use crate::number_input::{NumberInputEvent, NumberStep};
pub use base::{InputBase, InputContextMenuCapabilities, InputStyles};
pub use cursor::Selection;
pub use decorations::{TextDecoration, TextDecorationCollection};
pub use diagnostics::{
    Diagnostic, DiagnosticEntry, DiagnosticRelatedInformation, DiagnosticSet, DiagnosticSeverity,
    DiagnosticSummary, DiagnosticTag, RelatedInformation,
};
pub use display_map::{BufferPoint, DisplayMap, DisplayPoint, FoldRange, WrappingIndent};
pub use editor::{Editor, EditorState};
pub use highlighting::{
    DiagnosticColors, FoldIconRenderer, HighlightStyleResolver, InputEditorStyle, InputHighlighter,
    InputHighlighterFactory, SharedHighlightStyleResolver, SyntaxContext, SyntaxContextProvider,
};
pub use indent::TabSize;
pub use input::{Input, InputState};
pub use kind::{
    EditorExtras, EditorMode, InputExtras, InputMode, InputModeKind, MultiLineMode, TextareaMode,
};
pub(crate) use language::EditorLanguage;
pub use language::{LanguageProvider, set_language_config, set_language_provider};
pub(crate) use language_config::LanguageConfig;
pub use language_config::{AutoClosingPair, BracketPair, IndentationRules};
pub use lsp::{
    CodeActionItem, CodeActionMenuState, CodeActionProvider, CompletionMenuOptions,
    CompletionMenuState, CompletionProvider, DefinitionProvider, DocumentColorProvider,
    DocumentRangeSemanticTokensProvider, HoverPopoverState, HoverProvider, InputOverlayKind, Lsp,
    ShowDocumentHandler,
};
pub use lsp_types::Position;
pub use mask_pattern::MaskPattern;
#[cfg(target_os = "macos")]
#[doc(hidden)]
pub use native::set_text_content_type;
pub use native::{NativeMenu, NativeMenuItem};
pub use rope_ext::{InputEdit, Point, RopeExt, RopeLines};
pub use ropey::Rope;
pub use search::{SearchMatcher, SearchSession};
pub use state::*;
pub use textarea::{Textarea, TextareaState};
