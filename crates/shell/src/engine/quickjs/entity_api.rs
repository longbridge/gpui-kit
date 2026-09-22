//! Script access to retained state.
//!
//! An element description lives for one render pass, but a text input's
//! content, cursor and undo history must survive every pass — so they live in a
//! GPUI entity and the script holds a handle (design doc §7.3). This module is
//! the JavaScript face of [`crate::entities`].
//!
//! Every function here takes and returns scalars. That is not a style
//! preference: a closure that both takes `Ctx<'js>` and returns a borrowed
//! `Object<'js>` cannot unify the two elided lifetimes, so the handle object
//! itself is assembled in the JS prelude, exactly as element objects are.

use std::rc::Weak;

use gpui::ScrollStrategy;
use gpui_base::NumberStep;
use gpui_base::VirtualListScrollHandle;
use gpui_base::input::{InputBaseState, InputModeKind};
use gpui_base::slider::{SliderScale, SliderValue};
use rquickjs::{
    Ctx, Exception, FromJs, Function, Object, Persistent, Result as JsResult, Value, function::Func,
};

use crate::{
    entities::{EntityHandle, InputEventName, OtpEventName, SliderEventName},
    scope::{self, ScopePhase},
    spec::{Component, SpecId},
};

use super::{InputCallbackOwner, ShellRuntime};

/// A script callback, persisted at conversion time.
///
/// Shared with the dock binding, whose `on(...)` takes a handler the same way.
///
/// A closure cannot take both `Ctx<'js>` and `Function<'js>` — the two elided
/// lifetimes will not unify — so the function is saved into a `Persistent`
/// inside `FromJs`, where both are still the same lifetime. The same reason the
/// engine's `Arguments` type exists.
pub(super) struct Handler(pub(super) Persistent<Function<'static>>);

impl<'js> FromJs<'js> for Handler {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let function = value
            .as_function()
            .ok_or_else(|| Exception::throw_type(ctx, "expected a function"))?;
        Ok(Self(Persistent::save(ctx, function.clone())))
    }
}

