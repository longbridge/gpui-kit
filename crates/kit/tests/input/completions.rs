//! Completion workflows through the styled editor, its real popup, and native input.
//! The popup has no TestWindowExt observation; text, focus, and provider requests
//! are the public evidence, with acceptance proving that menu actions were routed.

use std::{cell::RefCell, rc::Rc};

use gpui_kit::{
    App, AppContext, Context, Entity, Result, Task, TestAppContext, Window, WindowHandle,
    component::input::{CompletionProvider, Editor, EditorState, Rope},
    div,
    prelude::*,
    px, size,
    test::TestWindowExt,
};
use lsp_types::{
    CompletionContext, CompletionItem, CompletionResponse, CompletionTextEdit,
    CompletionTriggerKind, Position, Range, TextEdit,
};

use crate::common;

#[derive(Debug, PartialEq)]
struct CompletionRequest {
    text: String,
    offset: usize,
    trigger: CompletionContext,
}

#[derive(Default)]
struct Suggestions {
    requests: RefCell<Vec<CompletionRequest>>,
}

impl CompletionProvider for Suggestions {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        trigger: CompletionContext,
        _: &mut Window,
        _: &mut App,
    ) -> Task<Result<CompletionResponse>> {
        let text = text.to_string();
        // This fixture uses single-line ASCII identifiers; LSP character
        // positions and byte offsets therefore coincide.
        let prefix = &text[..offset];
        let items = ["print", "println", "private"]
            .into_iter()
            .filter(|label| label.starts_with(prefix))
            .map(|label| CompletionItem {
                label: label.into(),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range: Range::new(Position::new(0, 0), Position::new(0, offset as u32)),
                    new_text: label.into(),
                })),
                ..Default::default()
            })
            .collect();
        self.requests.borrow_mut().push(CompletionRequest {
            text,
            offset,
            trigger,
        });
        Task::ready(Ok(CompletionResponse::Array(items)))
    }

    fn is_completion_trigger(&self, _: usize, new_text: &str, _: &mut App) -> bool {
        !new_text.is_empty() && new_text.chars().all(|ch| ch.is_ascii_alphabetic())
    }
}

struct CompletionEditor {
    state: Entity<EditorState>,
}

impl Render for CompletionEditor {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .child(Editor::new(&self.state).size_full())
    }
}

struct Fixture {
    handle: WindowHandle<gpui_kit::base::Root>,
    state: Entity<EditorState>,
    provider: Rc<Suggestions>,
}

impl Fixture {
    fn new(cx: &mut TestAppContext) -> Self {
        cx.update(gpui_kit::init);
        let provider = Rc::new(Suggestions::default());
        let (handle, view) =
            common::open_window(cx, Some(size(px(800.), px(480.))), |window, cx| {
                cx.new(|cx| CompletionEditor {
                    state: cx.new(|cx| {
                        let mut state = EditorState::new(window, cx).language("plaintext");
                        state.lsp_mut().completion_provider = Some(provider.clone());
                        state
                    }),
                })
            });
        let state = cx.update(|cx| view.read(cx).state.clone());
        let fixture = Self {
            handle,
            state,
            provider,
        };
        cx.update_window(handle.into(), |_, window, cx| {
            window.click(("input", fixture.state.entity_id()), cx);
        })
        .unwrap();
        fixture.settle(cx);
        fixture.assert_editor("", cx);
        fixture
    }

    fn settle(&self, cx: &mut TestAppContext) {
        // Provider responses and popup acceptance update entities asynchronously.
        // Drain them outside a borrowed window, then refresh native observations.
        cx.run_until_parked();
        cx.update_window(self.handle.into(), |_, window, cx| window.render_frame(cx))
            .unwrap();
        cx.run_until_parked();
    }

    fn input(&self, text: &str, cx: &mut TestAppContext) {
        cx.update_window(self.handle.into(), |_, window, cx| window.input(text, cx))
            .unwrap();
        self.settle(cx);
    }

