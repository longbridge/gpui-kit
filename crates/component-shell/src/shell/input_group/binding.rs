use std::{cell::RefCell, rc::Rc};

use gpui_component::input::{InputEvent, InputState, TextareaState};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallback,
    ComponentCallbackArgument, ComponentPayload, MaterializeRequest, MethodDescriptor, anyhow,
    gpui::{App, Entity, EntityId, Subscription, Window},
};

use super::Op;

#[derive(Clone)]
pub(super) enum NativeState {
    Input(Entity<InputState>),
    Textarea(Entity<TextareaState>),
}

macro_rules! dispatch {
    ($self:expr, |$state:ident| $body:expr) => {
        match $self {
            NativeState::Input($state) => $body,
            NativeState::Textarea($state) => $body,
        }
    };
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TextareaLayout {
    Rows(usize),
    AutoGrow(usize, usize),
}

#[derive(Default)]
struct Callbacks {
    change: Option<ComponentCallback>,
}

struct Host {
    callbacks: Rc<RefCell<Callbacks>>,
    layout: Option<TextareaLayout>,
    _events: Subscription,
}

impl NativeState {
    fn id(&self) -> EntityId {
        dispatch!(self, |state| state.entity_id())
    }

    fn subscribe(
        &self,
        callbacks: Rc<RefCell<Callbacks>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Subscription {
        dispatch!(self, |state| window.subscribe(
            state,
            cx,
            move |state, event: &InputEvent, window, cx| {
                let callback = match event {
                    InputEvent::Change => callbacks.borrow().change.clone(),
                    _ => None,
                };
                if let Some(callback) = callback {
                    let arguments = vec![ComponentCallbackArgument::String(
                        state.read(cx).value().to_string(),
                    )];
                    callback.invoke_and_report_with(
                        "InputGroup text callback failed",
                        &arguments,
                        window,
                        cx,
                    );
                }
            }
        ))
    }
}

pub(super) struct Binding {
    state: NativeState,
    callbacks: Callbacks,
    value: Option<String>,
    placeholder: Option<String>,
    layout: Option<TextareaLayout>,
    masked: Option<bool>,
}

pub(super) fn prepare(
    request: &mut MaterializeRequest<'_>,
    state: NativeState,
    operations: &[Op],
) -> anyhow::Result<Binding> {
    let callback = |pick: fn(&Op) -> Option<&ComponentArgument>| {
        operations
            .iter()
            .rev()
            .find_map(pick)
            .map(|argument| request.resolve_callback(argument))
            .transpose()
    };
    let callbacks = Callbacks {
        change: callback(|operation| {
            if let Op::OnChange(argument) = operation {
                Some(argument)
            } else {
                None
            }
        })?,
    };
    let value = operations.iter().rev().find_map(|operation| {
        if let Op::Value(value) = operation {
            Some(value)
        } else {
            None
        }
    });
    let placeholder = operations.iter().rev().find_map(|operation| {
        if let Op::Placeholder(value) = operation {
            Some(value)
        } else {
            None
        }
    });
    let layout = operations.iter().rev().find_map(|operation| {
        if let Op::Layout(value) = operation {
            Some(*value)
        } else {
            None
        }
    });
    let masked = operations.iter().rev().find_map(|operation| {
        if let Op::Masked(value) = operation {
            Some(*value)
        } else {
            None
        }
    });
    Ok(Binding {
        state,
        callbacks,
        value: value.cloned(),
        placeholder: placeholder.cloned(),
        layout,
        masked,
    })
}

impl Binding {
    pub(super) fn apply(self, window: &mut Window, cx: &mut App) {
        let Self {
            state,
            callbacks,
            value,
            placeholder,
            layout,
            masked,
        } = self;
        let initial_state = state.clone();
        let host =
            window.use_keyed_state(("shell-input-group", state.id()), cx, move |window, cx| {
                let callbacks = Rc::new(RefCell::new(Callbacks::default()));
                let events = initial_state.subscribe(callbacks.clone(), window, cx);
                Host {
                    callbacks,
                    layout: None,
                    _events: events,
                }
            });
        *host.read(cx).callbacks.borrow_mut() = callbacks;

        dispatch!(&state, |state| {
            if let Some(value) = value
                .as_ref()
                .filter(|value| state.read(cx).value().as_ref() != value.as_str())
            {
                // set_value is silent. Reassert only a different controlled
                // value so ordinary renders preserve selection and undo history.
                state.update(cx, |state, cx| state.set_value(value.clone(), window, cx));
            }
            if let Some(placeholder) = placeholder.as_ref().filter(|value| {
                state.read(cx).presentation().placeholder().as_ref() != value.as_str()
            }) {
                state.update(cx, |state, cx| {
                    state.set_placeholder(placeholder.clone(), window, cx)
                });
            }
        });
        if let NativeState::Input(state) = &state {
            if let Some(masked) =
                masked.filter(|masked| state.read(cx).presentation().is_masked() != *masked)
            {
                state.update(cx, |state, cx| state.set_masked(masked, window, cx));
            }
        }
        if let NativeState::Textarea(state) = &state {
            if let Some(layout) = layout.filter(|layout| host.read(cx).layout != Some(*layout)) {
                state.update(cx, |state, cx| match layout {
                    TextareaLayout::Rows(rows) => state.set_rows(rows, cx),
                    TextareaLayout::AutoGrow(min, max) => state.set_auto_grow(min, max, cx),
                });
                host.update(cx, |host, _| host.layout = Some(layout));
            }
        }
    }
}

fn rows(argument: &ComponentArgument) -> Result<usize, String> {
    match argument {
        ComponentArgument::Number(value)
            if value.is_finite()
                && *value >= 1.
                && value.fract() == 0.
                && *value < usize::MAX as f64 =>
        {
            Ok(*value as usize)
        }
        _ => Err("textarea rows must be a positive integer".into()),
    }
}

pub(super) fn layout_methods() -> Vec<MethodDescriptor> {
    vec![
        MethodDescriptor::new(
            "rows",
            vec![ArgumentDescriptor::new("rows", ArgumentSchema::Number)],
            |arguments| match arguments {
                [value] => Ok(ComponentPayload::new(Op::Layout(TextareaLayout::Rows(
                    rows(value)?,
                )))),
                _ => Err("rows expects one positive integer".into()),
            },
        )
        .with_documentation(
            "Sets a fixed row count on TextareaState. Use h(...) for an explicit viewport height.",
        ),
        MethodDescriptor::new(
            "auto_grow",
            vec![
                ArgumentDescriptor::new("min_rows", ArgumentSchema::Number),
                ArgumentDescriptor::new("max_rows", ArgumentSchema::Number),
            ],
            |arguments| match arguments {
                [min, max] => {
                    let (min, max) = (rows(min)?, rows(max)?);
                    if max < min {
                        return Err("auto_grow requires max_rows >= min_rows".into());
                    }
                    Ok(ComponentPayload::new(Op::Layout(TextareaLayout::AutoGrow(
                        min, max,
                    ))))
                }
                _ => Err("auto_grow expects positive minimum and maximum rows".into()),
            },
        )
        .with_documentation(
            "Grows the native textarea between the minimum and maximum row counts.",
        ),
    ]
}