/// Installs the host half of the retained-state API. The prelude wraps these
/// into `InputState`, `TextareaState`, `FocusHandle` and the elements built
/// from them.
pub fn install(ctx: &Ctx<'_>, module: &Object<'_>, runtime: Weak<ShellRuntime>) -> JsResult<()> {
    let _ = module;
    let globals = ctx.globals();
    for multiline in [false, true] {
        let token_runtime = runtime.clone();
        globals.set(
            if multiline {
                "__textarea_token_call"
            } else {
                "__input_token_call"
            },
            Func::from(
                move |ctx: Ctx<'_>,
                      handle: EntityHandle,
                      method: String,
                      args: super::PlainDataArguments|
                      -> JsResult<super::DataResult> {
                    if !crate::input_tokens::METHODS
                        .iter()
                        .any(|(name, _, readonly)| *name == method && *readonly)
                    {
                        refuse_mutation_in_render(&ctx, &method)?;
                    }
                    let runtime = alive(&ctx, &token_runtime)?;
                    scope::with_current(|window, cx| {
                        if multiline {
                            let state = runtime.entities().textarea(handle).ok_or_else(|| {
                                anyhow::anyhow!("textarea state has been released")
                            })?;
                            crate::input_tokens::invoke_textarea(
                                &state, &method, &args.0, window, cx,
                            )
                        } else {
                            let state = runtime
                                .entities()
                                .input(handle)
                                .ok_or_else(|| anyhow::anyhow!("input state has been released"))?;
                            crate::input_tokens::invoke_input(&state, &method, &args.0, window, cx)
                        }
                    })
                    .ok_or_else(|| {
                        Exception::throw_type(&ctx, "input operation requires a live host call")
                    })?
                    .map(super::DataResult)
                    .map_err(|e| super::state_operation_error(&ctx, e))
                },
            ),
        )?;
    }

    // Every entity call reaches its store through the runtime, because the
    // store belongs to the runtime rather than to the thread — see
    // `crate::entities`. Each closure carries its own `Weak`, since a `Func`
    // owns what it captures.
    let create = runtime.clone();
    globals.set(
        "__input_state_new",
        Func::from(
            move |ctx: Ctx<'_>,
                  placeholder: Option<String>,
                  value: Option<String>|
                  -> JsResult<EntityHandle> {
                refuse_creation_in_render(&ctx, "InputState.new(...)")?;

                let store = alive(&ctx, &create)?;
                scope::with_current(|window, cx| {
                    store
                        .entities()
                        .create_input(
                            placeholder,
                            value,
                            scope::current_application_generation(),
                            window,
                            cx,
                        )
                        .map_err(|_| entity_limit_reached(&ctx))
                })
                .ok_or_else(|| {
                    Exception::throw_type(
                        &ctx,
                        "InputState.new(...) needs a live host call; call it from init() \
                         or an event handler",
                    )
                })?
            },
        ),
    )?;

    let read = runtime.clone();
    globals.set(
        "__input_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<String> {
                read_value(&ctx, &live(&ctx, &read, handle)?)
            },
        ),
    )?;

    let write = runtime.clone();
    globals.set(
        "__input_set_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, value: String| -> JsResult<()> {
                write_value(&ctx, &live(&ctx, &write, handle)?, value)
            },
        ),
    )?;

    // The numeric half of a text state. There is no separate numeric state
    // type — `NumberInput` reads these fields off the same `InputState` — so
    // they are set here rather than on the element, and they survive the render
    // that described the element the way every other retained value does.
    let step_runtime = runtime.clone();
    globals.set(
        "__input_set_step",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, step: Option<f64>| -> JsResult<()> {
                let state = live(&ctx, &step_runtime, handle)?;
                scope::with_current(|window, cx| {
                    state.update(cx, |state, cx| {
                        state.set_step(step.map(NumberStep::from), window, cx)
                    });
                })
                .ok_or_else(|| needs_call(&ctx, "set_step()"))
            },
        ),
    )?;

    let min_runtime = runtime.clone();
    globals.set(
        "__input_set_min",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, min: Option<f64>| -> JsResult<()> {
                let state = live(&ctx, &min_runtime, handle)?;
                scope::with_current(|window, cx| {
                    state.update(cx, |state, cx| state.set_min(min, window, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "set_min()"))
            },
        ),
    )?;

    let max_runtime = runtime.clone();
    globals.set(
        "__input_set_max",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, max: Option<f64>| -> JsResult<()> {
                let state = live(&ctx, &max_runtime, handle)?;
                scope::with_current(|window, cx| {
                    state.update(cx, |state, cx| state.set_max(max, window, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "set_max()"))
            },
        ),
    )?;

    let masked_runtime = runtime.clone();
    globals.set(
        "__input_set_masked",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, masked: bool| -> JsResult<()> {
                let state = live(&ctx, &masked_runtime, handle)?;
                scope::with_current(|window, cx| {
                    state.update(cx, |state, cx| state.set_masked(masked, window, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "set_masked()"))
            },
        ),
    )?;

    let loading_runtime = runtime.clone();
    globals.set(
        "__input_set_loading",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, loading: bool| -> JsResult<()> {
                let state = live(&ctx, &loading_runtime, handle)?;
                scope::with_current(|window, cx| {
                    state.update(cx, |state, cx| state.set_loading(loading, window, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "set_loading()"))
            },
        ),
    )?;

    let subscribe_runtime = runtime.clone();
    globals.set(
        "__input_on",
        Func::from(
            move |ctx: Ctx<'_>,
                  handle: EntityHandle,
                  name: String,
                  handler: Handler|
                  -> JsResult<bool> {
                subscribe(&ctx, &subscribe_runtime, handle, &name, handler, "input")
            },
        ),
    )?;

    // Multi-line text. `TextareaState` is a different Rust type from
    // `InputState` — the same engine specialized on another mode — so it needs
    // its own creation and its own resolver. Everything the two states share
    // goes through the generic helpers below rather than being written twice.
    let create_textarea = runtime.clone();
    globals.set(
        "__textarea_state_new",
        Func::from(
            move |ctx: Ctx<'_>,
                  placeholder: Option<String>,
                  value: Option<String>,
                  rows: Option<usize>|
                  -> JsResult<EntityHandle> {
                refuse_creation_in_render(&ctx, "TextareaState.new(...)")?;

                let store = alive(&ctx, &create_textarea)?;
                scope::with_current(|window, cx| {
                    store
                        .entities()
                        .create_textarea(
                            placeholder,
                            value,
                            rows,
                            scope::current_application_generation(),
                            window,
                            cx,
                        )
                        .map_err(|_| entity_limit_reached(&ctx))
                })
                .ok_or_else(|| {
                    Exception::throw_type(
                        &ctx,
                        "TextareaState.new(...) needs a live host call; call it from init() \
                         or an event handler",
                    )
                })?
            },
        ),
    )?;

    let read_textarea = runtime.clone();
    globals.set(
        "__textarea_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<String> {
                read_value(&ctx, &live_textarea(&ctx, &read_textarea, handle)?)
            },
        ),
    )?;

    let write_textarea = runtime.clone();
    globals.set(
        "__textarea_set_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, value: String| -> JsResult<()> {
                write_value(&ctx, &live_textarea(&ctx, &write_textarea, handle)?, value)
            },
        ),
    )?;

    let subscribe_textarea = runtime.clone();
    globals.set(
        "__textarea_on",
        Func::from(
            move |ctx: Ctx<'_>,
                  handle: EntityHandle,
                  name: String,
                  handler: Handler|
                  -> JsResult<bool> {
                subscribe(
                    &ctx,
                    &subscribe_textarea,
                    handle,
                    &name,
                    handler,
                    "textarea",
                )
            },
        ),
    )?;

    let rows_runtime = runtime.clone();
    globals.set(
        "__textarea_set_rows",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, rows: usize| -> JsResult<()> {
                let state = live_textarea(&ctx, &rows_runtime, handle)?;
                scope::with_current_app(|cx| {
                    state.update(cx, |state, cx| state.set_rows(rows, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "set_rows()"))
            },
        ),
    )?;

    let grow_runtime = runtime.clone();
    globals.set(
        "__textarea_set_auto_grow",
        Func::from(
            move |ctx: Ctx<'_>,
                  handle: EntityHandle,
                  min_rows: usize,
                  max_rows: usize|
                  -> JsResult<()> {
                let state = live_textarea(&ctx, &grow_runtime, handle)?;
                scope::with_current_app(|cx| {
                    state.update(cx, |state, cx| state.set_auto_grow(min_rows, max_rows, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "set_auto_grow()"))
            },
        ),
    )?;

    let wrap_runtime = runtime.clone();
    globals.set(
        "__textarea_set_soft_wrap",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, wrap: bool| -> JsResult<()> {
                let state = live_textarea(&ctx, &wrap_runtime, handle)?;
                // Needs the window as well as the app: turning wrapping on
                // re-measures against the width the last layout produced.
                scope::with_current(|window, cx| {
                    state.update(cx, |state, cx| state.set_soft_wrap(wrap, window, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "set_soft_wrap()"))
            },
        ),
    )?;

    let discard_textarea = runtime.clone();
    globals.set(
        "__textarea_release",
        Func::from(move |handle: EntityHandle| {
            discard_textarea
                .upgrade()
                .is_some_and(|runtime| runtime.entities().release(handle))
        }),
    )?;

    let textarea_element = runtime.clone();
    globals.set(
        "__textarea_element",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<SpecId> {
                let store = alive(&ctx, &textarea_element)?;
                if store.entities().textarea(handle).is_none() {
                    return Err(Exception::throw_type(
                        &ctx,
                        "this textarea state has been released and can no longer be rendered",
                    ));
                }
                Ok(store.push_component(Component::Textarea(handle)))
            },
        ),
    )?;

    // Sliders. A slider's value is retained for the same reason an input's
    // text is, and for one more: a drag writes it from a GPUI listener with no
    // script in the loop, so there is nowhere else it could live.
    let create_slider = runtime.clone();
    globals.set(
        "__slider_state_new",
        Func::from(
            move |ctx: Ctx<'_>,
                  min: f64,
                  max: f64,
                  step: f64,
                  scale: String,
                  value: Vec<f64>|
                  -> JsResult<EntityHandle> {
                refuse_creation_in_render(&ctx, "SliderState.new(...)")?;

                let scale = match scale.as_str() {
                    "linear" => SliderScale::Linear,
                    "logarithmic" => SliderScale::Logarithmic,
                    _ => {
                        return Err(Exception::throw_type(
                            &ctx,
                            "SliderState.new scale must be \"linear\" or \"logarithmic\"",
                        ));
                    }
                };
                let value =
                    slider_value(&ctx, &value, "SliderState.new value")?.ok_or_else(|| {
                        Exception::throw_type(
                            &ctx,
                            "SliderState.new value must be a number, or a pair [start, end]",
                        )
                    })?;
                // `SliderState` asserts on these rather than reporting them,
                // and an assertion here takes the whole application with it.
                // The prelude checks the same three at the call site; this is
                // the backstop that keeps a host bug from aborting.
                let min = native_slider_number(&ctx, min, "SliderState.new min")?;
                let max = native_slider_number(&ctx, max, "SliderState.new max")?;
                let step = native_slider_number(&ctx, step, "SliderState.new step")?;
                let sane = min.is_finite()
                    && max.is_finite()
                    && step.is_finite()
                    && max > min
                    && step > 0.0
                    && (scale.is_linear() || min > 0.0);
                if !sane {
                    return Err(Exception::throw_type(
                        &ctx,
                        "SliderState.new needs a finite min below its max and a positive step, \
                         and a logarithmic scale needs a min above zero",
                    ));
                }

                let store = alive(&ctx, &create_slider)?;
                scope::with_current_app(|cx| {
                    store
                        .entities()
                        .create_slider(
                            min,
                            max,
                            step,
                            scale,
                            value,
                            scope::current_application_generation(),
                            cx,
                        )
                        .map_err(|_| entity_limit_reached(&ctx))
                })
                .ok_or_else(|| {
                    Exception::throw_type(
                        &ctx,
                        "SliderState.new(...) needs a live host call; call it from init() \
                         or an event handler",
                    )
                })?
            },
        ),
    )?;

    let read_slider = runtime.clone();
    globals.set(
        "__slider_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<Vec<f64>> {
                let state = live_slider(&ctx, &read_slider, handle)?;
                scope::with_current_app(|cx| match state.read(cx).value() {
                    SliderValue::Single(value) => vec![f64::from(value)],
                    SliderValue::Range(start, end) => vec![f64::from(start), f64::from(end)],
                })
                .ok_or_else(|| needs_call(&ctx, "value()"))
            },
        ),
    )?;

    let write_slider = runtime.clone();
    globals.set(
        "__slider_set_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, value: Vec<f64>| -> JsResult<()> {
                let state = live_slider(&ctx, &write_slider, handle)?;
                let value = slider_value(&ctx, &value, "set_value(value)")?.ok_or_else(|| {
                    Exception::throw_type(
                        &ctx,
                        "set_value(value) expects a number, or a pair [start, end]",
                    )
                })?;
                // Needs the window as well as the app, because base's own
                // setter does: it takes one so a control that has to re-measure
                // on a new value can.
                scope::with_current(|window, cx| {
                    state.update(cx, |state, cx| state.set_value(value, window, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "set_value()"))
            },
        ),
    )?;

    // One call for all three, because they are read together — a label saying
    // "40 of 100, in steps of 5" is three questions with one answer.
    let bounds_slider = runtime.clone();
    globals.set(
        "__slider_bounds",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<Vec<f64>> {
                let state = live_slider(&ctx, &bounds_slider, handle)?;
                scope::with_current_app(|cx| {
                    let state = state.read(cx);
                    vec![
                        f64::from(state.min_value()),
                        f64::from(state.max_value()),
                        f64::from(state.step_value()),
                    ]
                })
                .ok_or_else(|| needs_call(&ctx, "min_value()"))
            },
        ),
    )?;

    // ── CalendarState ────────────────────────────────────────────────────
    //
    // The state is bound; base's `Calendar` element is not, and that is a
    // decision rather than an omission. `Calendar` walks the month grid calling
    // an item renderer once per cell — up to forty-two calls into the VM per
    // frame, from inside GPUI's layout pass, for a control that is not
    // scrolling. Those cells are also the part with no behavior in them: base's
    // default renderer draws an unstyled box.
    //
    // What a script cannot work out for itself is the grid: which dates fall in
    // which week, where the neighbouring months' days go, and how many weeks
    // this month needs. `month_days()` answers exactly that and is public on
    // the state, so the script asks for the grid during `render` and draws a
    // button per day — what it would have done inside the renderer anyway,
    // minus forty-two boundary crossings.
    //
    // Dates cross as `"YYYY-MM-DD"`: `NaiveDate`'s own `Display`, sortable as
    // text, and readable by `new Date(s)` — so weekday names and localized
    // month labels are the script's, without this boundary inventing a date
    // type.
    let create_calendar = runtime.clone();
    globals.set(
        "__calendar_state_new",
        Func::from(move |ctx: Ctx<'_>| -> JsResult<EntityHandle> {
            refuse_creation_in_render(&ctx, "CalendarState.new()")?;
            let store = alive(&ctx, &create_calendar)?;
            scope::with_current(|window, cx| {
                store
                    .entities()
                    .create_calendar(scope::current_application_generation(), window, cx)
                    .map_err(|_| entity_limit_reached(&ctx))
            })
            .ok_or_else(|| {
                Exception::throw_type(
                    &ctx,
                    "CalendarState.new() needs a live host call; call it from init() or an \
                     event handler",
                )
            })?
        }),
    )?;

    let calendar_days = runtime.clone();
    globals.set(
        "__calendar_month_days",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<Vec<Vec<Vec<String>>>> {
                let state = calendar_state(&ctx, &calendar_days, handle)?;
                scope::with_current_app(|cx| {
                    state
                        .read(cx)
                        .month_days()
                        .into_iter()
                        .map(|weeks| {
                            weeks
                                .into_iter()
                                .map(|days| days.into_iter().map(|day| day.to_string()).collect())
                                .collect()
                        })
                        .collect()
                })
                .ok_or_else(|| needs_call(&ctx, "month_days()"))
            },
        ),
    )?;

    // The month the grid is for, and today. Three readers rather than one
    // answering all three: the prelude spells each as its own method, so a
    // combined call would have every one of them pay for the other two and for
    // the vector carrying them. `CalendarState` has three methods too.
    let calendar_year = runtime.clone();
    globals.set(
        "__calendar_year",
        Func::from(move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<i32> {
            let state = calendar_state(&ctx, &calendar_year, handle)?;
            scope::with_current_app(|cx| state.read(cx).current_year())
                .ok_or_else(|| needs_call(&ctx, "year()"))
        }),
    )?;

    let calendar_month = runtime.clone();
    globals.set(
        "__calendar_month",
        Func::from(move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<u32> {
            let state = calendar_state(&ctx, &calendar_month, handle)?;
            scope::with_current_app(|cx| u32::from(state.read(cx).current_month()))
                .ok_or_else(|| needs_call(&ctx, "month()"))
        }),
    )?;

    let calendar_today = runtime.clone();
    globals.set(
        "__calendar_today",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<String> {
                let state = calendar_state(&ctx, &calendar_today, handle)?;
                scope::with_current_app(|cx| state.read(cx).today().to_string())
                    .ok_or_else(|| needs_call(&ctx, "today()"))
            },
        ),
    )?;

    let calendar_value = runtime.clone();
    globals.set(
        "__calendar_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<Vec<Option<String>>> {
                let state = calendar_state(&ctx, &calendar_value, handle)?;
                scope::with_current_app(|cx| date_to_parts(state.read(cx).date()))
                    .ok_or_else(|| needs_call(&ctx, "value()"))
            },
        ),
    )?;

    let calendar_set_value = runtime.clone();
    globals.set(
        "__calendar_set_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, parts: Vec<Option<String>>| -> JsResult<()> {
                let state = calendar_state(&ctx, &calendar_set_value, handle)?;
                let date = date_from_parts(&ctx, &parts)?;
                scope::with_current(|window, cx| {
                    state.update(cx, |state, cx| state.set_date(date, window, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "set_value()"))
            },
        ),
    )?;

    // Moving the month is a mutation, so it is refused during a render pass for
    // the reason every other one is: a frame that moved the month it was
    // drawing would draw one month and describe another.
    for (name, forward) in [
        ("__calendar_next_month", true),
        ("__calendar_prev_month", false),
    ] {
        let runtime = runtime.clone();
        globals.set(
            name,
            Func::from(move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<()> {
                let api = if forward {
                    "next_month()"
                } else {
                    "prev_month()"
                };
                refuse_creation_in_render(&ctx, api)?;
                let state = calendar_state(&ctx, &runtime, handle)?;
                scope::with_current_app(|cx| {
                    state.update(cx, |state, cx| {
                        if forward {
                            state.next_month();
                        } else {
                            state.prev_month();
                        }
                        cx.notify();
                    });
                })
                .ok_or_else(|| needs_call(&ctx, api))
            }),
        )?;
    }

    let subscribe_calendar_runtime = runtime.clone();
    globals.set(
        "__calendar_on",
        Func::from(
            move |ctx: Ctx<'_>,
                  handle: EntityHandle,
                  name: String,
                  handler: Handler|
                  -> JsResult<bool> {
                subscribe_calendar(&ctx, &subscribe_calendar_runtime, handle, &name, handler)
            },
        ),
    )?;

    let discard_calendar = runtime.clone();
    globals.set(
        "__calendar_release",
        Func::from(move |handle: EntityHandle| {
            discard_calendar
                .upgrade()
                .is_some_and(|runtime| runtime.entities().release(handle))
        }),
    )?;

    let subscribe_slider_runtime = runtime.clone();
    globals.set(
        "__slider_on",
        Func::from(
            move |ctx: Ctx<'_>,
                  handle: EntityHandle,
                  name: String,
                  handler: Handler|
                  -> JsResult<bool> {
                subscribe_slider(&ctx, &subscribe_slider_runtime, handle, &name, handler)
            },
        ),
    )?;

    let discard_slider = runtime.clone();
    globals.set(
        "__slider_release",
        Func::from(move |handle: EntityHandle| {
            discard_slider
                .upgrade()
                .is_some_and(|runtime| runtime.entities().release(handle))
        }),
    )?;

    // Four constructors for one state, because base has four types and each
    // carries a different part of the behavior: the root announces the value,
    // the track takes the press, the indicator records the geometry every
    // pointer position is measured against, and the thumb drags itself.
    slider_constructor(&globals, "__slider_element", &runtime, Component::Slider)?;
    slider_constructor(
        &globals,
        "__slider_track_element",
        &runtime,
        Component::SliderTrack,
    )?;
    slider_constructor(
        &globals,
        "__slider_indicator_element",
        &runtime,
        Component::SliderIndicator,
    )?;
    slider_constructor(
        &globals,
        "__slider_thumb_element",
        &runtime,
        Component::SliderThumb,
    )?;

    // One-time codes. The digits are retained for the reason an input's text
    // is, and the blink for one more: it runs on a timer, and a description
    // only the script can rebuild has nowhere to put a timer's output.
    let create_otp = runtime.clone();
    globals.set(
        "__otp_state_new",
        Func::from(
            move |ctx: Ctx<'_>,
                  length: f64,
                  value: Option<String>,
                  masked: bool|
                  -> JsResult<EntityHandle> {
                refuse_creation_in_render(&ctx, "OtpState.new(...)")?;

                // The prelude refuses the same range at the call site; this is
                // the backstop that keeps a host bug from laying out a hundred
                // thousand cells.
                if !(length.is_finite() && length.fract() == 0.0 && (1.0..=64.0).contains(&length))
                {
                    return Err(Exception::throw_type(
                        &ctx,
                        "OtpState.new(length) expects a whole number between 1 and 64",
                    ));
                }

                let store = alive(&ctx, &create_otp)?;
                scope::with_current(|window, cx| {
                    store
                        .entities()
                        .create_otp(
                            length as usize,
                            value,
                            masked,
                            scope::current_application_generation(),
                            window,
                            cx,
                        )
                        .map_err(|_| entity_limit_reached(&ctx))
                })
                .ok_or_else(|| {
                    Exception::throw_type(
                        &ctx,
                        "OtpState.new(...) needs a live host call; call it from init() \
                         or an event handler",
                    )
                })?
            },
        ),
    )?;

    let read_otp = runtime.clone();
    globals.set(
        "__otp_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<String> {
                let state = live_otp(&ctx, &read_otp, handle)?;
                scope::with_current_app(|cx| state.read(cx).value().to_string())
                    .ok_or_else(|| needs_call(&ctx, "value()"))
            },
        ),
    )?;

    let write_otp = runtime.clone();
    globals.set(
        "__otp_set_value",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, value: String| -> JsResult<()> {
                let state = live_otp(&ctx, &write_otp, handle)?;
                // Deliberately unvalidated, as base is: only keystrokes are
                // digits-only, and base's legacy contract lets a caller display
                // an arbitrary value. Anything past the length is stored and
                // never drawn.
                scope::with_current(|window, cx| {
                    state.update(cx, |state, cx| state.set_value(value, window, cx));
                    refresh_current_view(cx);
                })
                .ok_or_else(|| needs_call(&ctx, "set_value()"))
            },
        ),
    )?;

    let length_otp = runtime.clone();
    globals.set(
        "__otp_len",
        Func::from(move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<f64> {
            let state = live_otp(&ctx, &length_otp, handle)?;
            scope::with_current_app(|cx| state.read(cx).len() as f64)
                .ok_or_else(|| needs_call(&ctx, "len()"))
        }),
    )?;

    let masked_otp = runtime.clone();
    globals.set(
        "__otp_is_masked",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<bool> {
                let state = live_otp(&ctx, &masked_otp, handle)?;
                scope::with_current_app(|cx| state.read(cx).is_masked())
                    .ok_or_else(|| needs_call(&ctx, "is_masked()"))
            },
        ),
    )?;

    let mask_otp = runtime.clone();
    globals.set(
        "__otp_set_masked",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, masked: bool| -> JsResult<()> {
                let state = live_otp(&ctx, &mask_otp, handle)?;
                scope::with_current(|window, cx| {
                    state.update(cx, |state, cx| state.set_masked(masked, window, cx));
                    refresh_current_view(cx);
                })
                .ok_or_else(|| needs_call(&ctx, "set_masked()"))
            },
        ),
    )?;

    let focus_otp = runtime.clone();
    globals.set(
        "__otp_focus",
        Func::from(move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<()> {
            let state = live_otp(&ctx, &focus_otp, handle)?;
            scope::with_current(|window, cx| {
                state.update(cx, |state, cx| state.focus(window, cx));
            })
            .ok_or_else(|| needs_call(&ctx, "focus()"))
        }),
    )?;

    let subscribe_otp_runtime = runtime.clone();
    globals.set(
        "__otp_on",
        Func::from(
            move |ctx: Ctx<'_>,
                  handle: EntityHandle,
                  name: String,
                  handler: Handler|
                  -> JsResult<bool> {
                subscribe_otp(&ctx, &subscribe_otp_runtime, handle, &name, handler)
            },
        ),
    )?;

    let discard_otp = runtime.clone();
    globals.set(
        "__otp_release",
        Func::from(move |handle: EntityHandle| {
            discard_otp
                .upgrade()
                .is_some_and(|runtime| runtime.entities().release(handle))
        }),
    )?;

    let otp_element = runtime.clone();
    globals.set(
        "__otp_element",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<SpecId> {
                let store = alive(&ctx, &otp_element)?;
                if store.entities().otp(handle).is_none() {
                    return Err(Exception::throw_type(
                        &ctx,
                        "this OTP state has been released and can no longer be rendered",
                    ));
                }
                Ok(store.push_component(Component::OtpInput(handle)))
            },
        ),
    )?;

    // Focus handles. A focus handle is not an input, but it is retained state
    // held by handle for the same reason, so it lives in the same store and
    // reaches the script the same way.
    let create_focus = runtime.clone();
    globals.set(
        "__focus_handle_new",
        Func::from(move |ctx: Ctx<'_>| -> JsResult<EntityHandle> {
            let phase = scope::current_phase();
            if matches!(phase, Some(ScopePhase::Render) | Some(ScopePhase::Layout)) {
                return Err(Exception::throw_type(
                    &ctx,
                    "cx.focus_handle() cannot run during render; a handle created there would \
                     be a new one every frame, so the focus it tracks would be dropped by the \
                     next repaint. Create it in init() or in an event handler and keep it on \
                     the view",
                ));
            }

            let store = alive(&ctx, &create_focus)?;
            scope::with_current_app(|cx| {
                store
                    .entities()
                    .create_focus(scope::current_application_generation(), cx)
                    .map_err(|_| entity_limit_reached(&ctx))
            })
            .ok_or_else(|| {
                Exception::throw_type(
                    &ctx,
                    "cx.focus_handle() needs a live host call; call it from init() or an \
                     event handler",
                )
            })?
        }),
    )?;

    let take_focus = runtime.clone();
    globals.set(
        "__focus_focus",
        Func::from(move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<()> {
            refuse_mutation_in_render(&ctx, "FocusHandle.focus()")?;
            let focus = live_focus(&ctx, &take_focus, handle)?;
            scope::with_current(|window, cx| window.focus(&focus, cx))
                .ok_or_else(|| needs_call(&ctx, "focus()"))
        }),
    )?;

    let read_focus = runtime.clone();
    globals.set(
        "__focus_is_focused",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<bool> {
                let focus = live_focus(&ctx, &read_focus, handle)?;
                // Through the window rather than the handle alone: focus is a
                // fact about one window, and `is_focused` needs it to answer.
                scope::with_current(|window, _| focus.is_focused(window))
                    .ok_or_else(|| needs_call(&ctx, "is_focused()"))
            },
        ),
    )?;

    let discard_focus = runtime.clone();
    globals.set(
        "__focus_release",
        Func::from(move |handle: EntityHandle| {
            discard_focus
                .upgrade()
                .is_some_and(|runtime| runtime.entities().release(handle))
        }),
    )?;

    // A virtualized list's scroll position, held for the same reason a focus
    // handle is: what the script holds is *where a request is left*.
    // `scroll_to_item` records an index the list consumes during its next
    // prepaint, so a handle rebuilt each frame would drop every request made
    // between two frames — which is every request a script can make.
    let create_virtual_scroll = runtime.clone();
    globals.set(
        "__virtual_scroll_new",
        Func::from(move |ctx: Ctx<'_>| -> JsResult<EntityHandle> {
            refuse_creation_in_render(&ctx, "VirtualListScrollHandle.new()")?;
            let store = alive(&ctx, &create_virtual_scroll)?;
            store
                .entities()
                .create_virtual_scroll(scope::current_application_generation())
                .map_err(|_| entity_limit_reached(&ctx))
        }),
    )?;

    let scroll_to_item = runtime.clone();
    globals.set(
        "__virtual_scroll_to_item",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle, index: usize, strategy: String| {
                let scroll = live_virtual_scroll(&ctx, &scroll_to_item, handle)?;
                let strategy = match strategy.as_str() {
                    "center" => ScrollStrategy::Center,
                    "top" => ScrollStrategy::Top,
                    other => {
                        return Err(Exception::throw_type(
                            &ctx,
                            &format!("unknown scroll strategy `{other}`; expected top or center"),
                        ));
                    }
                };
                // Recorded, not performed: the list is the only thing that
                // knows where item `index` is, and it finds out during its next
                // prepaint. Nothing here needs a live host call for that
                // reason — the request outlives this one.
                scroll.scroll_to_item(index, strategy);
                Ok(())
            },
        ),
    )?;

    let scroll_to_bottom = runtime.clone();
    globals.set(
        "__virtual_scroll_to_bottom",
        Func::from(move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<()> {
            live_virtual_scroll(&ctx, &scroll_to_bottom, handle)?.scroll_to_bottom();
            Ok(())
        }),
    )?;

    let discard_virtual_scroll = runtime.clone();
    globals.set(
        "__virtual_scroll_release",
        Func::from(move |handle: EntityHandle| {
            discard_virtual_scroll
                .upgrade()
                .is_some_and(|runtime| runtime.entities().release(handle))
        }),
    )?;

    let discard = runtime.clone();
    globals.set(
        "__input_release",
        Func::from(move |handle: EntityHandle| {
            discard
                .upgrade()
                .is_some_and(|runtime| runtime.entities().release(handle))
        }),
    )?;

    let number_input_element = runtime.clone();
    globals.set(
        "__number_input_element",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<SpecId> {
                let store = alive(&ctx, &number_input_element)?;
                if store.entities().input(handle).is_none() {
                    return Err(Exception::throw_type(
                        &ctx,
                        "this input state has been released and can no longer be rendered",
                    ));
                }
                Ok(store.push_component(Component::NumberInput(handle)))
            },
        ),
    )?;

    globals.set(
        "__input_element",
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<SpecId> {
                let store = alive(&ctx, &runtime)?;
                if store.entities().input(handle).is_none() {
                    return Err(Exception::throw_type(
                        &ctx,
                        "this input state has been released and can no longer be rendered",
                    ));
                }
                Ok(store.push_component(Component::Input(handle)))
            },
        ),
    )?;

    Ok(())
}

