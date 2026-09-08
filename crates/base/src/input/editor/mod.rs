use gpui::{App, Div, Entity, InteractiveElement as _, IntoElement, RenderOnce, Stateful, Window};

use super::{EditorMode, InputBaseState, InputModeKind};

/// State for source-code editing.
///
/// This is the shared editing engine in its code-editor kind. Languages, line
/// numbers, folding, indent guides, diagnostics, decorations, and the LSP
/// providers exist on this kind only, so an ordinary input or textarea never
/// exposes them.
pub type EditorState = InputBaseState<EditorMode>;

impl InputModeKind for EditorMode {
    const MULTI_LINE: bool = true;
    const CODE_EDITOR: bool = true;

    type Extras = super::EditorExtras;

    fn hover_definition_style(
        state: &InputBaseState<Self>,
        _cx: &App,
    ) -> Option<(std::ops::Range<usize>, gpui::HighlightStyle)> {
        state.hover_definition_style()
    }

    fn hover_definition_hitbox(
        state: &InputBaseState<Self>,
        window: &mut Window,
        _cx: &App,
    ) -> Option<gpui::Hitbox> {
        state.hover_definition_hitbox(window)
    }

    fn reset_language_features(state: &mut InputBaseState<Self>) {
        state.extras.lsp.reset();
    }

    fn reset_annotations(state: &mut InputBaseState<Self>) {
        state.extras.hover_popover = None;
        state.extras.decorations.clear();
    }

    fn editing_syntax_context(state: &InputBaseState<Self>, offset: usize) -> super::SyntaxContext {
        state.syntax_context_at(offset)
    }

    fn adjust_annotations(
        state: &mut InputBaseState<Self>,
        range: &std::ops::Range<usize>,
        new_len: usize,
    ) {
        state.extras.decorations.adjust_for_edit(range, new_len);
    }

    fn refresh_language_features(
        state: &mut InputBaseState<Self>,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        let text = state.text().clone();
        state.extras.lsp.update(&text, window, cx);
    }

    fn accept_inline_completion(
        state: &mut InputBaseState<Self>,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) -> bool {
        state.accept_inline_completion(window, cx)
    }

    fn has_inline_completion(state: &InputBaseState<Self>) -> bool {
        state.has_inline_completion()
    }

    fn on_click(
        state: &mut InputBaseState<Self>,
        event: &gpui::MouseDownEvent,
        offset: usize,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) -> bool {
        state.handle_click_hover_definition(event, offset, window, cx)
    }

    fn clear_hover_state(
        state: &mut InputBaseState<Self>,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        state.clear_hover_state(cx);
    }

    fn on_text_typed(
        state: &mut InputBaseState<Self>,
        range: &std::ops::Range<usize>,
        text: &str,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        state.handle_completion_trigger(range, text, window, cx);
        state.handle_auto_close(range, text, window, cx);
    }

    fn clear_inline_completion(
        state: &mut InputBaseState<Self>,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        state.clear_inline_completion(cx);
    }

    fn hide_context_menu(
        state: &mut InputBaseState<Self>,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        state.hide_context_menu(cx);
    }

    fn is_context_menu_open(state: &InputBaseState<Self>, cx: &App) -> bool {
        state.is_context_menu_open(cx)
    }

    fn handle_context_menu_action(
        state: &mut InputBaseState<Self>,
        action: Box<dyn gpui::Action>,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) -> bool {
        state.handle_action_for_context_menu(action, window, cx)
    }

    fn on_hover_definition(
        state: &mut InputBaseState<Self>,
        offset: usize,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        state.handle_hover_definition(offset, window, cx);
    }

    fn on_mouse_move(
        state: &mut InputBaseState<Self>,
        offset: usize,
        event: &gpui::MouseMoveEvent,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        state.handle_mouse_move(offset, event, window, cx);
    }

