mod clear_button;
mod content_type;
mod input;
mod number_input;
mod otp_input;
mod overlay;
pub(crate) mod popovers;
mod search;

pub(crate) use clear_button::*;
pub use content_type::*;
#[cfg(not(feature = "tree-sitter"))]
pub struct Tree;
/// The shared editing engine. Internal to the framework: components reach it
/// through the concrete state of their control, never across the public API.
pub(crate) use gpui_base::input::InputBaseState;
pub use gpui_base::input::{
    Backspace, BufferPoint, CodeActionItem, CodeActionProvider, CompletionMenuOptions,
    CompletionProvider, Copy, Cut, DefinitionProvider, Delete, DeleteToBeginningOfLine,
    DeleteToEndOfLine, DeleteToNextWordEnd, DeleteToPreviousWordStart, DisplayMap, DisplayPoint,
    DocumentColorProvider, DocumentRangeSemanticTokensProvider, EditorState, Enter, Escape,
    FoldRange, GoToDefinition, HighlightStyleResolver, HoverPopoverState, HoverProvider, Indent,
    IndentInline, InputEdit, InputEvent, InputHighlighter, InputHighlighterFactory, InputState,
    Lsp, MaskPattern, MoveDown, MoveEnd, MoveHome, MoveLeft, MovePageDown, MovePageUp, MoveRight,
    MoveToEnd, MoveToEndOfLine, MoveToNextWord, MoveToPreviousWord, MoveToStart, MoveToStartOfLine,
    MoveUp, Outdent, OutdentInline, Paste, Point, Redo, Replace, Rope, RopeExt, RopeLines, Search,
    SelectAll, SelectToEnd, SelectToEndOfLine, SelectToNextWordEnd, SelectToPreviousWordStart,
    SelectToStart, SelectToStartOfLine, Selection, ShowCharacterPalette, ShowDocumentHandler,
    TabSize, TextDecoration, TextDecorationCollection, TextareaState, ToggleCodeActions, Undo,
    WrappingIndent,
};
pub use gpui_base::input::{EditorMode, InputMode, InputModeKind, TextareaMode};
#[doc(hidden)]
mod editor;
mod state;
mod textarea;
pub use editor::Editor;
pub use input::*;
pub use lsp_types::Position;
pub use number_input::{NumberInput, NumberInputEvent, NumberStep, StepAction};
pub use otp_input::*;
pub use state::AnyInputState;
pub use textarea::Textarea;
