//! Completion workflows through the styled editor, its real popup, and native input.
//! The popup has no TestWindowExt observation; text, focus, and provider requests
//! are the public evidence, with acceptance proving that menu actions were routed.

use std::{
    cell::{Cell, RefCell},
    future::poll_fn,
    rc::Rc,
    task::{Poll, Waker},
};

use gpui_kit::{
    App, AppContext, Context, Entity, Result, SharedString, Subscription, Task, TestAppContext,
    Window, WindowHandle,
    component::input::{
        CodeActionProvider, CompletionProvider, Editor, EditorState, Input, InputEvent, InputState,
        Rope,
    },
    div,
    prelude::*,
    px, size,
    test::TestWindowExt,
};
use lsp_types::{
    CodeAction, CompletionContext, CompletionItem, CompletionResponse, CompletionTextEdit,
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
    response: Option<Result<CompletionResponse>>,
    waker: Option<Waker>,
}

impl Suggestions {
    fn fail(&self, index: usize) {
        let pending = self.pending.borrow()[index].clone();
        let mut pending = pending.borrow_mut();
        pending.response = Some(Err(
            std::io::Error::other("synthetic provider failure").into()
        ));
        if let Some(waker) = pending.waker.take() {
            waker.wake();
        }
    }

    fn respond(&self, index: usize, label: Option<&str>) {
        let pending = self.pending.borrow()[index].clone();
        let mut pending = pending.borrow_mut();
        pending.response = Some(Ok(CompletionResponse::Array(
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
        )));
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
                        Some(response) => Poll::Ready(response),
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
    readonly: bool,
    disabled: bool,
}

impl Render for CompletionEditor {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .child(Input::new(&self.other).id("other"))
            .child(
                Editor::new(&self.state)
                    .readonly(self.readonly)
                    .disabled(self.disabled)
                    .flex_1(),
            )
    }
}

struct Fixture {
    handle: WindowHandle<gpui_kit::base::Root>,
    state: Entity<EditorState>,
    provider: Rc<Suggestions>,
    view: Entity<CompletionEditor>,
    blurs: Rc<Cell<usize>>,
    _subscription: Subscription,
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
        Self::with_activation(cx, provider, true)
    }

    fn with_activation(cx: &mut TestAppContext, provider: Suggestions, activate: bool) -> Self {
        cx.update(gpui_kit::init);
        let provider = Rc::new(provider);
        let (handle, view) =
            common::open_window(cx, Some(size(px(800.), px(480.))), |window, cx| {
                cx.new(|cx| CompletionEditor {
                    readonly: false,
                    disabled: false,
                    other: cx.new(|cx| InputState::new(window, cx)),
                    state: cx.new(|cx| {
                        let mut state = EditorState::new(window, cx).language("plaintext");
                        state.lsp_mut().completion_provider = Some(provider.clone());
                        state
                    }),
                })
            });
        let state = cx.update(|cx| view.read(cx).state.clone());
        let blurs = Rc::new(Cell::new(0));
        let subscription = cx.update(|cx| {
            let blurs = blurs.clone();
            cx.subscribe(&state, move |_, event, _| {
                if matches!(event, InputEvent::Blur) {
                    blurs.set(blurs.get() + 1);
                }
            })
        });
        if activate {
            cx.update_window(handle.into(), |_, window, _| window.activate_window())
                .unwrap();
            cx.run_until_parked();
        }
        let fixture = Self {
            handle,
            state,
            provider,
            view,
            blurs,
            _subscription: subscription,
        };
        cx.update_window(handle.into(), |_, window, cx| {
            window.click(("input", fixture.state.entity_id()), cx);
        })
        .unwrap();
        fixture.settle(cx);
        fixture.assert_editor("", cx);
        fixture
    }

    fn protect(&self, readonly: bool, disabled: bool, cx: &mut TestAppContext) {
        self.view.update(cx, |view, cx| {
            view.readonly = readonly;
            view.disabled = disabled;
            cx.notify();
        });
        self.settle(cx);
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
    fixture.settle(cx);
    assert_eq!(fixture.blurs.get(), 1);
    fixture.provider.respond(0, Some("print"));
    fixture.settle(cx);
    cx.update_window(fixture.handle.into(), |_, window, cx| {
        assert_eq!(window.find("other").value(), Some("other field"));
        assert_eq!(window.find("other").focused(), Some(true));
        window.press("tab", cx);
    })
    .unwrap();
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("p\n", cx);
}

