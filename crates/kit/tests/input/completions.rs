//! Completion workflows through the styled editor, its real popup, and native input.
//! The popup has no TestWindowExt observation; text, focus, and provider requests
//! are the public evidence, with acceptance proving that menu actions were routed.

use std::{
    cell::RefCell,
    future::poll_fn,
    rc::Rc,
    task::{Poll, Waker},
};

use gpui_kit::{
    App, AppContext, Context, Entity, Result, Task, TestAppContext, Window, WindowHandle,
    component::input::{CompletionProvider, Editor, EditorState, Input, InputState, Rope},
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
    deferred: bool,
    pending: RefCell<Vec<Rc<RefCell<PendingResponse>>>>,
}

#[derive(Default)]
struct PendingResponse {
    response: Option<CompletionResponse>,
    waker: Option<Waker>,
}

impl Suggestions {
    fn respond(&self, index: usize, label: Option<&str>) {
        let pending = self.pending.borrow()[index].clone();
        let mut pending = pending.borrow_mut();
        pending.response = Some(CompletionResponse::Array(
            label
                .into_iter()
                .map(|label| CompletionItem {
                    label: label.into(),
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                        range: Range::new(
                            Position::new(0, 0),
                            Position::new(0, self.requests.borrow()[index].offset as u32),
                        ),
                        new_text: label.into(),
                    })),
                    ..Default::default()
                })
                .collect(),
        ));
        if let Some(waker) = pending.waker.take() {
            waker.wake();
        }
    }
}

impl CompletionProvider for Suggestions {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        trigger: CompletionContext,
        _: &mut Window,
        cx: &mut App,
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
        if self.deferred {
            let pending = Rc::new(RefCell::new(PendingResponse::default()));
            self.pending.borrow_mut().push(pending.clone());
            cx.spawn(async move |_| {
                poll_fn(move |cx| {
                    let mut pending = pending.borrow_mut();
                    match pending.response.take() {
                        Some(response) => Poll::Ready(Ok(response)),
                        None => {
                            pending.waker = Some(cx.waker().clone());
                            Poll::Pending
                        }
                    }
                })
                .await
            })
        } else {
            Task::ready(Ok(CompletionResponse::Array(items)))
        }
    }

    fn is_completion_trigger(&self, _: usize, new_text: &str, _: &mut App) -> bool {
        !new_text.is_empty() && new_text.chars().all(|ch| ch.is_ascii_alphabetic())
    }
}

struct CompletionEditor {
    state: Entity<EditorState>,
    other: Entity<InputState>,
}

impl Render for CompletionEditor {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .child(Input::new(&self.other).id("other"))
            .child(Editor::new(&self.state).flex_1())
    }
}

struct Fixture {
    handle: WindowHandle<gpui_kit::base::Root>,
    state: Entity<EditorState>,
    provider: Rc<Suggestions>,
}

impl Fixture {
    fn new(cx: &mut TestAppContext) -> Self {
        Self::with_provider(cx, Suggestions::default())
    }

    fn deferred(cx: &mut TestAppContext) -> Self {
        Self::with_provider(
            cx,
            Suggestions {
                deferred: true,
                ..Default::default()
            },
        )
    }

    fn with_provider(cx: &mut TestAppContext, provider: Suggestions) -> Self {
        cx.update(gpui_kit::init);
        let provider = Rc::new(provider);
        let (handle, view) =
            common::open_window(cx, Some(size(px(800.), px(480.))), |window, cx| {
                cx.new(|cx| CompletionEditor {
                    other: cx.new(|cx| InputState::new(window, cx)),
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

// Each response is released explicitly. run_until_parked drains runnable work
// without advancing timers or waiting for a response that the test still owns.
#[gpui_kit::test]
fn older_completion_response_cannot_replace_newer_suggestions(cx: &mut TestAppContext) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    fixture.input("r", cx);
    assert_eq!(fixture.provider.requests.borrow().len(), 2);
    fixture.provider.respond(1, Some("private"));
    fixture.settle(cx);
    fixture.provider.respond(0, Some("print"));
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("private", cx);
}

#[gpui_kit::test]
fn empty_newer_response_cannot_be_reopened_by_older_suggestions(cx: &mut TestAppContext) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    fixture.input("z", cx);
    assert_eq!(fixture.provider.requests.borrow().len(), 2);
    fixture.provider.respond(1, None);
    fixture.settle(cx);
    fixture.provider.respond(0, Some("print"));
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("pz\n", cx);
}

#[gpui_kit::test]
fn completion_response_after_focus_loss_cannot_reopen_on_refocus(cx: &mut TestAppContext) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    cx.update_window(fixture.handle.into(), |_, window, cx| {
        window.click("other", cx);
        window.input("other field", cx);
        assert_eq!(
            window.find(("input", fixture.state.entity_id())).focused(),
            Some(false)
        );
    })
    .unwrap();
    fixture.provider.respond(0, Some("print"));
    fixture.settle(cx);
    cx.update_window(fixture.handle.into(), |_, window, cx| {
        assert_eq!(window.find("other").value(), Some("other field"));
        assert_eq!(window.find("other").focused(), Some(true));
        window.click(("input", fixture.state.entity_id()), cx);
    })
    .unwrap();
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("p\n", cx);
}

#[gpui_kit::test]
fn closing_window_disposes_editor_with_completion_in_flight(cx: &mut TestAppContext) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    let editor = fixture.state.downgrade();
    cx.update_window(fixture.handle.into(), |_, window, _| window.remove_window())
        .unwrap();
    let provider = fixture.provider.clone();
    drop(fixture);
    cx.run_until_parked();
    assert!(editor.upgrade().is_none(), "closed editor must be released");
    provider.respond(0, Some("print"));
    cx.run_until_parked();
    assert!(editor.upgrade().is_none());
}

#[gpui_kit::test]
fn focus_round_trip_invalidates_completion_requested_before_blur(cx: &mut TestAppContext) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    cx.update_window(fixture.handle.into(), |_, window, cx| {
        window.click("other", cx);
        assert_eq!(window.find("other").focused(), Some(true));
    })
    .unwrap();
    fixture.settle(cx);
    cx.update_window(fixture.handle.into(), |_, window, cx| {
        window.click(("input", fixture.state.entity_id()), cx);
    })
    .unwrap();
    fixture.settle(cx);
    fixture.provider.respond(0, Some("print"));
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("p\n", cx);
}
