//! Inline presentation and change subscriptions shared by Input and Textarea.
use gpui_component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallback, ComponentPayload,
    InlineTokenCallbacks, MaterializeRequest, MethodDescriptor, anyhow,
    gpui::{self, App, Entity, IntoElement as _, RenderOnce, Window},
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
enum Op {
    Render(ComponentArgument),
    Click(ComponentArgument),
    Change(ComponentArgument),
}
#[derive(Clone)]
pub(super) enum State {
    Input(Entity<InputState>),
    Textarea(Entity<TextareaState>),
}
macro_rules! dispatch {
    ($state:expr, |$entity:ident| $body:expr) => {
        match $state {
            State::Input($entity) => $body,
            State::Textarea($entity) => $body,
        }
    };
}

pub(super) fn methods(include_change: bool) -> Vec<MethodDescriptor> {
    let mut methods = vec![
        MethodDescriptor::new(
            "token",
            vec![ArgumentDescriptor::new(
                "render",
                ArgumentSchema::Callback(
                    "(token: InlineTokenContext, cx: Context) => Element | null",
                ),
            )],
            |args| match args {
                [arg @ ComponentArgument::Callback(_)] => {
                    Ok(ComponentPayload::new(Op::Render(arg.clone())))
                }
                _ => Err("token expects a renderer".into()),
            },
        )
        .with_documentation(
            "Renders an atomic token from its current UTF-16 range and read-only context.",
        ),
        MethodDescriptor::new(
            "on_token_click",
            vec![ArgumentDescriptor::new(
                "listener",
                ArgumentSchema::Callback("(event: InlineTokenClickEvent, cx: Context) => void"),
            )],
            |args| match args {
                [arg @ ComponentArgument::Callback(_)] => {
                    Ok(ComponentPayload::new(Op::Click(arg.clone())))
                }
                _ => Err("on_token_click expects a listener".into()),
            },
        )
        .with_documentation(
            "Activates a reference after a completed unconsumed click, outside the editing borrow.",
        ),
    ];
    if include_change {
        methods.push(MethodDescriptor::new("on_change", vec![ArgumentDescriptor::new("listener", ArgumentSchema::Callback("(text: string, cx: Context) => void"))], |args| match args {
            [arg @ ComponentArgument::Callback(_)] => Ok(ComponentPayload::new(Op::Change(arg.clone()))), _ => Err("on_change expects a listener".into()),
        }).with_documentation("Reports user text or token identity changes; explicit draft restoration remains silent."));
    }
    methods
}

pub(super) struct Binding {
    state: State,
    callbacks: InlineTokenCallbacks,
    change: Option<ComponentCallback>,
}
pub(super) fn prepare(request: &MaterializeRequest<'_>, state: State) -> anyhow::Result<Binding> {
    let mut renderer = None;
    let mut listener = None;
    let mut change = None;
    for op in request
        .methods()
        .filter_map(|m| m.payload().downcast_ref::<Op>())
    {
        match op {
            Op::Render(arg) => renderer = Some(request.resolve_element_callback(arg)?),
            Op::Click(arg) => listener = Some(request.resolve_callback(arg)?),
            Op::Change(arg) => change = Some(request.resolve_callback(arg)?),
        }
    }
    let callbacks = dispatch!(&state, |state| InlineTokenCallbacks::new(
        state, renderer, listener
    ));
    Ok(Binding {
        state,
        callbacks,
        change,
    })
}
impl Binding {
    pub(super) fn input(&self, input: Input) -> Input {
        self.callbacks.apply(
            input,
            |input, render| input.token(move |token, window, cx| render(token, window, cx)),
            |input, listen| {
                input.on_token_click(move |event, window, cx| listen(event, window, cx))
            },
        )
    }
    pub(super) fn textarea(&self, input: Textarea) -> Textarea {
        self.callbacks.apply(
            input,
            |input, render| input.token(move |token, window, cx| render(token, window, cx)),
            |input, listen| {
                input.on_token_click(move |event, window, cx| listen(event, window, cx))
            },
        )
    }
    pub(super) fn wrap(self, element: gpui::AnyElement) -> gpui::AnyElement {
        Bound {
            element,
            binding: self,
        }
        .into_any_element()
    }
}
struct Host {
    listener: Rc<RefCell<Option<ComponentCallback>>>,
    _subscription: gpui::Subscription,
}
#[derive(gpui::IntoElement)]
struct Bound {
    element: gpui::AnyElement,
    binding: Binding,
}
impl RenderOnce for Bound {
    fn render(self, window: &mut Window, cx: &mut App) -> impl gpui::IntoElement {
        let id = dispatch!(&self.binding.state, |state| state.entity_id());
        let state = self.binding.state.clone();
        let host = window.use_keyed_state(("shell-input-change", id), cx, move |window, cx| {
            let listener = Rc::new(RefCell::new(None::<ComponentCallback>));
            let callback = listener.clone();
            let subscription = dispatch!(&state, |state| window.subscribe(
                state,
                cx,
                move |state, event: &InputEvent, window, cx| {
                    if !matches!(event, InputEvent::Change) {
                        return;
                    }
                    let listener = callback.borrow().clone();
                    if let Some(listener) = listener {
                        listener.invoke_and_report_with(
                            "input change callback failed",
                            &[gpui_shell::ComponentCallbackArgument::String(
                                state.read(cx).value().to_string(),
                            )],
                            window,
                            cx,
                        );
                    }
                }
            ));
            Host {
                listener,
                _subscription: subscription,
            }
        });
        *host.read(cx).listener.borrow_mut() = self.binding.change;
        self.element
    }
}