#[gpui_kit::test]
fn closing_window_disposes_editor_with_completion_in_flight(cx: &mut TestAppContext) {
    // GPUI 0.3.6 TestPlatform retains its active TestWindow after remove_window.
    // That window's PlatformInputHandler owns an ElementInputHandler<EditorState>
    // with a strong Entity. Keep this disposal fixture inactive; the blur tests
    // above explicitly activate their windows and observe InputEvent::Blur.
    let fixture = Fixture::with_activation(
        cx,
        Suggestions {
            deferred: true,
            ..Default::default()
        },
        false,
    );
    fixture.start_completion(cx);
    let Fixture {
        handle,
        state,
        provider,
        view,
        _subscription,
        ..
    } = fixture;
    let editor = state.downgrade();
    // Release the test's owners before the window update flushes entity drops.
    // Retaining the view until after that update leaves its child state queued.
    drop((_subscription, view, state));
    cx.update_window(handle.into(), |_, window, _| window.remove_window())
        .unwrap();
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
    assert_eq!(fixture.blurs.get(), 1);
    cx.update_window(fixture.handle.into(), |_, window, cx| {
        window.press("tab", cx);
    })
    .unwrap();
    fixture.settle(cx);
    fixture.assert_editor("p", cx);
    fixture.provider.respond(0, Some("print"));
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("p\n", cx);
}

#[gpui_kit::test]
fn escape_while_request_is_pending_rejects_its_late_response(cx: &mut TestAppContext) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    fixture.press("escape", cx);
    fixture.provider.respond(0, Some("print"));
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("p\n", cx);
    assert_eq!(fixture.provider.requests.borrow().len(), 1);
}

#[gpui_kit::test]
fn non_trigger_edit_invalidates_pending_completion(cx: &mut TestAppContext) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    fixture.input("!", cx);
    assert_eq!(fixture.provider.requests.borrow().len(), 1);
    fixture.provider.respond(0, Some("print"));
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("p!\n", cx);
}

#[gpui_kit::test]
fn deleting_prefix_invalidates_pending_completion(cx: &mut TestAppContext) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    fixture.press("backspace", cx);
    fixture.assert_editor("", cx);
    fixture.provider.respond(0, Some("print"));
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("\n", cx);
    assert_eq!(fixture.provider.requests.borrow().len(), 1);
}

#[gpui_kit::test]
fn caret_round_trip_does_not_revive_pending_completion(cx: &mut TestAppContext) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    fixture.press("left", cx);
    fixture.press("right", cx);
    fixture.provider.respond(0, Some("print"));
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("p\n", cx);
}

#[gpui_kit::test]
fn tab_indents_instead_of_accepting_popup_and_invalidates_its_response(cx: &mut TestAppContext) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    fixture.press("tab", cx);
    fixture.assert_editor("p  ", cx);
    fixture.provider.respond(0, Some("print"));
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("p  \n", cx);
}

#[gpui_kit::test]
fn tab_dismisses_visible_completion_without_accepting_it(cx: &mut TestAppContext) {
    let fixture = Fixture::new(cx);
    fixture.start_completion(cx);
    fixture.press("tab", cx);
    fixture.assert_editor("p  ", cx);
    fixture.press("enter", cx);
    fixture.assert_editor("p  \n", cx);
}

fn protection_invalidates_completion(cx: &mut TestAppContext, disabled: bool) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    fixture.protect(!disabled, disabled, cx);
    fixture.provider.respond(0, Some("print"));
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.input("ignored", cx);
    cx.update_window(fixture.handle.into(), |_, window, cx| {
        assert_eq!(
            window.find(("input", fixture.state.entity_id())).value(),
            Some("p")
        );
        assert_eq!(fixture.state.read(cx).value(), "p");
    })
    .unwrap();
    assert_eq!(fixture.provider.requests.borrow().len(), 1);
    fixture.protect(false, false, cx);
    // No click: re-enabling cannot rely on pointer cancellation to clear stale work.
    fixture.press("enter", cx);
    fixture.assert_editor("p\n", cx);
}