    fn drive_highlighter(
        highlighter: &std::rc::Rc<std::cell::RefCell<Option<Box<dyn super::InputHighlighter>>>>,
        edit: super::InputEdit,
        text: &ropey::Rope,
        folding: bool,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        let mut highlighter = highlighter.borrow_mut();
        let Some(highlighter) = highlighter.as_mut() else {
            return;
        };
        highlighter.update(Some(edit), text, folding, window, cx);
    }

    fn register_actions(
        element: Stateful<Div>,
        entity: &Entity<InputBaseState<Self>>,
        window: &mut Window,
    ) -> Stateful<Div> {
        element
            .on_action(window.listener_for(entity, InputBaseState::on_action_toggle_code_actions))
            .on_action(window.listener_for(entity, InputBaseState::on_action_go_to_definition))
    }
}

impl EditorState {
    /// The LSP providers and their cached results.
    ///
    /// This exists on the editor alone: an ordinary input or textarea has no
    /// language server, and no field to reach one through.
    pub fn lsp(&self) -> &super::Lsp {
        &self.extras.lsp
    }

    /// The LSP providers, mutably. Configure the providers through this.
    pub fn lsp_mut(&mut self) -> &mut super::Lsp {
        &mut self.extras.lsp
    }

    /// Install a syntax-context provider for editing decisions.
    ///
    /// Pairing, skip-over, and indent consult it; without one the engine
    /// uses character heuristics. `gpui-component` supplies a tree-sitter
    /// backed implementation. Explicit installs win over render-time sync.
    pub fn set_syntax_context_provider(
        &mut self,
        provider: std::rc::Rc<dyn super::SyntaxContextProvider>,
        cx: &mut gpui::Context<Self>,
    ) {
        self.extras.syntax_context_provider = Some(provider);
        self.extras.syntax_provider_customized = true;
        cx.notify();
    }

    /// Install a provider unless the app explicitly installed one.
    ///
    /// Render-time per-language sync calls this with the editor's current
    /// language. Reinstalls only on language change, preserving the
    /// provider's incremental parse cache across renders. Returns whether
    /// anything changed.
    pub fn ensure_syntax_context_provider(
        &mut self,
        language: &gpui::SharedString,
        provider: Option<std::rc::Rc<dyn super::SyntaxContextProvider>>,
    ) -> bool {
        if self.extras.syntax_provider_customized {
            return false;
        }
        if self.extras.syntax_provider_language.as_ref() == Some(language)
            && self.extras.syntax_context_provider.is_some() == provider.is_some()
        {
            return false;
        }
        self.extras.syntax_provider_language = Some(language.clone());
        self.extras.syntax_context_provider = provider;
        true
    }

    /// Syntax context at `offset`, or `Code` when no provider is installed.
    pub(crate) fn syntax_context_at(&self, offset: usize) -> super::SyntaxContext {
        self.extras
            .syntax_context_provider
            .as_ref()
            .map(|provider| provider.context_at(&self.text, offset))
            .unwrap_or(super::SyntaxContext::Code)
    }