fn live(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
) -> JsResult<gpui::Entity<gpui_base::input::InputState>> {
    alive(ctx, runtime)?
        .entities()
        .input(handle)
        .ok_or_else(|| Exception::throw_type(ctx, "this input state has been released"))
}

fn live_textarea(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
) -> JsResult<gpui::Entity<gpui_base::input::TextareaState>> {
    alive(ctx, runtime)?
        .entities()
        .textarea(handle)
        .ok_or_else(|| Exception::throw_type(ctx, "this textarea state has been released"))
}

/// Reads the text out of either state.
///
/// Generic over the mode marker because `value()` lives on the shared engine:
/// the single-line and multi-line states answer it the same way, and only the
/// resolver that produced the entity differs.
fn read_value<M: InputModeKind>(
    ctx: &Ctx<'_>,
    state: &gpui::Entity<InputBaseState<M>>,
) -> JsResult<String> {
    scope::with_current_app(|cx| state.read(cx).value().to_string())
        .ok_or_else(|| needs_call(ctx, "value()"))
}

fn write_value<M: InputModeKind>(
    ctx: &Ctx<'_>,
    state: &gpui::Entity<InputBaseState<M>>,
    value: String,
) -> JsResult<()> {
    scope::with_current(|window, cx| {
        state.update(cx, |state, cx| state.set_value(value, window, cx));
    })
    .ok_or_else(|| needs_call(ctx, "set_value()"))
}