#[gpui_kit::test]
fn readonly_transition_invalidates_inflight_completion(cx: &mut TestAppContext) {
    protection_invalidates_completion(cx, false);
}

#[gpui_kit::test]
fn disabled_transition_invalidates_inflight_completion(cx: &mut TestAppContext) {
    protection_invalidates_completion(cx, true);
}

#[gpui_kit::test]
fn provider_error_leaves_editor_usable_and_next_request_can_succeed(cx: &mut TestAppContext) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    fixture.provider.fail(0);
    fixture.settle(cx);
    fixture.assert_editor("p", cx);
    fixture.input("r", cx);
    assert_eq!(fixture.provider.requests.borrow().len(), 2);
    fixture.provider.respond(1, Some("private"));
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("private", cx);
}

#[gpui_kit::test]
fn failed_refresh_dismisses_previous_items_and_rejects_older_response(cx: &mut TestAppContext) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    fixture.provider.respond(0, Some("print"));
    fixture.settle(cx);
    fixture.input("r", cx);
    fixture.input("i", cx);
    assert_eq!(fixture.provider.requests.borrow().len(), 3);
    fixture.provider.fail(2);
    fixture.settle(cx);
    fixture.provider.respond(1, Some("private"));
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("pri\n", cx);
}

#[gpui_kit::test]
fn late_error_from_older_request_cannot_dismiss_newer_items(cx: &mut TestAppContext) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    fixture.input("r", cx);
    fixture.provider.respond(1, Some("private"));
    fixture.settle(cx);
    fixture.provider.fail(0);
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("private", cx);
}

#[cfg(target_os = "macos")]
const CODE_ACTIONS: &str = "cmd-.";
#[cfg(not(target_os = "macos"))]
const CODE_ACTIONS: &str = "ctrl-.";

struct Actions {
    id: &'static str,
    fail: bool,
    requests: RefCell<Vec<(String, std::ops::Range<usize>)>>,
    performed: RefCell<Vec<(String, bool)>>,
}

impl Actions {
    fn new(id: &'static str, fail: bool) -> Rc<Self> {
        Rc::new(Self {
            id,
            fail,
            requests: RefCell::new(Vec::new()),
            performed: RefCell::new(Vec::new()),
        })
    }
}

impl CodeActionProvider for Actions {
    fn id(&self) -> SharedString {
        self.id.into()
    }

