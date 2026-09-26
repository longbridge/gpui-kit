use crate::input::EditorMode;
use anyhow::Result;
use gpui::{App, Context, EntityInputHandler, Pixels, Task, Window, px};
use lsp_types::{
    CompletionContext, CompletionItem, CompletionResponse, InlineCompletionContext,
    InlineCompletionItem, InlineCompletionResponse, InlineCompletionTriggerKind,
    request::Completion,
};
use ropey::Rope;
use std::{cell::RefCell, ops::Range, rc::Rc, time::Duration};

use crate::input::InputBaseState;

/// Default debounce duration for inline completions.
const DEFAULT_INLINE_COMPLETION_DEBOUNCE: Duration = Duration::from_millis(300);

/// Display options for the LSP completion popover.
///
/// Accessed through [`super::Lsp::completion_menu`] so embedders can tweak the
/// popover without growing the [`InputBaseState`] API.
#[derive(Debug, Clone, Copy)]
pub struct CompletionMenuOptions {
    /// Maximum width of the popover.
    ///
    /// Defaults to 320 px, which is fine for most identifiers but can
    /// truncate longer labels. Widen this when hosting an editor that
    /// surfaces long completion labels.
    pub max_width: Pixels,
}

impl Default for CompletionMenuOptions {
    fn default() -> Self {
        Self {
            max_width: px(320.),
        }
    }
}

/// A trait for providing code completions based on the current input state and context.
pub trait CompletionProvider {
    /// Fetches completions based on the given byte offset.
    ///
    /// - The `offset` is in bytes of current cursor.
    ///
    /// textDocument/completion
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_completion
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        trigger: CompletionContext,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<CompletionResponse>>;

    /// Fetches an inline completion suggestion for the given position.
    ///
    /// This is called after a debounce period when the user stops typing.
    /// The provider can analyze the text and cursor position to determine
    /// what inline completion suggestion to show.
    ///
    ///
    /// # Arguments
    /// * `rope` - The current text content
    /// * `offset` - The cursor position in bytes
    ///
    /// textDocument/inlineCompletion
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#textDocument_inlineCompletion
    fn inline_completion(
        &self,
        _rope: &Rope,
        _offset: usize,
        _trigger: InlineCompletionContext,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<InlineCompletionResponse>> {
        Task::ready(Ok(InlineCompletionResponse::Array(vec![])))
    }

    /// Returns the debounce duration for inline completions.
    ///
    /// Default: 300ms
    #[inline]
    fn inline_completion_debounce(&self) -> Duration {
        DEFAULT_INLINE_COMPLETION_DEBOUNCE
    }

    fn resolve_completions(
        &self,
        _completion_indices: Vec<usize>,
        _completions: Rc<RefCell<Box<[Completion]>>>,
        _: &mut App,
    ) -> Task<Result<bool>> {
        Task::ready(Ok(false))
    }

    /// Determines if the completion should be triggered based on the given byte offset.
    ///
    /// This is called on the main thread.
    fn is_completion_trigger(&self, offset: usize, new_text: &str, cx: &mut App) -> bool;
}

pub(crate) struct InlineCompletion {
    /// Completion item to display as an inline completion suggestion
    pub(crate) item: Option<InlineCompletionItem>,
    /// Task for debouncing inline completion requests
    pub(crate) task: Task<Result<InlineCompletionResponse>>,
}

impl Default for InlineCompletion {
    fn default() -> Self {
        Self {
            item: None,
            task: Task::ready(Ok(InlineCompletionResponse::Array(vec![]))),
        }
    }
}

impl InputBaseState<EditorMode> {
    pub(crate) fn handle_completion_trigger(
        &mut self,
        range: &Range<usize>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.completion_inserting {
            return;
        }

        let Some(provider) = self.extras.lsp.completion_provider.clone() else {
            return;
        };

        // Always schedule inline completion (debounced).
        // It will check if menu is open before showing the suggestion.
        self.schedule_inline_completion(window, cx);

        let start = range.end;
        let new_offset = self.cursor();

        if !provider.is_completion_trigger(start, new_text, cx) {
            return;
        }

        let start_offset = self
            .extras
            .context_menu_content
            .completion
            .trigger_start_offset
            .unwrap_or(start);
        if new_offset < start_offset {
            return;
        }

        let query = self
            .text_for_range(
                self.range_to_utf16(&(start_offset..new_offset)),
                &mut None,
                window,
                cx,
            )
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        self.extras
            .context_menu_content
            .completion
            .trigger_start_offset = Some(start_offset);
        self.extras
            .context_menu_content
            .completion
            .query
            .clone_from(&query);

        let completion_context = CompletionContext {
            trigger_kind: lsp_types::CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(query),
        };

        let provider_responses =
            provider.completions(&self.text, new_offset, completion_context, window, cx);
        self.extras.context_menu_task = cx.spawn_in(window, async move |editor, cx| {
            let mut completions: Vec<CompletionItem> = vec![];
            if let Some(provider_responses) = provider_responses.await.ok() {
                match provider_responses {
                    CompletionResponse::Array(items) => completions.extend(items),
                    CompletionResponse::List(list) => completions.extend(list.items),
                }
            }

            if completions.is_empty() {
                editor.update(cx, |editor, cx| {
                    editor.extras.context_menu_content.completion.open = false;
                    editor.extras.context_menu_content.completion.items.clear();
                    editor.extras.context_menu_content.completion.bump();
                    cx.notify();
                })?;
                return Ok(());
            }

            editor
                .update_in(cx, |editor, window, cx| {
                    if !editor.focus_handle.is_focused(window) {
                        return;
                    }

                    editor.extras.context_menu_content.completion.items = completions;
                    editor.extras.context_menu_content.completion.open = !editor
                        .extras
                        .context_menu_content
                        .completion
                        .items
                        .is_empty();
                    editor.extras.context_menu_content.completion.bump();

                    cx.notify();
                })
                .ok();

            Ok(())
        });
    }