/// Subscribes a script handler to one named event on either text state.
///
/// The store resolves the handle to whichever entity it names, so the whole of
/// this — the event-name check, the captured grant, the released-state report —
/// is shared; `what` only names the state in the error a script sees.
fn subscribe(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
    name: &str,
    handler: Handler,
    what: &str,
) -> JsResult<bool> {
    let event = InputEventName::from_name(name).ok_or_else(|| {
        Exception::throw_type(
            ctx,
            &format!(
                "unknown input event `{name}`; expected one of: {}",
                InputEventName::NAMES.join(", ")
            ),
        )
    })?;

    let saved = handler.0;
    let dispatch = runtime.clone();
    let store = alive(ctx, runtime)?;
    // Captured here, not read when the event arrives: this subscription
    // outlives the call that made it, and an input on a plugin's form must
    // dispatch under that plugin's grant rather than under whatever the default
    // policy happens to be.
    let owner = InputCallbackOwner {
        policy: scope::policy(),
        application: scope::current_application_generation(),
        view: scope::current_view().map(|view| view.downgrade()),
    };

    let subscribed = scope::with_current(|window, cx| {
        store
            .entities()
            .subscribe_input(handle, event, window, cx, move |emitted, window, cx| {
                let Some(runtime) = dispatch.upgrade() else {
                    return;
                };
                runtime.dispatch_input_event(&saved, &owner, emitted, window, cx);
            })
    })
    .ok_or_else(|| {
        Exception::throw_type(
            ctx,
            "on(...) needs a live host call; subscribe from init() or an event handler",
        )
    })?;

    if !subscribed {
        return Err(Exception::throw_type(
            ctx,
            &format!("this {what} state has been released"),
        ));
    }
    Ok(true)
}