    fn code_actions(
        &self,
        state: Entity<EditorState>,
        range: std::ops::Range<usize>,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<CodeAction>>> {
        self.requests
            .borrow_mut()
            .push((state.read(cx).value().to_string(), range));
        if self.fail {
            return Task::ready(Err(
                std::io::Error::other("synthetic code action failure").into()
            ));
        }
        Task::ready(Ok(vec![CodeAction {
            title: self.id.into(),
            ..Default::default()
        }]))
    }

    fn perform_code_action(
        &self,
        _: Entity<EditorState>,
        action: CodeAction,
        push_to_history: bool,
        _: &mut Window,
        _: &mut App,
    ) -> Task<Result<()>> {
        self.performed
            .borrow_mut()
            .push((action.title, push_to_history));
        Task::ready(Ok(()))
    }
}

fn code_action_fixture(cx: &mut TestAppContext, providers: Vec<Rc<Actions>>) -> Fixture {
    let fixture = Fixture::new(cx);
    fixture.state.update(cx, |state, _| {
        state.lsp_mut().completion_provider = None;
        state.lsp_mut().code_action_providers = providers
            .into_iter()
            .map(|provider| provider as Rc<dyn CodeActionProvider>)
            .collect();
    });
    fixture.input("value", cx);
    fixture.press("shift-left", cx);
    fixture.press("shift-left", cx);
    fixture
}

#[gpui_kit::test]
fn code_action_selection_routes_to_its_provider_with_selected_range(cx: &mut TestAppContext) {
    let first = Actions::new("First action", false);
    let second = Actions::new("Second action", false);
    let fixture = code_action_fixture(cx, vec![first.clone(), second.clone()]);
    fixture.press(CODE_ACTIONS, cx);
    for provider in [&first, &second] {
        assert_eq!(*provider.requests.borrow(), vec![("value".into(), 3..5)]);
    }
    fixture.press("down", cx);
    fixture.press("enter", cx);
    assert!(first.performed.borrow().is_empty());
    assert_eq!(
        *second.performed.borrow(),
        vec![("Second action".into(), true)]
    );
    fixture.assert_editor("value", cx);
    fixture.press("right", cx);
    fixture.press("enter", cx);
    fixture.assert_editor("value\n", cx);
    assert_eq!(second.performed.borrow().len(), 1);
}

#[gpui_kit::test]
fn escape_dismisses_code_actions_without_performing_them(cx: &mut TestAppContext) {
    let provider = Actions::new("Action", false);
    let fixture = code_action_fixture(cx, vec![provider.clone()]);
    fixture.press(CODE_ACTIONS, cx);
    assert_eq!(provider.requests.borrow().len(), 1);
    fixture.press("escape", cx);
    fixture.press("right", cx);
    fixture.press("enter", cx);
    fixture.assert_editor("value\n", cx);
    assert!(provider.performed.borrow().is_empty());
}

#[gpui_kit::test]
fn failed_code_action_provider_does_not_hide_successful_provider(cx: &mut TestAppContext) {
    let failed = Actions::new("Failed action", true);
    let successful = Actions::new("Successful action", false);
    let fixture = code_action_fixture(cx, vec![failed.clone(), successful.clone()]);
    fixture.press(CODE_ACTIONS, cx);
    fixture.press("enter", cx);
    assert_eq!(failed.requests.borrow().len(), 1);
    assert!(failed.performed.borrow().is_empty());
    assert_eq!(
        *successful.performed.borrow(),
        vec![("Successful action".into(), true)]
    );
    fixture.assert_editor("value", cx);
}

#[gpui_kit::test]
fn all_code_action_providers_failing_preserves_normal_enter(cx: &mut TestAppContext) {
    let provider = Actions::new("Failed action", true);
    let fixture = code_action_fixture(cx, vec![provider.clone()]);
    fixture.press("right", cx);
    fixture.press(CODE_ACTIONS, cx);
    assert_eq!(provider.requests.borrow().len(), 1);
    fixture.press("enter", cx);
    fixture.assert_editor("value\n", cx);
    assert!(provider.performed.borrow().is_empty());
}

fn protection_round_trip_invalidates_completion(cx: &mut TestAppContext, disabled: bool) {
    let fixture = Fixture::deferred(cx);
    fixture.start_completion(cx);
    fixture.protect(!disabled, disabled, cx);
    fixture.protect(false, false, cx);
    fixture.assert_editor("p", cx);
    fixture.provider.respond(0, Some("print"));
    fixture.settle(cx);
    fixture.press("enter", cx);
    fixture.assert_editor("p\n", cx);
    assert_eq!(fixture.provider.requests.borrow().len(), 1);
}

#[gpui_kit::test]
fn readonly_round_trip_invalidates_response_released_after_reenable(cx: &mut TestAppContext) {
    protection_round_trip_invalidates_completion(cx, false);
}

#[gpui_kit::test]
fn disabled_round_trip_invalidates_response_released_after_reenable(cx: &mut TestAppContext) {
    protection_round_trip_invalidates_completion(cx, true);
}

#[derive(Default)]
struct InlineSuggestions {
    requests: RefCell<Vec<(String, usize)>>,
    fail: bool,
}

impl CompletionProvider for InlineSuggestions {
    fn completions(
        &self,
        _: &Rope,
        _: usize,
        _: CompletionContext,
        _: &mut Window,
        _: &mut App,
    ) -> Task<Result<CompletionResponse>> {
        panic!("inline-only provider must not receive popup requests")
    }

    fn is_completion_trigger(&self, _: usize, _: &str, _: &mut App) -> bool {
        false
    }

    fn inline_completion_debounce(&self) -> std::time::Duration {
        std::time::Duration::from_millis(1)
    }