    fn press(&self, key: &str, cx: &mut TestAppContext) {
        cx.update_window(self.handle.into(), |_, window, cx| window.press(key, cx))
            .unwrap();
        self.settle(cx);
    }

    fn assert_editor(&self, value: &str, cx: &mut TestAppContext) {
        cx.update_window(self.handle.into(), |_, window, cx| {
            window.render_frame(cx);
            let input = window.find(("input", self.state.entity_id()));
            assert_eq!(input.value(), Some(value));
            assert_eq!(input.focused(), Some(true));
            assert_eq!(self.state.read(cx).value(), value);
        })
        .unwrap();
    }

    fn start_completion(&self, cx: &mut TestAppContext) {
        self.input("p", cx);
        self.assert_editor("p", cx);
        assert_eq!(
            *self.provider.requests.borrow(),
            vec![CompletionRequest {
                text: "p".into(),
                offset: 1,
                trigger: CompletionContext {
                    trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                    trigger_character: Some("p".into()),
                },
            }]
        );
    }
}

#[gpui_kit::test]
fn typing_opens_completion_and_enter_accepts_without_a_newline(cx: &mut TestAppContext) {
    let fixture = Fixture::new(cx);
    fixture.start_completion(cx);
    // Enter is the production menu-acceptance key. Tab is used for indentation
    // and inline completion, so it is deliberately not treated as an alias.
    fixture.press("enter", cx);
    fixture.assert_editor("print", cx);
    assert_eq!(fixture.provider.requests.borrow().len(), 1);
    fixture.input("!", cx);
    fixture.assert_editor("print!", cx);
}

#[gpui_kit::test]
fn escape_cancels_completion_and_preserves_editor_text_and_focus(cx: &mut TestAppContext) {
    let fixture = Fixture::new(cx);
    fixture.start_completion(cx);
    fixture.press("escape", cx);
    fixture.assert_editor("p", cx);
    // Enter now edits the document, proving the dismissed menu cannot accept.
    fixture.press("enter", cx);
    fixture.assert_editor("p\n", cx);
    fixture.input("!", cx);
    fixture.assert_editor("p\n!", cx);
    assert_eq!(fixture.provider.requests.borrow().len(), 1);
}

#[gpui_kit::test]
fn arrow_navigation_accepts_the_selected_completion(cx: &mut TestAppContext) {
    let fixture = Fixture::new(cx);
    fixture.start_completion(cx);
    fixture.press("down", cx);
    fixture.press("down", cx);
    fixture.press("up", cx);
    fixture.assert_editor("p", cx);
    fixture.press("enter", cx);
    fixture.assert_editor("println", cx);
    assert_eq!(fixture.provider.requests.borrow().len(), 1);
}

#[gpui_kit::test]
fn continued_typing_refreshes_provider_filtered_suggestions(cx: &mut TestAppContext) {
    let fixture = Fixture::new(cx);
    fixture.start_completion(cx);
    fixture.press("down", cx);
    fixture.input("riv", cx);
    fixture.assert_editor("priv", cx);
    {
        let requests = fixture.provider.requests.borrow();
        let request = requests.last().expect("completion requested after typing");
        assert_eq!(request.text, "priv");
        assert_eq!(request.offset, 4);
        assert_eq!(request.trigger.trigger_character.as_deref(), Some("priv"));
    }
    // Only "private" matches now; acceptance also checks that the previous
    // selection does not leave the refreshed one-item list out of bounds.
    fixture.press("enter", cx);
    fixture.assert_editor("private", cx);
}

#[gpui_kit::test]
fn accepted_completion_is_one_undo_separate_from_the_typed_prefix(cx: &mut TestAppContext) {
    let fixture = Fixture::new(cx);
    fixture.start_completion(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("print", cx);
    fixture.press("secondary-z", cx);
    fixture.assert_editor("p", cx);
    fixture.press("secondary-z", cx);
    fixture.assert_editor("", cx);
    // History replay must not issue new completion requests.
    assert_eq!(fixture.provider.requests.borrow().len(), 1);
}