fn live_slider(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
) -> JsResult<gpui::Entity<gpui_base::slider::SliderState>> {
    alive(ctx, runtime)?
        .entities()
        .slider(handle)
        .ok_or_else(|| Exception::throw_type(ctx, "this slider state has been released"))
}

/// One number is one thumb; two are the ends of a range.
///
/// The pair crosses as an array in both directions because a bare number
/// cannot say which of the two the script meant, and a range slider told to
/// take a single value would silently become a single-value one.
fn slider_value(ctx: &Ctx<'_>, values: &[f64], api: &str) -> JsResult<Option<SliderValue>> {
    match values {
        [single] => Ok(Some(SliderValue::Single(native_slider_number(
            ctx, *single, api,
        )?))),
        [start, end] => Ok(Some(SliderValue::Range(
            native_slider_number(ctx, *start, api)?,
            native_slider_number(ctx, *end, api)?,
        ))),
        _ => Ok(None),
    }
}

fn native_slider_number(ctx: &Ctx<'_>, value: f64, name: &str) -> JsResult<f32> {
    let narrowed = value as f32;
    if !narrowed.is_finite() {
        return Err(Exception::throw_range(
            ctx,
            &format!("{name} does not fit the native slider number range"),
        ));
    }
    Ok(narrowed)
}