    fn inline_completion(
        &self,
        text: &Rope,
        offset: usize,
        trigger: lsp_types::InlineCompletionContext,
        _: &mut Window,
        _: &mut App,
    ) -> Task<Result<lsp_types::InlineCompletionResponse>> {
        assert_eq!(
            trigger.trigger_kind,
            lsp_types::InlineCompletionTriggerKind::Automatic
        );
        self.requests.borrow_mut().push((text.to_string(), offset));
        if self.fail {
            return Task::ready(Err(std::io::Error::other("synthetic inline failure").into()));
        }
        Task::ready(Ok(lsp_types::InlineCompletionResponse::Array(vec![
            lsp_types::InlineCompletionItem {
                insert_text: "rint".into(),
                filter_text: None,
                range: None,
                command: None,
                insert_text_format: None,
            },
        ])))
    }
}

fn inline_fixture(cx: &mut TestAppContext, fail: bool) -> (Fixture, Rc<InlineSuggestions>) {
    let fixture = Fixture::new(cx);
    let provider = Rc::new(InlineSuggestions {
        fail,
        ..Default::default()
    });
    fixture.state.update(cx, |state, _| {
        state.lsp_mut().completion_provider = Some(provider.clone());
    });
    fixture.input("p", cx);
    // Advance the deterministic executor clock; never sleep or wait on wall time.
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(1));
    fixture.settle(cx);
    assert_eq!(*provider.requests.borrow(), vec![("p".into(), 1)]);
    fixture.assert_editor("p", cx);
    (fixture, provider)
}

#[gpui_kit::test]
fn inline_completion_tab_accepts_without_issuing_another_request(cx: &mut TestAppContext) {
    let (fixture, provider) = inline_fixture(cx, false);
    fixture.press("tab", cx);
    fixture.assert_editor("print", cx);
    assert_eq!(provider.requests.borrow().len(), 1);
    fixture.press("enter", cx);
    fixture.assert_editor("print\n", cx);
}

#[gpui_kit::test]
fn escape_dismisses_inline_completion_and_tab_returns_to_indentation(cx: &mut TestAppContext) {
    let (fixture, _) = inline_fixture(cx, false);
    fixture.press("escape", cx);
    fixture.press("tab", cx);
    fixture.assert_editor("p  ", cx);
}

#[gpui_kit::test]
fn typing_clears_inline_suggestion_before_next_debounce(cx: &mut TestAppContext) {
    let (fixture, _) = inline_fixture(cx, false);
    fixture.input("!", cx);
    fixture.press("tab", cx);
    fixture.assert_editor("p!  ", cx);
}

#[gpui_kit::test]
fn inline_provider_error_preserves_tab_indentation(cx: &mut TestAppContext) {
    let (fixture, _) = inline_fixture(cx, true);
    fixture.press("tab", cx);
    fixture.assert_editor("p  ", cx);
}

#[derive(Clone, Copy)]
enum DefinitionResponse {
    Location,
    Empty,
    Error,
}

struct Definitions {
    response: Cell<DefinitionResponse>,
    requests: RefCell<Vec<(String, usize)>>,
    deferred: Cell<bool>,
    pending: RefCell<Vec<Rc<RefCell<DefinitionGate>>>>,
}

#[derive(Default)]
struct DefinitionGate {
    released: bool,
    waker: Option<Waker>,
}

impl Definitions {
    fn release(&self, index: usize) {
        let gate = self.pending.borrow()[index].clone();
        let mut gate = gate.borrow_mut();
        gate.released = true;
        if let Some(waker) = gate.waker.take() {
            waker.wake();
        }
    }
}