    /// Automatically insert or skip over a closing bracket or quote.
    ///
    /// Called from `on_text_typed` after every edit. Only acts on a single
    /// freshly typed opener or closer with a collapsed cursor. Bracket policy
    /// comes from [`super::EditRules`]; this method names no characters.
    pub(crate) fn handle_auto_close(
        &mut self,
        range: &std::ops::Range<usize>,
        text: &str,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if !self.is_editable() {
            return;
        }
        let Some(rules) = self.mode.edit_rules().cloned() else {
            return;
        };
        if !rules.auto_close {
            return;
        }
        if !self.selections.is_single() || !self.active_selection().is_empty() {
            return;
        }
        // Only single characters typed over a collapsed cursor.
        if text.chars().count() != 1 || range.start != range.end {
            return;
        }
        let typed: char = text.chars().next().unwrap_or_default();
        let cursor = self.cursor();
        // Syntax context: comments are literal text (no pairing, no skip);
        // inside strings only terminator skip-over applies, never new pairs.
        // Without a provider everything is Code (previous behavior).
        let in_string = self.syntax_context_at(range.start) == super::SyntaxContext::String;
        if self.syntax_context_at(range.start) == super::SyntaxContext::Comment {
            return;
        }

        // Escape check first: a quote preceded by an odd run of backslashes
        // is escaped (e.g. `"hello\"|`), so it is a literal — neither skip
        // nor pair, just leave the typed character alone.
        if typed == '"' || typed == '\'' {
            let mut backslashes = 0;
            for c in self.text.chars_at(range.start).reversed() {
                if c != '\\' {
                    break;
                }
                backslashes += 1;
            }
            if backslashes % 2 == 1 {
                return;
            }
        }

        // Seek-based neighbor lookup: no linear prefix scan, Unicode-safe.
        let after: Option<char> = self.text.chars_at(cursor).next();

        // Skip-over: typed a closer that already follows the cursor.
        // Remove the just-typed char and move past the existing one.
        // No word guard here: a matching follower is always a closer.
        if rules.is_closer(typed) && after == Some(typed) {
            // History-ignoring: the framework's insertion alone stays as
            // the undoable unit; undo never exposes the transient duplicate.
            let typed_utf16 = self.range_to_utf16(&(range.start..range.start + typed.len_utf8()));
            self.replace_silent_ignoring_history(Some(typed_utf16), "", window, cx);
            // NOTE: byte target is correct: closers are ASCII (1 byte).
            let target = range.start + typed.len_utf8();
            self.set_selected_range(target..target, cx);
            return;
        }

        // Auto-close: typed an opener, insert the matching closer.
        // Inside strings no new pairs open (the terminator skip above
        // already handled closing).
        if in_string {
            return;
        }
        let Some(closer) = rules.matching_close(typed) else {
            return;
        };
        if closer == typed {
            // Quotes: don't open a pair inside words (e.g. contractions).
            // Look at the char before the just-typed quote.
            let before_typed: Option<char> = (range.start > 0)
                .then(|| self.text.chars_at(range.start).reversed().next())
                .flatten();
            if before_typed.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                return;
            }
        }
        self.replace_text_in_range_silent(None, &closer.to_string(), window, cx);
        self.set_selected_range(cursor..cursor, cx);
    }
}

/// An unstyled source-code editor.
#[derive(IntoElement)]
pub struct Editor {
    state: Entity<EditorState>,
}

impl Editor {
    pub fn new(state: &Entity<EditorState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for Editor {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.state
    }
}

/// What a code editor exposes to the renderer. See [`crate::input::InputExtras`].
impl crate::input::InputExtras for super::EditorExtras {
    fn decoration_layers(&self) -> Vec<&[super::TextDecoration]> {
        self.decorations.iter().collect()
    }

    fn semantic_token_styles(
        &self,
        text: &ropey::Rope,
        range: &std::ops::Range<usize>,
        resolver: &dyn crate::input::HighlightStyleResolver,
    ) -> Vec<(std::ops::Range<usize>, gpui::HighlightStyle)> {
        self.lsp.semantic_tokens_for_range(text, range, resolver)
    }

    fn document_color_swatches(
        &self,
        text: &ropey::Rope,
        range: &std::ops::Range<usize>,
    ) -> Vec<(std::ops::Range<usize>, gpui::Hsla)> {
        self.lsp.document_colors_for_range(text, range)
    }

    fn hover_symbol_range(&self) -> Option<std::ops::Range<usize>> {
        self.hover_popover
            .as_ref()
            .map(|session| session.symbol_range.clone())
    }

    fn inline_completion_item(&self) -> Option<&lsp_types::InlineCompletionItem> {
        self.inline_completion.item.as_ref()
    }

    fn context_menu_capabilities(&self) -> (bool, bool) {
        (
            self.lsp.definition_provider.is_some(),
            !self.lsp.code_action_providers.is_empty(),
        )
    }
}