/// Subscribes a script handler to one named event on a slider state.
///
/// The shape is [`subscribe`]'s, and so are its reasons — the captured grant,
/// the released-state report, the subscription owned by the store rather than
/// by the script. What differs is the payload: a slider event carries a value,
/// which is what the handler is called with.
fn subscribe_slider(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
    name: &str,
    handler: Handler,
) -> JsResult<bool> {
    let event = SliderEventName::from_name(name).ok_or_else(|| {
        Exception::throw_type(
            ctx,
            &format!(
                "unknown slider event `{name}`; expected one of: {}",
                SliderEventName::NAMES.join(", ")
            ),
        )
    })?;

    let saved = handler.0;
    let dispatch = runtime.clone();
    let store = alive(ctx, runtime)?;
    let owner = InputCallbackOwner {
        policy: scope::policy(),
        application: scope::current_application_generation(),
        view: scope::current_view().map(|view| view.downgrade()),
    };

    let subscribed = scope::with_current(|window, cx| {
        store
            .entities()
            .subscribe_slider(handle, event, window, cx, move |value, window, cx| {
                let Some(runtime) = dispatch.upgrade() else {
                    return;
                };
                runtime.dispatch_slider_event(&saved, &owner, value, window, cx);
            })
    })
    .ok_or_else(|| {
        Exception::throw_type(
            ctx,
            "on(...) needs a live host call; subscribe from init() or an event handler",
        )
    })?;

    if !subscribed {
        return Err(Exception::throw_type(
            ctx,
            "this slider state has been released",
        ));
    }
    Ok(true)
}