impl gpui_kit::component::input::DefinitionProvider for Definitions {
    fn definitions(
        &self,
        text: &Rope,
        offset: usize,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<lsp_types::LocationLink>>> {
        self.requests.borrow_mut().push((text.to_string(), offset));
        let response = match self.response.get() {
            DefinitionResponse::Location => Ok(vec![lsp_types::LocationLink {
                origin_selection_range: Some(Range::new(Position::new(0, 0), Position::new(0, 6))),
                target_uri: "fixture:///definition".parse().unwrap(),
                target_range: Range::new(Position::new(0, 0), Position::new(0, 13)),
                target_selection_range: Range::new(Position::new(0, 7), Position::new(0, 13)),
            }]),
            DefinitionResponse::Empty => Ok(Vec::new()),
            DefinitionResponse::Error => {
                Err(std::io::Error::other("synthetic definition failure").into())
            }
        };
        if !self.deferred.get() {
            return Task::ready(response);
        }
        let gate = Rc::new(RefCell::new(DefinitionGate::default()));
        self.pending.borrow_mut().push(gate.clone());
        cx.spawn(async move |_| {
            poll_fn(move |cx| {
                let mut gate = gate.borrow_mut();
                if gate.released {
                    Poll::Ready(())
                } else {
                    gate.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            })
            .await;
            response
        })
    }
}

fn definition_fixture(
    cx: &mut TestAppContext,
    response: DefinitionResponse,
) -> (Fixture, Rc<Definitions>) {
    let fixture = Fixture::new(cx);
    // GoToDefinition is public but has no default shortcut. Hosts can bind it;
    // exercise that supported action path through a real key event.
    cx.update(|cx| {
        cx.bind_keys([gpui_kit::KeyBinding::new(
            "f12",
            gpui_kit::component::input::GoToDefinition,
            Some("Input"),
        )])
    });
    let provider = Rc::new(Definitions {
        response: Cell::new(response),
        requests: RefCell::new(Vec::new()),
        deferred: Cell::new(false),
        pending: RefCell::new(Vec::new()),
    });
    fixture.state.update(cx, |state, _| {
        state.lsp_mut().completion_provider = None;
        state.lsp_mut().definition_provider = Some(provider.clone());
    });
    fixture.input("source target", cx);
    for _ in 0..7 {
        fixture.press("left", cx);
    }
    cx.update(|cx| assert_eq!(fixture.state.read(cx).cursor(), 6));
    (fixture, provider)
}

#[gpui_kit::test]
fn go_to_definition_requests_current_symbol_without_hover_and_selects_target(
    cx: &mut TestAppContext,
) {
    let (fixture, provider) = definition_fixture(cx, DefinitionResponse::Location);
    fixture.press("f12", cx);
    assert_eq!(
        *provider.requests.borrow(),
        vec![("source target".into(), 6)]
    );
    fixture.assert_editor("source target", cx);
    cx.update(|cx| {
        assert_eq!(fixture.state.read(cx).selected_range(), 7..13);
        assert_eq!(fixture.state.read(cx).selected_value(), "target");
    });
}

#[gpui_kit::test]
fn go_to_definition_dispatches_show_document_and_respects_host_handling(cx: &mut TestAppContext) {
    let (fixture, provider) = definition_fixture(cx, DefinitionResponse::Location);
    let shown = Rc::new(RefCell::new(Vec::new()));
    fixture.state.update(cx, |state, _| {
        let shown = shown.clone();
        state.lsp_mut().show_document = Some(Rc::new(move |params, _, _| {
            shown.borrow_mut().push(params.clone());
            true
        }));
    });
    fixture.press("f12", cx);
    assert_eq!(
        *provider.requests.borrow(),
        vec![("source target".into(), 6)]
    );
    let shown = shown.borrow();
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].uri.as_str(), "fixture:///definition");
    assert_eq!(shown[0].external, Some(false));
    assert_eq!(shown[0].take_focus, Some(true));
    assert_eq!(
        shown[0].selection,
        Some(Range::new(Position::new(0, 7), Position::new(0, 13)))
    );
    cx.update(|cx| assert_eq!(fixture.state.read(cx).selected_range(), 6..6));
    fixture.assert_editor("source target", cx);
}

#[gpui_kit::test]
fn empty_definition_response_preserves_caret_and_editing(cx: &mut TestAppContext) {
    let (fixture, provider) = definition_fixture(cx, DefinitionResponse::Empty);
    fixture.press("f12", cx);
    assert_eq!(
        *provider.requests.borrow(),
        vec![("source target".into(), 6)]
    );
    cx.update(|cx| assert_eq!(fixture.state.read(cx).selected_range(), 6..6));
    fixture.input("!", cx);
    fixture.assert_editor("source! target", cx);
}

#[gpui_kit::test]
fn definition_provider_error_preserves_caret_and_allows_retry(cx: &mut TestAppContext) {
    let (fixture, provider) = definition_fixture(cx, DefinitionResponse::Error);
    fixture.press("f12", cx);
    assert_eq!(
        *provider.requests.borrow(),
        vec![("source target".into(), 6)]
    );
    fixture.assert_editor("source target", cx);
    cx.update(|cx| assert_eq!(fixture.state.read(cx).selected_range(), 6..6));
    provider.response.set(DefinitionResponse::Location);
    fixture.press("f12", cx);
    assert_eq!(
        *provider.requests.borrow(),
        vec![("source target".into(), 6), ("source target".into(), 6)]
    );
    cx.update(|cx| assert_eq!(fixture.state.read(cx).selected_range(), 7..13));
    fixture.assert_editor("source target", cx);
}