    pub(crate) fn hide_context_menu(&mut self, cx: &mut Context<Self>) {
        self.extras.context_menu_content.completion.open = false;
        self.extras.context_menu_content.code_action.open = false;
        self.extras.context_menu_task = Task::ready(Ok(()));
        cx.notify();
    }

    pub(crate) fn is_context_menu_open(&self, _cx: &gpui::App) -> bool {
        self.extras.context_menu_content.completion.open
            || self.extras.context_menu_content.code_action.open
    }

    pub(crate) fn handle_action_for_context_menu(
        &mut self,
        action: Box<dyn gpui::Action>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let closes_overlay =
            crate::input::Enter::is_primary(&*action) || action.partial_eq(&crate::input::Escape);
        let kind = if self.extras.context_menu_content.completion.open {
            Some(super::InputOverlayKind::Completion)
        } else if self.extras.context_menu_content.code_action.open {
            Some(super::InputOverlayKind::CodeAction)
        } else {
            None
        };
        let Some((kind, handler)) = kind.zip(self.overlay_action_handler.clone()) else {
            return false;
        };
        let handled = handler(kind, action, window, cx);
        if handled && closes_overlay {
            self.hide_context_menu(cx);
        }
        handled
    }

    /// Schedule an inline completion request after debouncing.
    pub(crate) fn schedule_inline_completion(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Clear any existing inline completion on text change
        self.clear_inline_completion(cx);

        let Some(provider) = self.extras.lsp.completion_provider.clone() else {
            return;
        };

        let offset = self.cursor();
        let text = self.text.clone();
        let debounce = provider.inline_completion_debounce();
        let background_executor = cx.background_executor().clone();

        self.extras.inline_completion.task = cx.spawn_in(window, async move |editor, cx| {
            // Debounce: wait before fetching to avoid unnecessary requests while typing
            background_executor.timer(debounce).await;

            // Now fetch the inline completion after the debounce period
            let task = editor.update_in(cx, |editor, window, cx| {
                // Check if cursor has moved during debounce
                if editor.cursor() != offset {
                    return None;
                }

                // Don't fetch if completion menu is open
                if editor.is_context_menu_open(cx) {
                    return None;
                }

                let trigger = InlineCompletionContext {
                    trigger_kind: InlineCompletionTriggerKind::Automatic,
                    selected_completion_info: None,
                };

                Some(provider.inline_completion(&text, offset, trigger, window, cx))
            })?;

            let Some(task) = task else {
                return Ok(InlineCompletionResponse::Array(vec![]));
            };

            let response = task.await?;

            editor.update_in(cx, |editor, _window, cx| {
                // Only apply if cursor still hasn't moved
                if editor.cursor() != offset {
                    return;
                }

                // Don't show if completion menu opened while we were fetching
                if editor.is_context_menu_open(cx) {
                    return;
                }

                if let Some(item) = match response.clone() {
                    InlineCompletionResponse::Array(items) => items.into_iter().next(),
                    InlineCompletionResponse::List(comp_list) => comp_list.items.into_iter().next(),
                } {
                    editor.extras.inline_completion.item = Some(item);
                    cx.notify();
                }
            })?;

            Ok(response)
        });
    }

    /// Check if an inline completion suggestion is currently displayed.
    #[inline]
    pub(crate) fn has_inline_completion(&self) -> bool {
        self.extras.inline_completion.item.is_some()
    }

    /// Clear the inline completion suggestion.
    pub(crate) fn clear_inline_completion(&mut self, cx: &mut Context<Self>) {
        self.extras.inline_completion = InlineCompletion::default();
        cx.notify();
    }

    /// Accept the inline completion, inserting it at the cursor position.
    /// Returns true if a completion was accepted, false if there was none.
    pub(crate) fn accept_inline_completion(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(completion_item) = self.extras.inline_completion.item.take() else {
            return false;
        };

        let cursor = self.cursor();
        let range_utf16 = self.range_to_utf16(&(cursor..cursor));
        let completion_text = completion_item.insert_text;
        self.replace_text_in_range_silent(Some(range_utf16), &completion_text, window, cx);
        true
    }
}