/// A constructor taking a live slider handle rather than an id.
///
/// The four parts differ only in which component they push, and all four have
/// to refuse a released handle: an element built from one would be a slider
/// with no state behind it, which materializes to nothing and says why only in
/// the log.
fn slider_constructor(
    globals: &Object<'_>,
    name: &str,
    runtime: &Weak<ShellRuntime>,
    build: fn(EntityHandle) -> Component,
) -> JsResult<()> {
    let runtime = runtime.clone();
    globals.set(
        name,
        Func::from(
            move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<SpecId> {
                let store = alive(&ctx, &runtime)?;
                if store.entities().slider(handle).is_none() {
                    return Err(Exception::throw_type(
                        &ctx,
                        "this slider state has been released and can no longer be rendered",
                    ));
                }
                Ok(store.push_component(build(handle)))
            },
        ),
    )
}

fn live_otp(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
) -> JsResult<gpui::Entity<gpui_base::OtpState>> {
    alive(ctx, runtime)?
        .entities()
        .otp(handle)
        .ok_or_else(|| Exception::throw_type(ctx, "this OTP state has been released"))
}

/// Refreshes script-derived presentation after a controlled retained-state
/// mutation. During render/layout mutation is outside the notify contract, and
/// trying to re-enter the view being rendered would panic.
fn refresh_current_view(cx: &mut gpui::App) {
    if !scope::current_phase().is_some_and(ScopePhase::allows_notify) {
        return;
    }
    if let Some(view) = scope::current_view() {
        view.update(cx, |view, cx| view.refresh(cx));
    }
}

/// Subscribes a script handler to one named event on a one-time-code state.
///
/// The shape is [`subscribe`]'s, and so are its reasons. What differs is the
/// set of names: an `OtpState` has no `submit` to offer, so asking for one is
/// reported rather than accepted and then never delivered.
fn subscribe_otp(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
    name: &str,
    handler: Handler,
) -> JsResult<bool> {
    let event = OtpEventName::from_name(name).ok_or_else(|| {
        Exception::throw_type(
            ctx,
            &format!(
                "unknown OTP event `{name}`; expected one of: {}",
                OtpEventName::NAMES.join(", ")
            ),
        )
    })?;

    let saved = handler.0;
    let dispatch = runtime.clone();
    let store = alive(ctx, runtime)?;
    let owner = InputCallbackOwner {
        policy: scope::policy(),
        application: scope::current_application_generation(),
        view: scope::current_view().map(|view| view.downgrade()),
    };

    let subscribed = scope::with_current(|window, cx| {
        store
            .entities()
            .subscribe_otp(handle, event, window, cx, move |emitted, window, cx| {
                let Some(runtime) = dispatch.upgrade() else {
                    return;
                };
                runtime.dispatch_otp_event(&saved, &owner, emitted, window, cx);
            })
    })
    .ok_or_else(|| {
        Exception::throw_type(
            ctx,
            "on(...) needs a live host call; subscribe from init() or an event handler",
        )
    })?;

    if !subscribed {
        return Err(Exception::throw_type(
            ctx,
            "this OTP state has been released",
        ));
    }
    Ok(true)
}

/// Refuses to create retained state during a render pass.
///
/// State created there would be new on every frame, so what the script thought
/// it was keeping — the text typed into it, the focus on it — would be dropped
/// by the next repaint.
fn refuse_creation_in_render(ctx: &Ctx<'_>, constructor: &str) -> JsResult<()> {
    if matches!(
        scope::current_phase(),
        Some(ScopePhase::Render) | Some(ScopePhase::Layout)
    ) {
        return Err(Exception::throw_type(
            ctx,
            &format!(
                "{constructor} cannot run during render; create state in init() or in an \
                 event handler and keep it on the view"
            ),
        ));
    }
    Ok(())
}