fn pending_definition_fixture(cx: &mut TestAppContext) -> (Fixture, Rc<Definitions>) {
    let (fixture, provider) = definition_fixture(cx, DefinitionResponse::Location);
    provider.deferred.set(true);
    fixture.press("f12", cx);
    assert_eq!(
        *provider.requests.borrow(),
        vec![("source target".into(), 6)]
    );
    (fixture, provider)
}

#[gpui_kit::test]
fn late_definition_response_does_not_move_changed_caret(cx: &mut TestAppContext) {
    let (fixture, provider) = pending_definition_fixture(cx);
    fixture.press("left", cx);
    provider.release(0);
    fixture.settle(cx);
    fixture.assert_editor("source target", cx);
    cx.update(|cx| assert_eq!(fixture.state.read(cx).selected_range(), 5..5));
}

#[gpui_kit::test]
fn late_definition_response_rejects_changed_text_even_at_original_caret(cx: &mut TestAppContext) {
    let (fixture, provider) = pending_definition_fixture(cx);
    fixture.input("!", cx);
    fixture.press("left", cx);
    cx.update(|cx| assert_eq!(fixture.state.read(cx).cursor(), 6));
    provider.release(0);
    fixture.settle(cx);
    fixture.assert_editor("source! target", cx);
    cx.update(|cx| assert_eq!(fixture.state.read(cx).selected_range(), 6..6));
}

#[gpui_kit::test]
fn late_definition_response_cannot_navigate_after_blur_and_keyboard_refocus(
    cx: &mut TestAppContext,
) {
    let (fixture, provider) = pending_definition_fixture(cx);
    cx.update_window(fixture.handle.into(), |_, window, cx| {
        window.click("other", cx);
        assert_eq!(window.find("other").focused(), Some(true));
    })
    .unwrap();
    fixture.settle(cx);
    assert_eq!(fixture.blurs.get(), 1);
    fixture.press("tab", cx);
    fixture.assert_editor("source target", cx);
    provider.release(0);
    fixture.settle(cx);
    fixture.assert_editor("source target", cx);
    cx.update(|cx| assert_eq!(fixture.state.read(cx).selected_range(), 6..6));
}

#[gpui_kit::test]
fn pending_keyboard_definition_survives_pointer_leaving_editor(cx: &mut TestAppContext) {
    let (fixture, provider) = pending_definition_fixture(cx);
    cx.update_window(fixture.handle.into(), |_, window, cx| {
        // Hover the other field without clicking: the pointer leaves the editor
        // while the keyboard action's focus, document, and caret stay valid.
        window.hover("other", cx);
        assert_eq!(window.find("other").focused(), Some(false));
        assert_eq!(
            window.find(("input", fixture.state.entity_id())).focused(),
            Some(true)
        );
    })
    .unwrap();
    fixture.settle(cx);
    assert_eq!(fixture.blurs.get(), 0);
    provider.release(0);
    fixture.settle(cx);
    fixture.assert_editor("source target", cx);
    cx.update(|cx| assert_eq!(fixture.state.read(cx).selected_range(), 7..13));
    assert_eq!(provider.requests.borrow().len(), 1);
}

#[gpui_kit::test]
fn second_keyboard_definition_cancels_older_request_at_same_caret(cx: &mut TestAppContext) {
    let (fixture, provider) = pending_definition_fixture(cx);
    // Keep the document and cursor identical, so only request ownership can
    // reject the older location after the latest lookup reports no definition.
    provider.response.set(DefinitionResponse::Empty);
    fixture.press("f12", cx);
    assert_eq!(
        *provider.requests.borrow(),
        vec![("source target".into(), 6), ("source target".into(), 6)]
    );
    provider.release(1);
    fixture.settle(cx);
    cx.update(|cx| assert_eq!(fixture.state.read(cx).selected_range(), 6..6));
    provider.release(0);
    fixture.settle(cx);
    fixture.assert_editor("source target", cx);
    cx.update(|cx| assert_eq!(fixture.state.read(cx).selected_range(), 6..6));
}