/// What a store that has reached [`MAX_LIVE_ENTITIES`] reads as in script.
///
/// Every retained thing is created through one fallible `EntityStore`
/// constructor, so this is the one translation of that refusal and the reason
/// no constructor carries a capacity check of its own.
///
/// [`MAX_LIVE_ENTITIES`]: crate::entities::MAX_LIVE_ENTITIES
pub(super) fn entity_limit_reached(ctx: &Ctx<'_>) -> rquickjs::Error {
    Exception::throw_range(
        ctx,
        "the application reached gpui-shell's retained entity limit; release unused handles",
    )
}

fn refuse_mutation_in_render(ctx: &Ctx<'_>, api: &str) -> JsResult<()> {
    if matches!(
        scope::current_phase(),
        Some(ScopePhase::Render) | Some(ScopePhase::Layout)
    ) {
        return Err(Exception::throw_type(
            ctx,
            &format!(
                "{api} cannot run during render or layout; mutate state from an event handler or task"
            ),
        ));
    }
    Ok(())
}

fn live_virtual_scroll(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
) -> JsResult<VirtualListScrollHandle> {
    alive(ctx, runtime)?
        .entities()
        .virtual_scroll(handle)
        .ok_or_else(|| Exception::throw_type(ctx, "this scroll handle has been released"))
}

fn live_focus(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
) -> JsResult<gpui::FocusHandle> {
    alive(ctx, runtime)?
        .entities()
        .focus(handle)
        .ok_or_else(|| Exception::throw_type(ctx, "this focus handle has been released"))
}

/// The runtime this handle's store belongs to, or a clear failure.
///
/// A `Weak` that no longer upgrades means the VM is being torn down while a
/// script call is still on the stack, which is a host bug rather than anything
/// the author wrote.
/// The calendar state behind a handle, or a thrown error naming what went wrong.
fn calendar_state(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
) -> JsResult<gpui::Entity<gpui_base::CalendarState>> {
    alive(ctx, runtime)?
        .entities()
        .calendar(handle)
        .ok_or_else(|| {
            Exception::throw_type(
                ctx,
                "this calendar state has been released; a handle cannot be used after release()",
            )
        })
}

/// A `Date` on the wire: one slot for a single day, two for a range.
///
/// The slot *count* is what carries the variant, and it has to. A single day
/// and a range whose end is not chosen yet hold the same one date, and both
/// render as the same string — but base branches on the difference in
/// `is_single`, `is_complete` and `is_in_range`, so a wire that dropped it
/// would quietly turn every `set_value("2026-08-15")` into a half-open range.
///
/// The prelude narrows this to `null`, a string or a pair before a script sees
/// it, and widens it back the same way. Shared with the event dispatch so the
/// two directions cannot drift.
pub(super) fn date_to_parts(date: gpui_base::Date) -> Vec<Option<String>> {
    match date {
        gpui_base::Date::Single(day) => vec![day.map(|day| day.to_string())],
        gpui_base::Date::Range(start, end) => vec![
            start.map(|day| day.to_string()),
            end.map(|day| day.to_string()),
        ],
    }
}

/// The same slots, read back. Two of them means a range was meant.
fn date_from_parts(ctx: &Ctx<'_>, parts: &[Option<String>]) -> JsResult<gpui_base::Date> {
    let day = |slot: Option<&String>| -> JsResult<Option<chrono::NaiveDate>> {
        let Some(text) = slot else {
            return Ok(None);
        };
        chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d")
            .map(Some)
            .map_err(|error| {
                Exception::throw_type(
                    ctx,
                    &format!("`{text}` is not a date in the form \"YYYY-MM-DD\": {error}"),
                )
            })
    };
    match parts {
        [single] => Ok(gpui_base::Date::Single(day(single.as_ref())?)),
        [start, end] => Ok(gpui_base::Date::Range(
            day(start.as_ref())?,
            day(end.as_ref())?,
        )),
        _ => Err(Exception::throw_type(
            ctx,
            "a calendar date is null, \"YYYY-MM-DD\", or a two-element array of those",
        )),
    }
}

/// Registers the calendar's one subscription.
fn subscribe_calendar(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    handle: EntityHandle,
    name: &str,
    handler: Handler,
) -> JsResult<bool> {
    if name != "change" {
        return Err(Exception::throw_type(
            ctx,
            &format!(
                "unknown calendar event `{name}`; the only one is \"change\", which reports \
                 a date being selected"
            ),
        ));
    }

    let saved = handler.0;
    let dispatch = runtime.clone();
    let store = alive(ctx, runtime)?;
    let owner = InputCallbackOwner {
        policy: scope::policy(),
        application: scope::current_application_generation(),
        view: scope::current_view().map(|view| view.downgrade()),
    };

    let subscribed = scope::with_current(|window, cx| {
        store
            .entities()
            .subscribe_calendar(handle, window, cx, move |event, window, cx| {
                let Some(runtime) = dispatch.upgrade() else {
                    return;
                };
                let gpui_base::CalendarEvent::Selected(date) = event;
                runtime.dispatch_calendar_event(&saved, &owner, *date, window, cx);
            })
    })
    .ok_or_else(|| {
        Exception::throw_type(
            ctx,
            "on(...) needs a live host call; subscribe from init() or an event handler",
        )
    })?;

    if !subscribed {
        return Err(Exception::throw_type(
            ctx,
            "this calendar state has been released; a handle cannot be used after release()",
        ));
    }
    Ok(subscribed)
}

fn alive(ctx: &Ctx<'_>, runtime: &Weak<ShellRuntime>) -> JsResult<std::rc::Rc<ShellRuntime>> {
    runtime
        .upgrade()
        .ok_or_else(|| Exception::throw_message(ctx, "the runtime has shut down"))
}

fn needs_call(ctx: &Ctx<'_>, what: &str) -> rquickjs::Error {
    Exception::throw_type(
        ctx,
        &format!("{what} needs a live host call; call it from render or an event handler"),
    )
}
