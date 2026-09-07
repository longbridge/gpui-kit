//! The QuickJS engine.
//!
//! Application code is JavaScript: ES modules, classes, arrow functions. This
//! module is the only place that knows that — everything above `engine/` deals
//! in [`SpecId`]s, [`Bridged`] values and [`ShellError`]s.
//!
//! Two shapes are worth knowing before reading:
//!
//! - **Elements are plain JS objects** carrying an integer `__id`, sharing one
//!   prototype that holds every bound method. A method call is therefore an
//!   ordinary prototype lookup rather than a proxy trap, which matters because
//!   per-call cost is the whole viability question (design doc §20).
//! - **The prototype is built by a JS prelude**, not by 3000 Rust closures: the
//!   prelude loops over the style-name list and installs one small JS function
//!   per name, each forwarding to a single Rust entry point.

use std::{
    cell::{Cell, RefCell, RefMut},
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    path::Path,
    rc::{Rc, Weak},
};

use anyhow::{Context as _, Result, anyhow};
use gpui::{App, AppContext as _, ClickEvent, Entity, Global, Subscription, WeakEntity, Window};
use rquickjs::{
    Array, Context as JsContext, Ctx, Error as JsError, Exception, FromJs, Function, Object,
    Persistent, Result as JsResult, Value,
    function::{Args as JsArgs, Func, Opt, This},
    loader::{BuiltinResolver, ImportAttributes, Loader, ModuleLoader, Resolver},
    module::Declared,
    module::{Declarations, Exports, Module, ModuleDef},
};
use rquickjs_jit::{JitConfig, JitRuntime};
use smallvec::SmallVec;

use crate::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallbackArgument,
    ComponentCallbackValue, ComponentDataValue, ComponentPayload, FrozenComponentRegistry,
    dependencies::{GitDependencyStore, MaterializedDependency},
    entities::{EntityHandle, EntityStore},
    host_modules::HostValue,
    metrics::Metrics,
    policy::Policy,
    runtime::{ApplicationGeneration, CallbackArena, CallbackEntry},
    scope::{self, ScopePhase},
    snapshot::RenderSnapshot,
    spec::{CallbackId, ChildViewSpec, Component, SpecArena, SpecId, SpecOp},
    style,
    value::Bridged,
    view::ScriptView,
};

const MAX_MODULE_BYTES: u64 = 8 * 1024 * 1024;

/// A script value that defines a view type — a JS class.
#[derive(Clone)]
pub struct ViewType {
    value: Persistent<Object<'static>>,
    module_lease: Option<ApplicationModuleLease>,
    application: Option<Rc<ApplicationGeneration>>,
}

#[cfg(test)]
mod retained_component_state_tests {
    use super::*;
    use gpui::{TestAppContext, VisualTestContext};

    #[test]
    fn immutable_state_lookup_reports_reentrant_mutable_borrow() {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let handle = runtime
            .component_states
            .borrow_mut()
            .insert("State", None, Box::new(1usize))
            .unwrap();
        let _borrow = runtime.component_states.borrow_mut();

        let error = runtime
            .with_component_state::<usize, _>(handle, "State", |_| ())
            .unwrap_err();

        assert!(error.to_string().contains("already mutably borrowed"));
    }

    #[test]
    fn root_drop_release_path_purges_generation_owned_state() {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let application = ApplicationGeneration::new(92);
        runtime
            .component_states
            .borrow_mut()
            .insert("State", Some(application.clone()), Box::new(()))
            .unwrap();

        runtime.release_application_generation_without_context(&application);

        assert_eq!(runtime.retained_component_state_count(), 0);
        assert!(!application.is_active());
    }

    #[test]
    fn successful_reload_release_purges_old_generation_and_preserves_new() {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let old = ApplicationGeneration::new(94);
        let new = ApplicationGeneration::new(95);
        let old_handle = runtime
            .component_states
            .borrow_mut()
            .insert("State", Some(old.clone()), Box::new(1usize))
            .unwrap();
        let new_handle = runtime
            .component_states
            .borrow_mut()
            .insert("State", Some(new.clone()), Box::new(2usize))
            .unwrap();

        runtime.release_application_generation_without_context(&old);

        assert!(
            runtime
                .with_component_state::<usize, _>(old_handle, "State", |_| ())
                .is_err()
        );
        assert_eq!(
            runtime
                .with_component_state::<usize, _>(new_handle, "State", |value| *value)
                .unwrap(),
            2
        );
        assert!(new.is_active());
    }

    #[gpui::test]
    fn generation_release_during_state_update_is_deferred_then_guaranteed(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let application = ApplicationGeneration::new(93);
        let handle = runtime
            .component_states
            .borrow_mut()
            .insert("State", Some(application.clone()), Box::new(1usize))
            .unwrap();
        for value in 1..crate::component_registry::MAX_RETAINED_COMPONENT_STATES {
            runtime
                .component_states
                .borrow_mut()
                .insert("State", Some(application.clone()), Box::new(value))
                .unwrap();
        }
        assert!(
            runtime
                .component_states
                .borrow_mut()
                .insert("State", None, Box::new(()))
                .is_err()
        );
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);

        context
            .update(|window, cx| {
                runtime.update_component_state::<usize, _>(
                    handle,
                    "State",
                    window,
                    cx,
                    |value, _, _| {
                        *value += 1;
                        runtime.release_application_generation_without_context(&application);
                    },
                )
            })
            .unwrap();

        assert_eq!(runtime.retained_component_state_count(), 0);
        assert!(!application.is_active());
        runtime
            .component_states
            .borrow_mut()
            .insert("State", None, Box::new(()))
            .expect("released generation must recover state capacity");
    }

    #[gpui::test]
    fn state_update_reports_reentrant_immutable_borrow(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let handle = runtime
            .component_states
            .borrow_mut()
            .insert("State", None, Box::new(1usize))
            .unwrap();
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let _borrow = runtime.component_states.borrow();

        let error = context
            .update(|window, cx| {
                runtime.update_component_state::<usize, _>(
                    handle,
                    "State",
                    window,
                    cx,
                    |_, _, _| (),
                )
            })
            .unwrap_err();

        assert!(error.to_string().contains("already borrowed"));
    }

    #[gpui::test]
    fn state_update_is_rejected_under_render_authority(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let handle = runtime
            .component_states
            .borrow_mut()
            .insert("State", None, Box::new(1usize))
            .unwrap();
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);

        let error = context
            .update(|window, cx| {
                let (_scope, _) =
                    scope::enter_runtime(&runtime, window, cx, ScopePhase::Render, None);
                runtime.update_component_state::<usize, _>(
                    handle,
                    "State",
                    window,
                    cx,
                    |_, _, _| (),
                )
            })
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("cannot be updated during render")
        );
    }
}

#[cfg(test)]
mod jit_suspension_tests {
    use super::*;

    #[test]
    fn nested_suspension_preserves_the_outer_state() {
        let runtime = ShellRuntime::new_isolated().unwrap();

        runtime
            .with_jit_suspended(|| {
                assert!(runtime.js_runtime.jit().is_suspended());
                runtime.with_jit_suspended(|| Ok(()))?;
                assert!(runtime.js_runtime.jit().is_suspended());
                Ok(())
            })
            .unwrap();

        assert!(!runtime.js_runtime.jit().is_suspended());
    }
}

#[cfg(test)]
mod component_callback_value_tests {
    use super::*;
    use gpui::{Empty, TestAppContext, VisualTestContext};
    use std::ops::Deref;

    #[test]
    fn returning_to_the_installed_app_effect_cancels_a_pending_replacement() {
        let key = "menu".to_owned();
        let mut pending = HashMap::new();
        let mut installed = HashMap::from([(
            key.clone(),
            InstalledAppEffect {
                revision: "a".into(),
                cleanup: None,
            },
        )]);

        assert!(queue_component_app_effect(
            &mut pending,
            &installed,
            &key,
            "b"
        ));
        assert_eq!(pending.get(&key).map(String::as_str), Some("b"));
        assert!(!queue_component_app_effect(
            &mut pending,
            &installed,
            &key,
            "a"
        ));
        assert!(!pending.contains_key(&key));

        installed.get_mut(&key).unwrap().revision = "b".into();
        assert!(queue_component_app_effect(
            &mut pending,
            &installed,
            &key,
            "a"
        ));
    }

    fn callback(
        runtime: &Rc<ShellRuntime>,
        source: &str,
        application: Option<Rc<ApplicationGeneration>>,
    ) -> (CallbackId, u64) {
        let value = runtime
            .with_js(|ctx| {
                let function: Function<'_> = ctx.eval(source)?;
                Ok(Persistent::save(&ctx, function))
            })
            .unwrap();
        let mut callbacks = runtime.callbacks.borrow_mut();
        let generation = callbacks.begin();
        let id = callbacks.push(CallbackEntry {
            value,
            view: None,
            application,
            registered_in: None,
        });
        callbacks.commit();
        (id, generation)
    }

    #[gpui::test]
    fn component_callback_results_are_closed_and_jobs_are_drained(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let (callback, _) = callback(
            &runtime,
            r#"(kind) => {
                if (kind === "undefined") return undefined;
                if (kind === "null") return null;
                if (kind === "bool") return true;
                if (kind === "number") return 2.5;
                if (kind === "nan") return NaN;
                if (kind === "infinity") return Infinity;
                if (kind === "negative-infinity") return -Infinity;
                if (kind === "function") return () => {};
                if (kind === "element") return { __id: 7 };
                if (kind === "promise") return Promise.resolve("later");
                if (kind === "rejected-promise") return Promise.reject(new Error("later"));
                Promise.resolve().then(() => { globalThis.__componentJobDrained = true; });
                return "queued";
            }"#,
            None,
        );
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let invoke = |kind: &str, context: &mut VisualTestContext| {
            context.update(|window, cx| {
                runtime.dispatch_component_callback_value(
                    callback,
                    &[ComponentCallbackArgument::String(kind.into())],
                    window,
                    cx,
                )
            })
        };

        assert_eq!(
            invoke("undefined", &mut context).unwrap(),
            ComponentCallbackValue::Null
        );
        assert_eq!(
            invoke("null", &mut context).unwrap(),
            ComponentCallbackValue::Null
        );
        assert_eq!(
            invoke("bool", &mut context).unwrap(),
            ComponentCallbackValue::Boolean(true)
        );
        assert_eq!(
            invoke("number", &mut context).unwrap(),
            ComponentCallbackValue::Number(2.5)
        );
        for kind in [
            "nan",
            "infinity",
            "negative-infinity",
            "function",
            "element",
            "promise",
            "rejected-promise",
        ] {
            assert!(invoke(kind, &mut context).unwrap_err().to_string().contains(
                "component callbacks may only return null, boolean, finite number, or string"
            ));
        }
        assert_eq!(
            invoke("queued", &mut context).unwrap(),
            ComponentCallbackValue::String("queued".into())
        );
        assert!(
            runtime
                .with_js(|ctx| ctx.eval::<bool, _>("globalThis.__componentJobDrained === true"))
                .unwrap()
        );
    }

    #[gpui::test]
    fn component_delegate_data_is_recursive_bounded_and_does_not_drain_jobs(
        cx: &mut TestAppContext,
    ) {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let (id, _) = callback(
            &runtime,
            r#"() => { Promise.resolve().then(() => globalThis.__delegateJob = true); return {id: "a", cells: [1, true, null]}; }"#,
            None,
        );
        let data_callback = crate::ComponentDataCallback::from_runtime(&runtime, id);
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let value = context
            .update(|window, cx| data_callback.snapshot_with(&[], window, cx))
            .unwrap();
        let crate::ComponentDataValue::Object(fields) = value else {
            panic!("object")
        };
        assert_eq!(fields[0].0, "id");
        assert!(
            !runtime
                .with_js(|ctx| ctx.eval::<bool, _>("globalThis.__delegateJob === true"))
                .unwrap()
        );
        let (leaky, _) = callback(&runtime, "() => { __div(); return {ok: true}; }", None);
        let leaky = crate::ComponentDataCallback::from_runtime(&runtime, leaky);
        let arena_len = runtime.arena.borrow().len();
        for _ in 0..2 {
            context
                .update(|window, cx| leaky.snapshot_with(&[], window, cx))
                .unwrap();
            assert_eq!(runtime.arena.borrow().len(), arena_len);
        }
    }

    #[gpui::test]
    fn component_delegate_element_is_lazy_and_rejects_non_elements(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let (good, _) = callback(&runtime, "(label) => label", None);
        let (bad, _) = callback(&runtime, "() => ({row: 1})", None);
        let good = crate::ComponentElementCallback::from_runtime(&runtime, good);
        let bad = crate::ComponentElementCallback::from_runtime(&runtime, bad);
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        assert!(
            context
                .update(|window, cx| good.build_with(
                    &[ComponentCallbackArgument::String("row".into())],
                    window,
                    cx
                ))
                .unwrap()
                .is_some()
        );
        assert!(
            context
                .update(|window, cx| bad.build_with(&[], window, cx))
                .is_err()
        );
        let row = ComponentDataValue::Object(vec![
            ("label".into(), ComponentDataValue::String("Ada".into())),
            (
                "__proto__".into(),
                ComponentDataValue::String("safe".into()),
            ),
        ]);
        let (row_renderer, _) = callback(
            &runtime,
            "(row) => Object.getPrototypeOf(row) === null && Object.prototype.hasOwnProperty.call(row, '__proto__') && row.__proto__ === 'safe' ? row.label : ({bad: true})",
            None,
        );
        let row_renderer = crate::ComponentElementCallback::from_runtime(&runtime, row_renderer);
        assert!(
            context
                .update(|window, cx| row_renderer.build_data_with(&[row], window, cx))
                .unwrap()
                .is_some()
        );
    }

    #[gpui::test]
    fn temporary_delegate_arena_restores_after_panic_and_later_materializes(
        cx: &mut TestAppContext,
    ) {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let before = runtime.arena.borrow().len();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            context.update(|window, cx| {
                let (_scope, _) = scope::enter_with_application(
                    &runtime,
                    window,
                    cx,
                    ScopePhase::Layout,
                    None,
                    crate::policy::default(),
                    None,
                );
                let _temporary = TemporarySpecArena::enter(&runtime);
                runtime
                    .with_js(|ctx| {
                        let div: Function = ctx.globals().get("__div")?;
                        div.call::<_, ()>(())
                    })
                    .unwrap();
                panic!("delegate dispatch scope panic probe");
            })
        }));
        assert!(panic.is_err());
        assert_eq!(scope::current_phase(), None);
        assert_eq!(runtime.arena.borrow().len(), before);
        drop(context);
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let (id, _) = callback(&runtime, "() => 'still works'", None);
        let callback = crate::ComponentElementCallback::from_runtime(&runtime, id);
        assert!(
            context
                .update(|window, cx| callback.build_with(&[], window, cx))
                .unwrap()
                .is_some()
        );
    }

    #[gpui::test]
    fn component_delegate_capabilities_reject_invalid_data_and_stale_lifetimes(
        cx: &mut TestAppContext,
    ) {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let application = ApplicationGeneration::new(991);
        let (invalid, _) = callback(
            &runtime,
            r#"(kind) => {
                if (kind === "promise") return Promise.resolve(1);
                if (kind === "function") return () => {};
                if (kind === "element") return {__id: 7};
                if (kind === "accessor") return Object.defineProperty({}, "value", {get() { globalThis.__delegateAccessorRan = true; throw new Error("getter ran"); }, enumerable: true});
                if (kind === "prototype") return Object.create({inherited: true});
                return Infinity;
            }"#,
            None,
        );
        let invalid = crate::ComponentDataCallback::from_runtime(&runtime, invalid);
        let (stale, generation) = callback(&runtime, "()=>[]", None);
        let stale = crate::ComponentDataCallback::from_runtime(&runtime, stale);
        let (retired, _) = callback(&runtime, "()=>[]", Some(application.clone()));
        let retired = crate::ComponentElementCallback::from_runtime(&runtime, retired);
        let (notify, _) = callback(&runtime, "(cx)=>{ cx.notify(); return []; }", None);
        let notify = crate::ComponentDataCallback::from_runtime(&runtime, notify);
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        for kind in [
            "promise",
            "function",
            "element",
            "accessor",
            "prototype",
            "number",
        ] {
            assert!(
                context
                    .update(|window, cx| invalid.snapshot_with(
                        &[ComponentCallbackArgument::String(kind.into())],
                        window,
                        cx
                    ))
                    .is_err()
            );
        }
        assert!(
            !runtime
                .with_js(|ctx| ctx.eval::<bool, _>("globalThis.__delegateAccessorRan === true"))
                .unwrap(),
            "delegate validation must reject accessors without invoking their getter"
        );
        let notify_error = context
            .update(|window, cx| notify.snapshot_with(&[], window, cx))
            .unwrap_err();
        assert!(
            notify_error.to_string().contains("layout"),
            "{notify_error:#}"
        );
        runtime.callbacks.borrow_mut().retire(generation);
        assert!(
            context
                .update(|window, cx| stale.snapshot_with(&[], window, cx))
                .unwrap_err()
                .to_string()
                .contains("superseded render")
        );
        application.retire();
        let retired_error = match context.update(|window, cx| retired.build_with(&[], window, cx)) {
            Ok(_) => panic!("retired element callback must fail"),
            Err(error) => error,
        };
        assert!(retired_error.to_string().contains("retired application"));
        let snapshot =
            crate::ComponentDelegateSnapshot::new(vec![ComponentDataValue::String("row".into())]);
        assert_eq!(snapshot.len(), 1);
        assert!(snapshot.row(1).is_err());
    }

    #[gpui::test]
    fn component_callback_results_obey_snapshot_and_application_lifetimes(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let application = ApplicationGeneration::new(700);
        let (retired_snapshot, generation) = callback(&runtime, "() => null", None);
        let (retired_application, _) = callback(&runtime, "() => null", Some(application.clone()));
        runtime.callbacks.borrow_mut().retire(generation);
        application.retire();
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);

        let snapshot_error = context
            .update(|window, cx| {
                runtime.dispatch_component_callback_value(retired_snapshot, &[], window, cx)
            })
            .unwrap_err();
        assert!(snapshot_error.to_string().contains("superseded render"));
        let application_error = context
            .update(|window, cx| {
                runtime.dispatch_component_callback_value(retired_application, &[], window, cx)
            })
            .unwrap_err();
        assert!(
            application_error
                .to_string()
                .contains("retired application")
        );
    }

    #[gpui::test]
    fn window_effects_are_once_per_event_and_reset_after_errors(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let (reporter_id, _) = callback(
            &runtime,
            "(message) => { globalThis.__effectError = message; }",
            None,
        );
        let reporter =
            crate::component_registry::ComponentCallback::from_runtime(&runtime, reporter_id);
        let effects = reporter.window_effects();
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let runs = Cell::new(0);

        context
            .update(|window, cx| {
                effects.event(window, cx, |event| -> anyhow::Result<()> {
                    assert!(
                        event
                            .run_once::<()>("retry", |_, _| anyhow::bail!("retry me"))
                            .is_err()
                    );
                    assert!(
                        event.run_once("retry", |_, _| Ok(()))?.executed(),
                        "a failed keyed body must remain retryable in the same event"
                    );
                    assert!(
                        event
                            .run_once("open", |_, _| {
                                runs.set(runs.get() + 1);
                                Ok(())
                            })?
                            .executed()
                    );
                    assert!(
                        !event
                            .run_once("open", |_, _| {
                                runs.set(runs.get() + 1);
                                Ok(())
                            })?
                            .executed()
                    );
                    anyhow::bail!("candidate failed")
                })
            })
            .unwrap_err();
        assert_eq!(runs.get(), 1);
        assert_eq!(
            runtime
                .with_js(|ctx| ctx.eval::<String, _>("globalThis.__effectError"))
                .unwrap(),
            "candidate failed"
        );

        context
            .update(|window, cx| {
                effects.event(window, cx, |event| {
                    assert!(
                        event
                            .run_once("open", |_, _| {
                                runs.set(runs.get() + 1);
                                Ok(())
                            })?
                            .executed()
                    );
                    Ok(())
                })
            })
            .unwrap();
        assert_eq!(runs.get(), 2, "a later event may reuse the same key");
    }

    #[gpui::test]
    fn window_effects_reject_reentry_and_stale_generations(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let application = ApplicationGeneration::new(701);
        let (reporter_id, generation) = callback(
            &runtime,
            "(message) => { globalThis.__effectError = message; }",
            Some(application.clone()),
        );
        let effects =
            crate::component_registry::ComponentCallback::from_runtime(&runtime, reporter_id)
                .window_effects();
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);

        let error = context
            .update(|window, cx| {
                effects.event(window, cx, |event| {
                    event
                        .run_once("nested", |window, cx| effects.event(window, cx, |_| Ok(())))
                        .map(|_| ())
                })
            })
            .unwrap_err();
        assert!(error.to_string().contains("already running"));

        runtime.callbacks.borrow_mut().retire(generation);
        let error = context
            .update(|window, cx| effects.event(window, cx, |_| Ok(())))
            .unwrap_err();
        assert!(error.to_string().contains("superseded render"));

        let (application_reporter, _) = callback(&runtime, "() => null", Some(application.clone()));
        let application_effects = crate::component_registry::ComponentCallback::from_runtime(
            &runtime,
            application_reporter,
        )
        .window_effects();
        application.retire();
        let error = context
            .update(|window, cx| application_effects.event(window, cx, |_| Ok(())))
            .unwrap_err();
        assert!(error.to_string().contains("retired application"));
    }

    #[gpui::test]
    fn window_effect_panic_resets_the_event_guard(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let (reporter_id, _) = callback(&runtime, "() => null", None);
        let effects =
            crate::component_registry::ComponentCallback::from_runtime(&runtime, reporter_id)
                .window_effects();
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            context.update(|window, cx| effects.event::<()>(window, cx, |_| panic!("effect panic")))
        }));
        assert!(panic.is_err());
        assert!(!effects.is_active());
    }

    #[gpui::test]
    fn window_effects_reject_render_and_allow_nested_distinct_handles(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().unwrap();
        let (a_id, _) = callback(&runtime, "() => null", None);
        let (b_id, _) = callback(&runtime, "() => null", None);
        let a = crate::component_registry::ComponentCallback::from_runtime(&runtime, a_id)
            .window_effects();
        let b = crate::component_registry::ComponentCallback::from_runtime(&runtime, b_id)
            .window_effects();
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);

        let error = context
            .update(|window, cx| {
                let (_guard, _) = scope::enter_with_application(
                    &runtime,
                    window,
                    cx,
                    ScopePhase::Render,
                    None,
                    crate::policy::default(),
                    None,
                );
                a.event(window, cx, |_| Ok(()))
            })
            .unwrap_err();
        assert!(error.to_string().contains("during the `render` phase"));

        context
            .update(|window, cx| {
                a.event(window, cx, |event| {
                    event
                        .run_once("a", |window, cx| b.event(window, cx, |_| Ok(())))
                        .map(|_| ())
                })
            })
            .unwrap();
    }
}

/// An application entry loaded by one [`ShellRuntime`].
///
/// The JavaScript class stays opaque so hosts cannot bypass initialization,
/// policy, task ownership, or application-generation cleanup. It is a
/// single-mount handle: the owning runtime consumes it on its first mount
/// attempt, including an attempt whose construction or initialization fails.
/// A foreign runtime is rejected before that consumption.
pub struct LoadedApplication {
    runtime: Weak<ShellRuntime>,
    view_type: ViewType,
    mounted: Cell<bool>,
}

impl ViewType {
    /// A view class handed straight to the host rather than read off a
    /// module's default export.
    ///
    /// It holds no module lease: the class object is itself a live reference
    /// into the module that defined it, so QuickJS keeps that module alive for
    /// exactly as long as this does — which a lease taken here would only
    /// duplicate.
    fn from_panel_class(class: Persistent<Object<'static>>) -> Self {
        Self {
            value: class,
            module_lease: None,
            application: scope::current_application_generation(),
        }
    }
}

impl std::fmt::Debug for ViewType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ViewType").finish_non_exhaustive()
    }
}

/// One instance of a view type.
#[derive(Clone)]
pub struct ViewObject {
    value: Persistent<Object<'static>>,
    #[allow(dead_code)] // Its drop owns the resolver registration lifetime.
    module_lease: Option<ApplicationModuleLease>,
    application: Option<Rc<ApplicationGeneration>>,
    jit_warm: Rc<Cell<bool>>,
}

impl std::fmt::Debug for ViewObject {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ViewObject").finish_non_exhaustive()
    }
}

impl ViewObject {
    fn unscoped(value: Persistent<Object<'static>>) -> Self {
        Self {
            value,
            module_lease: None,
            application: None,
            jit_warm: Rc::new(Cell::new(false)),
        }
    }

    fn restore<'js>(self, ctx: &Ctx<'js>) -> JsResult<Object<'js>> {
        self.value.restore(ctx)
    }

    pub(crate) fn application_generation(&self) -> Option<Rc<ApplicationGeneration>> {
        self.application.clone()
    }
}

/// A class or props value captured by a host function without leaking the
/// active QuickJS lifetime into the Rust callback type.
struct NestedViewClass(Persistent<Object<'static>>);

impl<'js> FromJs<'js> for NestedViewClass {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let class = value.as_object().ok_or_else(|| {
            Exception::throw_type(ctx, "cx.new(Class, props) expects a View subclass")
        })?;
        Ok(Self(Persistent::save(ctx, class.clone())))
    }
}

struct NestedViewProps(Persistent<Value<'static>>);

impl<'js> FromJs<'js> for NestedViewProps {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        Ok(Self(Persistent::save(ctx, value)))
    }
}

struct ViewStateCheckpoint(Persistent<Function<'static>>);

#[derive(Clone)]
struct NestedViewProvenance {
    application: Option<Rc<ApplicationGeneration>>,
    policy: Rc<Policy>,
}

impl NestedViewProvenance {
    fn is_current(&self) -> bool {
        let Some(policy) = scope::current_policy() else {
            return false;
        };
        if !Rc::ptr_eq(&self.policy, &policy) {
            return false;
        }
        match (&self.application, scope::current_application_generation()) {
            (Some(expected), Some(current)) => {
                expected.is_active() && Rc::ptr_eq(expected, &current)
            }
            (None, None) => true,
            _ => false,
        }
    }
}

#[derive(Clone)]
struct NestedViewAlias {
    handle: EntityHandle,
    provenance: NestedViewProvenance,
}

/// A synchronous-looking script operation deferred until the active
/// `Context::with` entry has returned. This is what lets the implementation
/// reuse the ordinary, non-reentrant transactional job drains.
enum PendingNestedOperation {
    Create {
        runtime: Weak<ShellRuntime>,
        token: u32,
        owner: Entity<ScriptView>,
        view_type: ViewType,
        policy: Rc<crate::policy::Policy>,
        props: Persistent<Value<'static>>,
    },
    Update {
        runtime: Weak<ShellRuntime>,
        token: u32,
        provenance: NestedViewProvenance,
        props: Persistent<Value<'static>>,
    },
    Notify {
        runtime: Weak<ShellRuntime>,
        token: u32,
        provenance: NestedViewProvenance,
    },
    Release {
        runtime: Weak<ShellRuntime>,
        token: u32,
        provenance: NestedViewProvenance,
    },
    /// A change to a dock area's layout.
    ///
    /// Deferred for the same reason a nested view is, and in two cases it *is*
    /// one. `load` rebuilds every panel through the registry, which constructs
    /// views; `add_panel` is given a view from `cx.new(Class)`, which has not
    /// been constructed yet when the call is made. Neither can happen while
    /// QuickJS holds its runtime lock, which is exactly where both are called
    /// from.
    ///
    /// Removal is queued too, though nothing about it needs to be: an edit that
    /// jumped the queue would apply to a layout the script had already asked to
    /// change, and "the calls take effect in the order they were made" is worth
    /// more than one saved hop.
    EditDock {
        runtime: Weak<ShellRuntime>,
        dock: EntityHandle,
        /// Boxed: a whole persisted layout is by far the largest thing this
        /// enum can carry, and every queued operation would otherwise be
        /// sized for it.
        edit: Box<dock_api::DockEdit>,
        provenance: NestedViewProvenance,
    },
}

struct NestedFlushGuard<'a>(&'a Cell<bool>);

impl Drop for NestedFlushGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

mod dock_api;
mod entity_api;
pub(crate) mod host;
mod host_modules;
mod overlay;
pub(crate) mod sandbox;
mod scheduler;

pub(crate) fn cancel_policy_tasks(policy: &Rc<Policy>) {
    scheduler::cancel_policy(policy);
}

pub(crate) fn cancel_application_tasks(generation: &Rc<ApplicationGeneration>) {
    scheduler::cancel_application_generation(generation);
}

pub(crate) fn cancel_view_tasks(runtime: &Rc<ShellRuntime>, entity_id: gpui::EntityId) {
    scheduler::cancel_view(runtime, entity_id);
}

#[cfg(test)]
pub(crate) fn task_count() -> usize {
    scheduler::task_count()
}

pub(super) struct InputCallbackOwner {
    policy: Rc<Policy>,
    application: Option<Rc<ApplicationGeneration>>,
    view: Option<WeakEntity<ScriptView>>,
}
mod standard;
mod template;
mod theme_api;
mod window_api;

/// The names each built-in module exports.
///
/// One module per crate that provides the capability, so an import says which
/// layer a script depends on: `gpui-base`'s components come from `"gpui-base"`,
/// `gpui-fps`'s overlay from `"gpui-fps"`, and `"gpui-kit"` carries only what GPUI
/// itself and this runtime provide. `"gpui"` is an explicit compatibility alias
/// for that module; other names belong to exactly one layer.
///
/// Anything installed onto `globalThis.__gpui` must be listed in one of these
/// or no `import { … }` will see it.
pub(crate) mod exports {
    /// GPUI's own elements and this runtime's script surface.
    pub(crate) const GPUI: &[&str] = &[
        // Views (`ScriptView`).
        "View",
        // Elements GPUI itself draws.
        "div",
        "svg",
        "image",
        // GPUI's own lazy lists. Base's virtual lists live in `gpui-base`;
        // these are GPUI's, and are exported where `div` is.
        "list",
        "uniform_list",
        "PathBuilder",
        "Background",
    ];

    /// Components, layout helpers and the theme, all owned by `gpui-base`.
    pub(crate) const GPUI_BASE: &[&str] = &[
        // Layout.
        "h_flex",
        "v_flex",
        // Controls.
        "Button",
        "Link",
        "Checkbox",
        "Switch",
        "Tabs",
        "Tab",
        "Progress",
        "ProgressTrack",
        "ProgressIndicator",
        "Avatar",
        "AvatarImage",
        "AvatarFallback",
        "Pagination",
        "pagination_items",
        "CalendarState",
        "Accordion",
        "AccordionItem",
        "AccordionHeader",
        "AccordionPanel",
        "AccordionTrigger",
        "Radio",
        "Toggle",
        "RadioGroup",
        "ToggleGroup",
        "Table",
        "TableHeader",
        "TableBody",
        "TableRow",
        "TableHead",
        "TableCell",
        "TableCaption",
        "h_resizable",
        "v_resizable",
        "resizable_panel",
        "Collapsible",
        "Popover",
        "HoverCard",
        "Popup",
        "Select",
        "Combobox",
        "DatePicker",
        "Scrollbar",
        "v_virtual_list",
        "h_virtual_list",
        "VirtualListScrollHandle",
        // Text editing.
        "Input",
        "InputState",
        "NumberInput",
        "Textarea",
        "TextView",
        "TextareaState",
        "SliderState",
        "Slider",
        "SliderTrack",
        "SliderIndicator",
        "SliderThumb",
        "OtpState",
        "OtpInput",
        // Dock. The area is the state and `dock_area` is one description of
        // it, which is the split `v_virtual_list` already has.
        "DockArea",
        "dock_area",
        "dock_content",
        // Theme (`theme_api`). The theme belongs to `gpui-base`, even though
        // mutation is legal only while a host call supplies the current App.
        "set_theme",
    ];

    /// The performance overlay, owned by `gpui-fps`: the element form, and
    /// the root-owned HUD a script switches on and off.
    pub(crate) const GPUI_FPS: &[&str] = &[
        "fps_monitor",
        "show_fps_monitor",
        "hide_fps_monitor",
        "fps_monitor_visible",
    ];

    /// Shell-owned shared types. Module components are exported from their
    /// host modules rather than through a public generic dispatcher.
    pub(crate) const GPUI_SHELL: &[&str] = &[];
}

/// Defines one `ModuleDef` per built-in module and the loader wiring for all of
/// them, so adding a layer — `gpui-component`, when its components arrive — is
/// a list and a line rather than another copy of the same three impls.
///
/// Every module re-exports values that were built at startup and stashed on
/// `globalThis.__gpui`; the split is in what each one names, not in where the
/// values live.
macro_rules! builtin_modules {
    ($(($module:ident, $specifier:literal, $names:expr)),+ $(,)?) => {
        $(
            struct $module;

            impl $module {
                const SPECIFIER: &'static str = $specifier;
                const NAMES: &'static [&'static str] = $names;
            }

            impl ModuleDef for $module {
                fn declare(declarations: &Declarations) -> JsResult<()> {
                    for name in Self::NAMES {
                        declarations.declare(*name)?;
                    }
                    Ok(())
                }

                fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> JsResult<()> {
                    let module: Object = ctx.globals().get("__gpui")?;
                    for name in Self::NAMES {
                        let value: Value = module.get(*name)?;
                        exports.export(*name, value)?;
                    }
                    Ok(())
                }
            }
        )+

        /// The specifiers a script may import, and nothing else.
        fn builtin_resolver() -> BuiltinResolver {
            BuiltinResolver::default()$(.with_module($module::SPECIFIER))+
        }

        /// Named in the refusal when a bare specifier is not one of them, so a
        /// script written against a different runtime than the one running it
        /// is told which it is talking to rather than only that the import
        /// failed.
        fn builtin_specifiers() -> String {
            [$($module::SPECIFIER),+, crate::DEFAULT_COMPONENT_MODULE]
                .map(|specifier| format!("`{specifier}`"))
                .join(", ")
        }

        fn builtin_loader() -> ModuleLoader {
            ModuleLoader::default()$(.with_module($module::SPECIFIER, $module))+
        }

        /// The built-in specifiers, for the test that keeps
        /// [`crate::RESERVED_SPECIFIERS`] honest.
        #[cfg(test)]
        fn builtin_specifier_list() -> Vec<&'static str> {
            vec![$($module::SPECIFIER),+, crate::DEFAULT_COMPONENT_MODULE]
        }

        /// Which module exports `name`, if any.
        #[cfg(test)]
        fn module_exporting(name: &str) -> Option<&'static str> {
            $(
                if $module::NAMES.contains(&name) {
                    return Some($module::SPECIFIER);
                }
            )+
            None
        }
    };
}

builtin_modules![
    (GpuiModule, "gpui-kit", exports::GPUI),
    (GpuiAliasModule, "gpui", exports::GPUI),
    (GpuiBaseModule, "gpui-base", exports::GPUI_BASE),
    (GpuiShellModule, "gpui-shell", exports::GPUI_SHELL),
    (GpuiFpsModule, "gpui-fps", exports::GPUI_FPS),
];

/// A value a script cannot derive, proving that a retained-state handle came
/// from this runtime's own component module.
///
/// Retained state travels as an ordinary object, the way elements and entities
/// already do, so the proof is what separates a handle the module produced from
/// one a script wrote by hand. That means it must not be reconstructible from
/// anything a script can observe — a process id, a clock, a counter — so it
/// comes from `RandomState`'s OS-seeded keys, the same secret Rust relies on to
/// keep hash maps from being collided on purpose.
fn random_component_state_proof() -> String {
    use std::hash::{BuildHasher as _, Hasher as _, RandomState};

    let mut proof = String::from("gpui-shell-state-");
    for round in 0..2u64 {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(round);
        proof.push_str(&format!("{:016x}", hasher.finish()));
    }
    proof
}

type AppEffectInstall =
    Box<dyn FnOnce(&mut App) -> crate::component_registry::ComponentAppEffectCleanup>;

struct InstalledAppEffect {
    revision: String,
    cleanup: Option<crate::component_registry::ComponentAppEffectCleanup>,
}

struct ComponentAppEffectGeneration {
    application: Rc<ApplicationGeneration>,
    /// The root view the effects were installed for. Retiring a generation
    /// runs its cleanups through this view, so a reload does not have to wait
    /// for the window to close before native state is restored.
    view: WeakEntity<ScriptView>,
    pending: HashMap<String, String>,
    installed: HashMap<String, InstalledAppEffect>,
    _release: Subscription,
}

fn queue_component_app_effect(
    pending: &mut HashMap<String, String>,
    installed: &HashMap<String, InstalledAppEffect>,
    key: &str,
    revision: &str,
) -> bool {
    if pending.get(key).is_some_and(|value| value == revision) {
        return false;
    }
    if installed
        .get(key)
        .is_some_and(|effect| effect.revision == revision)
    {
        pending.remove(key);
        return false;
    }
    pending.insert(key.to_owned(), revision.to_owned());
    true
}

pub struct ShellRuntime {
    /// Declared first because fields drop in declaration order and every
    /// `Persistent` handle must be released while the context still exists.
    /// QuickJS aborts the process if a value outlives its runtime.
    callbacks: RefCell<CallbackArena<Persistent<Function<'static>>>>,
    components: FrozenComponentRegistry,
    component_state_proof: String,
    component_states: RefCell<crate::component_registry::RetainedStateStore>,
    pending_component_state_releases: RefCell<Vec<Rc<ApplicationGeneration>>>,
    component_app_effects: RefCell<HashMap<usize, ComponentAppEffectGeneration>>,
    warned_deprecated_exports: RefCell<HashSet<&'static str>>,
    arena: RefCell<SpecArena>,
    /// Templates the script has defined, indexed by the id its closure keeps.
    ///
    /// An entry is emptied when the application that defined it is released —
    /// a hot reload re-evaluates the module and defines its templates again, so
    /// without that this would grow by one arena per call site per save. The
    /// slot itself stays, because the id is the index.
    templates: RefCell<Vec<Option<Rc<crate::spec::Template>>>>,
    /// The template being discovered, while one is. See [`template`].
    discovery: RefCell<Option<template::Discovery>>,
    /// Retained state created by this runtime's scripts, and only this one's.
    /// Declared before `context` for the same reason `callbacks` is: releasing
    /// an entity can run script destructors.
    entities: RefCell<EntityStore>,
    /// Operations requested by a native function are applied immediately
    /// after the enclosing QuickJS entry unlocks the context. The queue is
    /// declared before `context` because it owns persistent JS values.
    pending_nested: RefCell<VecDeque<PendingNestedOperation>>,
    flushing_nested: Cell<bool>,
    in_flight_nested: RefCell<HashMap<u32, NestedViewProvenance>>,
    initializing_views: RefCell<Vec<ViewObject>>,
    nested_view_handles: RefCell<HashMap<u32, NestedViewAlias>>,
    next_nested_view_token: Cell<u32>,
    /// The panel builders this runtime registered, by their interned name.
    ///
    /// Declared here rather than only in the process-wide
    /// [`PanelRegistry`](gpui_base::dock::PanelRegistry) because a panel the
    /// script *adds* needs the same `serialize`/`deserialize` hooks a restored
    /// one gets, and the registry hands out builders rather than the script
    /// behind them. Each holds a `Persistent` class, so this drops before
    /// `context` like every other field that does.
    panel_scripts: RefCell<HashMap<String, Rc<dock_api::ScriptPanelClass>>>,
    /// A runtime whose opaque QuickJS job queue could not reach an ownership
    /// boundary safely is never entered again. QuickJS exposes no selective
    /// pending-job removal, so terminal quarantine is the only way to prevent
    /// the unfinished wave from later running under another view.
    terminal_job_error: RefCell<Option<String>>,
    /// What the runtime is spending. See [`Self::metrics`].
    metrics: Metrics,
    /// An HTTP client supplied by tests that exercise a loopback server.
    ///
    /// This is deliberately runtime-scoped rather than process state: tests
    /// run concurrently, and changing proxy environment variables would leak
    /// into unrelated runtimes. Production builds do not carry this field and
    /// continue to construct the normal system-configured client in `fetch`.
    #[cfg(test)]
    test_http_client: RefCell<Option<reqwest::blocking::Client>>,
    /// The QuickJS context currently executing, while one is.
    ///
    /// `Context::with` takes the runtime's lock, so calling it from inside a
    /// host function — which is already running under that lock — panics on a
    /// re-entrant borrow. Almost nothing needs to: a host function is handed
    /// the `Ctx` it was called with. The exception is a hook base calls on the
    /// shell's behalf from deep inside an operation the script started, and
    /// `Panel::dump` is one — `dock.dump()` reaches every panel's `serialize()`
    /// with only an `&App` in between.
    ///
    /// A field rather than a thread-local, so two runtimes on one thread cannot
    /// hand each other a context. Safe for the reason [`crate::scope`]'s
    /// pointers are: it is installed by a frame on the stack and cleared before
    /// that frame returns, so nothing can read it after the borrow it names has
    /// ended.
    active_context: Cell<Option<std::ptr::NonNull<rquickjs::qjs::JSContext>>>,
    context: JsContext,
    /// Incremented per `load_app`, so a reload re-reads every module rather
    /// than serving the first version from QuickJS's module cache.
    app_modules: AppModules,
    dependency_store: GitDependencyStore,
    next_application_generation: Cell<u64>,
    /// Held so the context stays alive, and so the module loader can be scoped
    /// to an application directory when one is loaded.
    js_runtime: JitRuntime,
}

struct JitSuspension<'a> {
    jit: &'a rquickjs_jit::Jit,
    active: bool,
}

impl JitSuspension<'_> {
    fn begin(jit: &rquickjs_jit::Jit) -> Result<JitSuspension<'_>> {
        let active = !jit.is_suspended();
        if active {
            jit.suspend()?;
        }
        Ok(JitSuspension { jit, active })
    }

    fn finish(mut self) -> Result<()> {
        if self.active {
            self.jit.resume()?;
        }
        self.active = false;
        Ok(())
    }
}

impl Drop for JitSuspension<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.jit.resume();
        }
    }
}

impl Drop for ShellRuntime {
    fn drop(&mut self) {
        self.flush_component_state_releases();
        // Both hold `Persistent` script values, and a persistent handle
        // released after its runtime aborts the process.
        scheduler::shutdown(self);
        self.callbacks.borrow_mut().clear();
        // Each holds a `Persistent` view class, which must be released while
        // the context is still alive — and the panel registry keeps a second
        // reference to every one of them in an `App` global that outlives this,
        // so clearing the map is not enough on its own.
        for script in self.panel_scripts.borrow().values() {
            script.retire();
        }
        self.panel_scripts.borrow_mut().clear();
        // Retained entities are owned by GPUI but reachable only through this
        // runtime's handles; leaving them registered outlives the app that owns
        // them, which GPUI reports as a leaked handle on shutdown.
        self.entities.borrow_mut().clear();
    }
}

/// The App can find its default runtime without becoming its owner.
///
/// Shell views and host state own runtime lifetime. Once they are gone,
/// `ShellRuntime::new` can replace this expired registration naturally.
struct RuntimeGlobal(Weak<ShellRuntime>);

impl Global for RuntimeGlobal {}

impl ShellRuntime {
    fn with_jit_suspended<T>(&self, operation: impl FnOnce() -> Result<T>) -> Result<T> {
        if !self.js_runtime.has_jit() {
            return operation();
        }
        let jit = self.js_runtime.jit();
        let suspension = JitSuspension::begin(jit)?;
        let result = operation();
        suspension.finish()?;
        result
    }

    /// Loads an application's JavaScript entry without exposing engine values.
    ///
    /// Resolution, declaration refresh, module generations, capabilities and
    /// watcher-compatible module leases are identical to the shell's own app
    /// loading path.
    pub fn load_application(
        self: &Rc<Self>,
        directory: &Path,
        entry: &str,
    ) -> Result<LoadedApplication> {
        Ok(LoadedApplication {
            runtime: Rc::downgrade(self),
            view_type: self.load_app(directory, entry)?,
            mounted: Cell::new(false),
        })
    }

    /// Creates, initializes and mounts a loaded application as a [`ScriptView`].
    ///
    /// The owner consumes the handle before construction. This makes a failed
    /// attempt terminal too, matching the application-generation cleanup that
    /// construction and initialization failures perform.
    pub fn mount_application(
        self: &Rc<Self>,
        application: &LoadedApplication,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<ScriptView>> {
        anyhow::ensure!(
            application
                .runtime
                .upgrade()
                .is_some_and(|runtime| Rc::ptr_eq(&runtime, self)),
            "loaded application belongs to a different ShellRuntime"
        );
        anyhow::ensure!(
            !application.mounted.replace(true),
            "loaded application has already been mounted"
        );
        self.instantiate_view_with_policy(
            &application.view_type,
            crate::policy::default(),
            window,
            cx,
        )
    }

    /// Creates the application's default runtime and makes it available to
    /// shell callbacks registered on this [`App`].
    pub fn new(cx: &mut App) -> Result<Rc<Self>> {
        Self::new_with_components(cx, FrozenComponentRegistry::default())
    }

    /// Creates the application's default runtime with a frozen external
    /// component catalog and makes it available to shell callbacks.
    pub fn new_with_components(
        cx: &mut App,
        components: FrozenComponentRegistry,
    ) -> Result<Rc<Self>> {
        if Self::global(cx).is_some() {
            return Err(anyhow!(
                "a default gpui-shell runtime is already installed; use ShellRuntime::new_isolated() for an additional VM"
            ));
        }
        let runtime = Self::new_isolated_with_components(components)?;
        runtime.set_global(cx);
        Ok(runtime)
    }

    /// Creates a runtime without installing it as the application's default.
    ///
    /// More than one may be alive on a thread because authority travels on the
    /// call frame rather than in runtime-global state. Use this only when a host
    /// deliberately owns multiple isolated runtimes.
    pub fn new_isolated() -> Result<Rc<Self>> {
        Self::new_isolated_with_components_and_dependency_store(
            FrozenComponentRegistry::default(),
            GitDependencyStore::for_user()?,
            shell_jit_config()?,
        )
    }

    #[cfg(test)]
    pub(crate) fn new_isolated_interpreter() -> Result<Rc<Self>> {
        let js_runtime = JitRuntime::builder()
            .build_interpreter()
            .map_err(|error| anyhow!("failed to start the JavaScript interpreter: {error}"))?;
        Self::new_isolated_with_components_dependency_store_and_runtime(
            FrozenComponentRegistry::default(),
            GitDependencyStore::for_user()?,
            js_runtime,
        )
    }

    pub fn new_isolated_with_components(components: FrozenComponentRegistry) -> Result<Rc<Self>> {
        Self::new_isolated_with_components_and_dependency_store(
            components,
            GitDependencyStore::for_user()?,
            shell_jit_config()?,
        )
    }

    #[cfg(test)]
    fn new_isolated_with_dependency_store(
        dependency_store: GitDependencyStore,
    ) -> Result<Rc<Self>> {
        Self::new_isolated_with_components_and_dependency_store(
            FrozenComponentRegistry::default(),
            dependency_store,
            shell_jit_config()?,
        )
    }

    fn new_isolated_with_components_and_dependency_store(
        components: FrozenComponentRegistry,
        dependency_store: GitDependencyStore,
        jit_config: JitConfig,
    ) -> Result<Rc<Self>> {
        let js_runtime = JitRuntime::builder()
            .config(jit_config)
            .build()
            .map_err(|error| anyhow!("failed to start the JavaScript JIT runtime: {error}"))?;
        Self::new_isolated_with_components_dependency_store_and_runtime(
            components,
            dependency_store,
            js_runtime,
        )
    }

    fn new_isolated_with_components_dependency_store_and_runtime(
        components: FrozenComponentRegistry,
        dependency_store: GitDependencyStore,
        js_runtime: JitRuntime,
    ) -> Result<Rc<Self>> {
        let jit_enabled = js_runtime.has_jit();
        if jit_enabled {
            js_runtime.jit().suspend()?;
        }
        let entities = EntityStore::try_new()
            .ok_or_else(|| anyhow!("gpui-shell entity store id space is exhausted"))?;
        let context = JsContext::full(&js_runtime).map_err(js_setup_error)?;

        let app_modules = AppModules::default();
        let component_state_proof = random_component_state_proof();
        let component_module = RegisteredComponentModule {
            specifier: components.module_specifier(),
            source: components.javascript_module_source(&component_state_proof),
        };
        // Order is the namespace policy. The runtime's own modules resolve
        // first, so a host cannot take `gpui-kit` or `path` from under a script;
        // the application's files resolve last, so a HostModule cannot be
        // shadowed by a file that happens to share its name. `host_modules`
        // refuses reserved names at registration, which is what turns the first
        // half of that from a silent shadowing into a sentence.
        js_runtime.set_loader(
            (
                component_module.clone(),
                standard::resolver(),
                builtin_resolver(),
                host_modules::HostModuleLoader,
                app_modules.clone(),
            ),
            (
                component_module,
                standard::loader(),
                builtin_loader(),
                host_modules::HostModuleLoader,
                app_modules.clone(),
            ),
        );

        // Resource limits belong to the sandbox policy, but only the engine
        // owns the runtime handle, so the policy hands out values and this is
        // where they are applied. A runaway script must not be able to hold the
        // UI thread (§19.3).
        js_runtime.set_memory_limit(sandbox::memory_limit_bytes());
        js_runtime.set_max_stack_size(sandbox::max_stack_size_bytes());
        js_runtime.set_interrupt_handler(Some(Box::new(sandbox::interrupt_handler())));

        let runtime = Rc::new(Self {
            callbacks: RefCell::new(CallbackArena::default()),
            components,
            component_state_proof,
            component_states: RefCell::new(Default::default()),
            pending_component_state_releases: RefCell::new(Vec::new()),
            component_app_effects: RefCell::new(HashMap::new()),
            warned_deprecated_exports: RefCell::new(HashSet::new()),
            arena: RefCell::new(SpecArena::new()),
            templates: RefCell::new(Vec::new()),
            discovery: RefCell::new(None),
            entities: RefCell::new(entities),
            pending_nested: RefCell::new(VecDeque::new()),
            flushing_nested: Cell::new(false),
            in_flight_nested: RefCell::new(HashMap::new()),
            initializing_views: RefCell::new(Vec::new()),
            nested_view_handles: RefCell::new(HashMap::new()),
            next_nested_view_token: Cell::new(0),
            panel_scripts: RefCell::new(HashMap::new()),
            active_context: Cell::new(None),
            terminal_job_error: RefCell::new(None),
            metrics: Metrics::default(),
            #[cfg(test)]
            test_http_client: RefCell::new(None),
            context,
            app_modules,
            dependency_store,
            next_application_generation: Cell::new(1),
            js_runtime,
        });

        let installed = runtime.install_globals();
        if jit_enabled {
            runtime.js_runtime.jit().resume()?;
        }
        installed?;
        Ok(runtime)
    }

    pub fn component_registry(&self) -> &FrozenComponentRegistry {
        &self.components
    }

    pub(crate) fn component_entity_kind(&self, handle: EntityHandle) -> Option<&'static str> {
        self.entities.borrow().kind(handle)
    }

    pub(crate) fn with_component_state<T: std::any::Any, R>(
        &self,
        handle: u64,
        kind: &'static str,
        body: impl FnOnce(&T) -> R,
    ) -> anyhow::Result<R> {
        self.flush_component_state_releases();
        let result = self
            .component_states
            .try_borrow()
            .map_err(|_| anyhow!("retained component state is already mutably borrowed"))?
            .with(handle, kind, body);
        self.flush_component_state_releases();
        result
    }

    pub(crate) fn update_component_state<T: std::any::Any, R>(
        self: &Rc<Self>,
        handle: u64,
        kind: &'static str,
        window: &mut Window,
        cx: &mut App,
        body: impl FnOnce(&mut T, &mut Window, &mut App) -> R,
    ) -> anyhow::Result<R> {
        anyhow::ensure!(
            !matches!(
                scope::current_phase(),
                Some(ScopePhase::Render | ScopePhase::Layout)
            ),
            "retained component state cannot be updated during render or layout"
        );
        let (_scope, _) = scope::enter_runtime(self, window, cx, ScopePhase::Event, None);
        self.flush_component_state_releases();
        let result = self
            .component_states
            .try_borrow_mut()
            .map_err(|_| anyhow!("retained component state is already borrowed"))?
            .with_mut(handle, kind, |state| body(state, window, cx));
        self.flush_component_state_releases();
        result
    }

    fn release_component_states(&self, application: &Rc<ApplicationGeneration>) {
        self.pending_component_state_releases
            .borrow_mut()
            .push(application.clone());
        self.flush_component_state_releases();
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn schedule_component_app_effect(
        self: &Rc<Self>,
        application: Rc<ApplicationGeneration>,
        view: WeakEntity<ScriptView>,
        key: String,
        revision: String,
        window: &mut Window,
        cx: &mut App,
        install: AppEffectInstall,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            application.is_active(),
            "component app effect belongs to a retired application"
        );
        let view = view
            .upgrade()
            .ok_or_else(|| anyhow!("component app effects require a live root view"))?;
        let application_key = Rc::as_ptr(&application) as usize;

        let mut generations = self.component_app_effects.borrow_mut();
        if let std::collections::hash_map::Entry::Vacant(slot) = generations.entry(application_key)
        {
            let runtime = Rc::downgrade(self);
            let release = cx.observe_release(&view, move |_, cx| {
                if let Some(runtime) = runtime.upgrade() {
                    runtime.cleanup_component_app_effects(application_key, cx);
                }
            });
            slot.insert(ComponentAppEffectGeneration {
                application: application.clone(),
                view: view.downgrade(),
                pending: HashMap::new(),
                installed: HashMap::new(),
                _release: release,
            });
        }
        let generation = generations
            .get_mut(&application_key)
            .expect("inserted above");
        if !queue_component_app_effect(
            &mut generation.pending,
            &generation.installed,
            &key,
            &revision,
        ) {
            return Ok(());
        }
        drop(generations);

        let runtime = Rc::downgrade(self);
        window.defer(cx, move |_, cx| {
            let Some(runtime) = runtime.upgrade() else {
                return;
            };
            runtime.apply_component_app_effect(application_key, key, revision, install, cx);
        });
        Ok(())
    }

    fn apply_component_app_effect(
        &self,
        application_key: usize,
        key: String,
        revision: String,
        install: AppEffectInstall,
        cx: &mut App,
    ) {
        let old_cleanup = {
            let mut generations = self.component_app_effects.borrow_mut();
            let Some(generation) = generations.get_mut(&application_key) else {
                return;
            };
            if !generation.application.is_active()
                || generation.pending.get(&key) != Some(&revision)
            {
                return;
            }
            generation.pending.remove(&key);
            generation
                .installed
                .remove(&key)
                .and_then(|mut effect| effect.cleanup.take())
        };
        if let Some(cleanup) = old_cleanup {
            cleanup(cx);
        }
        let cleanup = install(cx);
        if let Some(generation) = self
            .component_app_effects
            .borrow_mut()
            .get_mut(&application_key)
        {
            generation.installed.insert(
                key,
                InstalledAppEffect {
                    revision,
                    cleanup: Some(cleanup),
                },
            );
        } else {
            cleanup(cx);
        }
    }

    /// Runs the cleanups an application generation installed, at the moment
    /// that generation retires.
    ///
    /// Without this, a reload would leave the previous generation's native
    /// state — menus, globals, window chrome — installed until the root view
    /// was released, and the replacing generation could not reach it: keys are
    /// scoped per generation, so the new install adds to the old rather than
    /// replacing it. The release subscription remains the backstop for paths
    /// that have no `App` to run cleanups against.
    fn retire_component_app_effects(
        &self,
        application: &Rc<ApplicationGeneration>,
        cx: &mut impl gpui::AppContext,
    ) {
        let application_key = Rc::as_ptr(application) as usize;
        let Some(generation) = self
            .component_app_effects
            .borrow_mut()
            .remove(&application_key)
        else {
            return;
        };
        let Some(view) = generation.view.upgrade() else {
            // The view is already gone, so the release subscription has run.
            return;
        };
        let mut generation = generation;
        view.update(cx, |_, cx| {
            for (_, mut effect) in generation.installed.drain() {
                if let Some(cleanup) = effect.cleanup.take() {
                    cleanup(cx);
                }
            }
        });
    }

    fn cleanup_component_app_effects(&self, application_key: usize, cx: &mut App) {
        let Some(mut generation) = self
            .component_app_effects
            .borrow_mut()
            .remove(&application_key)
        else {
            return;
        };
        for (_, mut effect) in generation.installed.drain() {
            if let Some(cleanup) = effect.cleanup.take() {
                cleanup(cx);
            }
        }
    }

    fn flush_component_state_releases(&self) {
        loop {
            let pending = std::mem::take(&mut *self.pending_component_state_releases.borrow_mut());
            if pending.is_empty() {
                return;
            }
            let Ok(mut states) = self.component_states.try_borrow_mut() else {
                self.pending_component_state_releases
                    .borrow_mut()
                    .extend(pending);
                return;
            };
            for application in pending {
                states.release_application(&application);
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn retained_component_state_count(&self) -> usize {
        self.component_states
            .try_borrow()
            .map_or(0, |states| states.len())
    }

    pub fn type_declarations(&self) -> String {
        crate::typings::declarations_with_components(&self.components)
    }

    pub fn write_type_declarations(&self, root: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
        crate::typings::write_application_with_components(root, &self.components)
    }

    pub(crate) fn set_global(self: &Rc<Self>, cx: &mut App) {
        cx.set_global(RuntimeGlobal(Rc::downgrade(self)));
    }

    pub(crate) fn global(cx: &App) -> Option<Rc<Self>> {
        cx.try_global::<RuntimeGlobal>()
            .and_then(|global| global.0.upgrade())
    }

    /// What the runtime is spending: script renders and materializations, with
    /// the time each took.
    ///
    /// The two counters follow different things — application activity and
    /// frame count — and the gap between them is what the snapshot lifecycle
    /// exists to produce. See [`crate::metrics`].
    pub(crate) fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    #[cfg(test)]
    pub(crate) fn use_direct_http_for_tests(&self) {
        *self.test_http_client.borrow_mut() = Some(
            standard::direct_test_http_client()
                .expect("the direct test HTTP client should be constructible"),
        );
    }

    #[cfg(test)]
    fn test_http_client(&self) -> Option<reqwest::blocking::Client> {
        self.test_http_client.borrow().clone()
    }

    /// Evaluates a fragment of script in this runtime's context.
    ///
    /// Test-only, and used by one caller: `tests::benchmark` has to time a loop
    /// of bare `__apply` calls to separate the cost of crossing the language
    /// boundary from the cost of what happens on the far side, and a view's
    /// `render()` cannot express that loop without the surrounding element
    /// construction being part of the measurement.
    #[cfg(test)]
    pub(crate) fn eval_for_benchmark(&self, source: &str) -> Result<()> {
        self.with_js(|ctx| ctx.eval::<(), _>(source))
    }

    /// Empties the scratch arena between benchmark rounds.
    ///
    /// A script render resets it on the way in; a benchmark that never renders
    /// would otherwise accumulate every round's nodes and measure the growth of
    /// the arena rather than the cost of writing to it.
    #[cfg(test)]
    pub(crate) fn reset_arena_for_benchmark(&self) {
        self.arena.borrow_mut().reset();
    }

    /// Returns JIT diagnostic counters for the real-process acceptance
    /// benchmark.
    #[cfg(test)]
    pub(crate) fn jit_metrics_for_benchmark(&self) -> rquickjs_jit::JitMetrics {
        self.js_runtime.metrics()
    }

    /// A reading of the two counters, taken now.
    ///
    /// The host gets the reading rather than the instrument: `Metrics` is the
    /// timing side, and a host holding it could reset the counters under a
    /// measurement someone else was taking. Subtract two readings with
    /// [`RuntimeMetrics::since`](crate::RuntimeMetrics::since) to measure an
    /// interval.
    pub fn read_metrics(&self) -> crate::metrics::RuntimeMetrics {
        self.metrics.read()
    }

    /// This runtime's retained state.
    ///
    /// Scoped to the runtime rather than shared, so one runtime cannot resolve
    /// another's handle — see [`crate::entities`].
    pub(crate) fn entities(&self) -> RefMut<'_, EntityStore> {
        self.entities.borrow_mut()
    }

    fn purge_released_view_aliases(&self, release: &crate::entities::EntityRelease) {
        self.nested_view_handles
            .borrow_mut()
            .retain(|_, alias| !release.contains(alias.handle));
    }

    /// Resolves a release entirely under the store borrow, then performs all
    /// GPUI and callback/task retirement after the borrow has ended.
    pub(crate) fn release_view_handle(
        &self,
        handle: EntityHandle,
        cx: &mut impl gpui::AppContext,
    ) -> bool {
        let release = { self.entities().release_view(handle) };
        let Some(release) = release else {
            return false;
        };
        self.purge_released_view_aliases(&release);
        release.retire(cx);
        true
    }

    pub(crate) fn release_application_generation(
        &self,
        application: &Rc<ApplicationGeneration>,
        cx: &mut impl gpui::AppContext,
    ) {
        self.retire_component_app_effects(application, cx);
        self.release_component_states(application);
        let release = { self.entities().release_application(application) };
        self.purge_released_view_aliases(&release);
        release.retire(cx);
        cancel_application_tasks(application);
        self.retire_templates(application);
    }

    /// The `Drop`-path counterpart of [`Self::release_application_generation`].
    ///
    /// Application effects are deliberately left to the release subscription
    /// here: their cleanups need an `App`, and this path exists precisely for
    /// callers that have none.
    pub(crate) fn release_application_generation_without_context(
        &self,
        application: &Rc<ApplicationGeneration>,
    ) {
        self.release_component_states(application);
        let release = { self.entities().release_application(application) };
        self.purge_released_view_aliases(&release);
        release.retire_without_context();
        cancel_application_tasks(application);
        self.retire_templates(application);
    }

    fn rollback_retained_since(
        &self,
        entities: crate::entities::EntityCheckpoint,
        tasks: scheduler::TaskCheckpoint,
        cx: &mut impl gpui::AppContext,
    ) {
        scheduler::rollback_runtime_tasks(tasks);
        let release = { self.entities().rollback(entities) };
        self.purge_released_view_aliases(&release);
        release.retire(cx);
    }

    #[cfg(test)]
    pub(crate) fn nested_view_alias_count(&self) -> usize {
        self.nested_view_handles.borrow().len()
    }

    fn job_queue_error(&self) -> Option<anyhow::Error> {
        self.terminal_job_error
            .borrow()
            .as_ref()
            .map(|message| anyhow!(message.clone()))
    }

    /// Permanently quarantines a runtime with an opaque unfinished job wave.
    fn fail_job_queue(&self) -> anyhow::Error {
        let message = "the QuickJS job queue exceeded gpui-shell's transactional limit; the \
                       script runtime was disabled so pending work cannot cross view authority"
            .to_owned();
        if self.terminal_job_error.borrow().is_none() {
            *self.terminal_job_error.borrow_mut() = Some(message.clone());
            scheduler::shutdown(self);
        }
        anyhow!(message)
    }

    /// Loads `main.js` from an application directory.
    ///
    /// Module resolution is scoped to that directory: an application can import
    /// its own files and the built-in modules, and nothing else. That is
    /// the first half of the sandbox's module policy (design doc §19.1).
    pub(crate) fn load_app(self: &Rc<Self>, dir: &Path, entry: &str) -> Result<ViewType> {
        let root = crate::runtime::resolve_app_root(dir, entry)?;
        if let Err(error) = self.write_type_declarations(&root) {
            tracing::debug!(
                "could not update declarations in {}: {error}",
                root.display()
            );
        }

        let dependencies = if root.join(crate::plugin::MANIFEST_FILE).is_file() {
            let manifest = crate::plugin::PluginManifest::read(&root)?;
            let dependencies = self.dependency_store.materialize_all(&manifest)?;
            // Beside the declarations, and for the same reason: the process
            // that will resolve these imports is the one that tells an editor
            // where they are, so a package cannot be typed against a checkout
            // this load is not going to use. Best-effort, like the
            // declarations — a read-only directory is not a reason to refuse to
            // run.
            if let Err(error) = self.dependency_store.link_for_editor(&root, &dependencies) {
                tracing::debug!(
                    "could not link dependencies for an editor in {}: {error:#}",
                    root.display()
                );
            }
            dependencies
        } else {
            BTreeMap::new()
        };

        // Every load is a new generation, which is what makes a reload pick up
        // a change in an imported module rather than only in the entry point.
        let module_lease = self
            .app_modules
            .register_with_dependencies(root.clone(), dependencies);
        let generation = module_lease.generation();
        let application = ApplicationGeneration::new(self.next_application_generation.get());
        self.next_application_generation.set(
            self.next_application_generation
                .get()
                .checked_add(1)
                .expect("a shell runtime exhausted its application generations"),
        );

        let entry = root.join(entry);
        let source = read_module_source(&entry)?;

        // The entry carries the generation too: it is a cached module like any
        // other, and a reload that re-read every import but served a stale
        // `main.js` would be the same bug one level up.
        let _application_scope = scope::enter_application(application.clone());
        let loaded = self.load_source_with_lease(
            &format!("{}?v={}", entry.to_string_lossy(), generation),
            &source,
            Some(module_lease),
            Some(application.clone()),
        );
        if loaded.is_err() {
            self.release_application_generation_without_context(&application);
        }
        loaded
    }

    /// Evaluates a module and returns its default export, which must be a view
    /// class.
    #[cfg(test)]
    pub(crate) fn load_source(self: &Rc<Self>, name: &str, source: &str) -> Result<ViewType> {
        self.load_source_with_lease(name, source, None, None)
    }

    fn load_source_with_lease(
        self: &Rc<Self>,
        name: &str,
        source: &str,
        module_lease: Option<ApplicationModuleLease>,
        application: Option<Rc<ApplicationGeneration>>,
    ) -> Result<ViewType> {
        self.with_jit_suspended(|| {
            self.with_js(|ctx| {
                let (module, promise) =
                    rquickjs::Module::declare(ctx.clone(), name, source)?.eval()?;
                promise.finish::<()>()?;

                let default: Value = module.get("default")?;
                let Some(class) = default.as_object() else {
                    return Err(Exception::throw_message(
                        ctx,
                        "main.js must `export default` a class that extends View",
                    ));
                };
                Ok(ViewType {
                    value: Persistent::save(ctx, class.clone()),
                    module_lease,
                    application,
                })
            })
        })
    }

    /// Constructs one instance of a view class.
    ///
    /// `init` is where a view creates the state it keeps across frames, and
    /// creating an entity needs a `Window` and an `App`. So construction opens
    /// a scope of its own rather than running in the gap between host calls.
    #[cfg(test)]
    pub(crate) fn instantiate(
        self: &Rc<Self>,
        view_type: &ViewType,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<ViewObject> {
        let application = view_type.application.clone();
        let (_guard, _generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            None,
            crate::policy::default(),
            application.clone(),
        );
        let instance = match self.construct(view_type) {
            Ok(instance) => instance,
            Err(error) => {
                if let Some(application) = application {
                    self.release_application_generation(&application, cx);
                }
                return Err(error);
            }
        };
        if let Err(error) = self.initialize(&instance, None) {
            if let Some(application) = application {
                self.release_application_generation(&application, cx);
            }
            return Err(error);
        }
        Ok(instance)
    }

    /// Constructs and initializes a script view under its final owner.
    ///
    /// `init()` may start asynchronous work. Creating the GPUI entity first is
    /// what gives those tasks an owner, so a later `cx.notify()` can invalidate
    /// this view and dropping the view can cancel its work.
    #[cfg(test)]
    pub(crate) fn instantiate_view(
        self: &Rc<Self>,
        view_type: &ViewType,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<ScriptView>> {
        self.instantiate_view_with_policy(view_type, crate::policy::default(), window, cx)
    }

    pub(crate) fn instantiate_view_with_policy(
        self: &Rc<Self>,
        view_type: &ViewType,
        policy: Rc<crate::policy::Policy>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<ScriptView>> {
        let application = view_type.application.clone();
        let (construct_scope, _) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            None,
            policy.clone(),
            application.clone(),
        );
        let object = match self.construct(view_type) {
            Ok(object) => object,
            Err(error) => {
                if let Some(application) = application {
                    self.release_application_generation(&application, cx);
                }
                return Err(error);
            }
        };
        drop(construct_scope);
        let view = cx.new(|_| ScriptView::with_policy(self.clone(), object, policy.clone()));
        let object = view.read(cx).object().clone();

        let (_initialize_scope, _) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            Some(view.clone()),
            policy,
            application.clone(),
        );
        let initialized = self.initialize(&object, None);
        let nested = self.flush_pending_nested_views(window, cx);
        if let Err(error) = initialized.and(nested) {
            if let Some(application) = application {
                self.release_application_generation(&application, cx);
            }
            return Err(error);
        }
        Ok(view)
    }

    /// Constructs, retains and initializes a nested view under its final GPUI
    /// entity owner.
    ///
    /// The handle enters the entity store before `init(props)` runs so anything
    /// init creates is tagged with the exact child owner. A failed init removes
    /// that handle and all records/tasks owned by the candidate child without
    /// touching application-wide state.
    pub(crate) fn instantiate_nested_view(
        self: &Rc<Self>,
        view_type: &ViewType,
        policy: Rc<crate::policy::Policy>,
        initial_props: Option<Persistent<Value<'static>>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<EntityHandle> {
        // Establish an empty queue boundary while the caller's scope is still
        // installed. Otherwise an older parent reaction would be executed by
        // the child-init drain and acquire the child's ownership/authority.
        scheduler::drain_jobs_transactionally(self, window, cx)?;
        let entity_checkpoint = { self.entities().checkpoint() };
        let task_checkpoint = scheduler::checkpoint_runtime_tasks(self);

        let application = view_type.application.clone();
        let (construct_scope, _) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            None,
            policy.clone(),
            application.clone(),
        );
        let constructed = self.construct(view_type);
        let construction_jobs = scheduler::drain_jobs_transactionally(self, window, cx);
        drop(construct_scope);
        let object = match construction_jobs.and(constructed) {
            Ok(object) => object,
            Err(error) => {
                self.rollback_retained_since(entity_checkpoint, task_checkpoint, cx);
                return Err(error);
            }
        };

        let view =
            cx.new(|cx| ScriptView::nested(self.clone(), object, policy.clone(), cx.entity_id()));
        let handle = match self
            .entities()
            .create_view(view.clone(), application.clone(), self)
        {
            Ok(handle) => handle,
            Err(_) => {
                self.rollback_retained_since(entity_checkpoint, task_checkpoint, cx);
                anyhow::bail!(
                    "the application reached gpui-shell's retained entity limit; release unused \
                     handles"
                );
            }
        };
        let object = view.read(cx).object().clone();

        let (_initialize_scope, _) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            Some(view.clone()),
            policy,
            application,
        );
        let initialized = self.initialize(&object, initial_props);
        let nested = self.flush_pending_nested_views(window, cx);
        // The queue was empty before init entered. Draining its whole causal
        // wave here therefore assigns only init continuations to the child and
        // prevents a throwing init from leaving work beyond local rollback.
        let init_jobs = scheduler::drain_jobs_transactionally(self, window, cx);
        if let Err(error) = initialized.and(nested).and(init_jobs) {
            scheduler::rollback_runtime_tasks(task_checkpoint);
            let released = self.release_view_handle(handle, cx);
            debug_assert!(released, "the candidate child handle must still be live");
            let residual = { self.entities().rollback(entity_checkpoint) };
            self.purge_released_view_aliases(&residual);
            residual.retire(cx);
            return Err(error);
        }
        Ok(handle)
    }

    /// Defers creation until the native host call has returned to Rust and the
    /// active `Context::with` lock has been released. The opaque token is all
    /// JavaScript keeps; it is resolved to the typed entity handle before the
    /// enclosing engine entry returns.
    fn queue_nested_view_creation(
        self: &Rc<Self>,
        ctx: &Ctx<'_>,
        class: Persistent<Object<'static>>,
        props: Persistent<Value<'static>>,
    ) -> JsResult<u32> {
        let parent = scope::current_view().ok_or_else(|| {
            Exception::throw_type(
                ctx,
                "cx.new(Class, props) needs a current script view; call it from a \
                 view's init(), event handler or task",
            )
        })?;
        let (parent_object, policy) = scope::with_current_app(|cx| {
            let parent = parent.read(cx);
            (parent.object().clone(), parent.policy())
        })
        .ok_or_else(|| nested_view_needs_call(ctx, "cx.new(Class, props)"))?;
        let provenance = self
            .initializing_views
            .borrow()
            .last()
            .cloned()
            .unwrap_or(parent_object);
        let application =
            scope::current_application_generation().or_else(|| provenance.application_generation());
        let token = self.next_nested_view_token.get();
        let next = token.checked_add(1).ok_or_else(|| {
            Exception::throw_range(ctx, "the nested Entity token space is exhausted")
        })?;
        self.next_nested_view_token.set(next);
        self.pending_nested
            .borrow_mut()
            .push_back(PendingNestedOperation::Create {
                runtime: Rc::downgrade(self),
                token,
                owner: parent,
                view_type: ViewType {
                    value: class,
                    module_lease: provenance.module_lease.clone(),
                    application,
                },
                policy,
                props,
            });
        Ok(token)
    }

    fn queue_nested_view_update(
        self: &Rc<Self>,
        ctx: &Ctx<'_>,
        token: u32,
        props: Persistent<Value<'static>>,
    ) -> JsResult<()> {
        let resolved = self.nested_view_handles.borrow().get(&token).cloned();
        let pending = self.pending_nested.borrow();
        let pending_create = pending.iter().find_map(|operation| match operation {
            PendingNestedOperation::Create {
                token: candidate,
                view_type,
                policy,
                ..
            } if *candidate == token => Some(NestedViewProvenance {
                application: view_type.application.clone(),
                policy: policy.clone(),
            }),
            _ => None,
        });
        let pending_release = pending.iter().any(|operation| {
            matches!(operation, PendingNestedOperation::Release { token: candidate, .. } if *candidate == token)
        });
        drop(pending);
        let provenance = resolved
            .as_ref()
            .map(|alias| alias.provenance.clone())
            .or(pending_create)
            .or_else(|| self.in_flight_nested.borrow().get(&token).cloned());
        if pending_release || provenance.as_ref().is_none_or(|owner| !owner.is_current()) {
            return Err(Exception::throw_type(
                ctx,
                "this Entity has been released and can no longer be updated",
            ));
        }
        if resolved
            .as_ref()
            .is_some_and(|alias| self.entities().view(alias.handle).is_none())
        {
            self.nested_view_handles.borrow_mut().remove(&token);
            return Err(Exception::throw_type(
                ctx,
                "this Entity has been released and can no longer be updated",
            ));
        }
        self.pending_nested
            .borrow_mut()
            .push_back(PendingNestedOperation::Update {
                runtime: Rc::downgrade(self),
                token,
                provenance: provenance.expect("validated nested provenance"),
                props,
            });
        Ok(())
    }

    fn queue_nested_view_notify(self: &Rc<Self>, ctx: &Ctx<'_>, token: u32) -> JsResult<()> {
        let pending = self.pending_nested.borrow();
        let pending_create = pending.iter().find_map(|operation| match operation {
            PendingNestedOperation::Create {
                token: candidate,
                view_type,
                policy,
                ..
            } if *candidate == token => Some(NestedViewProvenance {
                application: view_type.application.clone(),
                policy: policy.clone(),
            }),
            _ => None,
        });
        let pending_release = pending.iter().any(|operation| {
            matches!(operation, PendingNestedOperation::Release { token: candidate, .. } if *candidate == token)
        });
        drop(pending);
        let resolved = self.nested_view_handles.borrow().get(&token).cloned();
        let provenance = resolved
            .as_ref()
            .map(|alias| alias.provenance.clone())
            .or(pending_create)
            .or_else(|| self.in_flight_nested.borrow().get(&token).cloned());
        if pending_release || provenance.as_ref().is_none_or(|owner| !owner.is_current()) {
            return Err(Exception::throw_type(
                ctx,
                "cx.notify(entity) expects a live Entity from the current application",
            ));
        }
        if resolved
            .as_ref()
            .is_some_and(|alias| self.entities().view(alias.handle).is_none())
        {
            self.nested_view_handles.borrow_mut().remove(&token);
            return Err(Exception::throw_type(
                ctx,
                "cx.notify(entity) expects a live Entity from the current application",
            ));
        }
        self.pending_nested
            .borrow_mut()
            .push_back(PendingNestedOperation::Notify {
                runtime: Rc::downgrade(self),
                token,
                provenance: provenance.expect("validated nested provenance"),
            });
        Ok(())
    }

    fn queue_nested_view_release(self: &Rc<Self>, ctx: &Ctx<'_>, token: u32) -> JsResult<bool> {
        let pending = self.pending_nested.borrow();
        let pending_release = pending.iter().any(|operation| {
            matches!(operation, PendingNestedOperation::Release { token: candidate, .. } if *candidate == token)
        });
        let pending_create = pending.iter().find_map(|operation| match operation {
            PendingNestedOperation::Create {
                token: candidate,
                view_type,
                policy,
                ..
            } if *candidate == token => Some(NestedViewProvenance {
                application: view_type.application.clone(),
                policy: policy.clone(),
            }),
            _ => None,
        });
        drop(pending);
        let resolved = self.nested_view_handles.borrow().get(&token).cloned();
        let provenance = resolved
            .as_ref()
            .map(|alias| alias.provenance.clone())
            .or(pending_create)
            .or_else(|| self.in_flight_nested.borrow().get(&token).cloned());
        if provenance.as_ref().is_none_or(|owner| !owner.is_current()) {
            return Err(Exception::throw_type(
                ctx,
                "this Entity has been released and can no longer be released",
            ));
        }
        // Authority is checked before resolving entity liveness or changing
        // the alias table. A foreign caller must not distinguish a live token
        // from a dead one, nor clean up an alias owned by another application.
        if resolved
            .as_ref()
            .is_some_and(|alias| self.entities().view(alias.handle).is_none())
        {
            self.nested_view_handles.borrow_mut().remove(&token);
            return Ok(false);
        }
        if pending_release {
            return Ok(false);
        }
        self.pending_nested
            .borrow_mut()
            .push_back(PendingNestedOperation::Release {
                runtime: Rc::downgrade(self),
                token,
                provenance: provenance.expect("validated nested provenance"),
            });
        Ok(true)
    }

    /// Applies native nested-view requests only at an unlocked QuickJS
    /// boundary. Construction therefore goes through Task 2's exact
    /// `instantiate_nested_view` seam and its three bounded causal drains.
    pub(super) fn flush_pending_nested_views(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<()> {
        if self.flushing_nested.replace(true) {
            return Ok(());
        }
        let _flush_guard = NestedFlushGuard(&self.flushing_nested);
        loop {
            let operation = { self.pending_nested.borrow_mut().pop_front() };
            let Some(operation) = operation else {
                break;
            };
            let result = (|| -> Result<()> {
                match operation {
                    PendingNestedOperation::Create {
                        runtime,
                        token,
                        owner,
                        view_type,
                        policy,
                        props,
                    } => {
                        let runtime = runtime.upgrade().ok_or_else(|| {
                            anyhow!("the shell runtime shut down during child creation")
                        })?;
                        let provenance = NestedViewProvenance {
                            application: view_type.application.clone(),
                            policy: policy.clone(),
                        };
                        if !provenance.is_current() {
                            anyhow::bail!(
                                "this Entity creation does not belong to the current application"
                            );
                        }
                        runtime
                            .in_flight_nested
                            .borrow_mut()
                            .insert(token, provenance.clone());
                        let (_owner_scope, _) = scope::enter_with_application(
                            &runtime,
                            window,
                            cx,
                            ScopePhase::Event,
                            Some(owner),
                            policy.clone(),
                            view_type.application.clone(),
                        );
                        let handle = runtime.instantiate_nested_view(
                            &view_type,
                            policy,
                            Some(props),
                            window,
                            cx,
                        );
                        runtime.in_flight_nested.borrow_mut().remove(&token);
                        let handle = handle?;
                        runtime
                            .nested_view_handles
                            .borrow_mut()
                            .insert(token, NestedViewAlias { handle, provenance });
                        Ok(())
                    }
                    PendingNestedOperation::EditDock {
                        runtime,
                        dock,
                        edit,
                        provenance,
                    } => {
                        let runtime = runtime.upgrade().ok_or_else(|| {
                            anyhow!("the shell runtime shut down before a dock edit was applied")
                        })?;
                        if !provenance.is_current() {
                            anyhow::bail!(
                                "this dock edit does not belong to the current application"
                            );
                        }
                        let (_scope, _) = scope::enter_with_application(
                            &runtime,
                            window,
                            cx,
                            ScopePhase::Event,
                            scope::current_view(),
                            provenance.policy.clone(),
                            provenance.application.clone(),
                        );
                        dock_api::apply_edit(&runtime, dock, *edit, window, cx)
                    }
                    PendingNestedOperation::Update {
                        runtime,
                        token,
                        provenance,
                        props,
                    } => {
                        let runtime = runtime.upgrade().ok_or_else(|| {
                            anyhow!("the shell runtime shut down during child update")
                        })?;
                        if !provenance.is_current() {
                            anyhow::bail!("this Entity does not belong to the current application");
                        }
                        let handle = runtime
                            .nested_view_handles
                            .borrow()
                            .get(&token)
                            .filter(|alias| {
                                Rc::ptr_eq(&alias.provenance.policy, &provenance.policy)
                                    && match (
                                        &alias.provenance.application,
                                        &provenance.application,
                                    ) {
                                        (Some(left), Some(right)) => Rc::ptr_eq(left, right),
                                        (None, None) => true,
                                        _ => false,
                                    }
                            })
                            .map(|alias| alias.handle)
                            .ok_or_else(|| anyhow!("this Entity was released before its update"))?;
                        runtime.update_nested_view(handle, props, window, cx)?;
                        Ok(())
                    }
                    PendingNestedOperation::Notify {
                        runtime,
                        token,
                        provenance,
                    } => {
                        let runtime = runtime.upgrade().ok_or_else(|| {
                            anyhow!("the shell runtime shut down during child notification")
                        })?;
                        if !provenance.is_current() {
                            anyhow::bail!("this Entity does not belong to the current application");
                        }
                        let view = runtime
                            .nested_view_handles
                            .borrow()
                            .get(&token)
                            .filter(|alias| {
                                Rc::ptr_eq(&alias.provenance.policy, &provenance.policy)
                                    && match (
                                        &alias.provenance.application,
                                        &provenance.application,
                                    ) {
                                        (Some(left), Some(right)) => Rc::ptr_eq(left, right),
                                        (None, None) => true,
                                        _ => false,
                                    }
                            })
                            .and_then(|alias| runtime.entities().view(alias.handle))
                            .ok_or_else(|| {
                                anyhow!("this Entity was released before its notification")
                            })?;
                        view.update(cx, |view, cx| view.refresh(cx));
                        Ok(())
                    }
                    PendingNestedOperation::Release {
                        runtime,
                        token,
                        provenance,
                    } => {
                        let runtime = runtime.upgrade().ok_or_else(|| {
                            anyhow!("the shell runtime shut down during child release")
                        })?;
                        if !provenance.is_current() {
                            anyhow::bail!("this Entity does not belong to the current application");
                        }
                        let alias = runtime
                            .nested_view_handles
                            .borrow()
                            .get(&token)
                            .cloned()
                            .filter(|alias| {
                                Rc::ptr_eq(&alias.provenance.policy, &provenance.policy)
                                    && match (
                                        &alias.provenance.application,
                                        &provenance.application,
                                    ) {
                                        (Some(left), Some(right)) => Rc::ptr_eq(left, right),
                                        (None, None) => true,
                                        _ => false,
                                    }
                            })
                            .ok_or_else(|| {
                                anyhow!("this Entity was released before its release operation")
                            })?;
                        runtime.nested_view_handles.borrow_mut().remove(&token);
                        let handle = alias.handle;
                        let released = runtime.release_view_handle(handle, cx);
                        if !released {
                            anyhow::bail!("this Entity was released before its release operation");
                        }
                        Ok(())
                    }
                }
            })();
            if let Err(error) = result {
                self.pending_nested.borrow_mut().clear();
                return Err(error);
            }
        }
        Ok(())
    }

    /// Delivers props to the child under a bounded ordinary-state/resource
    /// rollback boundary, and refreshes only after the causal wave succeeds.
    /// Ordinary properties on reachable objects and callable objects are
    /// restorable only while their post-update descriptors remain legally
    /// redefinable/deletable. Private/internal JS state, non-configurable
    /// additions/hardening and destructive release of pre-existing native
    /// handles are outside that boundary.
    fn update_nested_view(
        self: &Rc<Self>,
        handle: EntityHandle,
        props: Persistent<Value<'static>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<()> {
        scheduler::drain_jobs_transactionally(self, window, cx)?;
        // End the store borrow before update or any continuation can re-enter
        // JavaScript.
        let view = { self.entities().view(handle) }
            .ok_or_else(|| anyhow!("this Entity has been released and cannot be updated"))?;
        let (object, policy, application) = {
            let child = view.read(cx);
            (
                child.object().clone(),
                child.policy(),
                child.application_generation(),
            )
        };
        // Only when the child has an `update` to run. Nothing else in this
        // path is script, so a child without one has nothing that could need
        // rolling back -- and the checkpoint is the most expensive thing here
        // by a wide margin: it walks every object reachable from the instance
        // and reads a descriptor for every property on each. A view that holds
        // its application, which is the ordinary way to write one, therefore
        // paid for the whole model on every `set_props`.
        let state_checkpoint = self.checkpoint_view_object_if_updatable(&object)?;
        let entity_checkpoint = { self.entities().checkpoint() };
        let task_checkpoint = scheduler::checkpoint_runtime_tasks(self);
        let (event_scope, _) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            Some(view.clone()),
            policy,
            application,
        );
        let updated = self.with_js(|ctx| {
            let props = props.restore(ctx)?;
            self.update_in_context(ctx, &object, props)
        });
        let update_jobs = scheduler::drain_jobs_transactionally(self, window, cx);
        drop(event_scope);
        match update_jobs.and(updated) {
            Ok(()) => {
                view.update(cx, |view, cx| view.refresh(cx));
                Ok(())
            }
            Err(error) => {
                let restored = match state_checkpoint {
                    Some(checkpoint) => self.restore_view_object(checkpoint),
                    None => Ok(()),
                };
                self.rollback_retained_since(entity_checkpoint, task_checkpoint, cx);
                if let Err(restore) = restored {
                    return Err(error.context(format!("failed to restore child state: {restore}")));
                }
                Err(error)
            }
        }
    }

    pub(crate) fn instantiate_for_view(
        self: &Rc<Self>,
        view_type: &ViewType,
        view: Entity<ScriptView>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<ViewObject> {
        let policy = view.read(cx).policy();
        let application = view_type.application.clone();
        let (_guard, _) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            Some(view),
            policy,
            application.clone(),
        );
        let object = match self.construct(view_type) {
            Ok(object) => object,
            Err(error) => {
                if let Some(application) = application {
                    self.release_application_generation(&application, cx);
                }
                return Err(error);
            }
        };
        let initialized = self.initialize(&object, None);
        let nested = self.flush_pending_nested_views(window, cx);
        if let Err(error) = initialized.and(nested) {
            if let Some(application) = application {
                self.release_application_generation(&application, cx);
            }
            return Err(error);
        }
        Ok(object)
    }

    fn construct(&self, view_type: &ViewType) -> Result<ViewObject> {
        self.with_jit_suspended(|| {
            self.with_js(|ctx| {
                let class = view_type.value.clone().restore(ctx)?;
                let construct: Function = ctx.globals().get("__construct")?;
                let instance: Object = construct.call((class,))?;
                Ok(ViewObject {
                    value: Persistent::save(ctx, instance),
                    module_lease: view_type.module_lease.clone(),
                    application: view_type.application.clone(),
                    jit_warm: Rc::new(Cell::new(false)),
                })
            })
        })
    }

    /// Captures reachable ordinary objects and callable objects without invoking getters.
    /// The returned closure restores descriptors in place only when their
    /// post-update state still permits the required redefinition/deletion,
    /// preserving object identity for callbacks and tasks that already captured
    /// the instance. Private/internal slots, non-configurable additions, and a
    /// property hardened from configurable to non-configurable cannot be
    /// restored by JavaScript reflection.
    /// The checkpoint, for a child that has script to run.
    ///
    /// `update_in_context` returns without calling anything when the instance
    /// has no `update`, so for such a child the walk journals a graph nothing
    /// is going to touch.
    fn checkpoint_view_object_if_updatable(
        &self,
        object: &ViewObject,
    ) -> Result<Option<ViewStateCheckpoint>> {
        let updatable = self.with_js(|ctx| {
            let instance = object.value.clone().restore(ctx)?;
            let update: Value = instance.get("update")?;
            Ok(!(update.is_undefined() || update.is_null()))
        })?;
        if !updatable {
            return Ok(None);
        }
        self.checkpoint_view_object(object).map(Some)
    }

    fn checkpoint_view_object(&self, object: &ViewObject) -> Result<ViewStateCheckpoint> {
        self.with_js(|ctx| {
            let instance = object.value.clone().restore(ctx)?;
            let checkpoint: Function = ctx.globals().get("__checkpoint_view")?;
            let restore: Function = checkpoint.call((instance,))?;
            Ok(ViewStateCheckpoint(Persistent::save(ctx, restore)))
        })
    }

    fn restore_view_object(&self, checkpoint: ViewStateCheckpoint) -> Result<()> {
        self.with_js(|ctx| {
            let restore = checkpoint.0.restore(ctx)?;
            restore.call::<_, ()>(())
        })
    }

    fn initialize(
        &self,
        object: &ViewObject,
        initial_props: Option<Persistent<Value<'static>>>,
    ) -> Result<()> {
        self.initializing_views.borrow_mut().push(object.clone());
        let initialized = self.with_jit_suspended(|| {
            self.with_js(|ctx| {
                let instance = object.value.clone().restore(ctx)?;
                let initialize: Function = ctx.globals().get("__initialize")?;
                let props = match initial_props {
                    Some(props) => props.restore(ctx)?,
                    None => Value::new_undefined(ctx.clone()),
                };
                initialize.call::<_, ()>((instance, props))
            })
        });
        let initializing = self.initializing_views.borrow_mut().pop();
        debug_assert!(initializing.is_some());
        initialized
    }

    fn update_in_context<'js>(
        &self,
        ctx: &Ctx<'js>,
        object: &ViewObject,
        props: Value<'js>,
    ) -> JsResult<()> {
        let instance = object.value.clone().restore(ctx)?;
        let update: Value = instance.get("update")?;
        if update.is_undefined() || update.is_null() {
            return Ok(());
        }
        let update = update.as_function().ok_or_else(|| {
            Exception::throw_type(ctx, "a nested view's update property must be a function")
        })?;
        update.call::<_, ()>((This(instance), props))
    }

    /// Runs the script's `render` and freezes what it described.
    ///
    /// This is the only path into the VM's render function, and it is called
    /// only when a view says its description may be out of date — never once
    /// per frame. Everything it produces belongs to the returned snapshot:
    /// the element descriptions, the root, and the handlers registered while
    /// building it.
    ///
    /// The build is transactional. The scratch arena and an open callback
    /// generation are staging; they are published together at the end, and a
    /// script that throws discards both, leaving whatever snapshot the caller
    /// already had untouched.
    pub(crate) fn build_snapshot(
        self: &Rc<Self>,
        object: &ViewObject,
        view: Option<Entity<ScriptView>>,
        policy: Rc<crate::policy::Policy>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<RenderSnapshot> {
        // One theme sync per description makes cx.theme() a JS-only cache read.
        // The native snapshot crosses the boundary only when this revision
        // changes, rather than once per component asking for the theme.
        crate::theme_tokens::sync(cx);
        self.arena.borrow_mut().reset();
        let callbacks = self.callbacks.borrow_mut().begin();

        let first_render = !object.jit_warm.get();
        let render = || {
            self.metrics.time_script_render(|| {
                let (_guard, generation) = scope::enter_with_application(
                    self,
                    window,
                    cx,
                    ScopePhase::Render,
                    view.clone(),
                    policy.clone(),
                    object.application_generation(),
                );
                (self.call_render(object, generation), policy)
            })
        };
        let (root, policy) = if first_render {
            self.with_jit_suspended(|| Ok(render()))?
        } else {
            render()
        };

        let root = match root {
            Ok(root) => {
                object.jit_warm.set(true);
                root
            }
            Err(error) => {
                self.callbacks.borrow_mut().abort();
                self.arena.borrow_mut().reset();
                if let Some(view) = view {
                    scheduler::drain_after_render(self, view, policy, window, cx);
                }
                return Err(error);
            }
        };

        self.callbacks.borrow_mut().commit();
        // Taking the arena publishes the description and leaves a fresh scratch
        // arena behind, so the snapshot owns its nodes outright rather than
        // sharing them with the next build.
        let arena = std::mem::take(&mut *self.arena.borrow_mut());
        let snapshot = RenderSnapshot::new(
            self,
            callbacks,
            root,
            arena,
            object.application_generation(),
            view.as_ref().map(Entity::downgrade),
        );

        // Promise callbacks only run when the host drains QuickJS's job queue.
        // That drain is deferred to the event loop rather than run here: a
        // continuation is application code of unbounded length, and a render is
        // the last path it belongs on. It costs one check when nothing is
        // queued, which is the usual case.
        if let Some(view) = view {
            scheduler::drain_after_render(self, view, policy, window, cx);
        }

        Ok(snapshot)
    }

    /// Runs the script and returns the element description as text.
    ///
    /// The description is plain data, so interface structure can be asserted in
    /// tests that never paint a frame. This runs the script; to read a
    /// description that has already been built, use
    /// [`RenderSnapshot::debug_tree`] instead — that path never enters the VM.
    pub(crate) fn render_to_spec(
        self: &Rc<Self>,
        object: &ViewObject,
        view: Option<Entity<ScriptView>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<String> {
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        Ok(self
            .build_snapshot(object, view, policy, window, cx)?
            .debug_tree())
    }

    /// Releases the handlers registered while one snapshot was built.
    ///
    /// Called by [`RenderSnapshot`] as it drops, which is what ties handler
    /// lifetime to snapshot lifetime rather than to a frame.
    pub(crate) fn retire_callbacks(&self, generation: u64) {
        self.callbacks.borrow_mut().retire(generation);
    }

    /// Retires every callback registered by one retained view, including
    /// generations still held by a rendered frame.
    pub(crate) fn retire_view_callbacks(&self, entity_id: gpui::EntityId) {
        self.callbacks.borrow_mut().retain(|entry| {
            entry
                .view
                .as_ref()
                .is_none_or(|owner| owner.entity_id() != entity_id)
        });
    }

    /// How many handlers are callable right now. See [`CallbackArena::len`].
    #[cfg(test)]
    pub(crate) fn live_callbacks(&self) -> usize {
        self.callbacks.borrow().len()
    }

    #[cfg(test)]
    pub(crate) fn live_callback_ids(&self) -> Vec<CallbackId> {
        self.callbacks.borrow().ids()
    }

    /// Describes one window of a virtualized list's items.
    ///
    /// The one call into script that is *not* a snapshot build and *not* an
    /// event: GPUI runs it from inside layout and prepaint, so it happens on a
    /// frame's budget rather than on an application's. See the exception
    /// recorded in [`crate::materialize`] for why that trade is the right one
    /// here and nowhere else.
    ///
    /// Three things make it safe to enter the VM from there:
    ///
    /// * The scope is [`ScopePhase::Layout`], which forbids `cx.notify()` —
    ///   a re-render requested from inside layout is a loop — along with
    ///   creating retained state, and runs on the render-time budget.
    /// * The batch describes itself into an arena of its own, swapped in for
    ///   the duration. The runtime's scratch arena belongs to whichever script
    ///   render is in progress; a batch writing into it would survive into that
    ///   render's snapshot. Swapping is strictly nested, so a list inside a
    ///   list is no different from one on its own.
    /// * Nothing is drained afterwards. `dispatch_click` runs QuickJS's job
    ///   queue on the way out because an event handler may have resolved a
    ///   promise; a continuation is application code of unbounded length, and
    ///   running one part-way through GPUI's layout pass is the last place it
    ///   belongs. Queued jobs wait for the event loop, as they would have
    ///   anyway.
    pub(crate) fn render_virtual_items(
        self: &Rc<Self>,
        id: CallbackId,
        get_key: CallbackId,
        range: std::ops::Range<usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<crate::spec::ItemSpecs> {
        let entry = self.callbacks.borrow().get(id)?;
        let key_entry = self.callbacks.borrow().get(get_key)?;

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("item renderer {id} belongs to a retired application");
            return None;
        }

        let view = entry.live_view()?;
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Layout,
            view,
            policy,
            entry.application.clone(),
        );
        // The renderer is a closure the script wrote inside `render(cx)`, and
        // the row helpers it calls take that `cx`. Layout is a frame of its own,
        // so without this the enclosing render's `cx` would read as stale here
        // and every helper would need a second, list-only plumbing.
        scope::adopt(entry.registered_in);

        let outer = std::mem::take(&mut *self.arena.borrow_mut());
        let described = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            let key_handler = key_entry.value.clone().restore(ctx)?;
            let payload = Object::new(ctx.clone())?;
            payload.set("start", range.start)?;
            payload.set("end", range.end)?;
            let produced: Value =
                handler.call((payload, context_object(ctx, ContextBinding::Call(generation))?))?;
            let items = produced.into_array().ok_or_else(|| {
                Exception::throw_type(
                    ctx,
                    "a virtual list's item renderer must return an array of elements, one per                      item in the range it was given",
                )
            })?;
            let mut roots = SmallVec::new();
            for item in items.iter::<Value>() {
                roots.push(element_id(ctx, &item?)?);
            }
            let mut keys = Vec::new();
            keys.try_reserve_exact(range.len()).map_err(|_| {
                Exception::throw_range(ctx, "the virtual list item-key table could not be allocated")
            })?;
            let mut unique = HashSet::new();
            for index in range.clone() {
                let key: String = key_handler.call((index,))?;
                if !unique.insert(key.clone()) {
                    return Err(Exception::throw_type(
                        ctx,
                        &format!("virtual list get_key returned duplicate key `{key}` in one visible range"),
                    ));
                }
                keys.push(key);
            }
            Ok((roots, keys))
        });
        let arena = std::mem::replace(&mut *self.arena.borrow_mut(), outer);

        match described {
            Ok((roots, keys)) => Some(crate::spec::ItemSpecs::new(arena, roots, keys)),
            Err(error) => {
                tracing::error!("error in virtual list item renderer: {error}");
                None
            }
        }
    }

    /// Draws one piece of a dock's chrome.
    ///
    /// The second call into script that runs on a frame's budget rather than an
    /// application's, and it is safe there for the same three reasons a virtual
    /// list's item renderer is: a [`ScopePhase::Layout`] scope, which forbids
    /// `cx.notify()` and creating retained state; an arena of its own, swapped
    /// in so the description cannot leak into whichever snapshot is being
    /// built; and no job drain on the way out, because a promise continuation
    /// is unbounded application code and GPUI's layout pass is the last place
    /// to run one.
    ///
    /// It differs from the list in one way. A chrome handler is called once per
    /// callback-and-payload combination, then its description is replayed from
    /// the owning dock's bounded cache. It may not register callbacks of its
    /// own — the `Layout` phase already refuses that — and its elements report
    /// what they do through
    /// [`DockCommand`](crate::dock::DockCommand)s instead.
    ///
    /// The outer `None` means the handler was unavailable or threw and must not
    /// be cached. An inner `None` root is a successful `null`: valid empty
    /// chrome which the caller can cache like any other description.
    pub(super) fn describe_dock_chrome(
        self: &Rc<Self>,
        id: CallbackId,
        payload: &serde_json::Value,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<(SpecArena, Option<SpecId>)> {
        let entry = self.callbacks.borrow().get(id)?;

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("dock chrome handler {id} belongs to a retired application");
            return None;
        }

        let view = entry.live_view()?;
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);

        self.metrics.time_frame_script(|| {
            let (_guard, generation) = scope::enter_with_application(
                self,
                window,
                cx,
                ScopePhase::Layout,
                view,
                policy,
                entry.application.clone(),
            );
            // The handler is a closure the script wrote inside `render(cx)`,
            // and the element helpers it calls take that `cx`. Layout is a
            // frame of its own, so without this the enclosing render's `cx`
            // would read as stale here — the same reason a list's item renderer
            // adopts it.
            scope::adopt(entry.registered_in);

            let outer = std::mem::take(&mut *self.arena.borrow_mut());
            let described = self.with_js(|ctx| {
                let handler = entry.value.clone().restore(ctx)?;
                let produced: Value = handler.call((
                    dock_api::to_js(ctx, payload)?,
                    context_object(ctx, ContextBinding::Call(generation))?,
                ))?;
                if produced.is_null() || produced.is_undefined() {
                    return Ok(None);
                }
                element_id(ctx, &produced).map(Some)
            });
            let arena = std::mem::replace(&mut *self.arena.borrow_mut(), outer);

            match described {
                Ok(root) => Some((arena, root)),
                Err(error) => {
                    tracing::error!("error in a dock chrome handler: {error}");
                    None
                }
            }
        })
    }

    /// Teaches the panel registry to rebuild `panel` from `class`, and answers
    /// with the interned name it registered under.
    pub(super) fn register_panel_class(
        self: &Rc<Self>,
        ctx: &Ctx<'_>,
        panel: &str,
        view_type: ViewType,
    ) -> JsResult<String> {
        let policy = scope::current_policy().ok_or_else(|| {
            Exception::throw_type(
                ctx,
                "DockArea.register_panel(name, Class) needs a live host call; call it from                  init() or an event handler",
            )
        })?;
        let application = policy.application().to_owned();
        let script = Rc::new(dock_api::ScriptPanelClass::new(
            Rc::downgrade(self),
            view_type,
            policy,
        ));

        let name = scope::with_current_app(|cx| {
            crate::dock::register_panel(&application, panel, script.clone(), cx)
        })
        .ok_or_else(|| {
            Exception::throw_type(
                ctx,
                "DockArea.register_panel(name, Class) needs a live host call; call it from                  init() or an event handler",
            )
        })?;

        self.panel_scripts
            .borrow_mut()
            .insert(name.to_owned(), script);
        Ok(name.to_owned())
    }

    /// The script behind an already-registered panel name.
    ///
    /// A panel the script *adds* gets the same `serialize`/`deserialize` hooks
    /// a restored one does — otherwise a layout would round-trip only after a
    /// restart, which is the one time nobody is watching.
    pub(super) fn panel_script(&self, name: &str) -> Option<Rc<dyn crate::dock::PanelScript>> {
        self.panel_scripts
            .borrow()
            .get(name)
            .map(|script| script.clone() as Rc<dyn crate::dock::PanelScript>)
    }

    /// One panel's `serialize()`, or `None` for a panel that has none.
    ///
    /// No scope is opened, and none can be: `Panel::dump` is a read, so there
    /// is no `&mut Window` to open one with. A `serialize()` that calls back
    /// into the host therefore fails the way any host call outside a scope
    /// does, which is the contract this method's caller documents.
    pub(super) fn call_panel_serialize(&self, object: &ViewObject) -> Option<serde_json::Value> {
        let produced = self.with_js_nested(|ctx| {
            let instance = object.value.clone().restore(ctx)?;
            let Some(serialize) = instance.get::<_, Option<Function>>("serialize")? else {
                return Ok(None);
            };
            let produced: Value = serialize.call((This(instance),))?;
            if produced.is_null() || produced.is_undefined() {
                return Ok(None);
            }
            host::to_json(ctx, &produced, 0).map(Some)
        });

        match produced {
            Ok(value) => value,
            Err(error) => {
                tracing::error!("error in a dock panel's serialize(): {error}");
                None
            }
        }
    }

    /// Hands a persisted payload to one panel's `deserialize(data)`.
    pub(super) fn call_panel_deserialize(
        self: &Rc<Self>,
        view: &Entity<ScriptView>,
        data: &serde_json::Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        let (object, policy, application) = {
            let view = view.read(cx);
            (
                view.object().clone(),
                view.policy(),
                view.application_generation(),
            )
        };

        let (_guard, _) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            Some(view.clone()),
            policy,
            application,
        );

        let result = self.with_js_nested(|ctx| {
            let instance = object.value.clone().restore(ctx)?;
            let Some(deserialize) = instance.get::<_, Option<Function>>("deserialize")? else {
                return Ok(());
            };
            deserialize.call::<_, ()>((This(instance), dock_api::to_js(ctx, data)?))
        });
        if let Err(error) = result {
            tracing::error!("error in a dock panel's deserialize(data): {error}");
        }
        // The view described itself before the payload arrived, so what it
        // described is now out of date.
        view.update(cx, |view, cx| view.refresh(cx));
    }

    /// Queues a layout change to be applied at the next unlocked boundary.
    pub(super) fn queue_dock_edit(
        self: &Rc<Self>,
        ctx: &Ctx<'_>,
        dock: EntityHandle,
        edit: dock_api::DockEdit,
        api: &str,
    ) -> JsResult<()> {
        let policy = scope::current_policy().ok_or_else(|| nested_view_needs_call(ctx, api))?;
        let provenance = NestedViewProvenance {
            application: scope::current_application_generation(),
            policy,
        };
        self.pending_nested
            .borrow_mut()
            .push_back(PendingNestedOperation::EditDock {
                runtime: Rc::downgrade(self),
                dock,
                edit: Box::new(edit),
                provenance,
            });
        Ok(())
    }

    /// The entity handle behind one `cx.new(Class)` token, once the creation it
    /// was queued alongside has been applied.
    pub(super) fn nested_view_for_token(&self, token: u32) -> Option<EntityHandle> {
        self.nested_view_handles
            .borrow()
            .get(&token)
            .filter(|alias| alias.provenance.is_current())
            .map(|alias| alias.handle)
    }

    /// The authority and owner a long-lived subscription runs under.
    pub(super) fn callback_owner(&self) -> InputCallbackOwner {
        InputCallbackOwner {
            policy: scope::policy(),
            application: scope::current_application_generation(),
            view: scope::current_view().map(|view| view.downgrade()),
        }
    }

    /// Tells a script that the layout it is watching changed.
    ///
    /// The event carries nothing: what changed is the whole layout, and
    /// `dump()` is how a subscriber reads it.
    pub(super) fn dispatch_dock_event(
        self: &Rc<Self>,
        handler: &Persistent<Function<'static>>,
        owner: &InputCallbackOwner,
        window: &mut Window,
        cx: &mut App,
    ) {
        if owner
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("dock callback belongs to a retired application");
            return;
        }
        let view = owner.view.as_ref().and_then(WeakEntity::upgrade);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            owner.policy.clone(),
            owner.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = handler.clone().restore(ctx)?;
            handler.call::<_, ()>((context_object(ctx, ContextBinding::Call(generation))?,))
        });
        if let Err(error) = result {
            tracing::error!("error in dock layout handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    /// Reports which stable item of a collection something happened to.
    pub(crate) fn dispatch_item_key(
        self: &Rc<Self>,
        id: CallbackId,
        key: &str,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.dispatch_item(id, "item click", window, cx, |_, handler, context| {
            handler.call::<_, ()>((key, context))
        });
    }

    /// Delivers a secondary press on a virtual list row: the row's key, then
    /// the press exactly as `on_mouse_down` would report it, with
    /// `local_position` measured from the row's own box.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_item_mouse_button(
        self: &Rc<Self>,
        id: CallbackId,
        key: &str,
        button: gpui::MouseButton,
        position: gpui::Point<gpui::Pixels>,
        click_count: usize,
        modifiers: gpui::Modifiers,
        bounds: Option<gpui::Bounds<gpui::Pixels>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.dispatch_item(id, "item press", window, cx, |ctx, handler, context| {
            let event =
                mouse_button_payload(ctx, button, position, click_count, modifiers, bounds)?;
            handler.call::<_, ()>((key, event, context))
        });
    }

    /// The lifetime checks, scope entry and job drain every row-level dispatch
    /// shares. The call itself is the caller's, because the two row events
    /// hand the handler different argument lists.
    fn dispatch_item(
        self: &Rc<Self>,
        id: CallbackId,
        what: &str,
        window: &mut Window,
        cx: &mut App,
        call: impl for<'js> FnOnce(&Ctx<'js>, Function<'js>, Object<'js>) -> JsResult<()>,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("{what} callback {id} belongs to a superseded render pass");
            return;
        };

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("{what} callback {id} belongs to a retired application");
            return;
        }

        let Some(view) = entry.live_view() else {
            tracing::debug!("{what} callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            let context = context_object(ctx, ContextBinding::Call(generation))?;
            call(ctx, handler, context)
        });

        if let Err(error) = result {
            tracing::error!("error in {what} handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    /// Delivers a one-time-code event to a long-lived script subscription.
    pub(super) fn dispatch_otp_event(
        self: &Rc<Self>,
        handler: &Persistent<Function<'static>>,
        owner: &InputCallbackOwner,
        _event: &gpui_base::OtpEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        if owner
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("OTP callback belongs to a retired application");
            return;
        }
        let view = owner.view.as_ref().and_then(WeakEntity::upgrade);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            owner.policy.clone(),
            owner.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = handler.clone().restore(ctx)?;
            let payload = Object::new(ctx.clone())?;
            handler.call::<_, ()>((
                payload,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in OTP handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    pub(crate) fn dispatch_click(
        self: &Rc<Self>,
        id: CallbackId,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("click callback {id} belongs to a superseded render pass");
            return;
        };

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("click callback {id} belongs to a retired application");
            return;
        }

        let Some(view) = entry.live_view() else {
            tracing::debug!("click callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );
        let click_count = event.click_count();
        let modifiers = event.modifiers();

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            let payload = Object::new(ctx.clone())?;
            payload.set("click_count", click_count)?;

            payload.set("modifiers", modifiers_object(&ctx, modifiers)?)?;

            handler.call::<_, ()>((
                payload,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in click handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    /// Delivers an input event to a long-lived script subscription.
    ///
    /// Unlike a rendered callback this handler outlives the pass that created
    /// it, so it lives with the entity rather than in the per-frame arena.
    pub(super) fn dispatch_input_event(
        self: &Rc<Self>,
        handler: &Persistent<Function<'static>>,
        owner: &InputCallbackOwner,
        event: &gpui_base::input::InputEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_base::input::InputEvent;

        // Both owner and policy are captured when the script subscribes. The
        // input entity may outlive a view, so only a weak owner is retained; if
        // that owner is gone the callback may still run, but notify has no dead
        // view to keep alive or invalidate.
        if owner
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("input callback belongs to a retired application");
            return;
        }
        let view = owner.view.as_ref().and_then(WeakEntity::upgrade);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            owner.policy.clone(),
            owner.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = handler.clone().restore(ctx)?;
            let payload = Object::new(ctx.clone())?;
            match event {
                InputEvent::PressEnter { secondary, shift } => {
                    payload.set("secondary", *secondary)?;
                    payload.set("shift", *shift)?;
                }
                InputEvent::Change | InputEvent::Focus | InputEvent::Blur => {}
            }
            handler.call::<_, ()>((
                payload,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in input handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    /// Delivers a slider event to a long-lived script subscription.
    ///
    /// The value is the whole payload rather than a field of an object,
    /// because the value is the whole of what a slider event carries: one
    /// number, or the pair a two-thumbed slider moves between.
    /// Hands one selected date to a calendar's handler.
    ///
    /// The payload is the same two-slot array `value()` answers, and the
    /// prelude narrows it the same way — so a handler and a read see the same
    /// shape for the same date.
    pub(super) fn dispatch_calendar_event(
        self: &Rc<Self>,
        handler: &Persistent<Function<'static>>,
        owner: &InputCallbackOwner,
        date: gpui_base::Date,
        window: &mut Window,
        cx: &mut App,
    ) {
        use rquickjs::IntoJs as _;

        if owner
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("calendar callback belongs to a retired application");
            return;
        }
        let view = owner.view.as_ref().and_then(WeakEntity::upgrade);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            owner.policy.clone(),
            owner.application.clone(),
        );

        // The same conversion `value()` answers with, not a second copy of it:
        // a handler and a read that disagreed about the shape of one date
        // would be a bug nobody could see from either side alone.
        let parts = entity_api::date_to_parts(date);
        let result = self.with_js(|ctx| {
            let handler = handler.clone().restore(ctx)?;
            handler.call::<_, ()>((
                parts.into_js(ctx)?,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });
        if let Err(error) = result {
            tracing::error!("error in calendar handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    pub(super) fn dispatch_slider_event(
        self: &Rc<Self>,
        handler: &Persistent<Function<'static>>,
        owner: &InputCallbackOwner,
        value: gpui_base::slider::SliderValue,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_base::slider::SliderValue;
        use rquickjs::IntoJs as _;

        // Captured when the script subscribed, for the same reason an input's
        // are: the state outlives any one view, so the grant a handler runs
        // under has to be the one it was registered with.
        if owner
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("slider callback belongs to a retired application");
            return;
        }
        let view = owner.view.as_ref().and_then(WeakEntity::upgrade);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            owner.policy.clone(),
            owner.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = handler.clone().restore(ctx)?;
            let payload = match value {
                SliderValue::Single(value) => f64::from(value).into_js(ctx)?,
                SliderValue::Range(start, end) => {
                    vec![f64::from(start), f64::from(end)].into_js(ctx)?
                }
            };
            handler.call::<_, ()>((
                payload,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in slider handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    /// Reports the panel sizes of a resizable group after a drag, in pixels and
    /// in the group's child order.
    ///
    /// Sizes are not state the script has to keep: base files them in window
    /// element state under the group's own id, so a drag survives every repaint
    /// that never enters the VM. This is a notification — persist it, mirror it
    /// into a title bar — and a group that ignores it still resizes.
    pub(crate) fn dispatch_resize(
        self: &Rc<Self>,
        id: CallbackId,
        sizes: Vec<f32>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("resize callback {id} belongs to a superseded render pass");
            return;
        };

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("resize callback {id} belongs to a retired application");
            return;
        }

        let Some(view) = entry.live_view() else {
            tracing::debug!("resize callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            let payload = rquickjs::Array::new(ctx.clone())?;
            for (index, size) in sizes.iter().enumerate() {
                payload.set(index, *size)?;
            }
            handler.call::<_, ()>((
                payload,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in resize handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    pub(crate) fn dispatch_mouse_move(
        self: &Rc<Self>,
        id: CallbackId,
        event: &gpui::MouseMoveEvent,
        local: gpui::Point<gpui::Pixels>,
        bounds: gpui::Bounds<gpui::Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("mouse move callback {id} belongs to a superseded render pass");
            return;
        };
        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("mouse move callback {id} belongs to a retired application");
            return;
        }
        let Some(view) = entry.live_view() else {
            tracing::debug!("mouse move callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );
        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            let payload = Object::new(ctx.clone())?;
            let position = Object::new(ctx.clone())?;
            position.set("x", f32::from(event.position.x))?;
            position.set("y", f32::from(event.position.y))?;
            payload.set("position", position)?;
            let local_position = Object::new(ctx.clone())?;
            local_position.set("x", f32::from(local.x))?;
            local_position.set("y", f32::from(local.y))?;
            payload.set("local_position", local_position)?;
            let event_bounds = Object::new(ctx.clone())?;
            event_bounds.set("x", f32::from(bounds.origin.x))?;
            event_bounds.set("y", f32::from(bounds.origin.y))?;
            event_bounds.set("width", f32::from(bounds.size.width))?;
            event_bounds.set("height", f32::from(bounds.size.height))?;
            payload.set("bounds", event_bounds)?;
            payload.set("modifiers", modifiers_object(&ctx, event.modifiers)?)?;
            handler.call::<_, ()>((
                payload,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });
        if let Err(error) = result {
            tracing::error!("error in mouse move handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    /// Delivers one dispatched action to a script handler.
    ///
    /// The id is handed over even though the handler was registered for one
    /// action: a script that routes several ids into one function has the name
    /// it needs without closing over it, and a handler that ignores the
    /// argument costs nothing.
    pub(crate) fn dispatch_action(
        self: &Rc<Self>,
        id: CallbackId,
        action: &str,
        window: &mut Window,
        cx: &mut App,
    ) {
        let action = action.to_owned();
        self.dispatch_simple_event(id, "action", window, cx, move |ctx| {
            let payload = Object::new(ctx.clone())?;
            payload.set("action", action)?;
            Ok(payload)
        });
    }

    /// Delivers one mouse press or release to a script handler.
    ///
    /// One method for four element builders because the payload is the same
    /// shape in all four: `on_mouse_down`, `on_mouse_up` and `on_mouse_up_out`
    /// differ only in which button and phase GPUI filtered on before calling,
    /// and `on_mouse_down_out` differs only in where the pointer was — which
    /// the caller can read off `local_position` for itself.
    ///
    /// `bounds` is what the element measured on its last prepaint. An element
    /// that has not been painted yet has none, and rather than refuse the event
    /// the local coordinates are simply omitted: a press is still a press, and
    /// the window position is always there.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_mouse_button(
        self: &Rc<Self>,
        id: CallbackId,
        button: gpui::MouseButton,
        position: gpui::Point<gpui::Pixels>,
        click_count: usize,
        modifiers: gpui::Modifiers,
        bounds: Option<gpui::Bounds<gpui::Pixels>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.dispatch_simple_event(id, "mouse button", window, cx, move |ctx| {
            mouse_button_payload(ctx, button, position, click_count, modifiers, bounds)
        });
    }

    /// Delivers one wheel or trackpad scroll to a script handler.
    pub(crate) fn dispatch_scroll_wheel(
        self: &Rc<Self>,
        id: CallbackId,
        event: &gpui::ScrollWheelEvent,
        line_height: gpui::Pixels,
        bounds: Option<gpui::Bounds<gpui::Pixels>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let pixels = event.delta.pixel_delta(line_height);
        let lines = match event.delta {
            gpui::ScrollDelta::Lines(lines) => Some(lines),
            gpui::ScrollDelta::Pixels(_) => None,
        };
        let position = event.position;
        let modifiers = event.modifiers;
        let touch_phase = match event.touch_phase {
            gpui::TouchPhase::Started => "started",
            gpui::TouchPhase::Moved => "moved",
            gpui::TouchPhase::Ended => "ended",
            gpui::TouchPhase::Cancelled => "cancelled",
        };
        self.dispatch_simple_event(id, "scroll wheel", window, cx, move |ctx| {
            let payload = Object::new(ctx.clone())?;
            let delta = Object::new(ctx.clone())?;
            delta.set("x", f32::from(pixels.x))?;
            delta.set("y", f32::from(pixels.y))?;
            payload.set("delta", delta)?;
            match lines {
                Some(lines) => {
                    let delta_lines = Object::new(ctx.clone())?;
                    delta_lines.set("x", lines.x)?;
                    delta_lines.set("y", lines.y)?;
                    payload.set("delta_lines", delta_lines)?;
                }
                None => payload.set("delta_lines", rquickjs::Undefined)?,
            }
            payload.set("touch_phase", touch_phase)?;
            set_pointer_geometry(ctx, &payload, position, bounds)?;
            payload.set("modifiers", modifiers_object(ctx, modifiers)?)?;
            Ok(payload)
        });
    }

    /// The lifetime checks, scope entry and job drain every dispatch shares,
    /// with only the payload left to the caller.
    ///
    /// "Simple" is about the payload, not the event: what these have in common
    /// is that the handler takes one object and a `cx` and its return value is
    /// ignored. A dispatch that has to read an answer back — a controlled
    /// value, a confirm — does its own thing.
    fn dispatch_simple_event(
        self: &Rc<Self>,
        id: CallbackId,
        what: &str,
        window: &mut Window,
        cx: &mut App,
        payload: impl for<'js> FnOnce(&Ctx<'js>) -> JsResult<Object<'js>>,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("{what} callback {id} belongs to a superseded render pass");
            return;
        };
        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("{what} callback {id} belongs to a retired application");
            return;
        }
        let Some(view) = entry.live_view() else {
            tracing::debug!("{what} callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );
        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            handler.call::<_, ()>((
                payload(ctx)?,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });
        if let Err(error) = result {
            tracing::error!("error in {what} handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    pub(crate) fn dispatch_modifiers_changed(
        self: &Rc<Self>,
        id: CallbackId,
        event: &gpui::ModifiersChangedEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        let modifiers = event.modifiers;
        let capslock = event.capslock;
        self.dispatch_simple_event(id, "modifiers changed", window, cx, move |ctx| {
            let payload = Object::new(ctx.clone())?;
            payload.set("modifiers", modifiers_object(ctx, modifiers)?)?;
            let capslock_object = Object::new(ctx.clone())?;
            capslock_object.set("on", capslock.on)?;
            payload.set("capslock", capslock_object)?;
            Ok(payload)
        });
    }

    /// Delivers one key press or release to a script handler.
    ///
    /// `is_held` distinguishes the two events rather than a second method
    /// doing it: `KeyUpEvent` carries a keystroke and nothing else, so `None`
    /// is what "this was a release" looks like on the wire, and the payload
    /// keeps the same shape either way.
    ///
    /// The keystroke is handed over twice, and both forms earn their place.
    /// `key` and `modifiers` are what GPUI holds, and a script that wants one
    /// half of a chord reads them. `keystroke` is `Keystroke`'s own `unparse`
    /// — the `"cmd-shift-s"` spelling a key binding is written in — which is
    /// the form a comparison is actually written against, and reproducing it
    /// from the parts is exactly the fiddly work that belongs on this side.
    pub(crate) fn dispatch_key(
        self: &Rc<Self>,
        id: CallbackId,
        keystroke: &gpui::Keystroke,
        is_held: Option<bool>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("key callback {id} belongs to a superseded render pass");
            return;
        };
        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("key callback {id} belongs to a retired application");
            return;
        }
        let Some(view) = entry.live_view() else {
            tracing::debug!("key callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );
        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            let payload = Object::new(ctx.clone())?;
            payload.set("key", keystroke.key.as_str())?;
            payload.set("keystroke", script_keystroke(keystroke))?;
            match keystroke.key_char.as_deref() {
                Some(char) => payload.set("key_char", char)?,
                None => payload.set("key_char", rquickjs::Undefined)?,
            }
            if let Some(is_held) = is_held {
                payload.set("is_held", is_held)?;
            }
            payload.set("modifiers", modifiers_object(ctx, keystroke.modifiers)?)?;
            handler.call::<_, ()>((
                payload,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });
        if let Err(error) = result {
            tracing::error!("error in key handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    /// Controlled-value handlers report intent; the script stores the value and
    /// notifies. The host never mutates script state on its behalf.
    pub(crate) fn dispatch_change(
        self: &Rc<Self>,
        id: CallbackId,
        checked: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("change callback {id} belongs to a superseded render pass");
            return;
        };

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("change callback {id} belongs to a retired application");
            return;
        }

        let Some(view) = entry.live_view() else {
            tracing::debug!("change callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            handler.call::<_, ()>((
                checked,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in change handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    pub(crate) fn dispatch_host_event(
        self: &Rc<Self>,
        id: CallbackId,
        payload: HostValue,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("host component callback {id} belongs to a superseded render pass");
            return;
        };
        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            return;
        }
        let Some(view) = entry.live_view() else {
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );
        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            let payload = host_modules::into_js(ctx, payload)?;
            handler.call::<_, ()>((
                payload,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });
        if let Err(error) = result {
            tracing::error!("error in host component handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    /// Reports which way a `NumberInput` stepped, by the two names base's
    /// `StepAction` carries.
    ///
    /// A string rather than a boolean, and not because two directions could not
    /// be one: `dispatch_change`'s `true` means "checked", and a handler reading
    /// `true` as "up" would be reading the wrong word. The script gets
    /// `"increment"` or `"decrement"`, which is what base calls them.
    pub(crate) fn dispatch_step(
        self: &Rc<Self>,
        id: CallbackId,
        action: &'static str,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("step callback {id} belongs to a superseded render pass");
            return;
        };

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("step callback {id} belongs to a retired application");
            return;
        }

        let Some(view) = entry.live_view() else {
            tracing::debug!("step callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            handler.call::<_, ()>((
                action,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in step handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    /// Delivers a handler that reports only that something happened.
    ///
    /// `on_confirm` and `on_dismiss` have no value to carry: the combobox root
    /// they come from holds neither the options nor the selection, so the news
    /// is the action itself. The script still receives `(payload, cx)` with an
    /// empty payload, so every rendered handler has the same shape whether or
    /// not there was anything to put in it.
    pub(crate) fn dispatch_signal(
        self: &Rc<Self>,
        id: CallbackId,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("signal callback {id} belongs to a superseded render pass");
            return;
        };

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("signal callback {id} belongs to a retired application");
            return;
        }

        let Some(view) = entry.live_view() else {
            tracing::debug!("signal callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            handler.call::<_, ()>((
                Object::new(ctx.clone())?,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in signal handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    pub(crate) fn dispatch_component_callback_value(
        self: &Rc<Self>,
        id: CallbackId,
        arguments: &[ComponentCallbackArgument],
        window: &mut Window,
        cx: &mut App,
    ) -> Result<ComponentCallbackValue> {
        let entry = self
            .callbacks
            .borrow()
            .get(id)
            .ok_or_else(|| anyhow!("component callback {id} belongs to a superseded render"))?;
        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            return Err(anyhow!(
                "component callback {id} belongs to a retired application"
            ));
        }
        let view = entry
            .live_view()
            .ok_or_else(|| anyhow!("component callback {id} owner has been released"))?;
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );
        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            let mut js_arguments = JsArgs::new(ctx.clone(), arguments.len() + 1);
            for argument in arguments {
                js_arguments.push_arg(callback_argument_to_js(ctx, argument)?)?;
            }
            js_arguments.push_arg(context_object(ctx, ContextBinding::Call(generation))?)?;
            let value: Value<'_> = handler.call_arg(js_arguments)?;
            if value.is_null() || value.is_undefined() {
                Ok(ComponentCallbackValue::Null)
            } else if let Some(value) = value.as_bool() {
                Ok(ComponentCallbackValue::Boolean(value))
            } else if let Some(value) = value.as_number().filter(|value| value.is_finite()) {
                Ok(ComponentCallbackValue::Number(value))
            } else if let Some(value) = value.as_string() {
                Ok(ComponentCallbackValue::String(value.to_string()?))
            } else {
                Err(Exception::throw_type(
                    &ctx,
                    "component callbacks may only return null, boolean, finite number, or string",
                ))
            }
        });
        scheduler::drain_runtime_jobs(self, window, cx);
        result
    }

    pub(crate) fn dispatch_component_data_callback(
        self: &Rc<Self>,
        id: CallbackId,
        arguments: &[ComponentCallbackArgument],
        window: &mut Window,
        cx: &mut App,
    ) -> Result<ComponentDataValue> {
        let entry = self
            .callbacks
            .borrow()
            .get(id)
            .ok_or_else(|| anyhow!("component callback {id} belongs to a superseded render"))?;
        if entry.application.as_ref().is_some_and(|a| !a.is_active()) {
            return Err(anyhow!(
                "component callback {id} belongs to a retired application"
            ));
        }
        let view = entry
            .live_view()
            .ok_or_else(|| anyhow!("component callback {id} owner has been released"))?;
        let policy = view
            .as_ref()
            .map(|v| v.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        self.metrics.time_frame_script(|| {
            let (_guard, generation) = scope::enter_with_application(
                self,
                window,
                cx,
                ScopePhase::Layout,
                view,
                policy,
                entry.application.clone(),
            );
            scope::adopt(entry.registered_in);
            let temporary = TemporarySpecArena::enter(self);
            let result = self.with_js(|ctx| {
                let handler = entry.value.clone().restore(ctx)?;
                let mut args = JsArgs::new(ctx.clone(), arguments.len() + 1);
                push_component_callback_arguments(ctx, &mut args, arguments)?;
                args.push_arg(context_object(ctx, ContextBinding::Call(generation))?)?;
                let value: Value = handler.call_arg(args)?;
                component_data_from_js(&ctx, value, 0, &mut ComponentDataBudget::default())
            });
            drop(temporary.finish());
            result.map_err(Into::into)
        })
    }

    pub(crate) fn dispatch_component_element_callback(
        self: &Rc<Self>,
        id: CallbackId,
        arguments: &[ComponentCallbackArgument],
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Option<gpui::AnyElement>> {
        let entry = self
            .callbacks
            .borrow()
            .get(id)
            .ok_or_else(|| anyhow!("component callback {id} belongs to a superseded render"))?;
        if entry.application.as_ref().is_some_and(|a| !a.is_active()) {
            return Err(anyhow!(
                "component callback {id} belongs to a retired application"
            ));
        }
        let view = entry
            .live_view()
            .ok_or_else(|| anyhow!("component callback {id} owner has been released"))?;
        let policy = view
            .as_ref()
            .map(|v| v.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        self.metrics.time_frame_script(|| {
            let (_guard, generation) = scope::enter_with_application(
                self,
                window,
                cx,
                ScopePhase::Layout,
                view,
                policy,
                entry.application.clone(),
            );
            scope::adopt(entry.registered_in);
            let temporary = TemporarySpecArena::enter(self);
            let described = self.with_js(|ctx| {
                let handler = entry.value.clone().restore(ctx)?;
                let mut args = JsArgs::new(ctx.clone(), arguments.len() + 1);
                push_component_callback_arguments(ctx, &mut args, arguments)?;
                args.push_arg(context_object(ctx, ContextBinding::Call(generation))?)?;
                let value: Value = handler.call_arg(args)?;
                if value.is_null() || value.is_undefined() {
                    Ok(None)
                } else {
                    element_id(ctx, &value).map(Some)
                }
            });
            let arena = temporary.finish();
            match described {
                Ok(Some(root)) => {
                    crate::materialize::try_materialize_subtree(self, &arena, root, window, cx)
                        .map(Some)
                }
                Ok(None) => Ok(None),
                Err(error) => Err(error.into()),
            }
        })
    }

    pub(crate) fn dispatch_component_element_data_callback(
        self: &Rc<Self>,
        id: CallbackId,
        arguments: &[ComponentDataValue],
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Option<gpui::AnyElement>> {
        let entry = self
            .callbacks
            .borrow()
            .get(id)
            .ok_or_else(|| anyhow!("component callback {id} belongs to a superseded render"))?;
        if entry.application.as_ref().is_some_and(|a| !a.is_active()) {
            return Err(anyhow!(
                "component callback {id} belongs to a retired application"
            ));
        }
        let view = entry
            .live_view()
            .ok_or_else(|| anyhow!("component callback {id} owner has been released"))?;
        let policy = view
            .as_ref()
            .map(|v| v.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        self.metrics.time_frame_script(|| {
            let (_guard, generation) = scope::enter_with_application(
                self,
                window,
                cx,
                ScopePhase::Layout,
                view,
                policy,
                entry.application.clone(),
            );
            scope::adopt(entry.registered_in);
            let temporary = TemporarySpecArena::enter(self);
            let described = self.with_js(|ctx| {
                let handler = entry.value.clone().restore(ctx)?;
                let mut args = JsArgs::new(ctx.clone(), arguments.len() + 1);
                for argument in arguments {
                    args.push_arg(component_data_into_js(ctx, argument)?)?;
                }
                args.push_arg(context_object(ctx, ContextBinding::Call(generation))?)?;
                let value: Value = handler.call_arg(args)?;
                if value.is_null() || value.is_undefined() {
                    Ok(None)
                } else {
                    element_id(ctx, &value).map(Some)
                }
            });
            let arena = temporary.finish();
            match described {
                Ok(Some(root)) => {
                    crate::materialize::try_materialize_subtree(self, &arena, root, window, cx)
                        .map(Some)
                }
                Ok(None) => Ok(None),
                Err(error) => Err(error.into()),
            }
        })
    }

    pub(crate) fn with_component_callback_event<R>(
        self: &Rc<Self>,
        id: CallbackId,
        window: &mut Window,
        cx: &mut App,
        body: impl FnOnce(&mut Window, &mut App) -> Result<R>,
    ) -> Result<R> {
        if let Some(phase) = scope::current_phase() {
            anyhow::ensure!(
                phase.allows_notify(),
                "component window effects are not allowed during the `{}` phase",
                phase.as_str()
            );
        }
        let entry = self
            .callbacks
            .borrow()
            .get(id)
            .ok_or_else(|| anyhow!("component callback {id} belongs to a superseded render"))?;
        anyhow::ensure!(
            entry
                .application
                .as_ref()
                .is_none_or(|application| application.is_active()),
            "component callback {id} belongs to a retired application"
        );
        let view = entry
            .live_view()
            .ok_or_else(|| anyhow!("component callback {id} owner has been released"))?;
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, _) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application,
        );
        body(window, cx)
    }

    /// Renders once, and on a "not a function" failure renders again with the
    /// diagnostic prototype installed so the error can name the method and
    /// suggest a correction. See the prelude for why this is two passes.
    fn call_render(&self, object: &ViewObject, generation: u64) -> Result<SpecId> {
        match self.call_render_once(object, generation) {
            Ok(id) => Ok(id),
            Err(error) if error.to_string().contains("not a function") => {
                self.set_diagnostics(true);
                self.arena.borrow_mut().reset();
                // The first attempt already recorded handlers into the open
                // generation; the retry describes the same tree again, so it
                // must start from an empty index space.
                self.callbacks.borrow_mut().rollback();
                let diagnosed = self.call_render_once(object, generation);
                self.set_diagnostics(false);
                match diagnosed {
                    Ok(id) => Ok(id),
                    Err(better) => Err(better),
                }
            }
            Err(error) => Err(error),
        }
    }

    fn set_diagnostics(&self, enabled: bool) {
        let _ = self.with_js(|ctx| ctx.globals().set("__diagnostics", enabled));
    }

    fn call_render_once(&self, object: &ViewObject, generation: u64) -> Result<SpecId> {
        self.with_js(|ctx| {
            let prepare_theme: Function = ctx.globals().get("__prepare_theme")?;
            prepare_theme.call::<_, ()>(())?;
            let instance = object.value.clone().restore(ctx)?;
            let render: Function = instance.get("render").map_err(|_| {
                Exception::throw_message(ctx, "view class has no render(cx) method")
            })?;
            let produced: Value = render.call((
                This(instance),
                context_object(ctx, ContextBinding::Call(generation))?,
            ))?;
            element_id(ctx, &produced)
        })
    }

    /// Runs `body` inside the JS context, flattening any exception into an
    /// ordinary error carrying the script's message and stack.
    fn with_js<T>(&self, body: impl FnOnce(&Ctx<'_>) -> JsResult<T>) -> Result<T> {
        if let Some(error) = self.job_queue_error() {
            return Err(error);
        }
        let pending_checkpoint = self.pending_nested.borrow().len();
        sandbox::begin_host_execution();
        let result = self.context.with(|ctx| {
            // Restored rather than cleared, because a `with_js` reached from
            // inside a host call is the case this exists for.
            let outer = self.active_context.replace(Some(ctx.as_raw()));
            let produced = match body(&ctx) {
                Ok(value) => Ok(value),
                Err(error) => Err(anyhow!("{}", describe(&ctx, error))),
            };
            self.active_context.set(outer);
            produced
        });
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                self.pending_nested
                    .borrow_mut()
                    .truncate(pending_checkpoint);
                Err(error)
            }
        }
    }

    /// Runs `body` against the context already executing, when one is.
    ///
    /// For a hook base calls on the shell's behalf from inside an operation the
    /// script started: the runtime's lock is already held, so asking
    /// [`Self::with_js`] for it again would panic on a re-entrant borrow.
    /// Outside such a call this is exactly `with_js`.
    ///
    /// It deliberately does not re-enter the sandbox's host-execution guard or
    /// take a pending-operation checkpoint. The call it is nested inside took
    /// both, and they are what that call will unwind to.
    fn with_js_nested<T>(&self, body: impl FnOnce(&Ctx<'_>) -> JsResult<T>) -> Result<T> {
        let Some(raw) = self.active_context.get() else {
            return self.with_js(body);
        };
        // Safe under the same condition `Ctx::from_raw` names: the runtime's
        // lock is held for as long as the call that installed this pointer is
        // on the stack, and this borrow does not outlive `body`.
        let ctx = unsafe { Ctx::from_raw(raw) };
        match body(&ctx) {
            Ok(value) => Ok(value),
            Err(error) => Err(anyhow!("{}", describe(&ctx, error))),
        }
    }

    /// Opens a detached node that collects the declarations of one state style.
    fn begin_state(&self, ctx: &Ctx<'_>, id: SpecId, name: &str) -> JsResult<SpecId> {
        let interned = match name {
            "hover" => "hover",
            "active" => "active",
            "focus" => "focus",
            // Not a runtime state, but the same mechanism: a detached node
            // collecting ordinary style methods. A `SliderIndicator` draws its
            // filled part from this one.
            "range_style" => "range_style",
            // An `OtpInput`'s three: one template for every cell, one layered
            // on the cell taking the next digit, and one for the caret in it.
            "cell_style" => "cell_style",
            "cell_active_style" => "cell_active_style",
            "caret_style" => "caret_style",
            other => {
                return Err(Exception::throw_type(
                    ctx,
                    &format!(
                        "unknown state style `{other}`; expected hover, active, focus, \
                         range_style, cell_style, cell_active_style or caret_style"
                    ),
                ));
            }
        };

        let node = self.arena.borrow_mut().push(Component::Div);
        self.arena
            .borrow_mut()
            .claim(node)
            .map_err(|error| Exception::throw_type(ctx, &error.to_string()))?;
        self.push_op_checked(ctx, self.push_op(id, SpecOp::StateStyle(interned, node)))?;
        Ok(node)
    }

    /// Fills a named element slot, detaching the element from the tree.
    ///
    /// The same `claim` a state style's declarations use: an element a
    /// component renders in a place of its own must not also be rendered among
    /// its children, and a script that tries to use it twice gets an error
    /// rather than a duplicate.
    fn fill_slot(&self, ctx: &Ctx<'_>, id: SpecId, name: &str, element: SpecId) -> JsResult<()> {
        let interned = match name {
            "content" => "content",
            "image" => "image",
            "fallback" => "fallback",
            "header" => "header",
            "footer" => "footer",
            "panel" => "panel",
            "trigger" => "trigger",
            // A number input's three. Unlike the two above, none of them is
            // optional in practice: base's step buttons are unstyled, so an
            // undecorated one is invisible and unhittable, and the frame has no
            // editor of its own.
            "input" => "input",
            "decrement_button" => "decrement_button",
            "increment_button" => "increment_button",
            other => {
                return Err(Exception::throw_type(
                    ctx,
                    &format!("unknown element slot `{other}`"),
                ));
            }
        };

        self.arena
            .borrow_mut()
            .claim(element)
            .map_err(|error| Exception::throw_type(ctx, &error.to_string()))?;
        self.push_op_checked(ctx, self.push_op(id, SpecOp::Slot(interned, element)))
    }

    fn push_node(&self, component: Component) -> SpecId {
        self.arena.borrow_mut().push(component)
    }

    /// Records an element for a component the bindings build themselves.
    pub(super) fn push_component(&self, component: Component) -> SpecId {
        self.push_node(component)
    }

    /// The description being recorded, for a binding whose node needs a check
    /// the arena is the one holding — a dock area, which may be mounted once.
    pub(super) fn arena_mut(&self) -> std::cell::RefMut<'_, SpecArena> {
        self.arena.borrow_mut()
    }

    fn push_op(&self, id: SpecId, op: SpecOp) -> Result<(), crate::spec::SpecError> {
        self.arena.borrow_mut().push_op(id, op)
    }
}

fn shell_jit_config() -> Result<JitConfig> {
    let builder = JitConfig::builder();

    // Debug builds exercise many short-lived runtimes concurrently. Keep them on
    // the interpreter tier: native compilation currently crashes inside the JIT
    // backend on Linux and Windows under that workload. Release builds retain the
    // production thresholds and continue to tier hot functions up to native code.
    #[cfg(debug_assertions)]
    let builder = builder.call_threshold(u32::MAX).loop_threshold(u32::MAX);

    builder
        .build()
        .map_err(|error| anyhow!("invalid JavaScript JIT configuration: {error}"))
}

/// Resolves and loads an application's own modules, and nothing else.
///
/// `FileResolver` from rquickjs is not usable here: it tests candidate paths
/// relative to the process working directory, so an absolute application path
/// never matches. Owning the resolver also puts the sandbox's module policy in
/// one place — a module must live inside the application root, which is what
/// stops `import "../../../etc/passwd"` before it reaches the filesystem.
#[derive(Clone)]
struct RegisteredComponentModule {
    /// The name this catalog answers to, or `None` for the empty catalog, which
    /// answers to nothing.
    specifier: Option<&'static str>,
    source: String,
}

impl Resolver for RegisteredComponentModule {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> JsResult<String> {
        if self.specifier == Some(name) {
            Ok(name.to_owned())
        } else {
            Err(JsError::new_resolving_message(
                base,
                name,
                "not the registered component module",
            ))
        }
    }
}

impl Loader for RegisteredComponentModule {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> JsResult<Module<'js, Declared>> {
        if self.specifier != Some(name) {
            return Err(JsError::new_loading_message(
                name,
                "not the registered component module",
            ));
        }
        Module::declare(ctx.clone(), name, self.source.clone())
    }
}

#[derive(Clone, Default)]
struct AppModules {
    applications: Rc<RefCell<Vec<ApplicationModules>>>,
    /// Bumped on every load so a reload re-reads every file.
    ///
    /// QuickJS caches an evaluated module by name, and an ES module cannot be
    /// unloaded — so re-evaluating `main.js` alone left every module it imports
    /// at the version that was on disk the first time. A hot reload that
    /// silently ignores every file except the entry point is worse than no hot
    /// reload, because it looks like it worked. Tagging the resolved name with
    /// a generation makes each reload a different module as far as the cache is
    /// concerned. The previous generation stays in the cache until the runtime
    /// shuts down; that is the cost, and it is a development-only one.
    next_generation: Rc<Cell<u32>>,
}

#[derive(Clone)]
struct ApplicationModules {
    root: std::path::PathBuf,
    generation: u32,
    dependencies: BTreeMap<String, MaterializedDependency>,
}

#[derive(Clone)]
struct ApplicationModuleLease(Rc<ApplicationModuleRegistration>);

struct ApplicationModuleRegistration {
    applications: Rc<RefCell<Vec<ApplicationModules>>>,
    root: std::path::PathBuf,
    generation: u32,
}

impl ApplicationModuleLease {
    fn generation(&self) -> u32 {
        self.0.generation
    }
}

impl Drop for ApplicationModuleRegistration {
    fn drop(&mut self) {
        self.applications.borrow_mut().retain(|application| {
            application.root != self.root || application.generation != self.generation
        });
    }
}

impl AppModules {
    #[cfg(test)]
    fn register(&self, root: std::path::PathBuf) -> ApplicationModuleLease {
        self.register_with_dependencies(root, BTreeMap::new())
    }

    fn register_with_dependencies(
        &self,
        root: std::path::PathBuf,
        dependencies: BTreeMap<String, MaterializedDependency>,
    ) -> ApplicationModuleLease {
        let generation = self.next_generation.get().wrapping_add(1);
        self.next_generation.set(generation);
        self.applications.borrow_mut().push(ApplicationModules {
            root: root.clone(),
            generation,
            dependencies,
        });
        ApplicationModuleLease(Rc::new(ApplicationModuleRegistration {
            applications: self.applications.clone(),
            root,
            generation,
        }))
    }

    /// Strips the generation tag a resolved name carries.
    fn untag(name: &str) -> &str {
        name.split_once("?v=").map(|(path, _)| path).unwrap_or(name)
    }

    fn application_for_base(&self, base: &str) -> Option<ApplicationModules> {
        let generation = Self::generation(base)?;
        let base = Path::new(Self::untag(base));
        self.applications
            .borrow()
            .iter()
            .filter(|application| {
                application.generation == generation
                    && (base.starts_with(&application.root)
                        || application
                            .dependencies
                            .values()
                            .any(|dependency| base.starts_with(&dependency.root)))
            })
            .max_by_key(|application| application.root.components().count())
            .cloned()
    }

    fn generation(name: &str) -> Option<u32> {
        name.rsplit_once("?v=")?.1.parse().ok()
    }

    #[cfg(test)]
    fn registration_count(&self) -> usize {
        self.applications.borrow().len()
    }

    fn candidate(
        &self,
        application: &ApplicationModules,
        base: &str,
        name: &str,
    ) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        let base_path = Path::new(Self::untag(base));
        let importing_dependency = application
            .dependencies
            .values()
            .find(|dependency| base_path.starts_with(&dependency.root));
        let (joined, boundary) = if name.starts_with('.') {
            let boundary = importing_dependency
                .map(|dependency| dependency.root.clone())
                .unwrap_or_else(|| application.root.clone());
            (base_path.parent()?.join(name), boundary)
        } else if let Some((dependency_name, dependency)) = application
            .dependencies
            .iter()
            .filter(|(dependency_name, _)| {
                name == dependency_name.as_str()
                    || name
                        .strip_prefix(dependency_name.as_str())
                        .is_some_and(|tail| tail.starts_with('/'))
            })
            .max_by_key(|(dependency_name, _)| dependency_name.len())
        {
            if name == dependency_name {
                return Some((dependency.entry.clone(), dependency.root.clone()));
            }
            let subpath = name.strip_prefix(dependency_name)?.strip_prefix('/')?;
            (dependency.root.join(subpath), dependency.root.clone())
        } else if importing_dependency.is_some() {
            return None;
        } else {
            (application.root.join(name), application.root.clone())
        };

        for candidate in [joined.clone(), joined.with_extension("js")] {
            if candidate.is_file() {
                return candidate.canonicalize().ok().map(|path| (path, boundary));
            }
        }
        None
    }
}

impl Resolver for AppModules {
    fn resolve<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> JsResult<String> {
        let Some(application) = self.application_for_base(base) else {
            return Err(Exception::throw_message(
                ctx,
                &format!("cannot identify the application importing `{name}` from `{base}`"),
            ));
        };
        let Some((path, boundary)) = self.candidate(&application, base, name) else {
            // A bare specifier reached the last resolver in the chain, so it is
            // neither a built-in nor a file. Saying which built-ins this
            // runtime does have is the difference between "you typed it wrong"
            // and "this binary is older than the script it is loading" — and
            // the second is what a moved module looks like from here.
            if !name.starts_with('.') && !name.contains('/') {
                return Err(Exception::throw_message(
                    ctx,
                    &format!(
                        "cannot resolve module `{name}`: this runtime's built-in modules are {}, \
                         and an application may otherwise import only its own files. If the \
                         script expects a module this runtime does not have, the two are \
                         different versions.",
                        builtin_specifiers()
                    ),
                ));
            }
            return Err(Exception::throw_message(
                ctx,
                &format!("cannot resolve module `{name}` from `{base}`"),
            ));
        };

        if !path.starts_with(&boundary) {
            return Err(Exception::throw_message(
                ctx,
                &format!(
                    "module `{name}` resolves outside the application directory `{}`",
                    boundary.display()
                ),
            ));
        }

        Ok(format!(
            "{}?v={}",
            path.to_string_lossy(),
            application.generation
        ))
    }
}

impl Loader for AppModules {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> JsResult<Module<'js, Declared>> {
        let path = Self::untag(name);
        let source = read_module_source(Path::new(path))
            .map_err(|error| Exception::throw_message(ctx, &error.to_string()))?;
        Module::declare(ctx.clone(), name, source)
    }
}

fn read_module_source(path: &Path) -> Result<String> {
    use std::io::Read as _;

    let mut file =
        std::fs::File::open(path).with_context(|| format!("reading module {}", path.display()))?;
    let size = file.metadata()?.len();
    if size > MAX_MODULE_BYTES {
        anyhow::bail!(
            "module `{}` is {size} bytes, over the {MAX_MODULE_BYTES}-byte limit",
            path.display()
        );
    }
    let mut source = String::with_capacity(size as usize);
    file.by_ref()
        .take(MAX_MODULE_BYTES + 1)
        .read_to_string(&mut source)
        .with_context(|| format!("reading module {}", path.display()))?;
    if source.len() as u64 > MAX_MODULE_BYTES {
        anyhow::bail!(
            "module `{}` grew over the {MAX_MODULE_BYTES}-byte limit",
            path.display()
        );
    }
    Ok(source)
}

/// Installed once per context. It builds the element prototype from the style
/// name list, which is why adding a style method upstream needs no change here.
const PRELUDE: &str = r#"
globalThis.__gpui = (() => {
  const methods = {};

  // Two prototypes, and a measured reason for having two.
  //
  // QuickJS reports a missing method as `TypeError: not a function` without
  // naming the property, so a mistyped style name would arrive with no clue —
  // and giving the call site a real diagnostic is the entire reason the style
  // surface is methods rather than a string of class names (§13.2).
  //
  // A Proxy prototype solves that, but the M0 benchmark measured it at ~30% of
  // the whole description pass (1.09 ms → 1.42 ms for 443 nodes). So the fast
  // prototype is the default, and a render that fails with "not a function" is
  // re-run once against the diagnostic prototype purely to produce the message.
  // Errors are rare; a 30% tax on every frame is not.
  const diagnostic = new Proxy(methods, {
    get(target, name, receiver) {
      const found = Reflect.get(target, name, receiver);
      if (found !== undefined) return found;
      if (typeof name !== "string" || name.startsWith("__")) return undefined;
      return () => __unknown(name);
    },
  });

  globalThis.__diagnostics = false;

  const element = (id) => {
    const object = Object.create(globalThis.__diagnostics ? diagnostic : methods);
    object.__id = id;
    return object;
  };

  const define = (name) => {
    methods[name] = function (...args) {
      __apply(this.__id, name, args);
      return this;
    };
  };

  // Styles do not go through `__apply`, and the reason is arithmetic. They are
  // most of what a description records, and the generic form above pays three
  // times over for information the prelude already has: it allocates a rest
  // array to hold arguments a style never has more than one of, it sends a
  // method name that has to be copied into a Rust string, and it arrives at a
  // dispatcher that has to look that string back up in a table. Closing over
  // the table index instead removes all three, and measured at roughly half
  // the cost of a recorded style call. `define` stays for the behaviours,
  // where the argument shapes vary and a second form would not repay itself.
  const defineNullaryStyle = (name, index) => {
    methods[name] = function () {
      __applyNullaryStyle(this.__id, index);
      return this;
    };
  };
  const defineParamStyle = (name, index) => {
    methods[name] = function (value) {
      __applyParamStyle(this.__id, index, value);
      return this;
    };
  };

  for (let i = 0; i < __nullaryStyles.length; i += 1) {
    defineNullaryStyle(__nullaryStyles[i], __nullaryStyleIndexes[i]);
  }
  for (let i = 0; i < __paramStyles.length; i += 1) {
    defineParamStyle(__paramStyles[i], i);
  }
  for (const name of __behaviorNames) define(name);

  // Attaching is the other call a description makes once per element, and it
  // carries no argument a `Bridged` could describe — two element ids, both
  // already numbers. It gets an entry point of its own for the same reason the
  // styles do.
  //
  // A retained child view is a child too — the same shape GPUI has, where an
  // `Entity<V>` is itself renderable — so `.child(handle)` mounts one. An
  // element always carries `__id`, so the branch costs the hot path one
  // `undefined` test and the slow side is the case that used to fail with
  // rquickjs' "Error converting from undefined to f64".
  const childId = (child) => {
    const id = child?.__id;
    if (id !== undefined) return id;
    // A string is an element. GPUI implements `IntoElement` for `&str`,
    // `String` and `SharedString`, so `.child("hello")` is how text is written
    // there, and the style comes from the element holding it.
    const kind = typeof child;
    if (kind === "string" || kind === "number" || kind === "boolean") {
      return __text(String(child));
    }
    // A template's sentinel, reached only while a template body is running.
    // Checked after elements and strings so the ordinary description pays
    // nothing for it.
    if (child?.__slot !== undefined) return __text_slot(child.__slot);
    if (child?.__entity) return __child_view(child.__handle);
    throw new TypeError(
      "child(value) expects an element, a string, or an entity from cx.new(Class, props)",
    );
  };
  methods.child = function (child) {
    __attach(this.__id, childId(child));
    return this;
  };
  methods.children = function (list) {
    for (const child of list) __attach(this.__id, childId(child));
    return this;
  };
  // A named slot. The element is consumed exactly as `child` consumes one, so
  // it cannot also be added to the tree — which is the point: the component
  // renders it somewhere of its own, or not at all.
  const slot = (name) =>
    function (element) {
      __apply(this.__id, name, [element.__id]);
      return this;
    };

  methods.content = slot("content");
  methods.trigger = slot("trigger");

  // A number input's three. Every element carries them, as it carries `content`
  // and `trigger`, because one prototype is shared by all of them.
  methods.input = slot("input");
  methods.decrement_button = slot("decrement_button");
  methods.increment_button = slot("increment_button");

  // An avatar's two. Base renders the image, or the fallback when there is no
  // image, and never both.
  methods.image = slot("image");
  methods.fallback = slot("fallback");

  // An accordion item's two. Both are read back for their own type rather than
  // rendered, so both must be the part they name.
  methods.header = slot("header");
  methods.footer = slot("footer");
  methods.panel = slot("panel");

  // Focus is held by handle, so the element records the handle rather than the
  // wrapper object around it — the same unwrapping `Input.new(state)` does.
  methods.track_focus = function (handle) {
    if (typeof handle?.__handle !== "number") {
      throw new TypeError(
        "track_focus(handle) expects a FocusHandle from cx.focus_handle(), not a name or an element",
      );
    }
    __apply(this.__id, "track_focus", [handle.__handle]);
    return this;
  };
  // A virtualized list's scroll position, unwrapped exactly as `track_focus`
  // unwraps a focus handle, and checked here for the same reason: a name or an
  // element would be dropped on the Rust side and the list would simply never
  // respond to `scroll_to_item`.
  methods.track_scroll = function (handle) {
    if (typeof handle?.__handle !== "number") {
      throw new TypeError(
        "track_scroll(handle) expects a VirtualListScrollHandle from VirtualListScrollHandle.new()",
      );
    }
    __apply(this.__id, "track_scroll", [handle.__handle]);
    return this;
  };
  // The second handle a combobox root needs: the one the keyboard moves to when
  // the surface opens. Checked here for the same reason `track_focus` is — a
  // name or an element would otherwise be dropped on the Rust side and the
  // focus would simply never move.
  methods.content_focus_handle = function (handle) {
    if (typeof handle?.__handle !== "number") {
      throw new TypeError(
        "content_focus_handle(handle) expects a FocusHandle from cx.focus_handle(), not a name or an element",
      );
    }
    __apply(this.__id, "content_focus_handle", [handle.__handle]);
    return this;
  };
  // State styles reuse the ordinary style methods on a detached element, so
  // there is no second grammar for "what a style is".
  const state = (name) =>
    function (declare) {
      const target = element(__state(this.__id, name));
      declare(target);
      return this;
    };

  methods.hover = state("hover");
  methods.active = state("active");
  methods.focus = state("focus");
  // Not a state: the filled part of a slider, declared the same way because
  // it is the same thing — a detached element collecting styles. The shell
  // positions the box; this says what it looks like.
  methods.range_style = state("range_style");
  // An OtpInput's cells. Not states either: the shell decides which template
  // a cell gets, from the state, on every frame — but they are declared the
  // same way, because what they collect is the same thing.
  methods.cell_style = state("cell_style");
  methods.cell_active_style = state("cell_active_style");
  methods.caret_style = state("caret_style");

  // The argument checks are here rather than only on the Rust side because a
  // list built with the pieces in the wrong order — a render function where the
  // sizes go — would otherwise fail as a type error naming neither.
  // The three checks every lazy list makes. Only the render hint differs:
  // `list` is called per item, the other two per visible range.
  const checkListArgs = (shape, item_count, get_key, render, renderHint) => {
    if (!Number.isInteger(item_count) || item_count < 0) {
      throw new TypeError(shape + " needs a whole, non-negative item_count");
    }
    if (typeof get_key !== "function") {
      throw new TypeError(
        shape + " needs get_key(index) to return each item's stable string key",
      );
    }
    if (typeof render !== "function") {
      throw new TypeError(shape + " needs a render function; it is called " + renderHint);
    }
  };

  const RANGE_HINT = "once per visible range, not once per item";

  const virtualList = (build, name) => (id, item_count, item_sizes, get_key, render) => {
    const shape = name + "(id, item_count, item_sizes, get_key, render)";
    checkListArgs(shape, item_count, get_key, render, RANGE_HINT);
    if (Array.isArray(item_sizes) && item_sizes.length !== item_count) {
      throw new TypeError(
        shape + " was given " + item_sizes.length + " item sizes for " + item_count +
          " items; pass one number for a uniform extent, or one per item",
      );
    }
    return element(build(String(id), item_count, item_sizes, get_key, render));
  };

  // `list` and `uniform_list`: GPUI's own lazy lists. Both cross the boundary
  // the way a virtual list does -- one renderer per visible range -- so a
  // `list` renderer written per item is folded into a range here, once, rather
  // than teaching the host a second calling convention.
  const lazyList = (build, name, perItem) => (id, item_count, get_key, render) => {
    const shape = name + "(id, item_count, get_key, render)";
    checkListArgs(
      shape,
      item_count,
      get_key,
      render,
      perItem ? "once per item on screen, with the item's index" : RANGE_HINT,
    );
    const describe = perItem
      ? (range, cx) => {
          const items = [];
          for (let index = range.start; index < range.end; index++) {
            items.push(render(index, cx));
          }
          return items;
        }
      : render;
    return element(build(String(id), item_count, get_key, describe));
  };

  const finiteNonNegative = (value, name) => {
    if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
      throw new TypeError(name + " must be a finite non-negative number");
    }
    return value;
  };

  const finitePositive = (value, name) => {
    if (typeof value !== "number" || !Number.isFinite(value) || value <= 0) {
      throw new TypeError(name + " must be a finite positive number");
    }
    return value;
  };

  // A table index is one-based and whole. 0 and 1.5 are not values a screen
  // reader rounds off; they are cells announced in the wrong column, so they
  // are refused at the call site rather than cast quietly on the Rust side.
  const oneBased = (value, name) => {
    if (!Number.isInteger(value) || value < 1) {
      throw new TypeError(name + " must be a whole number of at least 1");
    }
    return value;
  };

  methods.transition = function (property, options) {
    property = String(property);
    if (!["opacity", "width", "height", "left", "top"].includes(property)) {
      throw new TypeError(
        "transition(property, policy) supports opacity, width, height, left or top; got " +
          JSON.stringify(property),
      );
    }
    const policy = typeof options === "number" ? { duration: options } : (options ?? {});
    const duration = finiteDuration(policy.duration ?? 0, "transition duration");
    const delay = finiteDuration(policy.delay ?? 0, "transition delay");
    const easing = policy.easing ?? "ease-out";
    if (!["linear", "ease-in", "ease-out", "ease-in-out"].includes(easing)) {
      throw new TypeError(
        "transition easing must be linear, ease-in, ease-out or ease-in-out; got " +
          JSON.stringify(easing),
      );
    }
    __apply(this.__id, "transition", [
      property,
      duration,
      delay,
      easing,
    ]);
    return this;
  };

  methods.spring = function (property, options) {
    property = String(property);
    if (!["opacity", "width", "height", "left", "top"].includes(property)) {
      throw new TypeError(
        "spring(property, policy) supports opacity, width, height, left or top; got " +
          JSON.stringify(property),
      );
    }
    const policy = options ?? {};
    const response = finiteDuration(policy.response ?? 250, "spring response");
    const damping = finiteNonNegative(policy.damping ?? 1, "spring damping");
    const epsilon = finitePositive(policy.epsilon ?? 0.001, "spring epsilon");
    __apply(this.__id, "spring", [
      property,
      response,
      damping,
      epsilon,
    ]);
    return this;
  };

  // Announced, not laid out: `axis` sets the semantic orientation of a
  // grouping container and never turns it into a row or a column. Checked here
  // so a typo reports at the call site instead of silently announcing the
  // container's default.
  methods.axis = function (value) {
    value = String(value);
    if (!["horizontal", "vertical"].includes(value)) {
      throw new TypeError(
        "axis(value) must be horizontal or vertical; got " + JSON.stringify(value),
      );
    }
    __apply(this.__id, "axis", [value]);
    return this;
  };

  // A bar's visibility policy. Unset follows the theme, which is what every
  // other scrollbar in the application does; the three named modes are checked
  // here so a typo reports at the call site instead of silently falling back.
  methods.mode = function (value) {
    value = String(value);
    if (!["scrolling", "hover", "always"].includes(value)) {
      throw new TypeError(
        "mode(value) must be scrolling, hover or always; got " + JSON.stringify(value),
      );
    }
    __apply(this.__id, "mode", [value]);
    return this;
  };

  // The content size a bar measures its thumb against. Both halves are
  // required: one axis sized by the script and the other by the scroll area is
  // a thumb that lies about one of them.
  methods.scroll_size = function (width, height) {
    __apply(this.__id, "scroll_size", [
      finiteNonNegative(width, "scroll_size width"),
      finiteNonNegative(height, "scroll_size height"),
    ]);
    return this;
  };

  // Which corner of an anchored surface is pinned to its trigger. The names
  // come from the host so that the check here, the parser behind it and the
  // union in gpui-kit.d.ts cannot disagree. Checked at the call site because an
  // unrecognized anchor would otherwise open the surface in the component's
  // default corner, which looks like a positioning bug rather than a typo.
  methods.anchor = function (value) {
    value = String(value);
    if (!__anchorNames.includes(value)) {
      throw new TypeError(
        "anchor(value) must be one of " +
          __anchorNames.join(", ") +
          "; got " +
          JSON.stringify(value),
      );
    }
    __apply(this.__id, "anchor", [value]);
    return this;
  };

  methods.frame_budget = function (milliseconds) {
    __apply(this.__id, "frame_budget", [
      finitePositive(milliseconds, "frame_budget"),
    ]);
    return this;
  };

  // A popover opened by the wrong button is silence, not a visual mistake, so
  // an unknown button name is refused rather than falling back to the left one.
  methods.mouse_button = function (value) {
    value = String(value);
    if (!["left", "right", "middle"].includes(value)) {
      throw new TypeError(
        "mouse_button(value) must be left, right or middle; got " + JSON.stringify(value),
      );
    }
    __apply(this.__id, "mouse_button", [value]);
    return this;
  };

  // Milliseconds, as everywhere else a script names a duration.
  const finiteDuration = (value, name) => {
    value = finiteNonNegative(value, name);
    if (value > 86400000) throw new RangeError(name + " must not exceed 86400000 milliseconds");
    return value;
  };
  const delay = (name) =>
    function (ms) {
      __apply(this.__id, name, [finiteDuration(ms, name)]);
      return this;
    };

  methods.open_delay = delay("open_delay");
  methods.close_delay = delay("close_delay");

  // Two arguments rather than a range literal, which JavaScript has no spelling
  // for. The floor is required — a panel always has one, and base's own is
  // 100px — while the ceiling is optional, because most panels have none.
  methods.size_range = function (min, max) {
    const args = [finiteNonNegative(min, "size_range min")];
    if (max !== undefined && max !== null) {
      args.push(finiteNonNegative(max, "size_range max"));
    }
    __apply(this.__id, "size_range", args);
    return this;
  };

  // `size` and `visible` on a resizable panel are base's own inherent builders
  // — the initial size along the group's axis, and whether the panel is drawn —
  // and in Rust each shadows the `Styled` method of the same name for that one
  // type. Own properties on the panel object shadow the shared prototype by the
  // same mechanism, so a script writes what the Rust writes and `.size(200)`
  // still means width-and-height, `.visible()` still means `visibility`,
  // everywhere else.
  const resizablePanel = () => {
    const object = element(__resizable_panel());
    object.size = function (pixels) {
      __apply(this.__id, "panel_size", [finiteNonNegative(pixels, "resizable_panel size")]);
      return this;
    };
    object.visible = function (value) {
      __apply(this.__id, "panel_visible", [Boolean(value)]);
      return this;
    };
    return object;
  };

  const coordinate = (value, name) => {
    if (typeof value === "number" && Number.isFinite(value)) return value;
    if (typeof value === "string" && /^-?(?:\d+(?:\.\d*)?|\.\d+)%$/.test(value)) return value;
    throw new TypeError(name + " must be a finite pixel number or percentage string");
  };

  const background = (kind, values, opacityFactor = 1, colorSpace = "srgb") => Object.freeze({
    __background: true,
    kind,
    values: Object.freeze(values),
    opacityFactor,
    colorSpace,
    opacity(factor) {
      return background(kind, values, finiteNonNegative(factor, "background opacity"), colorSpace);
    },
    color_space(space) {
      space = String(space).toLowerCase();
      if (!['srgb', 'oklab'].includes(space)) throw new TypeError("background color_space must be srgb or oklab");
      return background(kind, values, opacityFactor, space);
    },
  });

  const asBackground = (value) => value?.__background
    ? value
    : background("solid", [String(value)]);

  const pathBuilder = (fill, width) => {
    const commands = [];
    const builder = {};
    const command = (name, arity, coordinateCount = arity) => (...args) => {
      if (args.length < arity) throw new TypeError(name + " expects at least " + arity + " argument(s)");
      for (let index = 0; index < coordinateCount; index++) coordinate(args[index], name + " coordinate");
      commands.push(Object.freeze([name, ...args]));
      return builder;
    };
    builder.move_to = command("move_to", 2);
    builder.line_to = command("line_to", 2);
    builder.curve_to = command("curve_to", 4);
    builder.cubic_bezier_to = command("cubic_bezier_to", 6);
    builder.arc_to = (...args) => {
      if (args.length < 7) throw new TypeError("arc_to expects at least 7 argument(s)");
      coordinate(args[0], "arc x radius");
      coordinate(args[1], "arc y radius");
      if (typeof args[2] !== "number" || !Number.isFinite(args[2])) throw new TypeError("arc rotation must be finite");
      coordinate(args[5], "arc destination x");
      coordinate(args[6], "arc destination y");
      commands.push(Object.freeze(["arc_to", ...args]));
      return builder;
    };
    builder.close = () => { commands.push(Object.freeze(["close"])); return builder; };
    builder.dash_array = (values) => {
      if (fill) throw new TypeError("dash_array is only available on stroke paths");
      if (!Array.isArray(values) || values.some((value) => typeof value !== "number" || !Number.isFinite(value) || Math.fround(value) <= 0)) {
        throw new TypeError("dash_array(values) expects positive finite pixel numbers");
      }
      commands.push(Object.freeze(["dash_array", ...values]));
      return builder;
    };
    builder.add_polygon = (points, closed = true) => {
      if (!Array.isArray(points) || points.length === 0) throw new TypeError("add_polygon(points) expects a non-empty array");
      points.forEach((point, index) => {
        if (!Array.isArray(point) || point.length < 2) throw new TypeError("each polygon point must be [x, y]");
        command(index === 0 ? "move_to" : "line_to", 2)(point[0], point[1]);
      });
      if (closed) builder.close();
      return builder;
    };
    builder.build = () => Object.freeze({
      __path: true,
      fill,
      width,
      commands: Object.freeze(commands.slice()),
    });
    return builder;
  };

  const paintPath = (pathValue, paintValue) => {
    if (!pathValue?.__path) throw new TypeError("window.paint_path(path, background) expects a Path built by PathBuilder");
    const paint = asBackground(paintValue);
    const object = element(__path(
      pathValue.fill,
      paint.kind,
      paint.values.map(String),
      paint.opacityFactor,
      paint.colorSpace,
      pathValue.width,
    ));
    for (const [name, ...args] of pathValue.commands) __apply(object.__id, name, args);
    return object;
  };

  // Mirrors GPUI's `FluentBuilder::map`: unlike an ordinary element method,
  // the callback decides the return type. Keeping this in JavaScript also
  // avoids a host crossing for a purely fluent control-flow helper.
  methods.map = function (transform) {
    return transform(this);
  };

  methods.when = function (condition, branch) {
    if (!condition) return this;
    const produced = branch(this);
    if (produced === undefined || produced === null) {
      throw new Error("when(...) must return the element");
    }
    return produced;
  };

  // Retained state is held by handle; the methods close over it so nothing has
  // to read it back off `this`.
  const inputState = (handle) => ({
    __handle: handle,
    value: () => __input_value(handle),
    set_value: (next) => __input_set_value(handle, String(next ?? "")),
    on: (event, handler) => __input_on(handle, String(event), handler),
    // What makes a text state a number state. There is no `NumberInputState`:
    // the step, the bounds and the mask are fields on this one, so a plain
    // input becomes a numeric one by being told about them.
    set_step: (step) => __input_set_step(handle, step === null || step === undefined ? null : Number(step)),
    set_min: (min) => __input_set_min(handle, min === null || min === undefined ? null : Number(min)),
    set_max: (max) => __input_set_max(handle, max === null || max === undefined ? null : Number(max)),
    set_masked: (masked) => __input_set_masked(handle, Boolean(masked)),
    set_loading: (loading) => __input_set_loading(handle, Boolean(loading)),
    release: () => __input_release(handle),
  });

  // The multi-line state shares almost all of its surface with the single-line
  // one, and adds the three calls that only mean anything once text can wrap.
  const textareaState = (handle) => ({
    __handle: handle,
    value: () => __textarea_value(handle),
    set_value: (next) => __textarea_set_value(handle, String(next ?? "")),
    on: (event, handler) => __textarea_on(handle, String(event), handler),
    set_rows: (rows) => __textarea_set_rows(handle, oneBased(rows, "set_rows(rows)")),
    set_auto_grow: (min_rows, max_rows) =>
      __textarea_set_auto_grow(
        handle,
        oneBased(min_rows, "set_auto_grow(min_rows, max_rows) min_rows"),
        oneBased(max_rows, "set_auto_grow(min_rows, max_rows) max_rows"),
      ),
    set_soft_wrap: (wrap) => __textarea_set_soft_wrap(handle, Boolean(wrap)),
    release: () => __textarea_release(handle),
  });

  // A slider's value crosses as an array either way, because a bare number
  // cannot say whether the script meant one thumb or two.
  const sliderValue = (values) => (values.length === 1 ? values[0] : values);
  const sliderValues = (value, api) => {
    const finite = (each) => typeof each === "number" && Number.isFinite(each);
    if (Array.isArray(value)) {
      if (value.length !== 2 || !value.every(finite)) {
        throw new TypeError(api + " expects a finite number, or a pair [start, end] of them");
      }
      return [value[0], value[1]];
    }
    if (!finite(value)) {
      throw new TypeError(api + " expects a finite number, or a pair [start, end] of them");
    }
    return [value];
  };

  const sliderState = (handle) => ({
    __handle: handle,
    value: () => sliderValue(__slider_value(handle)),
    set_value: (next) => __slider_set_value(handle, sliderValues(next, "set_value(value)")),
    min_value: () => __slider_bounds(handle)[0],
    max_value: () => __slider_bounds(handle)[1],
    step_value: () => __slider_bounds(handle)[2],
    on: (event, handler) => __slider_on(handle, String(event), handler),
    release: () => __slider_release(handle),
  });

  // A one-time code. `len` is read rather than set: base fixes it when the
  // state is created and offers no setter, because it is what the state is.
  const otpState = (handle) => ({
    __handle: handle,
    value: () => __otp_value(handle),
    set_value: (next) => __otp_set_value(handle, String(next ?? "")),
    len: () => __otp_len(handle),
    is_masked: () => __otp_is_masked(handle),
    set_masked: (masked) => __otp_set_masked(handle, Boolean(masked)),
    focus: () => __otp_focus(handle),
    on: (event, handler) => __otp_on(handle, String(event), handler),
    release: () => __otp_release(handle),
  });

  // A calendar's month, and the date chosen in it.
  //
  // `month_days()` is the reason this is bound at all: which dates fall in
  // which week, where the neighbouring months' days go, and how many weeks
  // this month needs. Everything else here is what it takes to move that grid
  // and read what was picked from it.
  //
  // The wire is a flat two-slot array either way; the narrowing to `null`, a
  // string or a pair happens here so a script never sees the flat form.
  // Reads the variant off the slot count, exactly as `calendarParts` writes
  // it. Looking at whether the second slot is null instead would collapse a
  // range whose end is not chosen yet into a bare string — losing the one
  // thing that distinguishes it from a single date, and losing it only in the
  // half-finished state a range picker spends most of its time in.
  const calendarDate = (parts) => {
    if (parts.length === 2) return [parts[0] ?? null, parts[1] ?? null];
    return parts[0] ?? null;
  };
  // How many slots go over is what says which `Date` was meant, and it has to:
  // a single day and a range whose end is not chosen yet are different states
  // to base — `is_single`, `is_complete` and `is_in_range` all branch on it —
  // but they read back as the same string, so the wire cannot recover the
  // difference from the values alone.
  const calendarParts = (value, api) => {
    if (value === null || value === undefined) return [null];
    if (Array.isArray(value)) {
      if (value.length !== 2) {
        throw new TypeError(`${api} range expects a two-element array [start, end]`);
      }
      return [value[0] ?? null, value[1] ?? null];
    }
    if (typeof value !== "string") {
      throw new TypeError(`${api} expects null, a "YYYY-MM-DD" string, or a pair of those`);
    }
    return [value];
  };
  const calendarState = (handle) => ({
    __handle: handle,
    month_days: () => __calendar_month_days(handle),
    year: () => __calendar_year(handle),
    month: () => __calendar_month(handle),
    today: () => __calendar_today(handle),
    value: () => calendarDate(__calendar_value(handle)),
    set_value: (next) => __calendar_set_value(handle, calendarParts(next, "set_value(value)")),
    next_month: () => __calendar_next_month(handle),
    prev_month: () => __calendar_prev_month(handle),
    on: (event, handler) =>
      __calendar_on(handle, String(event), (parts, cx) => handler(calendarDate(parts), cx)),
    release: () => __calendar_release(handle),
  });

  const focusHandle = (handle) => ({
    __handle: handle,
    focus: () => __focus_focus(handle),
    is_focused: () => __focus_is_focused(handle),
    release: () => __focus_release(handle),
  });

  // `__entity` is the discriminant `.child()` needs: every retained handle in
  // this API is a `{__handle: number}` wrapper, so a focus handle and an entity
  // are otherwise indistinguishable, and mounting the wrong one would report a
  // released view rather than the mistake that was made.
  const entity = (handle) => ({
    __entity: true,
    __handle: handle,
    set_props: (props) => __view_set_props(handle, props),
    release: () => __view_release(handle),
  });

  // A dockable layout, and the commands its chrome carries.
  //
  // Retained for a reason none of the other handles share: the layout is what
  // the *user* changed. A drag, a resize, a closed tab and a collapsed dock all
  // happen without this script rendering, so a dock rebuilt from a description
  // would put every one of them back the way the last render described it.
  const dockPlacement = (value, api) => {
    const name = String(value ?? "center");
    if (!["center", "left", "right", "bottom"].includes(name)) {
      throw new TypeError(api + ' expects "center", "left", "right" or "bottom"');
    }
    return name;
  };

  const wholeAt = (value, api) => {
    if (!Number.isSafeInteger(value) || value < 0) {
      throw new TypeError(api + " expects a whole, non-negative position");
    }
    return value;
  };

  const finiteDockNumber = (value, api) => {
    if (typeof value !== "number" || !Number.isFinite(value)) {
      throw new TypeError(api + " expects a finite number");
    }
    return value;
  };

  const nonNegativeDockNumber = (value, api) => {
    const number = finiteDockNumber(value, api);
    if (number < 0) throw new RangeError(api + " expects a non-negative number");
    return number;
  };

  const dockBounds = (value) => {
    const api = "add_panel(view, options) bounds";
    if (value === undefined || value === null) return null;
    if (typeof value !== "object") throw new TypeError(api + " expects an object");
    return {
      x: finiteDockNumber(value.x, api + ".x"),
      y: finiteDockNumber(value.y, api + ".y"),
      width: nonNegativeDockNumber(value.width, api + ".width"),
      height: nonNegativeDockNumber(value.height, api + ".height"),
    };
  };

  // Every chrome handler is given base's own state for one container, with the
  // area it belongs to added on this side — the commands need it, and this side
  // already knows it, so it never has to cross.
  const dockTarget = (value, api) => {
    const handle = value?.__dock;
    if (typeof handle !== "number") {
      throw new TypeError(
        api + " expects the group, dock or tile your chrome handler was given as its first argument",
      );
    }
    return handle;
  };

  const groupNode = (group, api) => {
    if (typeof group?.node !== "number") {
      throw new TypeError(api + " expects a tab group, which is what tab_bar and empty_group are given");
    }
    return group.node;
  };

  const tilePanel = (tile, api) => {
    if (typeof tile?.panel?.id !== "number") {
      throw new TypeError(api + " expects a tile, which is what tile_drag_bar and tile_resize_handles are given");
    }
    return tile.panel.id;
  };

  // Commands, not callbacks. A chrome handler runs once per frame for as long
  // as the dock is on screen, so a handler registered inside one would pile up
  // exactly the way a virtual list's row handlers would. A command carries no
  // script value at all: it names a container and what to ask it, and base does
  // the rest.
  methods.select_tab = function (group, index) {
    const api = "select_tab(group, index)";
    __apply(this.__id, "select_tab", [dockTarget(group, api), groupNode(group, api), wholeAt(index, api)]);
    return this;
  };
  methods.close_panel = function (group, panel) {
    const api = "close_panel(group, panel_id)";
    __apply(this.__id, "close_panel", [dockTarget(group, api), groupNode(group, api), Number(panel)]);
    return this;
  };
  methods.toggle_zoom = function (group) {
    const api = "toggle_zoom(group)";
    __apply(this.__id, "toggle_zoom", [dockTarget(group, api), groupNode(group, api)]);
    return this;
  };
  methods.drag_tab = function (group, index) {
    const api = "drag_tab(group, index)";
    __apply(this.__id, "drag_tab", [dockTarget(group, api), groupNode(group, api), wholeAt(index, api)]);
    return this;
  };
  // The one command with an optional argument: a tab bar that names no slot
  // means "append", which is what a drop past the last tab is.
  methods.drop_tab = function (group, index) {
    const api = "drop_tab(group, index)";
    const at = index === undefined || index === null ? null : wholeAt(index, api);
    __apply(this.__id, "drop_tab", [dockTarget(group, api), groupNode(group, api), at]);
    return this;
  };
  methods.toggle_dock = function (dock) {
    const api = "toggle_dock(dock)";
    __apply(this.__id, "toggle_dock", [dockTarget(dock, api), dockPlacement(dock?.placement, api)]);
    return this;
  };
  methods.resize_dock = function (dock) {
    const api = "resize_dock(dock)";
    __apply(this.__id, "resize_dock", [dockTarget(dock, api), dockPlacement(dock?.placement, api)]);
    return this;
  };
  methods.move_tile = function (tile) {
    const api = "move_tile(tile)";
    __apply(this.__id, "move_tile", [dockTarget(tile, api), tilePanel(tile, api)]);
    return this;
  };
  methods.resize_tile = function (tile, side) {
    const api = "resize_tile(tile, side)";
    const name = String(side ?? "");
    if (!["left", "right", "top", "bottom", "bottom_right"].includes(name)) {
      throw new TypeError(api + ' expects "left", "right", "top", "bottom" or "bottom_right"');
    }
    __apply(this.__id, "resize_tile", [dockTarget(tile, api), tilePanel(tile, api), name]);
    return this;
  };
  methods.raise_tile = function (tile) {
    const api = "raise_tile(tile)";
    __apply(this.__id, "raise_tile", [dockTarget(tile, api), tilePanel(tile, api)]);
    return this;
  };
  methods.toggle_tile_zoom = function (tile) {
    const api = "toggle_tile_zoom(tile)";
    __apply(this.__id, "toggle_tile_zoom", [dockTarget(tile, api), tilePanel(tile, api)]);
    return this;
  };
  methods.close_tile = function (tile) {
    const api = "close_tile(tile)";
    __apply(this.__id, "close_tile", [dockTarget(tile, api), tilePanel(tile, api)]);
    return this;
  };

  const DOCK_CHROME = [
    "tab_bar",
    "empty_group",
    "drop_indicator",
    "dock",
    "tile_drag_bar",
    "tile_resize_handles",
  ];

  // The six hooks are own properties of the one element that has them, rather
  // than prototype methods: every other element in the tree would otherwise
  // carry a `dock` and a `tab_bar` that mean nothing on it.
  const dockAreaElement = (area) => {
    const handle = area?.__dock;
    if (typeof handle !== "number") {
      throw new TypeError("dock_area(area) expects a DockArea from DockArea.new(id)");
    }
    const object = element(__dock_area_element(handle));
    for (const hook of DOCK_CHROME) {
      object[hook] = function (handler) {
        if (typeof handler !== "function") {
          throw new TypeError(hook + "(handler) expects a function returning an element");
        }
        __apply(this.__id, hook, [
          (payload, cx) => {
            payload.__dock = handle;
            return handler(payload, cx);
          },
        ]);
        return this;
      };
    }
    return object;
  };

  const dockArea = (handle) => ({
    __dock: handle,
    __handle: handle,
    add_panel: (view, options) => {
      if (typeof view?.__handle !== "number" || !view.__entity) {
        throw new TypeError(
          "add_panel(view, options) expects a view from cx.new(Class): a panel's body is a view, not an element",
        );
      }
      const settings = options ?? {};
      if (typeof settings.name !== "string" || settings.name.length === 0) {
        throw new TypeError(
          "add_panel(view, options) needs a name: it is what the panel is filed under in a saved layout, and what register_panel finds it again by",
        );
      }
      // No id comes back: the view is still being constructed when this is
      // called, so the panel it will hold does not exist yet. `panels()` names
      // every panel once the call that added them has returned.
      __dock_add_panel(handle, view.__handle, {
        name: settings.name,
        placement: dockPlacement(settings.placement, "add_panel placement"),
        size:
          settings.size === undefined || settings.size === null
            ? null
            : nonNegativeDockNumber(settings.size, "add_panel(view, options) size"),
        bounds: dockBounds(settings.bounds),
        closable: settings.closable === undefined ? true : Boolean(settings.closable),
        zoomable: settings.zoomable === undefined ? true : Boolean(settings.zoomable),
        visible: settings.visible === undefined ? true : Boolean(settings.visible),
      });
    },
    remove_panel: (id) => __dock_remove_panel(handle, wholeAt(id, "remove_panel(id)")),
    panels: () => JSON.parse(__dock_panels(handle)),
    // The layout as plain data, and back. `load` takes effect once this call
    // has returned: rebuilding a panel constructs a view, and a view cannot be
    // constructed while script is running.
    dump: () => JSON.parse(__dock_dump(handle)),
    load: (state) => __dock_load(handle, state),
    has_dock: (placement) => __dock_has(handle, dockPlacement(placement, "has_dock(placement)")),
    is_dock_open: (placement) =>
      __dock_is_open(handle, dockPlacement(placement, "is_dock_open(placement)")),
    toggle_dock: (placement) =>
      __dock_toggle(handle, dockPlacement(placement, "toggle_dock(placement)")),
    remove_dock: (placement) =>
      __dock_remove(handle, dockPlacement(placement, "remove_dock(placement)")),
    dock_size: (placement) => __dock_size(handle, dockPlacement(placement, "dock_size(placement)")),
    set_dock_size: (placement, size) =>
      __dock_set_size(
        handle,
        dockPlacement(placement, "set_dock_size(placement, size)"),
        nonNegativeDockNumber(size, "set_dock_size(placement, size)"),
      ),
    set_dock_collapsible: (placement, collapsible) =>
      __dock_set_collapsible(
        handle,
        dockPlacement(placement, "set_dock_collapsible(placement, collapsible)"),
        Boolean(collapsible),
      ),
    is_locked: () => __dock_is_locked(handle),
    set_locked: (locked) => __dock_set_locked(handle, Boolean(locked)),
    is_zoomed: () => __dock_is_zoomed(handle),
    zoom_out: () => __dock_zoom_out(handle),
    on: (event, handler) => __dock_on(handle, String(event), handler),
    release: () => __dock_release(handle),
  });

  const virtualScrollHandle = (handle) => ({
    __handle: handle,
    // The strategy is base's own word for where the item lands. `top` puts it
    // at the near edge, `center` in the middle; base's default is `top`.
    scroll_to_item: (index, strategy) => {
      if (!Number.isInteger(index) || index < 0) {
        throw new TypeError("scroll_to_item(index) needs a whole, non-negative index");
      }
      __virtual_scroll_to_item(handle, index, String(strategy ?? "top"));
    },
    scroll_to_bottom: () => __virtual_scroll_to_bottom(handle),
    release: () => __virtual_scroll_release(handle),
  });

  let deferInit = false;
  globalThis.__construct = (Class) => {
    deferInit = true;
    try {
      return new Class();
    } finally {
      deferInit = false;
    }
  };
  // `init` gets the async flavor, and both of its paths get the same one. That
  // is the honest shape: `init` exists to set up things that outlive the call —
  // tasks, timers, retained handles — so the context it hands to them must
  // outlive it too. A call-scoped `cx` here could not be given to the very work
  // `init` is for.
  globalThis.__initialize = (instance, props) => {
    if (typeof instance.init === "function") instance.init(props, __async_cx());
  };
  // This journals ordinary reachable object and callable descriptors only. Restoration succeeds
  // only while post-update descriptors remain legally redefinable/deletable.
  // Reflection cannot see private/internal slots or undo non-configurable
  // additions/hardening; the public declaration documents that boundary.
  globalThis.__checkpoint_view = (instance) => {
    const snapshots = [];
    const seen = new Set();
    const pending = [instance];
    let propertyCount = 0;
    while (pending.length > 0) {
      const value = pending.pop();
      if (
        value === null ||
        (typeof value !== "object" && typeof value !== "function") ||
        seen.has(value)
      ) continue;
      if (snapshots.length >= 10_000) {
        throw new RangeError("a nested view update reached the 10,000-object rollback limit");
      }
      seen.add(value);
      const descriptors = Object.getOwnPropertyDescriptors(value);
      const keys = Reflect.ownKeys(descriptors);
      propertyCount += keys.length;
      if (propertyCount > 100_000) {
        throw new RangeError("a nested view update reached the 100,000-property rollback limit");
      }
      snapshots.push([value, descriptors]);
      for (const key of keys) {
        const descriptor = descriptors[key];
        if (Object.prototype.hasOwnProperty.call(descriptor, "value")) {
          pending.push(descriptor.value);
        }
      }
    }
    return () => {
      for (let index = snapshots.length - 1; index >= 0; index -= 1) {
        const [value, descriptors] = snapshots[index];
        const saved = new Set(Reflect.ownKeys(descriptors));
        for (const key of Reflect.ownKeys(value)) {
          if (!saved.has(key)) {
            const current = Object.getOwnPropertyDescriptor(value, key);
            if (current?.configurable) delete value[key];
          }
        }
        Object.defineProperties(value, descriptors);
      }
    };
  };

  class View {
    constructor(props) {
      // `new MyView(props)` from script reaches `init` without the host's
      // generation, so the context here is the async flavor — it resolves
      // whichever call is running, and says so if there is none.
      if (!deferInit && typeof this.init === "function") this.init(props, __async_cx());
    }
  }

  // A dialog and a sheet are views whose `render` is the author's function.
  // That is the whole of the wrapping: a script view is an object with a
  // `render`, so a content function already is one, once it is given the name.
  const contentView = (build, api) => {
    if (typeof build !== "function") {
      throw new TypeError(
        api + " takes a function returning an element, not an element and not a view class",
      );
    }
    return { render: () => build() };
  };

  // `setItem` converts its argument, and `getItem` answers a string or `null` —
  // the Web Storage API exactly, so an application that wants structure reaches
  // for `JSON.stringify` as it would on the web. `length` is a getter because
  // it is a property there, not a call.
  const storage = (session) => ({
    get length() {
      return __storage_length(session);
    },
    key: (index) => {
      const at = Number(index);
      return Number.isInteger(at) && at >= 0 ? __storage_key(session, at) : null;
    },
    getItem: (key) => __storage_get(session, String(key)),
    setItem: (key, value) => __storage_set(session, String(key), String(value)),
    removeItem: (key) => __storage_remove(session, String(key)),
    clear: () => __storage_clear(session),
    flush: () => __storage_flush(session),
  });

  // Overlays are window-level, not view-level: `cx.notify()` re-renders this
  // view, `window.open_dialog()` changes what the user is looking at. Grouped
  // under `window` because that is where `gpui-component` puts them — the
  // script API reads the same as the Rust it sits beside — and because it is
  // somewhere to grow: `Window` in Rust also answers focus, size and
  // appearance.
  //
  // A global rather than a module export. It names the window the
  // script is already inside, which is not something a file opts into by
  // importing it, and `window` is the one identifier every JavaScript author
  // already reaches for. Nothing collides: this runtime has no DOM.
  globalThis.window = {
    open_dialog: (build, options) =>
      __open_dialog(contentView(build, "window.open_dialog"), options ?? undefined),
    close_dialog: () => __close_dialog(),
    close_all_dialogs: () => __close_all_dialogs(),
    has_active_dialog: () => __has_active_dialog(),

    open_sheet: (build) => __open_sheet(undefined, contentView(build, "window.open_sheet")),
    open_sheet_at: (placement, build) =>
      __open_sheet(String(placement), contentView(build, "window.open_sheet_at")),
    close_sheet: () => __close_sheet(),
    has_active_sheet: () => __has_active_sheet(),

    push_toast: (options) => __push_toast(options),
    remove_toast: (id) => __remove_toast(String(id)),
    clear_toasts: () => __clear_toasts(),

    // The Web Storage API, where a browser keeps it. The two differ only in how
    // long they last: `localStorage` is a file the host placed, `sessionStorage`
    // is memory that goes with the process.
    localStorage: storage(false),
    sessionStorage: storage(true),

    // `Window::paint_path` in GPUI, so `window` here. It is the one element
    // constructor that is not a free function, and it is one because the thing
    // it mirrors is a method on the window rather than on the app.
    paint_path: paintPath,

    // What the window measures. All legal from `render`: a view that sizes
    // itself from the viewport, or spaces itself in rems, has to ask during
    // the pass that draws it.
    rem_size: () => __window_rem_size(),
    line_height: () => __window_line_height(),
    viewport_size: () => __window_viewport_size(),
    bounds: () => __window_bounds(),
    mouse_position: () => __window_mouse_position(),
    appearance: () => __window_appearance(),
    is_window_active: () => __window_is_active(),
    is_fullscreen: () => __window_is_fullscreen(),
    is_maximized: () => __window_is_maximized(),

    // What the window can be told. Refused from `render` for the reason
    // `cx.notify()` is: a frame that changes the window it is drawing into is
    // a frame arguing with itself.
    // `Window::dispatch_action` in GPUI, so `window` here — it walks the focus
    // path of *this* window. `cx.bind_keys` is the other half and is on `cx`
    // for the same reason: the keymap is `App`'s.
    dispatch_action: (action) => __dispatch_action(String(action)),

    set_rem_size: (size) => __window_set_rem_size(Number(size)),
    refresh: () => __window_refresh(),
    focus_next: () => __window_focus_next(),
    focus_prev: () => __window_focus_prev(),
    activate_window: () => __window_activate(),
    minimize_window: () => __window_minimize(),
    zoom_window: () => __window_zoom(),
    toggle_fullscreen: () => __window_toggle_fullscreen(),
  };

  globalThis.localStorage = globalThis.window.localStorage;
  globalThis.sessionStorage = globalThis.window.sessionStorage;

  let cachedThemeSource;
  let cachedTheme;
  let cachedThemeRevision = -1;
  globalThis.__theme_dirty = true;
  const refreshTheme = () => {
    const revision = __theme_revision();
    if (!globalThis.__theme_dirty && revision === cachedThemeRevision) return;
    const source = __theme_snapshot();
    if (source !== cachedThemeSource) {
      cachedThemeSource = source;
      cachedTheme = JSON.parse(source);
      Object.freeze(cachedTheme.colors);
      Object.freeze(cachedTheme.spacing);
      Object.freeze(cachedTheme.radius);
      Object.freeze(cachedTheme);
    }
    cachedThemeRevision = revision;
    globalThis.__theme_dirty = false;
  };
  globalThis.__prepare_theme = refreshTheme;
  const currentTheme = () => {
    if (globalThis.__theme_dirty || cachedTheme === undefined) refreshTheme();
    return cachedTheme;
  };

  // Every member of `cx` gates on `check()` and then does ordinary ambient
  // work. That gate is the whole difference between a call-scoped `cx` and an
  // async one, so a member added here is right for both flavors at once and
  // the two cannot drift.
  const contextMembers = (check) => ({
    theme: () => {
      check();
      return currentTheme();
    },
    open_url: (url) => {
      check();
      return __open_url(String(url));
    },
    read_from_clipboard: () => {
      check();
      return __clipboard_read_text();
    },
    write_to_clipboard: (text) => {
      check();
      return __clipboard_write_text(String(text));
    },
    focus_handle: () => {
      check();
      return focusHandle(__focus_handle_new());
    },
    // `App::bind_keys`, so `cx`. The keymap belongs to the application rather
    // than to a window, which is why binding a chord in one view makes it live
    // everywhere its `context` predicate matches.
    bind_keys: (bindings) => {
      check();
      if (!Array.isArray(bindings)) {
        throw new TypeError(
          "cx.bind_keys(bindings) expects an array of { keystroke, action, context? }",
        );
      }
      return __bind_keys(bindings);
    },
    new: (Class, props) => {
      check();
      if (typeof Class !== "function" || !(Class.prototype instanceof View)) {
        throw new TypeError("cx.new(Class, props) expects a View subclass");
      }
      return entity(__view_new(Class, props));
    },
    spawn: (body, opts) => {
      check();
      return __spawn(body, opts);
    },
    sleep: (ms) => {
      check();
      return __sleep(ms);
    },
    timer: {
      after: (ms, handler, opts) => {
        check();
        return __timer_after(ms, handler, opts);
      },
      every: (ms, handler, opts) => {
        check();
        return __timer_every(ms, handler, opts);
      },
    },
  });

  // A description recorded once and filled per call.
  //
  // **Not part of the script surface**, and deliberately so. Asking an author
  // to mark their hot paths is a performance annotation in the source: two ways
  // to write the same interface, restrictions that only report at first call,
  // and a decision nobody should have to make while describing a panel. The
  // machinery is kept because the runtime is meant to apply it *itself* — see
  // `engine/quickjs/template.rs` — and `globalThis.__template` is how the tests
  // that pin its behaviour reach it, the same standing `__apply` has.
  //
  // The body runs a single time, with a sentinel in each parameter position;
  // wherever a sentinel comes to rest in what it describes is a slot, and what
  // is left over is structure. Every call after that grafts the structure and
  // writes its arguments into the slots, entering no builder method at all.
  //
  // The sentinel refuses to become a primitive. `${price}` inside a body would
  // otherwise consume it and bake this first call's value into the structure,
  // which is a panel that silently stops updating — so it throws where it was
  // written instead.
  const templateSlot = (index) => {
    const refuse = () => {
      throw new TypeError(
        "a template argument can be passed to a builder call but not computed on. " +
          "Format or compare the value where the template is called, and pass the result",
      );
    };
    return {
      __slot: index,
      toString: refuse,
      valueOf: refuse,
      [Symbol.toPrimitive]: refuse,
    };
  };

  const template = (build) => {
    if (typeof build !== "function") {
      throw new TypeError("template(build) expects a function that builds one element");
    }
    let id = -1;
    return (...args) => {
      if (id < 0) {
        const slots = [];
        for (let i = 0; i < build.length; i += 1) slots.push(templateSlot(i));
        __template_begin(build.length);
        let root;
        try {
          root = build(...slots);
        } catch (error) {
          __template_abort();
          throw error;
        }
        id = __template_end(root?.__id);
      }
      return element(__template_instantiate(id, args));
    };
  };

  globalThis.__template = template;

  return {
    __element: element,
    View,
    div: () => element(__div()),
    h_flex: () => element(__h_flex()),
    v_flex: () => element(__v_flex()),
    svg: (path) => element(__svg(String(path))),
    image: (path) => element(__image(String(path))),
    PathBuilder: {
      fill: () => pathBuilder(true, 0),
      stroke: (width) => pathBuilder(false, finitePositive(width, "stroke width")),
    },
    Background: {
      solid: (color) => background("solid", [String(color)]),
      stop: (color, percentage) => {
        if (typeof percentage !== "number" || !Number.isFinite(percentage)) throw new TypeError("background stop percentage must be finite");
        return Object.freeze({ __backgroundStop: true, color: String(color), percentage });
      },
      linear_gradient: (angle, from, to) => {
        angle = Number(angle);
        if (!Number.isFinite(angle)) throw new TypeError("linear gradient angle must be finite");
        const stop = (value, fallback, name) => {
          if (typeof value === "string") return [value, fallback];
          if (!value?.__backgroundStop) throw new TypeError(name + " must be a color or Background.stop(color, percentage)");
          return [value.color, value.percentage];
        };
        const a = stop(from, 0, "gradient from stop");
        const b = stop(to, 1, "gradient to stop");
        return background("linear-gradient", [String(angle), a[0], String(a[1]), b[0], String(b[1])]);
      },
      pattern_slash: (color, width, interval) => background("pattern-slash", [
        String(color),
        String(finitePositive(width, "pattern width")),
        String(finitePositive(interval, "pattern interval")),
      ]),
      checkerboard: (color, size) => background("checkerboard", [
        String(color),
        String(finitePositive(size, "checkerboard size")),
      ]),
    },
    component: (module, name) => ({
      new: (id, props) => {
        if (typeof id !== "string" || id.length === 0) {
          throw new TypeError("a component needs a non-empty string id");
        }
        return element(__component(String(module), String(name), id, props ?? {}));
      },
    }),
    __context_members: contextMembers,
    Button: { new: (id) => element(__button(String(id))) },
    Link: { new: (id) => element(__link(String(id))) },
    Checkbox: { new: (id) => element(__checkbox(String(id))) },
    Switch: { new: (id) => element(__switch(String(id))) },
    Tabs: { new: (id) => element(__tabs(String(id))) },
    Tab: { new: (id) => element(__tab(String(id))) },
    Progress: { new: (id) => element(__progress(String(id))) },
    // Base's own shape: a root that chooses between two slots, and two slot
    // types that are not elements on their own.
    Accordion: { new: (id) => element(__accordion(String(id))) },
    AccordionItem: { new: () => element(__accordion_item()) },
    // `AccordionHeader::new` takes the trigger, exactly as `Popup.new` takes
    // its own: a heading whose button arrived later would be a heading that
    // announced nothing for a frame.
    AccordionHeader: {
      new: (trigger) => {
        if (typeof trigger?.__id !== "number") {
          throw new TypeError(
            "AccordionHeader.new(trigger) expects an AccordionTrigger element: the heading owns the button that opens the item, and base has none of its own",
          );
        }
        return element(__accordion_header()).trigger(trigger);
      },
    },
    AccordionPanel: { new: () => element(__accordion_panel()) },
    AccordionTrigger: { new: (id) => element(__accordion_trigger(String(id))) },
    Pagination: { new: (id) => element(__pagination(String(id))) },
    // Not a component: the one thing base contributes that a script cannot
    // write for itself is which page numbers to show, and that is arithmetic.
    // Called during `render`, once, and the buttons are built from what it
    // answers.
    pagination_items: (current_page, total_pages, visible_pages) =>
      __pagination_items(
        Number(current_page),
        Number(total_pages),
        visible_pages === undefined ? 7 : Number(visible_pages),
      ),
    Avatar: { new: () => element(__avatar()) },
    AvatarImage: { new: (path) => element(__avatar_image(String(path))) },
    AvatarFallback: { new: () => element(__avatar_fallback()) },
    ProgressTrack: { new: () => element(__progress_track()) },
    ProgressIndicator: { new: () => element(__progress_indicator()) },
    fps_monitor: () => element(__fps_monitor()),
    show_fps_monitor: (options) => __show_fps_monitor(options),
    hide_fps_monitor: () => __hide_fps_monitor(),
    fps_monitor_visible: () => __fps_monitor_visible(),
    Radio: { new: (id) => element(__radio(String(id))) },
    Toggle: { new: (id) => element(__toggle(String(id))) },
    RadioGroup: { new: (id) => element(__radio_group(String(id))) },
    ToggleGroup: { new: (id) => element(__toggle_group(String(id))) },
    Table: { new: (id) => element(__table(String(id))) },
    TableHeader: { new: (id) => element(__table_header(String(id))) },
    TableBody: { new: (id) => element(__table_body(String(id))) },
    TableCaption: { new: (id) => element(__table_caption(String(id))) },
    // Free functions, not `Type.new(...)`, because that is what base exports:
    // the group has no type a script ever names.
    h_resizable: (id) => element(__h_resizable(String(id))),
    v_resizable: (id) => element(__v_resizable(String(id))),
    resizable_panel: resizablePanel,
    TableRow: {
      new: (id, row_index) =>
        element(__table_row(String(id), oneBased(row_index, "TableRow.new row index"))),
    },
    TableHead: {
      new: (id, column_index) =>
        element(__table_head(String(id), oneBased(column_index, "TableHead.new column index"))),
    },
    TableCell: {
      new: (id, column_index) =>
        element(__table_cell(String(id), oneBased(column_index, "TableCell.new column index"))),
    },
    Collapsible: { new: () => element(__collapsible()) },
    Popover: { new: (id) => element(__popover(String(id))) },
    HoverCard: { new: (id) => element(__hover_card(String(id))) },
    // The trigger is a constructor argument, as it is in base: a popup with no
    // trigger has no bounds to anchor to, so there is no useful moment between
    // `new` and the trigger being known.
    Popup: {
      new: (id, trigger) => {
        if (typeof trigger?.__id !== "number") {
          throw new TypeError(
            "Popup.new(id, trigger) expects the trigger element; a popup anchors its content to the trigger's bounds, so it cannot be built without one",
          );
        }
        return element(__popup(String(id))).trigger(trigger);
      },
    },
    Select: { new: (id) => element(__select(String(id))) },
    Combobox: { new: (id) => element(__combobox(String(id))) },
    DatePicker: {
      new: (id, focus_handle) => {
        if (typeof focus_handle?.__handle !== "number") {
          throw new TypeError(
            "DatePicker.new(id, focus_handle) expects a FocusHandle from cx.focus_handle(); the picker takes the keyboard through that handle, and base has no builder to supply one later",
          );
        }
        return element(__date_picker(String(id), focus_handle.__handle));
      },
    },
    // Free functions, not `VirtualList.new(...)`, because that is what base
    // exports: `v_virtual_list` and `h_virtual_list` are the whole of its
    // public surface, and the list has no type a script ever names.
    //
    // The count is a separate argument from the sizes, which base does not
    // separate — its one vector is both. See the `.d.ts` for why: mirroring it
    // would put one number per row across the boundary on every render.
    v_virtual_list: virtualList(__v_virtual_list, "v_virtual_list"),
    h_virtual_list: virtualList(__h_virtual_list, "h_virtual_list"),
    list: lazyList(__list, "list", true),
    uniform_list: lazyList(__uniform_list, "uniform_list", false),
    VirtualListScrollHandle: { new: () => virtualScrollHandle(__virtual_scroll_new()) },
    Scrollbar: {
      new: (id) => element(__scrollbar(String(id))),
      // `horizontal` and `vertical` are `new` plus the orientation the group
      // containers already spell `axis`, so there is one word for orientation
      // in the whole API rather than a second one here.
      horizontal: (id) => element(__scrollbar(String(id))).axis("horizontal"),
      vertical: (id) => element(__scrollbar(String(id))).axis("vertical"),
    },
    InputState: {
      new: (options) =>
        inputState(__input_state_new(options?.placeholder ?? null, options?.value ?? null)),
    },
    Input: { new: (state) => element(__input_element(state.__handle)) },

    NumberInput: {
      new: (state) => element(__number_input_element(state.__handle)),
    },
    TextareaState: {
      new: (options) =>
        textareaState(
          __textarea_state_new(
            options?.placeholder ?? null,
            options?.value ?? null,
            options?.rows === undefined || options?.rows === null
              ? null
              : oneBased(options.rows, "TextareaState.new rows"),
          ),
        ),
    },
    Textarea: { new: (state) => element(__textarea_element(state.__handle)) },
    TextView: {
      html: (id, text) => element(__text_view(String(id), String(text), "html")),
      markdown: (id, text) => element(__text_view(String(id), String(text), "markdown")),
    },
    CalendarState: { new: () => calendarState(__calendar_state_new()) },
    SliderState: {
      new: (options) => {
        const settings = options ?? {};
        const min = settings.min ?? 0;
        const max = settings.max ?? 100;
        const step = settings.step ?? 1;
        const scale = String(settings.scale ?? "linear");
        for (const [name, value] of [["min", min], ["max", max], ["step", step]]) {
          if (typeof value !== "number" || !Number.isFinite(value)) {
            throw new TypeError("SliderState.new " + name + " must be a finite number");
          }
        }
        if (max <= min) throw new TypeError("SliderState.new needs a max greater than its min");
        if (step <= 0) throw new TypeError("SliderState.new step must be greater than 0");
        if (!["linear", "logarithmic"].includes(scale)) {
          throw new TypeError("SliderState.new scale must be linear or logarithmic");
        }
        // A logarithmic scale maps through log(value / min), which has no
        // answer at or below zero. Base asserts on it, and an assertion in the
        // host is a lost application rather than a reported mistake.
        if (scale === "logarithmic" && min <= 0) {
          throw new TypeError("SliderState.new with a logarithmic scale needs a min greater than 0");
        }
        return sliderState(
          __slider_state_new(
            min,
            max,
            step,
            scale,
            sliderValues(settings.value ?? min, "SliderState.new value"),
          ),
        );
      },
    },
    Slider: { new: (state) => element(__slider_element(state.__handle)) },
    SliderTrack: { new: (state) => element(__slider_track_element(state.__handle)) },
    SliderIndicator: { new: (state) => element(__slider_indicator_element(state.__handle)) },
    SliderThumb: { new: (state) => element(__slider_thumb_element(state.__handle)) },
    OtpState: {
      new: (length, options) => {
        // A code of no cells accepts no keystroke and shows nothing, and a
        // typed-in length of six hundred thousand is a frozen window. Neither
        // is something base refuses, so it is refused here.
        if (!Number.isInteger(length) || length < 1 || length > 64) {
          throw new TypeError("OtpState.new(length) expects a whole number between 1 and 64");
        }
        const settings = options ?? {};
        return otpState(
          __otp_state_new(length, settings.value ?? null, Boolean(settings.masked)),
        );
      },
    },
    OtpInput: { new: (state) => element(__otp_element(state.__handle)) },
    DockArea: {
      new: (id, options) => {
        const version = options?.version;
        if (version !== undefined && version !== null && (!Number.isSafeInteger(version) || version < 0)) {
          throw new TypeError("DockArea.new(id, options) version expects a whole, non-negative safe integer");
        }
        return dockArea(__dock_area_new(String(id), version ?? null));
      },
      // Not a method on an area: a builder is registered for the whole
      // application, and a layout is restored into whichever area asks for it.
      // Registering the same name twice replaces the class, which is what a hot
      // reload does.
      register_panel: (name, Class) => {
        if (typeof name !== "string" || name.length === 0) {
          throw new TypeError(
            "DockArea.register_panel(name, Class) needs the name the panel is added under",
          );
        }
        if (typeof Class !== "function" || !(Class.prototype instanceof View)) {
          throw new TypeError(
            "DockArea.register_panel(name, Class) expects the View subclass the panel is rebuilt from",
          );
        }
        return __dock_register_panel(name, Class);
      },
    },
    // Free functions, not `DockArea.element(...)`: the area is the state and
    // this is one description of it, the same split `v_virtual_list` has.
    dock_area: dockAreaElement,
    dock_content: () => element(__dock_content()),
  };
})();
"#;

impl ShellRuntime {
    fn install_globals(self: &Rc<Self>) -> Result<()> {
        let runtime = Rc::downgrade(self);
        self.with_js(move |ctx| {
            let globals = ctx.globals();

            // Two tables rather than one list of names: the prelude binds a
            // different prototype method over each, and both close over the
            // index that identifies the style, so that recording one never
            // puts its name on the wire.
            let nullary = rquickjs::Array::new(ctx.clone())?;
            let nullary_indexes = rquickjs::Array::new(ctx.clone())?;
            for (position, (name, index)) in style::nullary_styles().into_iter().enumerate() {
                nullary.set(position, name)?;
                nullary_indexes.set(position, index)?;
            }
            globals.set("__nullaryStyles", nullary)?;
            globals.set("__nullaryStyleIndexes", nullary_indexes)?;

            let parametric = rquickjs::Array::new(ctx.clone())?;
            for (index, name) in style::param_styles().enumerate() {
                parametric.set(index, name)?;
            }
            globals.set("__paramStyles", parametric)?;

            let behaviors = rquickjs::Array::new(ctx.clone())?;
            let mut behavior_count = 0;
            for (index, name) in [
                "on_click",
                "on_link_click",
                "on_mouse_move",
                "on_hover",
                "on_key_down",
                "on_key_up",
                "on_modifiers_changed",
                "on_mouse_down",
                "on_mouse_up",
                "on_mouse_down_out",
                "on_scroll_wheel",
                "on_action",
                "key_context",
                "aria_level",
                "keep_mounted",
                "on_item_click",
                "on_item_secondary_click",
                "on_change",
                "on_open_change",
                "on_confirm",
                "on_dismiss",
                "on_step",
                "disabled",
                "selectable",
                "scrollable",
                "selected",
                "checked",
                "accessibility_label",
                "tooltip",
                "role",
                "aria_selected",
                "aria_active_descendant",
                "tab_index",
                "tab_stop",
                "href",
                "id",
                "overflow_scroll",
                "overflow_x_scroll",
                "overflow_y_scroll",
                "overflow_scrollbar",
                "overflow_x_scrollbar",
                "overflow_y_scrollbar",
                "viewport_from_layout",
                "controls_right",
                "on_resize",
                "set_position",
                "pressed",
                "start",
                "value",
                "indeterminate",
                "row_count",
                "column_count",
                "open",
                "default_open",
                "overlay_closable",
                "with_item_to_measure_index",
            ]
            .into_iter()
            .enumerate()
            {
                behaviors.set(index, name)?;
                behavior_count = index + 1;
            }
            for descriptor in self.components.descriptors() {
                for method in descriptor.methods() {
                    behaviors.set(behavior_count, method.name())?;
                    behavior_count += 1;
                }
            }
            globals.set("__behaviorNames", behaviors)?;

            // The prelude checks an anchor at the call site, so it needs the
            // same eight names the parser accepts rather than a second copy of
            // them.
            let anchors = rquickjs::Array::new(ctx.clone())?;
            for (index, name) in crate::materialize::ANCHOR_NAMES.into_iter().enumerate() {
                anchors.set(index, name)?;
            }
            globals.set("__anchorNames", anchors)?;

            constructor(&globals, "__div", runtime.clone(), || Component::Div)?;
            constructor(&globals, "__h_flex", runtime.clone(), || Component::HFlex)?;
            constructor(&globals, "__v_flex", runtime.clone(), || Component::VFlex)?;
            let component_runtime = runtime.clone();
            globals.set(
                "__component",
                Func::from(move |ctx: Ctx<'_>, module: String, name: String, id: String, props: host_modules::Argument| -> JsResult<SpecId> {
                    let props = props.0;
                    if id.is_empty() {
                        return Err(Exception::throw_type(&ctx, "a component needs a non-empty string id"));
                    }
                    let registry = crate::host_modules::modules();
                    registry
                        .get(&module)
                        .and_then(|found| found.resolve_component(&name))
                        .map_err(|error| Exception::throw_message(&ctx, error.message()))?;
                    Ok(upgrade(&component_runtime, &ctx)?.push_node(Component::Module(
                        crate::spec::ModuleComponentSpec {
                            module: module.into(),
                            component: name.into(),
                            id: id.into(),
                            props,
                            policy: crate::scope::policy(),
                        },
                    )))
                }),
            )?;
            text_constructor(&globals, "__text", runtime.clone(), Component::Text)?;
            let text_view_runtime = runtime.clone();
            globals.set(
                "__text_view",
                Func::from(move |ctx: Ctx<'_>, id: String, text: String, format: String| -> JsResult<SpecId> {
                    let format = match format.as_str() {
                        "html" => crate::spec::TextViewFormat::Html,
                        "markdown" => crate::spec::TextViewFormat::Markdown,
                        _ => return Err(Exception::throw_type(&ctx, "TextView format must be html or markdown")),
                    };
                    Ok(upgrade(&text_view_runtime, &ctx)?.push_node(Component::TextView {
                        id: id.into(),
                        text: text.into(),
                        format,
                    }))
                }),
            )?;
            text_constructor(&globals, "__svg", runtime.clone(), Component::Svg)?;
            text_constructor(&globals, "__image", runtime.clone(), Component::Image)?;
            text_constructor(
                &globals,
                "__accordion",
                runtime.clone(),
                Component::Accordion,
            )?;
            constructor(&globals, "__accordion_item", runtime.clone(), || {
                Component::AccordionItem
            })?;
            constructor(&globals, "__accordion_header", runtime.clone(), || {
                Component::AccordionHeader
            })?;
            constructor(&globals, "__accordion_panel", runtime.clone(), || {
                Component::AccordionPanel
            })?;
            text_constructor(
                &globals,
                "__accordion_trigger",
                runtime.clone(),
                Component::AccordionTrigger,
            )?;
            text_constructor(
                &globals,
                "__pagination",
                runtime.clone(),
                Component::Pagination,
            )?;
            globals.set("__pagination_items", Func::from(pagination_items))?;
            constructor(&globals, "__avatar", runtime.clone(), || Component::Avatar)?;
            text_constructor(
                &globals,
                "__avatar_image",
                runtime.clone(),
                Component::AvatarImage,
            )?;
            constructor(&globals, "__avatar_fallback", runtime.clone(), || {
                Component::AvatarFallback
            })?;
            let path_runtime = runtime.clone();
            globals.set(
                "__path",
                Func::from(
                    move |ctx: Ctx<'_>,
                          fill: bool,
                          kind: String,
                          values: Array<'_>,
                          opacity: f64,
                          color_space: String,
                          width: f64|
                          -> JsResult<SpecId> {
                        if !width.is_finite() || width < 0.0 {
                            return Err(Exception::throw_type(
                                &ctx,
                                "path stroke width must be finite and non-negative",
                            ));
                        }
                        if !opacity.is_finite() || opacity < 0.0 {
                            return Err(Exception::throw_type(
                                &ctx,
                                "path background opacity must be finite and non-negative",
                            ));
                        }
                        let value_count =
                            crate::engine::quickjs::host_modules::bridge_array_len(&ctx, &values)?;
                        let mut value_strings = Vec::new();
                        value_strings.try_reserve_exact(value_count).map_err(|_| {
                            Exception::throw_range(&ctx, "path background values are too large")
                        })?;
                        for index in 0..value_count {
                            value_strings.push(values.get::<String>(index)?);
                        }
                        let number = |index: usize, name: &str| -> JsResult<f32> {
                            value_strings
                                .get(index)
                                .and_then(|value| value.parse::<f32>().ok())
                                .filter(|value| value.is_finite())
                                .ok_or_else(|| Exception::throw_type(&ctx, name))
                        };
                        let text = |index: usize, name: &str| -> JsResult<String> {
                            value_strings
                                .get(index)
                                .cloned()
                                .ok_or_else(|| Exception::throw_type(&ctx, name))
                        };
                        let kind = match kind.as_str() {
                            "solid" => crate::spec::BackgroundKind::Solid {
                                color: text(0, "solid background needs a color")?,
                            },
                            "linear-gradient" => crate::spec::BackgroundKind::LinearGradient {
                                angle: number(0, "gradient angle must be finite")?,
                                from: (
                                    text(1, "gradient needs a from color")?,
                                    number(2, "gradient from percentage must be finite")?,
                                ),
                                to: (
                                    text(3, "gradient needs a to color")?,
                                    number(4, "gradient to percentage must be finite")?,
                                ),
                                color_space,
                            },
                            "pattern-slash" => crate::spec::BackgroundKind::PatternSlash {
                                color: text(0, "slash pattern needs a color")?,
                                width: number(1, "slash pattern width must be finite")?,
                                interval: number(2, "slash pattern interval must be finite")?,
                            },
                            "checkerboard" => crate::spec::BackgroundKind::Checkerboard {
                                color: text(0, "checkerboard needs a color")?,
                                size: number(1, "checkerboard size must be finite")?,
                            },
                            _ => {
                                return Err(Exception::throw_type(&ctx, "unknown Background kind"));
                            }
                        };
                        Ok(upgrade(&path_runtime, &ctx)?.push_node(Component::Path {
                            fill,
                            background: crate::spec::BackgroundSpec {
                                kind,
                                opacity: opacity as f32,
                            },
                            stroke_width: width as f32,
                        }))
                    },
                ),
            )?;
            text_constructor(&globals, "__button", runtime.clone(), Component::Button)?;
            text_constructor(&globals, "__link", runtime.clone(), Component::Link)?;
            text_constructor(&globals, "__checkbox", runtime.clone(), Component::Checkbox)?;
            text_constructor(&globals, "__switch", runtime.clone(), Component::Switch)?;
            text_constructor(
                &globals,
                "__scrollbar",
                runtime.clone(),
                Component::Scrollbar,
            )?;
            text_constructor(&globals, "__tabs", runtime.clone(), Component::Tabs)?;
            text_constructor(&globals, "__tab", runtime.clone(), Component::Tab)?;
            text_constructor(&globals, "__progress", runtime.clone(), Component::Progress)?;
            constructor(&globals, "__progress_track", runtime.clone(), || {
                Component::ProgressTrack
            })?;
            constructor(&globals, "__progress_indicator", runtime.clone(), || {
                Component::ProgressIndicator
            })?;
            constructor(&globals, "__fps_monitor", runtime.clone(), || {
                Component::FpsMonitor
            })?;
            text_constructor(&globals, "__radio", runtime.clone(), Component::Radio)?;
            text_constructor(&globals, "__toggle", runtime.clone(), Component::Toggle)?;
            text_constructor(
                &globals,
                "__radio_group",
                runtime.clone(),
                Component::RadioGroup,
            )?;
            text_constructor(
                &globals,
                "__toggle_group",
                runtime.clone(),
                Component::ToggleGroup,
            )?;
            text_constructor(&globals, "__table", runtime.clone(), Component::Table)?;
            text_constructor(
                &globals,
                "__table_header",
                runtime.clone(),
                Component::TableHeader,
            )?;
            text_constructor(
                &globals,
                "__table_body",
                runtime.clone(),
                Component::TableBody,
            )?;
            text_constructor(
                &globals,
                "__table_caption",
                runtime.clone(),
                Component::TableCaption,
            )?;
            indexed_constructor(
                &globals,
                "__table_row",
                runtime.clone(),
                Component::TableRow,
            )?;
            indexed_constructor(
                &globals,
                "__table_head",
                runtime.clone(),
                Component::TableHead,
            )?;
            indexed_constructor(
                &globals,
                "__table_cell",
                runtime.clone(),
                Component::TableCell,
            )?;
            // The axis comes from the constructor, so each one is a closure
            // over the variant rather than a second builder method.
            text_constructor(&globals, "__h_resizable", runtime.clone(), |id| {
                Component::Resizable(id, gpui::Axis::Horizontal)
            })?;
            text_constructor(&globals, "__v_resizable", runtime.clone(), |id| {
                Component::Resizable(id, gpui::Axis::Vertical)
            })?;
            constructor(&globals, "__resizable_panel", runtime.clone(), || {
                Component::ResizablePanel
            })?;
            constructor(&globals, "__collapsible", runtime.clone(), || {
                Component::Collapsible
            })?;
            text_constructor(&globals, "__popover", runtime.clone(), Component::Popover)?;
            text_constructor(
                &globals,
                "__hover_card",
                runtime.clone(),
                Component::HoverCard,
            )?;
            virtual_list_constructor(
                &globals,
                "__v_virtual_list",
                runtime.clone(),
                gpui::Axis::Vertical,
            )?;
            virtual_list_constructor(
                &globals,
                "__h_virtual_list",
                runtime.clone(),
                gpui::Axis::Horizontal,
            )?;
            list_constructor(
                &globals,
                "__list",
                runtime.clone(),
                crate::spec::ListKind::Measured,
            )?;
            list_constructor(
                &globals,
                "__uniform_list",
                runtime.clone(),
                crate::spec::ListKind::Uniform,
            )?;
            text_constructor(&globals, "__popup", runtime.clone(), Component::Popup)?;
            text_constructor(&globals, "__select", runtime.clone(), Component::Select)?;
            text_constructor(&globals, "__combobox", runtime.clone(), Component::Combobox)?;

            // The one constructor that takes retained state as well as an id.
            // Base's `DatePicker::new` requires the focus handle, so a picker
            // whose handle has already been released is refused where it was
            // written rather than rendered as an unreachable trigger.
            let date_picker_runtime = runtime.clone();
            globals.set(
                "__date_picker",
                Func::from(
                    move |ctx: Ctx<'_>,
                          id: String,
                          handle: crate::entities::EntityHandle|
                          -> JsResult<SpecId> {
                        let store = upgrade(&date_picker_runtime, &ctx)?;
                        if store.entities().focus(handle).is_none() {
                            return Err(Exception::throw_type(
                                &ctx,
                                "the focus handle given to DatePicker.new(id, focus_handle) has \
                                 been released; a date picker takes the keyboard through that \
                                 handle, so it needs a live one",
                            ));
                        }
                        Ok(store.push_node(Component::DatePicker(id, handle)))
                    },
                ),
            )?;

            let create_view = runtime.clone();
            globals.set(
                "__view_new",
                Func::from(
                    move |ctx: Ctx<'_>, class: NestedViewClass, props: NestedViewProps| {
                        refuse_nested_view_mutation(
                            &ctx,
                            "cx.new(Class, props)",
                            "create",
                        )?;
                        let runtime = upgrade(&create_view, &ctx)?;
                        runtime.queue_nested_view_creation(&ctx, class.0, props.0)
                    },
                ),
            )?;

            let update_view = runtime.clone();
            globals.set(
                "__view_set_props",
                Func::from(move |ctx: Ctx<'_>, token: u32, props: NestedViewProps| {
                    refuse_nested_view_mutation(&ctx, "entity.set_props(props)", "update")?;
                    let runtime = upgrade(&update_view, &ctx)?;
                    runtime.queue_nested_view_update(&ctx, token, props.0)
                }),
            )?;

            let release_view = runtime.clone();
            globals.set(
                "__view_release",
                Func::from(move |ctx: Ctx<'_>, token: u32| -> JsResult<bool> {
                    refuse_nested_view_mutation(&ctx, "entity.release()", "release")?;
                    let runtime = upgrade(&release_view, &ctx)?;
                    runtime.queue_nested_view_release(&ctx, token)
                }),
            )?;

            let mount_view = runtime.clone();
            globals.set(
                "__child_view",
                Func::from(move |ctx: Ctx<'_>, token: u32| -> JsResult<SpecId> {
                    let runtime = upgrade(&mount_view, &ctx)?;
                    if runtime.pending_nested.borrow().iter().any(|operation| {
                        matches!(operation, PendingNestedOperation::Release { token: candidate, .. } if *candidate == token)
                    }) {
                        return Err(Exception::throw_type(
                            &ctx,
                            "this Entity has been released and can no longer be mounted",
                        ));
                    }
                    let handle = runtime
                        .nested_view_handles
                        .borrow()
                        .get(&token)
                        .filter(|alias| alias.provenance.is_current())
                        .map(|alias| alias.handle)
                        .ok_or_else(|| {
                            Exception::throw_type(
                                &ctx,
                                "this Entity has been released and can no longer be mounted",
                            )
                        })?;
                    // Resolve and clone before borrowing the arena. The
                    // snapshot keeps this entity alive after handle release.
                    let view = { runtime.entities().view(handle) }.ok_or_else(|| {
                        Exception::throw_type(
                            &ctx,
                            "this Entity has been released and can no longer be mounted",
                        )
                    })?;
                    runtime
                        .arena
                        .borrow_mut()
                        .push_child_view(ChildViewSpec::new(handle, view))
                        .map_err(|error| Exception::throw_type(&ctx, &error.to_string()))
                }),
            )?;

            let state_runtime = runtime.clone();
            globals.set(
                "__state",
                Func::from(
                    move |ctx: Ctx<'_>, id: u32, name: String| -> JsResult<SpecId> {
                        upgrade(&state_runtime, &ctx)?.begin_state(&ctx, id, &name)
                    },
                ),
            )?;

            globals.set(
                "__unknown",
                Func::from(|ctx: Ctx<'_>, name: String| -> JsResult<()> {
                    Err(Exception::throw_type(&ctx, &unknown_method(&name)))
                }),
            )?;

            // A `cx` for code the host is not calling with one in hand: the
            // `View` constructor, and `init` through it. Ambient, so it works
            // wherever a call is running and says so where none is.
            globals.set(
                "__async_cx",
                Func::from(async_context_object),
            )?;

            let attach_runtime = runtime.clone();
            globals.set(
                "__attach",
                Func::from(move |ctx: Ctx<'_>, id: u32, child: u32| -> JsResult<()> {
                    upgrade(&attach_runtime, &ctx)?.attach(&ctx, id, child)
                }),
            )?;

            let nullary_style_runtime = runtime.clone();
            globals.set(
                "__applyNullaryStyle",
                Func::from(move |ctx: Ctx<'_>, id: u32, index: u16| -> JsResult<()> {
                    let runtime = upgrade(&nullary_style_runtime, &ctx)?;
                    runtime.push_op_checked(&ctx, runtime.push_op(id, SpecOp::NullaryStyle(index)))
                }),
            )?;

            let param_style_runtime = runtime.clone();
            globals.set(
                "__applyParamStyle",
                Func::from(
                    move |ctx: Ctx<'_>, id: u32, index: usize, value: Opt<StyleArgument>| {
                        let runtime = upgrade(&param_style_runtime, &ctx)?;
                        runtime.apply_param_style(&ctx, id, index, value.0)
                    },
                ),
            )?;

            let apply_runtime = runtime.clone();
            globals.set(
                "__apply",
                Func::from(
                    move |ctx: Ctx<'_>, id: u32, name: String, args: Arguments| {
                        let runtime = upgrade(&apply_runtime, &ctx)?;
                        runtime.apply(&ctx, id, &name, args)
                    },
                ),
            )?;

            // Templates. `begin` swaps the description being recorded for a
            // fresh one so the body's ids start at zero, `end` takes it back
            // out, and `abort` puts the interrupted one back when the body
            // threw. Three calls rather than one because the body runs in
            // JavaScript between them.
            let begin_runtime = runtime.clone();
            globals.set(
                "__template_begin",
                Func::from(move |ctx: Ctx<'_>, arity: usize| {
                    upgrade(&begin_runtime, &ctx)?.begin_template(&ctx, arity)
                }),
            )?;
            let end_runtime = runtime.clone();
            globals.set(
                "__template_end",
                Func::from(move |ctx: Ctx<'_>, root: Option<SpecId>| {
                    upgrade(&end_runtime, &ctx)?.end_template(&ctx, root)
                }),
            )?;
            let abort_runtime = runtime.clone();
            globals.set(
                "__template_abort",
                Func::from(move |ctx: Ctx<'_>| {
                    upgrade(&abort_runtime, &ctx)?.abort_template();
                    JsResult::Ok(())
                }),
            )?;
            let instantiate_runtime = runtime.clone();
            globals.set(
                "__template_instantiate",
                Func::from(instantiate_template_binding(instantiate_runtime)),
            )?;
            let text_slot_runtime = runtime.clone();
            globals.set(
                "__text_slot",
                Func::from(move |ctx: Ctx<'_>, argument: u16| {
                    upgrade(&text_slot_runtime, &ctx)?.text_slot(&ctx, argument)
                }),
            )?;

            // Test-only probes for `tests::benchmark`. Each one accepts a
            // prefix of `__apply`'s signature and does nothing with it, so the
            // difference between two of them is the cost of converting the one
            // argument that was added — and the difference between the last one
            // and `__apply` is everything `apply` itself does. There is no way
            // to measure that split from script alone: a crossing that does
            // nothing has to exist for the crossing to be priced.
            #[cfg(test)]
            {
                globals.set("__benchId", Func::from(|_id: u32| {}))?;
                globals.set("__benchName", Func::from(|_id: u32, _name: String| {}))?;
                globals.set(
                    "__benchArgs",
                    Func::from(|_id: u32, _name: String, _args: Arguments| {}),
                )?;
            }

            // Before the prelude, which builds the `window` object over these.
            overlay::install(ctx, &ctx.globals())?;
            window_api::install(ctx)?;

            ctx.eval::<(), _>(PRELUDE)?;

            // Registered exports live apart from the built-in module object.
            // Names such as `InputState` exist in both `gpui-base` and
            // `gpui-component`; sharing `__gpui` would make the latter silently
            // replace the former for every module that reads the global table.
            let module: Object = ctx.globals().get("__gpui")?;
            let component_module = Object::new(ctx.clone())?;
            ctx.globals()
                .set("__gpui_components", component_module.clone())?;
            for descriptor in self.components.states() {
                let state_runtime = runtime.clone();
                let descriptor = descriptor.clone();
                component_module.set(
                    descriptor.export(),
                    Func::from(move |ctx: Ctx<'_>, arguments: Arguments| -> JsResult<u64> {
                        let runtime = upgrade(&state_runtime, &ctx)?;
                        runtime.retained_state_transaction(&ctx, &descriptor, &arguments)
                    }),
                )?;
            }
            for (id, descriptor) in self.components.registered() {
                for constructor_descriptor in descriptor.constructors() {
                    let constructor_runtime = runtime.clone();
                    let component_name = descriptor.name();
                    let constructor = constructor_descriptor.clone();
                    component_module.set(
                        constructor_descriptor.export(),
                        Func::from(move |ctx: Ctx<'_>, arguments: Arguments| -> JsResult<SpecId> {
                            let runtime = upgrade(&constructor_runtime, &ctx)?;
                            if let Some(deprecation) = &constructor.deprecation() {
                                runtime.warn_deprecated_export(
                                    constructor.export(),
                                    deprecation.replacement(),
                                    deprecation.message(),
                                );
                            }
                            let payload = runtime.component_payload_transaction(
                                &ctx,
                                constructor.export(),
                                &constructor.arguments(),
                                &arguments,
                                |arguments| constructor.payload(arguments),
                            )?;
                            let component = crate::spec::RegisteredComponentSpec::new(
                                id,
                                component_name,
                                payload,
                            );
                            Ok(runtime.push_node(Component::Registered(component)))
                        }),
                    )?;
                }
            }
            host::install(ctx, &module)?;
            host_modules::install(ctx)?;
            theme_api::install(ctx, &module)?;
            entity_api::install(ctx, &module, runtime.clone())?;
            dock_api::install(ctx, &module, runtime.clone())?;
            scheduler::install(ctx, &module)?;
            // Standard Runtime constructors and prototypes must exist before
            // the sandbox freezes built-ins, or they would remain mutable.
            standard::install(ctx)?;
            sandbox::install(ctx)?;

            Ok(())
        })
    }

    /// Adds an element to another element's children.
    fn attach(&self, ctx: &Ctx<'_>, id: SpecId, child: SpecId) -> JsResult<()> {
        // A `resizable_panel()` is not an element anywhere else: base's panel
        // reads its size out of the group's state and panics outright without
        // one. Refused here, where the script can be pointed at the line that
        // did it, rather than at paint time.
        let orphan = {
            let arena = self.arena.borrow();
            let component = |node| arena.node(node).and_then(crate::spec::SpecNode::component);
            matches!(component(child), Some(Component::ResizablePanel))
                && !matches!(component(id), Some(Component::Resizable(..)))
        };
        if orphan {
            return Err(Exception::throw_type(
                ctx,
                "resizable_panel() belongs to an h_resizable() or v_resizable(): its size \
                 and its drag handle are the group's. Use a div() here instead",
            ));
        }
        let attached = self.arena.borrow_mut().attach(id, child);
        self.push_op_checked(ctx, attached)
    }

    /// Records a style method that takes an argument, addressed by index.
    ///
    /// The dispatch `apply` would do for the same call has already happened:
    /// the prelude closed over the position in the parametric table when it
    /// bound the method, so this resolves a name by indexing rather than by
    /// looking a string up.
    fn apply_param_style(
        &self,
        ctx: &Ctx<'_>,
        id: SpecId,
        index: usize,
        value: Option<StyleArgument>,
    ) -> JsResult<()> {
        let name = style::param_style_at(index)
            .ok_or_else(|| Exception::throw_type(ctx, "unknown element method"))?;
        let value = match value {
            Some(StyleArgument::Value(value)) => value,
            // The value is not known yet, so neither is whether it is valid.
            // A placeholder is recorded, the position is noted, and the same
            // `style::apply_param` check runs at instantiation with the real
            // argument in hand — so a bad colour still reports, one call later.
            Some(StyleArgument::Slot(argument)) => {
                let placeholder: SmallVec<[Bridged; 2]> = smallvec::smallvec![Bridged::Nil];
                self.push_op_checked(ctx, self.push_op(id, SpecOp::ParamStyle(name, placeholder)))?;
                return self.record_slot_at_last_op(ctx, id, 0, argument);
            }
            Some(StyleArgument::Handler) => {
                return Err(Exception::throw_type(
                    ctx,
                    &format!("`{name}` does not take a function"),
                ));
            }
            // Said by the same code that says it for every other bound method,
            // so a missing argument reads the same wherever it is missed.
            None => {
                let error = crate::value::arg(&[], 0, name)
                    .expect_err("argument 0 of an empty list is always missing");
                return Err(Exception::throw_type(ctx, error.message()));
            }
        };

        let args: SmallVec<[Bridged; 2]> = smallvec::smallvec![value];
        // Validate eagerly so a bad argument reports at the call site instead
        // of surfacing during materialize.
        style::apply_param(name, &args, Default::default())
            .map_err(|error| Exception::throw_type(ctx, error.message()))?;
        self.push_op_checked(ctx, self.push_op(id, SpecOp::ParamStyle(name, args)))
    }

    fn apply(&self, ctx: &Ctx<'_>, id: SpecId, method: &str, args: Arguments) -> JsResult<()> {
        // A sentinel among the arguments means this call is being recorded into
        // a template rather than into a description, and the position it landed
        // in is a slot. Checked before the dispatch below because the ordinary
        // path would reject the sentinel as neither a value nor a function.
        if let Some((position, argument)) = args.first_slot() {
            if self.is_registered_component(id) {
                return Err(Exception::throw_type(
                    ctx,
                    &format!(
                        "`{method}` cannot take a template argument yet; registered component methods are validated when their values are recorded"
                    ),
                ));
            }
            return self.apply_slot(ctx, id, method, position, argument);
        }

        let registered_method = self.registered_method_descriptor(id, method);
        // The declarations withhold these from a registered component that does
        // not declare them, so the two lists must agree — see the test below.
        let registered_common_behavior =
            crate::typings::REGISTERED_COMMON_BEHAVIORS.contains(&method);
        let registered_common_slot = matches!(
            method,
            "content"
                | "trigger"
                | "input"
                | "decrement_button"
                | "increment_button"
                | "image"
                | "fallback"
                | "header"
                | "footer"
                | "panel"
        );
        if registered_common_behavior
            && self.is_registered_component(id)
            && registered_method.is_none()
        {
            return Err(Exception::throw_type(ctx, &unknown_method(method)));
        }
        if registered_common_behavior && let Some(descriptor) = registered_method.as_ref() {
            self.arena
                .borrow()
                .check_live(id)
                .map_err(|error| Exception::throw_type(ctx, &error.to_string()))?;
            let mut callback = None;
            self.component_payload_transaction(
                ctx,
                descriptor.name(),
                descriptor.arguments(),
                &args,
                |arguments| {
                    if method == "on_click"
                        && let [ComponentArgument::Callback(id)] = arguments
                    {
                        callback = Some(*id);
                    }
                    descriptor.record(arguments)
                },
            )?;
            if method == "on_click" {
                let callback = callback.ok_or_else(|| {
                    Exception::throw_type(
                        ctx,
                        "on_click descriptor must declare exactly one callback argument",
                    )
                })?;
                return self.push_op_checked(
                    ctx,
                    self.push_op(id, SpecOp::Callback("on_click", callback)),
                );
            }
        }
        if !registered_common_behavior && let Some(descriptor) = registered_method {
            self.arena
                .borrow()
                .check_live(id)
                .map_err(|error| Exception::throw_type(ctx, &error.to_string()))?;
            let payload = self.component_payload_transaction(
                ctx,
                descriptor.name(),
                descriptor.arguments(),
                &args,
                |arguments| descriptor.record(arguments),
            )?;
            return self.push_op_checked(
                ctx,
                self.push_op(
                    id,
                    SpecOp::RegisteredMethod(crate::RecordedComponentMethod::new(
                        descriptor.name(),
                        payload,
                    )),
                ),
            );
        }
        if self.is_registered_component(id)
            && !registered_common_behavior
            && !registered_common_slot
        {
            return Err(Exception::throw_type(ctx, &unknown_method(method)));
        }

        match method {
            "child" => {
                let child = args
                    .first_value()
                    .and_then(|value| value.as_f32().ok())
                    .ok_or_else(|| {
                        Exception::throw_type(ctx, "child(element) expects an element")
                    })? as SpecId;
                self.attach(ctx, id, child)
            }
            "content" | "trigger" | "input" | "decrement_button" | "increment_button" | "image"
            | "fallback" | "header" | "footer" | "panel" => {
                let element = args
                    .first_value()
                    .and_then(|value| value.as_f32().ok())
                    .ok_or_else(|| {
                        Exception::throw_type(ctx, &format!("{method}(element) expects an element"))
                    })? as SpecId;
                self.fill_slot(ctx, id, method, element)
            }
            // The script's own name for an action, plus the handler. It is not
            // a `Callback` op because the name is discovered at run time and a
            // `Callback` holds a `&'static str`; see `SpecOp::ActionCallback`.
            "on_action" => {
                if scope::current_phase() == Some(ScopePhase::Layout) {
                    return Err(Exception::throw_type(
                        ctx,
                        "`on_action` cannot be registered from a virtual list's item \
                         renderer: the rows are rebuilt every frame, so a handler \
                         registered there would pile up for as long as the view stood",
                    ));
                }
                let action = args
                    .first_value()
                    .and_then(|value| value.as_str().ok().map(str::to_owned))
                    .filter(|action| !action.is_empty())
                    .ok_or_else(|| {
                        Exception::throw_type(
                            ctx,
                            "on_action(action, handler) expects the action's name first, \
                             as a non-empty string",
                        )
                    })?;
                let saved = args.handler_at(1).ok_or_else(|| {
                    Exception::throw_type(
                        ctx,
                        "on_action(action, handler) expects a function second",
                    )
                })?;
                let callback = self.callbacks.borrow_mut().push(CallbackEntry {
                    value: saved,
                    view: scope::current_view().map(|view| view.downgrade()),
                    application: scope::current_application_generation(),
                    registered_in: scope::current_generation(),
                });
                self.push_op_checked(
                    ctx,
                    self.push_op(id, SpecOp::ActionCallback(action.into(), callback)),
                )
            }
            // The only two element methods that take an argument *and* a
            // handler. The button is folded into the recorded op name — three
            // fixed names GPUI's own `MouseButton` maps onto — so the op stays
            // the `(&'static str, CallbackId)` pair every other callback uses.
            "on_mouse_down" | "on_mouse_up" => {
                if scope::current_phase() == Some(ScopePhase::Layout) {
                    return Err(Exception::throw_type(
                        ctx,
                        &format!(
                            "`{method}` cannot be registered from a virtual list's item \
                             renderer: the rows are rebuilt every frame, so a handler \
                             registered there would pile up for as long as the view stood"
                        ),
                    ));
                }
                let button = args
                    .first_value()
                    .and_then(|value| value.as_str().ok().map(str::to_owned))
                    .ok_or_else(|| {
                        Exception::throw_type(
                            ctx,
                            &format!(
                                "{method}(button, handler) expects a button first: \
                                 \"left\", \"right\" or \"middle\""
                            ),
                        )
                    })?;
                let saved = args.handler_at(1).ok_or_else(|| {
                    Exception::throw_type(
                        ctx,
                        &format!("{method}(button, handler) expects a function second"),
                    )
                })?;
                let name = match (method, button.as_str()) {
                    ("on_mouse_down", "left") => "on_mouse_down_left",
                    ("on_mouse_down", "right") => "on_mouse_down_right",
                    ("on_mouse_down", "middle") => "on_mouse_down_middle",
                    ("on_mouse_up", "left") => "on_mouse_up_left",
                    ("on_mouse_up", "right") => "on_mouse_up_right",
                    ("on_mouse_up", "middle") => "on_mouse_up_middle",
                    (_, other) => {
                        return Err(Exception::throw_type(
                            ctx,
                            &format!(
                                "`{other}` is not a mouse button; \
                                 expected \"left\", \"right\" or \"middle\""
                            ),
                        ));
                    }
                };
                let callback = self.callbacks.borrow_mut().push(CallbackEntry {
                    value: saved,
                    view: scope::current_view().map(|view| view.downgrade()),
                    application: scope::current_application_generation(),
                    registered_in: scope::current_generation(),
                });
                self.push_op_checked(ctx, self.push_op(id, SpecOp::Callback(name, callback)))
            }
            "on_click"
            | "on_link_click"
            | "on_resize"
            | "on_change"
            | "on_open_change"
            | "on_confirm"
            | "on_dismiss"
            | "on_step"
            | "on_item_click"
            | "on_item_secondary_click"
            | "on_mouse_move"
            | "on_hover"
            | "on_key_down"
            | "on_key_up"
            | "on_modifiers_changed"
            | "on_mouse_down_out"
            | "on_scroll_wheel"
            | "tab_bar"
            | "empty_group"
            | "drop_indicator"
            | "dock"
            | "tile_drag_bar"
            | "tile_resize_handles" => {
                // A handler registered from inside a virtual list's item
                // renderer has nowhere to live. Callbacks belong to the
                // snapshot that registered them and are retired with it; the
                // snapshot outlives thousands of frames, while the rows are
                // rebuilt on every one — so twenty handlers a frame would
                // accumulate, unreachable and unreleased, for as long as the
                // description stood. Refused where it was written rather than
                // leaked quietly. `on_item_click` on the list is the one
                // handler that covers the rows, and it is registered from
                // `render()` like every other.
                if scope::current_phase() == Some(ScopePhase::Layout) {
                    return Err(Exception::throw_type(
                        ctx,
                        &format!(
                            "`{method}` cannot be registered from a virtual list's item \
                             renderer: the rows are rebuilt every frame, so a handler \
                             registered there would pile up for as long as the view stood. \
                             Use `on_item_click((key, cx) => ...)` or \
                             `on_item_secondary_click((key, event, cx) => ...)` on the list \
                             itself, and read the row out of your own data with the stable key \
                             it gives you"
                        ),
                    ));
                }
                let saved = args.first_handler().ok_or_else(|| {
                    Exception::throw_type(ctx, &format!("{method}(handler) expects a function"))
                })?;
                let callback = self.callbacks.borrow_mut().push(CallbackEntry {
                    value: saved,
                    view: scope::current_view().map(|view| view.downgrade()),
                    application: scope::current_application_generation(),
                    registered_in: scope::current_generation(),
                });
                let name = callback_op_name(method).expect("this arm's own list");
                self.push_op_checked(ctx, self.push_op(id, SpecOp::Callback(name, callback)))
            }
            "disabled"
            | "selectable"
            | "scrollable"
            | "selected"
            | "checked"
            | "accessibility_label"
            | "tooltip"
            | "role"
            | "aria_selected"
            | "aria_active_descendant"
            | "track_focus"
            | "track_scroll"
            | "with_item_to_measure_index"
            | "content_focus_handle"
            | "key_context"
            | "aria_level"
            | "keep_mounted"
            | "tab_index"
            | "tab_stop"
            | "href"
            | "id"
            | "overflow_scroll"
            | "overflow_x_scroll"
            | "overflow_y_scroll"
            | "overflow_scrollbar"
            | "overflow_x_scrollbar"
            | "overflow_y_scrollbar"
            | "mode"
            | "scroll_size"
            | "viewport_from_layout"
            | "controls_right"
            | "panel_visible"
            | "panel_size"
            | "size_range"
            | "set_position"
            | "pressed"
            | "start"
            | "value"
            | "indeterminate"
            | "axis"
            | "row_count"
            | "column_count"
            | "open"
            | "default_open"
            | "overlay_closable"
            | "anchor"
            | "frame_budget"
            | "mouse_button"
            | "open_delay"
            | "close_delay"
            | "transition"
            | "spring" => {
                let bridged = args.values(method)?;
                let name = match method {
                    "disabled" => "disabled",
                    "selectable" => "selectable",
                    "scrollable" => "scrollable",
                    "selected" => "selected",
                    "checked" => "checked",
                    "tooltip" => "tooltip",
                    "role" => "role",
                    "aria_selected" => "aria_selected",
                    "aria_active_descendant" => "aria_active_descendant",
                    "track_focus" => "track_focus",
                    "track_scroll" => "track_scroll",
                    "with_item_to_measure_index" => "with_item_to_measure_index",
                    "content_focus_handle" => "content_focus_handle",
                    "key_context" => "key_context",
                    "aria_level" => "aria_level",
                    "keep_mounted" => "keep_mounted",
                    "tab_index" => "tab_index",
                    "tab_stop" => "tab_stop",
                    "id" => "id",
                    "overflow_scroll" => "overflow_scroll",
                    "overflow_x_scroll" => "overflow_x_scroll",
                    "overflow_y_scroll" => "overflow_y_scroll",
                    "overflow_scrollbar" => "overflow_scrollbar",
                    "overflow_x_scrollbar" => "overflow_x_scrollbar",
                    "overflow_y_scrollbar" => "overflow_y_scrollbar",
                    "mode" => "mode",
                    "scroll_size" => "scroll_size",
                    "viewport_from_layout" => "viewport_from_layout",
                    "controls_right" => "controls_right",
                    "panel_visible" => "panel_visible",
                    "panel_size" => "panel_size",
                    "size_range" => "size_range",
                    "set_position" => "set_position",
                    "pressed" => "pressed",
                    "start" => "start",
                    "value" => "value",
                    "indeterminate" => "indeterminate",
                    "axis" => "axis",
                    "row_count" => "row_count",
                    "column_count" => "column_count",
                    "open" => "open",
                    "default_open" => "default_open",
                    "overlay_closable" => "overlay_closable",
                    "anchor" => "anchor",
                    "frame_budget" => "frame_budget",
                    "mouse_button" => "mouse_button",
                    "open_delay" => "open_delay",
                    "close_delay" => "close_delay",
                    "transition" => "transition",
                    "spring" => "spring",
                    "href" => "href",
                    _ => "accessibility_label",
                };
                if name == "id"
                    && bridged
                        .first()
                        .and_then(|value| value.as_str().ok())
                        .is_none()
                {
                    return Err(Exception::throw_type(
                        ctx,
                        "id(name) expects a string; it is the element's stable identity, so it \
                         must not change between renders",
                    ));
                }
                // Dropping a non-string here would leave an element that
                // looks tooltipped and shows nothing on hover. It is also the
                // one place to say that the element form is not bound yet.
                if name == "tooltip"
                    && bridged
                        .first()
                        .and_then(|value| value.as_str().ok())
                        .is_none()
                {
                    return Err(Exception::throw_type(
                        ctx,
                        "tooltip(text) expects a string; a tooltip built from an element is not \
                         bound yet",
                    ));
                }
                // A bar that silently sits at zero because the percentage
                // arrived as a string is the kind of bug that gets blamed on
                // the layout. Say it at the call site instead.
                if name == "value" && bridged.first().and_then(finite_number).is_none() {
                    return Err(Exception::throw_type(
                        ctx,
                        "value(percent) expects a number between 0 and 100",
                    ));
                }
                if name == "set_position" {
                    let position = bridged.first().and_then(finite_whole_number);
                    let size = bridged.get(1).and_then(finite_whole_number);
                    if !matches!((position, size), (Some(position), Some(size)) if position >= 1.0 && size >= position && size <= usize::MAX as f32)
                    {
                        return Err(Exception::throw_type(
                            ctx,
                            "set_position(position, size) expects whole finite numbers with 1 <= position <= size",
                        ));
                    }
                }
                if name == "size_range" {
                    let Some(min) = bridged.first().and_then(finite_number) else {
                        return Err(Exception::throw_range(
                            ctx,
                            "size_range minimum does not fit the native pixel range",
                        ));
                    };
                    let max = match bridged.get(1) {
                        Some(value) => Some(finite_number(value).ok_or_else(|| {
                            Exception::throw_range(
                                ctx,
                                "size_range maximum does not fit the native pixel range",
                            )
                        })?),
                        None => None,
                    };
                    if max.is_some_and(|max| max < min) {
                        return Err(Exception::throw_range(
                            ctx,
                            "size_range maximum must be greater than or equal to its minimum",
                        ));
                    }
                }
                if matches!(name, "row_count" | "column_count")
                    && !bridged
                        .first()
                        .and_then(finite_whole_number)
                        .is_some_and(|count| count >= 0.0 && count <= usize::MAX as f32)
                {
                    return Err(Exception::throw_type(
                        ctx,
                        &format!("{name}(count) expects a non-negative whole finite number"),
                    ));
                }
                // An unknown role is silence in the accessibility tree, and
                // silence is exactly what `role` was called to prevent. The
                // one filtered variant is named separately, because "unknown
                // role" is not the answer to a script that asked for it.
                if name == "role" {
                    let Some(named) = bridged.first().and_then(|value| value.as_str().ok()) else {
                        return Err(Exception::throw_type(
                            ctx,
                            "role(name) expects a string; see the Role type in gpui-kit.d.ts",
                        ));
                    };
                    if named == crate::a11y::FILTERED_ROLE {
                        return Err(Exception::throw_type(
                            ctx,
                            "role(\"generic_container\") announces nothing: GPUI filters that \
                             role out of the accessibility tree. Leave the role off instead, \
                             or name the role the element really has",
                        ));
                    }
                    if crate::a11y::role_from_name(named).is_none() {
                        return Err(Exception::throw_type(
                            ctx,
                            &format!(
                                "unknown accessibility role `{named}`; the names mirror \
                                 gpui::Role in snake_case — see the Role type in gpui-kit.d.ts"
                            ),
                        ));
                    }
                }
                // A tab index of 1.5 is not a position in the tab order; it is
                // a number the script computed wrongly, and rounding it here
                // would put the control somewhere nobody chose.
                if name == "tab_index"
                    && !bridged
                        .first()
                        .and_then(|value| value.as_f32().ok())
                        .is_some_and(|index| index.fract() == 0.0)
                {
                    return Err(Exception::throw_type(
                        ctx,
                        "tab_index(index) expects a whole number",
                    ));
                }
                if name == "href" {
                    let Some(target) = bridged.first().and_then(|value| value.as_str().ok()) else {
                        return Err(Exception::throw_type(ctx, "href(url) expects a string"));
                    };
                    let valid = reqwest::Url::parse(target).is_ok_and(|url| {
                        matches!(url.scheme(), "http" | "https") && url.host_str().is_some()
                    });
                    if !valid {
                        return Err(Exception::throw_type(
                            ctx,
                            "href(url) expects an absolute HTTP(S) URL with a host",
                        ));
                    }
                }
                self.push_op_checked(ctx, self.push_op(id, SpecOp::Method(name, bridged)))
            }
            // A dock command carries no script value: it names a container in
            // the area and what to ask it. That is why a tab can report its
            // click at all — a chrome handler runs once per frame for as long
            // as the dock is on screen, so a callback registered inside one
            // would pile up the way a virtual list's row handlers would.
            "select_tab" | "close_panel" | "toggle_zoom" | "drag_tab" | "drop_tab"
            | "toggle_dock" | "resize_dock" | "move_tile" | "resize_tile" | "raise_tile"
            | "toggle_tile_zoom" | "close_tile" => {
                let bridged = args.values(method)?;
                let name = match method {
                    "select_tab" => "select_tab",
                    "close_panel" => "close_panel",
                    "toggle_zoom" => "toggle_zoom",
                    "drag_tab" => "drag_tab",
                    "drop_tab" => "drop_tab",
                    "toggle_dock" => "toggle_dock",
                    "resize_dock" => "resize_dock",
                    "move_tile" => "move_tile",
                    "resize_tile" => "resize_tile",
                    "raise_tile" => "raise_tile",
                    "toggle_tile_zoom" => "toggle_tile_zoom",
                    _ => "close_tile",
                };
                if !bridged
                    .first()
                    .and_then(|value| value.as_f32().ok())
                    .is_some_and(|handle| handle >= 0.0)
                {
                    return Err(Exception::throw_type(
                        ctx,
                        &format!(
                            "{name}(...) expects the group, dock or tile its chrome handler was                              given as its first argument"
                        ),
                    ));
                }
                self.push_op_checked(ctx, self.push_op(id, SpecOp::Method(name, bridged)))
            }
            "move_to" | "line_to" | "curve_to" | "cubic_bezier_to" | "arc_to" | "close"
            | "dash_array" => {
                let bridged = args.values(method)?;
                let name = match method {
                    "move_to" => "move_to",
                    "line_to" => "line_to",
                    "curve_to" => "curve_to",
                    "cubic_bezier_to" => "cubic_bezier_to",
                    "arc_to" => "arc_to",
                    "close" => "close",
                    _ => "dash_array",
                };
                self.push_op_checked(ctx, self.push_op(id, SpecOp::Method(name, bridged)))
            }
            _ => {
                if let Some(index) = style::nullary_index(method) {
                    return self
                        .push_op_checked(ctx, self.push_op(id, SpecOp::NullaryStyle(index)));
                }
                if let Some(name) = style::param_style_name(method) {
                    let bridged = args.values(name)?;
                    // Validate eagerly so a bad argument reports at the call
                    // site instead of surfacing during materialize.
                    style::apply_param(name, &bridged, Default::default())
                        .map_err(|error| Exception::throw_type(ctx, error.message()))?;
                    return self
                        .push_op_checked(ctx, self.push_op(id, SpecOp::ParamStyle(name, bridged)));
                }
                Err(Exception::throw_type(ctx, &unknown_method(method)))
            }
        }
    }

    fn registered_component_id(&self, id: SpecId) -> Option<crate::ComponentId> {
        let component_id = {
            let arena = self.arena.borrow();
            let Component::Registered(component) = arena.node(id)?.component()? else {
                return None;
            };
            component.id()
        };
        Some(component_id)
    }

    fn warn_deprecated_export(
        &self,
        export: &'static str,
        replacement: &'static str,
        message: &'static str,
    ) {
        if self.warned_deprecated_exports.borrow_mut().insert(export) {
            tracing::warn!(
                "JavaScript component export `{export}` is deprecated; use `{replacement}`. {message}"
            );
        }
    }

    #[cfg(test)]
    pub(crate) fn deprecation_warning_count(&self, export: &str) -> usize {
        usize::from(self.warned_deprecated_exports.borrow().contains(export))
    }

    fn is_registered_component(&self, id: SpecId) -> bool {
        self.registered_component_id(id).is_some()
    }

    fn registered_method_descriptor(
        &self,
        id: SpecId,
        method: &str,
    ) -> Option<crate::MethodDescriptor> {
        let component_id = self.registered_component_id(id)?;
        self.components
            .descriptor(component_id)?
            .methods()
            .iter()
            .find(|descriptor| descriptor.name() == method)
            .cloned()
    }

    fn validate_component_arguments(
        &self,
        ctx: &Ctx<'_>,
        api: &str,
        descriptors: &[ArgumentDescriptor],
        arguments: &Arguments,
    ) -> JsResult<Vec<ComponentArgument>> {
        if arguments.0.len() > descriptors.len() {
            return Err(Exception::throw_type(
                ctx,
                &format!(
                    "{api}(...) expects at most {} argument{}",
                    descriptors.len(),
                    if descriptors.len() == 1 { "" } else { "s" }
                ),
            ));
        }
        descriptors
            .iter()
            .enumerate()
            .map(|(index, descriptor)| {
                let argument = arguments.0.get(index);
                self.validate_component_argument(ctx, api, descriptor, argument)
            })
            .collect()
    }

    fn component_payload_transaction(
        &self,
        ctx: &Ctx<'_>,
        api: &str,
        descriptors: &[ArgumentDescriptor],
        arguments: &Arguments,
        factory: impl FnOnce(&[ComponentArgument]) -> Result<ComponentPayload, String>,
    ) -> JsResult<ComponentPayload> {
        self.flush_component_state_releases();
        let callback_checkpoint = self.callbacks.borrow().checkpoint();
        let result = (|| {
            let arguments = self.validate_component_arguments(ctx, api, descriptors, arguments)?;
            let mut element_ids = Vec::new();
            for argument in &arguments {
                collect_component_elements(argument, &mut element_ids);
            }
            let mut unique = HashSet::with_capacity(element_ids.len());
            if let Some(duplicate) = element_ids
                .iter()
                .copied()
                .find(|element| !unique.insert(*element))
            {
                return Err(Exception::throw_type(
                    ctx,
                    &format!("{api}(...) cannot consume element {duplicate} twice"),
                ));
            }
            let payload =
                factory(&arguments).map_err(|error| Exception::throw_type(ctx, &error))?;
            for element in element_ids {
                self.arena
                    .borrow_mut()
                    .claim(element)
                    .map_err(|error| Exception::throw_type(ctx, &error.to_string()))?;
            }
            Ok(payload)
        })();
        if result.is_err() {
            self.callbacks.borrow_mut().rollback_to(callback_checkpoint);
        }
        self.flush_component_state_releases();
        result
    }

    fn retained_state_transaction(
        &self,
        ctx: &Ctx<'_>,
        descriptor: &crate::StateDescriptor,
        arguments: &Arguments,
    ) -> JsResult<u64> {
        self.flush_component_state_releases();
        let callback_checkpoint = self.callbacks.borrow().checkpoint();
        let result = (|| {
            let arguments = self.validate_component_arguments(
                ctx,
                descriptor.export(),
                descriptor.arguments(),
                arguments,
            )?;
            let state = scope::with_current(|window, cx| {
                descriptor.create(&arguments, window, cx)
            })
            .ok_or_else(|| {
                Exception::throw_type(
                    ctx,
                    "retained component state can only be created during a live Window/App call",
                )
            })?
            .map_err(|error| Exception::throw_type(ctx, &error))?;
            let owner = scope::current_application_generation();
            self.component_states
                .try_borrow_mut()
                .map_err(|_| {
                    Exception::throw_type(ctx, "retained component state is already borrowed")
                })?
                .insert(descriptor.kind(), owner, state)
                .map_err(|error| Exception::throw_range(ctx, &error))
        })();
        if result.is_err() {
            self.callbacks.borrow_mut().rollback_to(callback_checkpoint);
        }
        self.flush_component_state_releases();
        result
    }

    fn validate_component_argument(
        &self,
        ctx: &Ctx<'_>,
        api: &str,
        descriptor: &ArgumentDescriptor,
        argument: Option<&Argument>,
    ) -> JsResult<ComponentArgument> {
        if let ArgumentSchema::Optional(inner) = &descriptor.schema() {
            return match argument {
                None | Some(Argument::Value(Bridged::Nil)) => Ok(ComponentArgument::Optional(None)),
                Some(argument) => self
                    .validate_component_argument_kind(ctx, api, descriptor.name(), inner, argument)
                    .map(|argument| ComponentArgument::Optional(Some(Box::new(argument)))),
            };
        }
        let argument = argument.ok_or_else(|| {
            Exception::throw_type(
                ctx,
                &format!(
                    "{api}({}) expects {}",
                    descriptor.name(),
                    schema_name(&descriptor.schema())
                ),
            )
        })?;
        self.validate_component_argument_kind(
            ctx,
            api,
            descriptor.name(),
            &descriptor.schema(),
            argument,
        )
    }

    fn validate_component_argument_kind(
        &self,
        ctx: &Ctx<'_>,
        api: &str,
        name: &str,
        schema: &ArgumentSchema,
        argument: &Argument,
    ) -> JsResult<ComponentArgument> {
        let validated = match (schema, argument) {
            (ArgumentSchema::String, Argument::Value(Bridged::Str(value))) => {
                Some(ComponentArgument::String(value.clone()))
            }
            (ArgumentSchema::Number, Argument::Value(Bridged::Number(value)))
                if value.is_finite() =>
            {
                Some(ComponentArgument::Number(*value))
            }
            (ArgumentSchema::Boolean, Argument::Value(Bridged::Bool(value))) => {
                Some(ComponentArgument::Boolean(*value))
            }
            (ArgumentSchema::Element, Argument::Element(id)) => {
                self.arena
                    .borrow()
                    .ensure_claimable(*id)
                    .map_err(|error| Exception::throw_type(ctx, &error.to_string()))?;
                Some(ComponentArgument::Element(*id))
            }
            (ArgumentSchema::Entity(kind), Argument::Entity(handle))
                if self.entities.borrow().kind(*handle) == Some(*kind) =>
            {
                Some(ComponentArgument::Entity {
                    kind,
                    handle: *handle,
                })
            }
            (ArgumentSchema::Entity(kind), Argument::RetainedState { handle, proof }) => {
                let matches = if proof != &self.component_state_proof {
                    false
                } else {
                    self.component_states
                        .try_borrow()
                        .map_err(|_| {
                            Exception::throw_type(
                                ctx,
                                "retained component state is already mutably borrowed",
                            )
                        })?
                        .kind(*handle)
                        == Some(*kind)
                };
                matches.then_some(ComponentArgument::Entity {
                    kind,
                    handle: *handle,
                })
            }
            (ArgumentSchema::Callback(_), Argument::Handler(handler)) => {
                let callback = self.callbacks.borrow_mut().push(CallbackEntry {
                    value: handler.clone(),
                    view: scope::current_view().map(|view| view.downgrade()),
                    application: scope::current_application_generation(),
                    registered_in: scope::current_generation(),
                });
                Some(ComponentArgument::Callback(callback))
            }
            (ArgumentSchema::Enum(values), Argument::Value(Bridged::Str(value)))
                if values.contains(&value.as_str()) =>
            {
                Some(ComponentArgument::Enum(value.clone()))
            }
            (ArgumentSchema::Array(item), Argument::Array(values)) => {
                let mut validated = Vec::with_capacity(values.len());
                for value in values {
                    validated
                        .push(self.validate_component_argument_kind(ctx, api, name, item, value)?);
                }
                Some(ComponentArgument::Array(validated))
            }
            (ArgumentSchema::Optional(inner), value) => Some(ComponentArgument::Optional(Some(
                Box::new(self.validate_component_argument_kind(ctx, api, name, inner, value)?),
            ))),
            _ => None,
        };
        validated.ok_or_else(|| {
            Exception::throw_type(
                ctx,
                &format!("{api}({name}) expects {}", schema_name(schema)),
            )
        })
    }

    fn push_op_checked<E: std::fmt::Display>(
        &self,
        ctx: &Ctx<'_>,
        result: Result<(), E>,
    ) -> JsResult<()> {
        result.map_err(|error| Exception::throw_type(ctx, &error.to_string()))
    }
}

fn finite_number(value: &Bridged) -> Option<f32> {
    value.as_f32().ok().filter(|number| number.is_finite())
}

fn finite_whole_number(value: &Bridged) -> Option<f32> {
    finite_number(value).filter(|number| number.fract() == 0.0)
}

fn constructor(
    globals: &Object<'_>,
    name: &str,
    runtime: Weak<ShellRuntime>,
    build: fn() -> Component,
) -> JsResult<()> {
    globals.set(
        name,
        Func::from(move |ctx: Ctx<'_>| -> JsResult<SpecId> {
            Ok(upgrade(&runtime, &ctx)?.push_node(build()))
        }),
    )
}

fn text_constructor(
    globals: &Object<'_>,
    name: &str,
    runtime: Weak<ShellRuntime>,
    build: fn(String) -> Component,
) -> JsResult<()> {
    globals.set(
        name,
        Func::from(move |ctx: Ctx<'_>, value: String| -> JsResult<SpecId> {
            Ok(upgrade(&runtime, &ctx)?.push_node(build(value)))
        }),
    )
}

/// A constructor whose second argument is a one-based accessibility index.
///
/// `TableRow::new(id, row_index)` and the two cell types take their index in
/// the constructor rather than through a builder, because a cell that does not
/// know its column is not merely unstyled — it announces itself in the wrong
/// place. The script side refuses anything that is not a whole number of at
/// least one, so the cast here cannot quietly floor a fraction into a
/// plausible-looking index.
fn indexed_constructor(
    globals: &Object<'_>,
    name: &str,
    runtime: Weak<ShellRuntime>,
    build: fn(String, usize) -> Component,
) -> JsResult<()> {
    globals.set(
        name,
        Func::from(
            move |ctx: Ctx<'_>, value: String, index: usize| -> JsResult<SpecId> {
                Ok(upgrade(&runtime, &ctx)?.push_node(build(value, index)))
            },
        ),
    )
}

/// How far apart a virtualized list's items are: `number | number[]`.
///
/// Two forms rather than base's one vector, because the length of that vector
/// is also the item count — so mirroring it literally would put one number per
/// row across the language boundary on every script render. A hundred thousand
/// rows of a fixed height is one number here.
enum ItemExtents {
    Uniform(f64),
    PerItem(Vec<f64>),
}

impl<'js> FromJs<'js> for ItemExtents {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        if let Some(uniform) = value.as_number() {
            return Ok(Self::Uniform(uniform));
        }
        let items = value.as_array().ok_or_else(|| {
            Exception::throw_type(ctx, "item_sizes must be a number or an array of numbers")
        })?;
        let length = host_modules::bridge_array_len(ctx, &items).map_err(|_| {
            Exception::throw_range(
                ctx,
                &format!(
                    "item_sizes may contain at most {} entries",
                    host_modules::MAX_BRIDGE_ARRAY_ITEMS
                ),
            )
        })?;
        let mut extents = Vec::new();
        extents.try_reserve_exact(length).map_err(|_| {
            Exception::throw_range(ctx, "the item_sizes array could not be allocated")
        })?;
        for extent in items.iter::<f64>() {
            extents.push(extent?);
        }
        Ok(Self::PerItem(extents))
    }
}

/// A script function taken as a constructor argument, saved on the way in.
///
/// Persisted inside `FromJs` for the reason [`Arguments`] gives: a closure
/// cannot unify the `Ctx<'js>` it takes with a borrowed value of the same
/// lifetime, so the crossing happens where both are still one lifetime.
struct ItemRenderer(Persistent<Function<'static>>);

impl<'js> FromJs<'js> for ItemRenderer {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let function = value.as_function().ok_or_else(|| {
            Exception::throw_type(
                ctx,
                "a virtual list needs a render function; it is called with the visible range,                  not once per item",
            )
        })?;
        Ok(Self(Persistent::save(ctx, function.clone())))
    }
}

/// A script function resolving one stable string key from a current index.
struct ItemKeyResolver(Persistent<Function<'static>>);

impl<'js> FromJs<'js> for ItemKeyResolver {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let function = value.as_function().ok_or_else(|| {
            Exception::throw_type(
                ctx,
                "a virtual list needs get_key(index) to return each item's stable string key",
            )
        })?;
        Ok(Self(Persistent::save(ctx, function.clone())))
    }
}

/// The aggregate item count whose native extent vectors one render may own.
///
/// Base wants one `Size` per item and the vector is built here, so a count the
/// script fat-fingered — a byte offset, a timestamp — would be an allocation
/// measured in gigabytes before anything else had a chance to notice. This is
/// shared across every list in the description so several individually valid
/// lists cannot bypass it.
const MAX_VIRTUAL_ITEMS_PER_RENDER: usize = 1_000_000;

/// The guard both lazy-list constructors run before they allocate anything.
///
/// The phase check is why an item renderer cannot build a list: callbacks
/// belong to the snapshot that registered them, and by the time a renderer
/// runs that generation is closed, so a callback pushed there is one no lookup
/// could ever match. The budget claim has to come before the size table,
/// because a count the script fat-fingered is an allocation measured in
/// gigabytes.
fn guard_lazy_list(
    ctx: &Ctx<'_>,
    runtime: &Weak<ShellRuntime>,
    count: usize,
) -> JsResult<Rc<ShellRuntime>> {
    if scope::current_phase() == Some(ScopePhase::Layout) {
        return Err(Exception::throw_type(
            ctx,
            "a list cannot be built from inside another list's item renderer: its own \
             renderer would belong to no render pass and would never be called. Describe \
             the nested list from the view's render() instead",
        ));
    }
    let store = upgrade(runtime, ctx)?;
    if !store
        .arena
        .borrow_mut()
        .claim_virtual_items(count, MAX_VIRTUAL_ITEMS_PER_RENDER)
    {
        return Err(Exception::throw_type(
            ctx,
            &format!(
                "the lists in one render may describe at most \
                 {MAX_VIRTUAL_ITEMS_PER_RENDER} items in total"
            ),
        ));
    }
    Ok(store)
}

/// Files a lazy list's two script functions against the open generation.
fn register_item_callbacks(
    store: &Rc<ShellRuntime>,
    get_key: ItemKeyResolver,
    render: ItemRenderer,
) -> (CallbackId, CallbackId) {
    let entry = |value| {
        store.callbacks.borrow_mut().push(CallbackEntry {
            value,
            view: scope::current_view().map(|view| view.downgrade()),
            application: scope::current_application_generation(),
            registered_in: scope::current_generation(),
        })
    };
    (entry(get_key.0), entry(render.0))
}

/// `v_virtual_list` and `h_virtual_list`.
///
/// The item renderer is registered as an ordinary callback, so it belongs to
/// the snapshot being built and is retired with it. That is also why it cannot
/// be registered from inside another item renderer: by then the generation that
/// would own it has been committed, and a callback pushed with no open
/// generation is one no lookup can ever match.
fn virtual_list_constructor(
    globals: &Object<'_>,
    name: &str,
    runtime: Weak<ShellRuntime>,
    axis: gpui::Axis,
) -> JsResult<()> {
    globals.set(
        name,
        Func::from(
            move |ctx: Ctx<'_>,
                  id: String,
                  count: usize,
                  extents: ItemExtents,
                  get_key: ItemKeyResolver,
                  render: ItemRenderer|
                  -> JsResult<SpecId> {
                let store = guard_lazy_list(&ctx, &runtime, count)?;

                let extent = |value: f64| -> JsResult<gpui::Size<gpui::Pixels>> {
                    if !value.is_finite() || value < 0.0 {
                        return Err(Exception::throw_type(
                            &ctx,
                            "every item size must be a finite, non-negative number of pixels",
                        ));
                    }
                    // Only the extent along the list's own axis is read; the
                    // other is inferred by measuring one item. Writing zero
                    // there says so, rather than inventing a number base would
                    // ignore.
                    Ok(match axis {
                        gpui::Axis::Vertical => gpui::size(gpui::px(0.), gpui::px(value as f32)),
                        gpui::Axis::Horizontal => gpui::size(gpui::px(value as f32), gpui::px(0.)),
                    })
                };

                let reserve = |values: &mut Vec<gpui::Size<gpui::Pixels>>| {
                    values.try_reserve_exact(count).map_err(|_| {
                        Exception::throw_range(
                            &ctx,
                            "the virtual list's native size table could not be allocated",
                        )
                    })
                };
                let sizes = match extents {
                    ItemExtents::Uniform(value) => {
                        let value = extent(value)?;
                        let mut values = Vec::new();
                        reserve(&mut values)?;
                        values.resize(count, value);
                        values
                    }
                    ItemExtents::PerItem(values) => {
                        if values.len() != count {
                            return Err(Exception::throw_type(
                                &ctx,
                                &format!(
                                    "this list was given {} item sizes for {count} items; pass                                      one number for a uniform extent, or one per item",
                                    values.len()
                                ),
                            ));
                        }
                        let mut extents = Vec::new();
                        reserve(&mut extents)?;
                        for value in values {
                            extents.push(extent(value)?);
                        }
                        extents
                    }
                };

                let (get_key, callback) = register_item_callbacks(&store, get_key, render);
                Ok(store.push_node(Component::VirtualList(Rc::new(
                    crate::spec::VirtualListSpec::new(
                        id,
                        axis,
                        Rc::new(sizes),
                        get_key,
                        callback,
                    ),
                ))))
            },
        ),
    )
}

/// `list` and `uniform_list`.
///
/// The same registration as a virtual list's, and confined for the same
/// reasons: the renderer belongs to the snapshot being built, and cannot be
/// registered from inside another list's item renderer. The item budget is
/// claimed too, because `gpui::list` keeps one entry per item whether or not
/// the item is ever drawn.
fn list_constructor(
    globals: &Object<'_>,
    name: &str,
    runtime: Weak<ShellRuntime>,
    kind: crate::spec::ListKind,
) -> JsResult<()> {
    globals.set(
        name,
        Func::from(
            move |ctx: Ctx<'_>,
                  id: String,
                  count: usize,
                  get_key: ItemKeyResolver,
                  render: ItemRenderer|
                  -> JsResult<SpecId> {
                let store = guard_lazy_list(&ctx, &runtime, count)?;

                let (get_key, callback) = register_item_callbacks(&store, get_key, render);
                Ok(
                    store.push_node(Component::List(Rc::new(crate::spec::ListSpec::new(
                        id, kind, count, get_key, callback,
                    )))),
                )
            },
        ),
    )
}

/// The spec a `render` returned.
///
/// A retained child view counts, the way an `Entity<V>` is itself renderable in
/// GPUI: a view whose whole job is to hold one child should be able to say so
/// by returning it, rather than wrapping it in a container it does not want.
fn element_id(ctx: &Ctx<'_>, value: &Value<'_>) -> JsResult<SpecId> {
    // A string is an element, so a view whose whole output is a word may say so
    // — `render` returns `impl IntoElement` in GPUI, and `&str` implements it.
    if value.as_string().is_some() {
        let make: rquickjs::Function = ctx.globals().get("__text")?;
        return make.call((value.get::<String>()?,));
    }
    let Some(object) = value.as_object() else {
        return Err(Exception::throw_type(
            ctx,
            "render(cx) must return an element, an Entity, or a string",
        ));
    };
    if let Ok(id) = object.get::<_, u32>("__id") {
        return Ok(id as SpecId);
    }
    if object.get::<_, bool>("__entity").unwrap_or(false) {
        let child_view: rquickjs::Function = ctx.globals().get("__child_view")?;
        let handle: u32 = object.get("__handle")?;
        return child_view.call((handle,));
    }
    value
        .as_object()
        .and_then(|object| object.get::<_, u32>("__id").ok())
        .ok_or_else(|| {
            Exception::throw_type(
                ctx,
                "render(cx) must return an element, an Entity, or a string",
            )
        })
}

fn push_component_callback_arguments<'js>(
    ctx: &Ctx<'js>,
    arguments: &mut JsArgs<'js>,
    values: &[ComponentCallbackArgument],
) -> JsResult<()> {
    for value in values {
        arguments.push_arg(callback_argument_to_js(ctx, value)?)?;
    }
    Ok(())
}

const MAX_COMPONENT_DATA_DEPTH: usize = 16;
const MAX_COMPONENT_DATA_NODES: usize = 4096;
const MAX_COMPONENT_DATA_STRING_BYTES: usize = 1024 * 1024;
const MAX_COMPONENT_DATA_OBJECT_KEYS: usize = 1024;

#[derive(Default)]
struct ComponentDataBudget {
    nodes: usize,
    string_bytes: usize,
    keys: usize,
}

struct TemporarySpecArena {
    runtime: Rc<ShellRuntime>,
    outer: Option<SpecArena>,
}

impl TemporarySpecArena {
    fn enter(runtime: &Rc<ShellRuntime>) -> Self {
        let outer = std::mem::take(&mut *runtime.arena.borrow_mut());
        Self {
            runtime: runtime.clone(),
            outer: Some(outer),
        }
    }

    fn finish(mut self) -> SpecArena {
        let generated = std::mem::replace(
            &mut *self.runtime.arena.borrow_mut(),
            self.outer.take().expect("temporary arena outer"),
        );
        generated
    }
}

impl Drop for TemporarySpecArena {
    fn drop(&mut self) {
        if let Some(outer) = self.outer.take() {
            *self.runtime.arena.borrow_mut() = outer;
        }
    }
}

fn component_data_from_js<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
    depth: usize,
    budget: &mut ComponentDataBudget,
) -> JsResult<ComponentDataValue> {
    if depth > MAX_COMPONENT_DATA_DEPTH {
        return Err(Exception::throw_range(
            ctx,
            "component delegate data is nested too deeply",
        ));
    }
    budget.nodes = budget.nodes.checked_add(1).ok_or_else(|| {
        Exception::throw_range(ctx, "component delegate data contains too many values")
    })?;
    if budget.nodes > MAX_COMPONENT_DATA_NODES {
        return Err(Exception::throw_range(
            ctx,
            "component delegate data contains too many values",
        ));
    }
    if value.is_null() || value.is_undefined() {
        return Ok(ComponentDataValue::Null);
    }
    if let Some(v) = value.as_bool() {
        return Ok(ComponentDataValue::Boolean(v));
    }
    if let Some(v) = value.as_number() {
        if !v.is_finite() {
            return Err(Exception::throw_type(
                ctx,
                "component delegate numbers must be finite",
            ));
        }
        return Ok(ComponentDataValue::Number(v));
    }
    if let Some(v) = value.as_string() {
        let v = v.to_string()?;
        budget.string_bytes = budget.string_bytes.saturating_add(v.len());
        if budget.string_bytes > MAX_COMPONENT_DATA_STRING_BYTES {
            return Err(Exception::throw_range(
                ctx,
                "component delegate strings exceed the byte limit",
            ));
        }
        return Ok(ComponentDataValue::String(v));
    }
    if value.as_function().is_some() {
        return Err(Exception::throw_type(
            ctx,
            "component delegate data cannot contain functions",
        ));
    }
    if value.is_promise() {
        return Err(Exception::throw_type(
            ctx,
            "component delegate data cannot contain promises",
        ));
    }
    if let Some(array) = value.as_array() {
        let len = host_modules::bridge_array_len(ctx, &array)?;
        let mut out = Vec::new();
        out.try_reserve_exact(len)
            .map_err(|_| Exception::throw_range(ctx, "component delegate array is too large"))?;
        for ix in 0..len {
            out.push(component_data_from_js(
                ctx,
                array.get(ix)?,
                depth + 1,
                budget,
            )?);
        }
        return Ok(ComponentDataValue::Array(out));
    }
    if let Some(object) = value.as_object() {
        let object_constructor: Object = ctx.globals().get("Object")?;
        let get_prototype: Function = object_constructor.get("getPrototypeOf")?;
        let prototype: Value = get_prototype.call((object.clone(),))?;
        let ordinary_prototype: Value = object_constructor.get("prototype")?;
        if !prototype.is_null() && prototype != ordinary_prototype {
            return Err(Exception::throw_type(
                ctx,
                "component delegate objects must have Object.prototype or null prototype",
            ));
        }
        let get_symbols: Function = object_constructor.get("getOwnPropertySymbols")?;
        let symbols: Array = get_symbols.call((object.clone(),))?;
        if host_modules::bridge_array_len(ctx, &symbols)? != 0 {
            return Err(Exception::throw_type(
                ctx,
                "component delegate objects cannot contain symbol keys",
            ));
        }
        let get_descriptors: Function = object_constructor.get("getOwnPropertyDescriptors")?;
        let descriptors: Object = get_descriptors.call((object.clone(),))?;
        let mut out = Vec::new();
        for key in object.keys::<String>() {
            budget.keys += 1;
            if budget.keys > MAX_COMPONENT_DATA_OBJECT_KEYS {
                return Err(Exception::throw_range(
                    ctx,
                    "component delegate objects contain too many keys",
                ));
            }
            let key = key?;
            budget.string_bytes = budget.string_bytes.saturating_add(key.len());
            if budget.string_bytes > MAX_COMPONENT_DATA_STRING_BYTES {
                return Err(Exception::throw_range(
                    ctx,
                    "component delegate strings exceed the byte limit",
                ));
            }
            let descriptor: Object = descriptors.get(key.as_str())?;
            let getter: Value = descriptor.get("get")?;
            let setter: Value = descriptor.get("set")?;
            if !getter.is_undefined() || !setter.is_undefined() {
                return Err(Exception::throw_type(
                    ctx,
                    "component delegate objects cannot contain accessors",
                ));
            }
            if matches!(key.as_str(), "__id" | "__handle" | "__componentStateHandle") {
                return Err(Exception::throw_type(
                    ctx,
                    "component delegate data cannot contain shell handles or elements",
                ));
            }
            let value: Value = descriptor.get("value")?;
            out.push((key, component_data_from_js(ctx, value, depth + 1, budget)?));
        }
        return Ok(ComponentDataValue::Object(out));
    }
    Err(Exception::throw_type(
        ctx,
        "component delegate callbacks may return only plain data",
    ))
}

fn component_data_into_js<'js>(ctx: &Ctx<'js>, value: &ComponentDataValue) -> JsResult<Value<'js>> {
    Ok(match value {
        ComponentDataValue::Null => Value::new_null(ctx.clone()),
        ComponentDataValue::Boolean(value) => Value::new_bool(ctx.clone(), *value),
        ComponentDataValue::Number(value) => Value::new_number(ctx.clone(), *value),
        ComponentDataValue::String(value) => {
            rquickjs::String::from_str(ctx.clone(), value)?.into_value()
        }
        ComponentDataValue::Array(values) => {
            let array = Array::new(ctx.clone())?;
            for (index, value) in values.iter().enumerate() {
                array.set(index, component_data_into_js(ctx, value)?)?;
            }
            array.into_value()
        }
        ComponentDataValue::Object(fields) => {
            let object = Object::new(ctx.clone())?;
            object.set_prototype(None)?;
            for (key, value) in fields {
                object.prop(
                    key.as_str(),
                    rquickjs::object::Property::from(component_data_into_js(ctx, value)?)
                        .writable()
                        .enumerable()
                        .configurable(),
                )?;
            }
            object.into_value()
        }
    })
}

/// How a `cx` reaches the host call it speaks for.
///
/// GPUI draws this line with the borrow checker: `App` and `Context<T>` are
/// borrows that cannot outlive their call, and `AsyncApp` is the one flavor you
/// may hold across an `await`. A script has no borrow checker, so the same line
/// is drawn at run time — and it is the *only* difference between the two
/// kinds of `cx`. Every member gates on [`ContextBinding::check`] and then does
/// the same ambient work, so the two cannot drift apart.
#[derive(Clone, Copy)]
pub(crate) enum ContextBinding {
    /// One host call, named by its generation. Refuses once that call has
    /// returned, which is what catches a `cx` stashed in a closure and used
    /// from a later frame.
    Call(u64),
    /// Whichever host call is running now. Survives an `await`, because it
    /// names no frame that could go stale — the mirror of GPUI's `AsyncApp`.
    Ambient,
}

impl ContextBinding {
    /// Refuses a `cx` that cannot speak for a live call, before any member acts.
    fn check(self, ctx: &Ctx<'_>) -> JsResult<()> {
        self.with_app(ctx, |_| ())
    }

    /// The `App` of the call this `cx` speaks for.
    fn with_app<R>(self, ctx: &Ctx<'_>, body: impl FnOnce(&mut App) -> R) -> JsResult<R> {
        match self {
            Self::Call(generation) => scope::with_context(generation, |_, app| body(app))
                .map_err(|error| Exception::throw_type(ctx, &error.to_string())),
            Self::Ambient => scope::with_current_app(body).ok_or_else(|| {
                Exception::throw_type(
                    ctx,
                    "this cx has no host call to speak for. An async cx works inside the task \
                     that owns it — from a handler the scheduler resumed — not from a bare \
                     promise callback or after the task was cancelled.",
                )
            }),
        }
    }
}

/// `pagination_items(current, total, visible)`.
///
/// A plain calculation rather than a component: what it answers is which page
/// numbers to draw and where the gaps fall, and the buttons themselves are the
/// script's. Legal from `render`, because that is where a script builds them.
fn pagination_items<'js>(
    ctx: Ctx<'js>,
    current_page: f64,
    total_pages: f64,
    visible_pages: f64,
) -> JsResult<Array<'js>> {
    let clamp = |value: f64| -> JsResult<usize> {
        if !value.is_finite() || value < 0.0 {
            return Err(Exception::throw_type(
                &ctx,
                "pagination_items(current_page, total_pages, visible_pages?) expects \
                 non-negative numbers",
            ));
        }
        Ok(value as usize)
    };
    let items = gpui_base::PaginationState::new(clamp(current_page)?, clamp(total_pages)?)
        .visible_pages(clamp(visible_pages)?)
        .items();

    let out = Array::new(ctx.clone())?;
    for (index, item) in items.into_iter().enumerate() {
        let object = Object::new(ctx.clone())?;
        match item {
            gpui_base::PaginationItem::Page(page) => object.set("page", page as u32)?,
            // Base's range is half-open; a script showing "pages 4–8" wants
            // the last page the gap covers, not the one after it.
            gpui_base::PaginationItem::Ellipsis(range) => {
                let bounds = Array::new(ctx.clone())?;
                bounds.set(0, range.start as u32)?;
                bounds.set(1, range.end.saturating_sub(1) as u32)?;
                object.set("ellipsis", bounds)?;
            }
        }
        out.set(index, object)?;
    }
    Ok(out)
}

/// One chord, spelled the same way on every platform.
///
/// Not `Keystroke::unparse`, and the difference is the whole point of this
/// function. GPUI spells the platform modifier for the platform it was built
/// for — `cmd-` on macOS, `super-` on Linux, `win-` on Windows — which is right
/// for a keymap a person reads and wrong for a string a program compares.
/// A script is one file running on all three, so
///
/// ```js
/// if (event.keystroke === "cmd-s") this.save();
/// ```
///
/// would work on macOS and silently do nothing everywhere else. That failure
/// is invisible in review and invisible in a test suite that runs on one
/// platform.
///
/// `cmd` is the spelling because the other side of this API already accepts it
/// everywhere: `Keystroke::parse`, which `cx.bind_keys` goes through, takes
/// `cmd`, `super` and `win` on every platform. Picking the one that parses
/// everywhere makes a binding and the event it produces agree by construction.
///
/// The modifier order is GPUI's own, so a chord that round-trips through
/// `parse` comes back identical.
fn script_keystroke(keystroke: &gpui::Keystroke) -> String {
    let mut out = String::new();
    if keystroke.modifiers.function {
        out.push_str("fn-");
    }
    if keystroke.modifiers.control {
        out.push_str("ctrl-");
    }
    if keystroke.modifiers.alt {
        out.push_str("alt-");
    }
    if keystroke.modifiers.platform {
        out.push_str("cmd-");
    }
    if keystroke.modifiers.shift {
        out.push_str("shift-");
    }
    out.push_str(&keystroke.key);
    out
}

/// The modifier keys, in the shape every event payload carries them.
fn modifiers_object<'js>(ctx: &Ctx<'js>, modifiers: gpui::Modifiers) -> JsResult<Object<'js>> {
    let object = Object::new(ctx.clone())?;
    object.set("shift", modifiers.shift)?;
    object.set("control", modifiers.control)?;
    object.set("alt", modifiers.alt)?;
    object.set("platform", modifiers.platform)?;
    object.set("function", modifiers.function)?;
    Ok(object)
}

/// The window position, and the element-relative position and box when the
/// element has been painted.
///
/// `local_position` and `bounds` are omitted rather than zeroed for an element
/// that has not been prepainted yet, so a script reading `undefined` knows the
/// geometry was unavailable instead of being told the press landed at its
/// top-left corner.
/// The object an `on_mouse_down` or `on_mouse_up` handler is handed, built in
/// one place so a press reported through a row carries the same fields as one
/// reported through the element it landed on.
fn mouse_button_payload<'js>(
    ctx: &Ctx<'js>,
    button: gpui::MouseButton,
    position: gpui::Point<gpui::Pixels>,
    click_count: usize,
    modifiers: gpui::Modifiers,
    bounds: Option<gpui::Bounds<gpui::Pixels>>,
) -> JsResult<Object<'js>> {
    let payload = Object::new(ctx.clone())?;
    payload.set(
        "button",
        match button {
            gpui::MouseButton::Right => "right",
            gpui::MouseButton::Middle => "middle",
            _ => "left",
        },
    )?;
    payload.set("click_count", click_count as u32)?;
    set_pointer_geometry(ctx, &payload, position, bounds)?;
    payload.set("modifiers", modifiers_object(ctx, modifiers)?)?;
    Ok(payload)
}

fn set_pointer_geometry<'js>(
    ctx: &Ctx<'js>,
    payload: &Object<'js>,
    position: gpui::Point<gpui::Pixels>,
    bounds: Option<gpui::Bounds<gpui::Pixels>>,
) -> JsResult<()> {
    let window_position = Object::new(ctx.clone())?;
    window_position.set("x", f32::from(position.x))?;
    window_position.set("y", f32::from(position.y))?;
    payload.set("position", window_position)?;
    match bounds {
        Some(bounds) => {
            let local = position - bounds.origin;
            let local_position = Object::new(ctx.clone())?;
            local_position.set("x", f32::from(local.x))?;
            local_position.set("y", f32::from(local.y))?;
            payload.set("local_position", local_position)?;
            let event_bounds = Object::new(ctx.clone())?;
            event_bounds.set("x", f32::from(bounds.origin.x))?;
            event_bounds.set("y", f32::from(bounds.origin.y))?;
            event_bounds.set("width", f32::from(bounds.size.width))?;
            event_bounds.set("height", f32::from(bounds.size.height))?;
            payload.set("bounds", event_bounds)?;
        }
        None => {
            payload.set("local_position", rquickjs::Undefined)?;
            payload.set("bounds", rquickjs::Undefined)?;
        }
    }
    Ok(())
}

/// `globalThis.__async_cx()`. A free function rather than a closure because it
/// has to be generic over the JS lifetime.
fn async_context_object<'js>(ctx: Ctx<'js>) -> JsResult<Object<'js>> {
    context_object(&ctx, ContextBinding::Ambient)
}

/// The script-side `cx`.
///
/// It carries no state a script can reach — only the binding above, closed over
/// by the members — so `Object.keys(cx)` still shows nothing but methods and a
/// generation cannot be forged.
fn context_object<'js>(ctx: &Ctx<'js>, binding: ContextBinding) -> JsResult<Object<'js>> {
    let object = Object::new(ctx.clone())?;

    let module: Object = ctx.globals().get("__gpui")?;
    let members: Function = module.get("__context_members")?;
    let check = Func::from(move |ctx: Ctx<'_>| -> JsResult<()> { binding.check(&ctx) });
    let members: Object = members.call((check,))?;
    for name in members.keys::<String>() {
        let name = name?;
        let member: Value = members.get(&name as &str)?;
        object.set(name, member)?;
    }
    object.set(
        "notify",
        Func::from(
            move |ctx: Ctx<'_>, target: Opt<Value<'_>>| -> JsResult<()> {
                let phase = scope::current_phase();
                if !phase.is_some_and(ScopePhase::allows_notify) {
                    let api = if target.0.as_ref().is_some_and(|value| !value.is_undefined()) {
                        "cx.notify(entity)"
                    } else {
                        "cx.notify()"
                    };
                    return Err(Exception::throw_type(
                        &ctx,
                        &format!(
                            "{api} is not allowed during the `{}` phase; \
                         request a re-render from an event handler instead",
                            phase.map(ScopePhase::as_str).unwrap_or("none")
                        ),
                    ));
                }

                let current = scope::current_view();
                let target = match target.0 {
                    None => None,
                    Some(value) if value.is_undefined() => None,
                    Some(value) => {
                        let object = value.as_object().ok_or_else(|| {
                            Exception::throw_type(&ctx, "cx.notify(entity) expects an Entity")
                        })?;
                        let branded = object.get::<_, bool>("__entity").unwrap_or(false);
                        let token = object.get::<_, u32>("__handle").map_err(|_| {
                            Exception::throw_type(&ctx, "cx.notify(entity) expects an Entity")
                        })?;
                        if !branded {
                            return Err(Exception::throw_type(
                                &ctx,
                                "cx.notify(entity) expects an Entity",
                            ));
                        }
                        Some(token)
                    }
                };
                let notify_ctx = ctx.clone();
                binding.with_app(&ctx, move |app| -> JsResult<()> {
                    let view = match target {
                        None => current,
                        Some(token) => {
                            let current = current.as_ref().ok_or_else(|| {
                                Exception::throw_type(
                                    &notify_ctx,
                                    "cx.notify(entity) requires a current script view",
                                )
                            })?;
                            let runtime = { current.read(app).runtime() };
                            runtime.queue_nested_view_notify(&notify_ctx, token)?;
                            None
                        }
                    };
                    if let Some(view) = view {
                        // Two halves, and both matter. Invalidating says the script
                        // description may have moved, which is the only thing that
                        // lets the next frame enter the VM; notifying hands the
                        // scheduling and coalescing of that frame back to GPUI.
                        view.update(app, |view, cx| view.refresh(cx));
                    }
                    Ok(())
                })?
            },
        ),
    )?;

    // GPUI dispatches an event to every handler on the path unless one of them
    // says otherwise, so a script that puts a handler on a row inside a list
    // hears both. These are the two halves of `App`'s own answer to that, under
    // their own names — the mirror is exact, so what a script learns here is
    // what GPUI documents.
    object.set(
        "stop_propagation",
        Func::from(move |ctx: Ctx<'_>| -> JsResult<()> {
            binding.with_app(&ctx, |app| app.stop_propagation())
        }),
    )?;
    object.set(
        "propagate",
        Func::from(move |ctx: Ctx<'_>| -> JsResult<()> {
            binding.with_app(&ctx, |app| app.propagate())
        }),
    )?;

    object.set(
        "phase",
        Func::from(|| {
            scope::current_phase()
                .map(ScopePhase::as_str)
                .unwrap_or("none")
                .to_owned()
        }),
    )?;

    Ok(object)
}

/// One converted argument.
///
/// A JS closure cannot unify the `Ctx<'js>` lifetime with a `Vec<Value<'js>>`
/// parameter, so conversion happens inside `FromJs`, where both lifetimes are
/// still the same one. Handlers become `Persistent` here for the same reason.
enum Argument {
    Value(Bridged),
    Handler(Persistent<Function<'static>>),
    Element(SpecId),
    Entity(EntityHandle),
    RetainedState {
        handle: u64,
        proof: String,
    },
    Array(Vec<Argument>),
    /// A template's sentinel: this position is filled per call rather than now.
    /// Only reachable while a template body is being discovered, because
    /// nothing else hands one out.
    Slot(u16),
}

struct Arguments(SmallVec<[Argument; 2]>);

/// The single argument of a parametric style method.
///
/// Separate from [`Argument`] only because it arrives without an array around
/// it: a style takes one value, and building a JavaScript array to carry it was
/// measurable in the description pass. A function is carried as a marker rather
/// than saved, because no style takes one and the only thing left to do with it
/// is name the method that was handed one.
enum StyleArgument {
    Value(Bridged),
    Handler,
    /// A template's sentinel. See [`Argument::Slot`].
    Slot(u16),
}

impl<'js> FromJs<'js> for StyleArgument {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        if value.as_function().is_some() {
            return Ok(Self::Handler);
        }
        if let Some(slot) = slot_index(&value) {
            return Ok(Self::Slot(slot));
        }
        Ok(Self::Value(bridge(ctx, &value)?))
    }
}

/// The template parameter a value stands for, if it is a sentinel.
///
/// Costs one tag check for the values a description is actually made of — a
/// string, a number, a boolean — because only an object can carry the marker.
/// The property lookup happens for objects, which reach a builder call rarely
/// and reached one as an error before templates existed.
fn slot_index(value: &Value<'_>) -> Option<u16> {
    let object = value.as_object()?;
    let index: Option<f64> = object.get(template::SENTINEL).ok()?;
    let index = index?;
    (index.is_finite() && index >= 0.0 && index <= f64::from(u16::MAX)).then_some(index as u16)
}

/// Converts one non-function script value.
///
/// The one place the four bridged cases are named, so that [`Arguments`] and
/// [`StyleArgument`] cannot come to disagree about what a script value is.
fn bridge(ctx: &Ctx<'_>, value: &Value<'_>) -> JsResult<Bridged> {
    Ok(if value.is_null() || value.is_undefined() {
        Bridged::Nil
    } else if let Some(flag) = value.as_bool() {
        Bridged::Bool(flag)
    } else if let Some(number) = value.as_number() {
        Bridged::Number(number)
    } else if let Some(text) = value.as_string() {
        Bridged::Str(text.to_string()?)
    } else {
        return Err(Exception::throw_type(
            ctx,
            "unsupported argument type; expected null, boolean, number, string or function",
        ));
    })
}

impl Arguments {
    fn values(&self, method: &str) -> JsResult<SmallVec<[Bridged; 2]>> {
        self.0
            .iter()
            .map(|argument| match argument {
                Argument::Value(value) => Ok(value.clone()),
                Argument::Handler(_) => Err(JsError::new_from_js_message(
                    "function",
                    "value",
                    format!("`{method}` does not take a function"),
                )),
                Argument::Slot(_) => Err(JsError::new_from_js_message(
                    "template argument",
                    "value",
                    format!(
                        "`{method}` cannot take a template argument yet; a template fills text \
                         children, style arguments and handlers. Compute the value where the \
                         template is called and pass the result"
                    ),
                )),
                Argument::Element(_)
                | Argument::Entity(_)
                | Argument::RetainedState { .. }
                | Argument::Array(_) => Err(JsError::new_from_js_message(
                    "object",
                    "value",
                    format!("`{method}` does not take an object or array"),
                )),
            })
            .collect()
    }

    /// The first sentinel among these arguments, and which parameter it stands
    /// for.
    fn first_slot(&self) -> Option<(usize, u16)> {
        self.0
            .iter()
            .enumerate()
            .find_map(|(index, argument)| match argument {
                Argument::Slot(slot) => Some((index, *slot)),
                _ => None,
            })
    }

    fn first_value(&self) -> Option<&Bridged> {
        match self.0.first() {
            Some(Argument::Value(value)) => Some(value),
            _ => None,
        }
    }

    fn first_handler(&self) -> Option<Persistent<Function<'static>>> {
        self.handler_at(0)
    }

    /// The handler at one position, for the two methods that take an argument
    /// before it.
    fn handler_at(&self, index: usize) -> Option<Persistent<Function<'static>>> {
        match self.0.get(index) {
            Some(Argument::Handler(handler)) => Some(handler.clone()),
            _ => None,
        }
    }
}

impl<'js> FromJs<'js> for Arguments {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let array = value
            .into_array()
            .ok_or_else(|| Exception::throw_type(ctx, "expected an argument list"))?;

        let length = host_modules::bridge_array_len(ctx, &array)?;
        let mut converted = SmallVec::with_capacity(length);
        let mut budget = ComponentArgumentBudget::default();
        for index in 0..length {
            converted.push(component_argument_from_js(
                ctx,
                array.get(index)?,
                0,
                &mut budget,
            )?);
        }

        Ok(Self(converted))
    }
}

const MAX_COMPONENT_ARGUMENT_DEPTH: usize = 32;
const MAX_COMPONENT_ARGUMENT_NODES: usize = 10_000;

#[derive(Default)]
struct ComponentArgumentBudget {
    nodes: usize,
}

fn component_argument_from_js<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
    depth: usize,
    budget: &mut ComponentArgumentBudget,
) -> JsResult<Argument> {
    if depth > MAX_COMPONENT_ARGUMENT_DEPTH {
        return Err(Exception::throw_range(
            ctx,
            "component arguments are nested too deeply",
        ));
    }
    budget.nodes = budget.nodes.checked_add(1).ok_or_else(|| {
        Exception::throw_range(ctx, "component arguments contain too many nested values")
    })?;
    if budget.nodes > MAX_COMPONENT_ARGUMENT_NODES {
        return Err(Exception::throw_range(
            ctx,
            "component arguments contain too many nested values",
        ));
    }
    if let Some(handler) = value.as_function() {
        return Ok(Argument::Handler(Persistent::save(ctx, handler.clone())));
    }
    if let Some(slot) = slot_index(&value) {
        return Ok(Argument::Slot(slot));
    }
    if let Some(array) = value.as_array() {
        let length = host_modules::bridge_array_len(ctx, &array)?;
        let mut converted = Vec::with_capacity(length);
        for index in 0..length {
            converted.push(component_argument_from_js(
                ctx,
                array.get(index)?,
                depth + 1,
                budget,
            )?);
        }
        return Ok(Argument::Array(converted));
    }
    if let Some(object) = value.as_object() {
        if let Ok(id) = object.get::<_, u32>("__id") {
            return Ok(Argument::Element(id));
        }
        if let Ok(handle) = object.get::<_, u64>("__handle") {
            return Ok(Argument::Entity(handle));
        }
        if let (Ok(handle), Ok(proof)) = (
            object.get::<_, u64>("__componentStateHandle"),
            object.get::<_, String>("__componentStateProof"),
        ) {
            return Ok(Argument::RetainedState { handle, proof });
        }
    }
    Ok(Argument::Value(bridge(ctx, &value)?))
}

/// One callback argument as a JavaScript value.
fn callback_argument_to_js<'js>(
    ctx: &Ctx<'js>,
    argument: &ComponentCallbackArgument,
) -> JsResult<Value<'js>> {
    use rquickjs::IntoJs as _;

    match argument {
        ComponentCallbackArgument::String(value) => value.clone().into_js(ctx),
        ComponentCallbackArgument::Number(value) => (*value).into_js(ctx),
        ComponentCallbackArgument::Boolean(value) => (*value).into_js(ctx),
        ComponentCallbackArgument::Array(values) => {
            let array = Array::new(ctx.clone())?;
            for (index, value) in values.iter().enumerate() {
                array.set(index, callback_argument_to_js(ctx, value)?)?;
            }
            array.into_js(ctx)
        }
    }
}

fn schema_name(schema: &ArgumentSchema) -> String {
    match schema {
        ArgumentSchema::String => "a string".into(),
        ArgumentSchema::Number => "a finite number".into(),
        ArgumentSchema::Boolean => "a boolean".into(),
        ArgumentSchema::Element => "an Element".into(),
        ArgumentSchema::Entity(kind) => format!("a {kind} entity"),
        ArgumentSchema::Callback(_) => "a function".into(),
        ArgumentSchema::Enum(values) => values
            .iter()
            .map(|value| format!("`{value}`"))
            .collect::<Vec<_>>()
            .join(", "),
        ArgumentSchema::Array(item) => format!("an array of {}", schema_name(item)),
        ArgumentSchema::Optional(item) => format!("an optional {}", schema_name(item)),
    }
}

fn collect_component_elements(argument: &ComponentArgument, elements: &mut Vec<SpecId>) {
    match argument {
        ComponentArgument::Element(element) => elements.push(*element),
        ComponentArgument::Array(arguments) => {
            for argument in arguments {
                collect_component_elements(argument, elements);
            }
        }
        ComponentArgument::Optional(Some(argument)) => {
            collect_component_elements(argument, elements);
        }
        ComponentArgument::String(_)
        | ComponentArgument::Number(_)
        | ComponentArgument::Boolean(_)
        | ComponentArgument::Entity { .. }
        | ComponentArgument::Callback(_)
        | ComponentArgument::Enum(_)
        | ComponentArgument::Optional(None) => {}
    }
}

/// Binds `__template_instantiate`, which is the one global taking live script
/// values alongside its `Ctx`.
///
/// A closure would give the two elided lifetimes no reason to be the same one,
/// and `Value<'js>` is invariant — so the binding is built by a function that
/// names the lifetime once and quantifies over it.
fn instantiate_template_binding(
    runtime: Weak<ShellRuntime>,
) -> impl for<'js> Fn(Ctx<'js>, u32, Vec<rquickjs::Value<'js>>) -> JsResult<SpecId> {
    move |ctx, id, arguments| {
        let runtime = upgrade(&runtime, &ctx)?;
        runtime.instantiate_template(&ctx, id, arguments)
    }
}

/// The recorded op name for a method that takes one handler and nothing else.
///
/// The list the `apply` arm matches on, in a form the template path can reach
/// too: a slot in a handler position has to record the same `SpecOp::Callback`
/// name the ordinary path would, and two copies of this list would drift.
fn callback_op_name(method: &str) -> Option<&'static str> {
    Some(match method {
        "on_click" => "on_click",
        "on_link_click" => "on_link_click",
        "on_mouse_move" => "on_mouse_move",
        "on_hover" => "on_hover",
        "on_key_down" => "on_key_down",
        "on_key_up" => "on_key_up",
        "on_modifiers_changed" => "on_modifiers_changed",
        "on_mouse_down_out" => "on_mouse_down_out",
        "on_scroll_wheel" => "on_scroll_wheel",
        "on_item_click" => "on_item_click",
        "on_item_secondary_click" => "on_item_secondary_click",
        "on_resize" => "on_resize",
        "tab_bar" => "tab_bar",
        "empty_group" => "empty_group",
        "drop_indicator" => "drop_indicator",
        "dock" => "dock",
        "tile_drag_bar" => "tile_drag_bar",
        "tile_resize_handles" => "tile_resize_handles",
        "on_change" => "on_change",
        "on_confirm" => "on_confirm",
        "on_dismiss" => "on_dismiss",
        "on_step" => "on_step",
        "on_open_change" => "on_open_change",
        _ => return None,
    })
}

/// The element methods the prelude checks against a fixed vocabulary, read out
/// of the prelude source itself.
///
/// Two lists of the same names would drift, and the drift is invisible: a
/// descriptor method that disagrees simply stops working, for every value. So
/// the list lives in `component_registry`, where registration can refuse the
/// disagreement, and this reads the truth back out of the prelude to check it.
#[cfg(test)]
fn prelude_checked_vocabularies() -> Vec<(String, Vec<String>)> {
    let mut found = Vec::new();
    for (index, _) in PRELUDE.match_indices("\n  methods.") {
        let rest = &PRELUDE[index + "\n  methods.".len()..];
        let Some(end) = rest.find(" = function") else {
            continue;
        };
        let name = rest[..end].to_owned();
        // Only this method's own body: a fixed window spills into the next
        // method's comment and reads its check as this one's.
        let body = match rest.find("\n  };") {
            Some(close) => &rest[..close],
            None => rest,
        };
        if body.contains("__anchorNames.includes") {
            found.push((
                name,
                crate::materialize::ANCHOR_NAMES
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
            ));
            continue;
        }
        let Some(open) = body.find('[') else { continue };
        let Some(close) = body[open..].find("].includes(value)") else {
            continue;
        };
        let literals = body[open + 1..open + close]
            .split(',')
            .filter_map(|part| {
                let part = part.trim().trim_matches('"');
                (!part.is_empty()).then(|| part.to_owned())
            })
            .collect::<Vec<_>>();
        if !literals.is_empty() {
            found.push((name, literals));
        }
    }
    found.sort();
    found.dedup();
    found
}

#[cfg(test)]
fn prelude_owned_element_methods() -> Vec<&'static str> {
    let generic_dispatch = PRELUDE
        .find("for (const name of __behaviorNames) define(name);")
        .expect("the prelude installs descriptor methods generically");
    let mut names = Vec::new();
    let mut rest = &PRELUDE[generic_dispatch..];
    while let Some(at) = rest.find("\n  methods.") {
        rest = &rest[at + "\n  methods.".len()..];
        let Some(end) = rest.find(" = function") else {
            continue;
        };
        let name = &rest[..end];
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            names.push(name);
        }
    }
    names.sort_unstable();
    names.dedup();
    names
}

fn unknown_method(name: &str) -> String {
    match style::suggest(name) {
        Some(candidate) => format!("unknown element method `{name}` (did you mean `{candidate}`?)"),
        None => format!(
            "unknown element method `{name}`; it is neither a style method nor one of \
             child, children, when, on_click, on_change, disabled, selected, checked, \
             overflow_scroll, overflow_x_scroll, overflow_y_scroll, overflow_scrollbar, \
             overflow_x_scrollbar, overflow_y_scrollbar"
        ),
    }
}

fn refuse_nested_view_mutation(ctx: &Ctx<'_>, api: &str, action: &str) -> JsResult<()> {
    let Some(phase @ (ScopePhase::Render | ScopePhase::Layout)) = scope::current_phase() else {
        return Ok(());
    };
    Err(Exception::throw_type(
        ctx,
        &format!(
            "{api} cannot run during {}; {action} retained views from init(), an event handler \
             or a task",
            phase.as_str()
        ),
    ))
}

fn nested_view_needs_call(ctx: &Ctx<'_>, api: &str) -> JsError {
    Exception::throw_type(
        ctx,
        &format!("{api} needs a live host call; use it from init(), an event handler or a task"),
    )
}

fn upgrade(runtime: &Weak<ShellRuntime>, ctx: &Ctx<'_>) -> JsResult<Rc<ShellRuntime>> {
    runtime
        .upgrade()
        .ok_or_else(|| Exception::throw_message(ctx, "the shell runtime has already shut down"))
}

/// Turns a QuickJS error into a message that includes the script's own stack,
/// which is the part an author actually needs.
fn describe(ctx: &Ctx<'_>, error: JsError) -> String {
    if !matches!(error, JsError::Exception) {
        return error.to_string();
    }
    let value = ctx.catch();
    match value.as_exception() {
        Some(exception) => match exception.stack() {
            Some(stack) => format!(
                "{}\n{stack}",
                exception.message().unwrap_or_else(|| "error".into())
            ),
            None => exception.message().unwrap_or_else(|| "error".into()),
        },
        None => format!("{value:?}"),
    }
}

fn js_setup_error(error: JsError) -> anyhow::Error {
    anyhow!("failed to start the JavaScript runtime: {error}")
}

#[cfg(test)]
mod keystroke_tests {
    use gpui::{Keystroke, Modifiers};

    /// The chord a script compares against is the same on every platform.
    ///
    /// Built from `Modifiers` directly rather than from a simulated key press,
    /// so the assertion is about the spelling and not about whichever platform
    /// this suite happens to run on — which is the exact hole that let
    /// `Keystroke::unparse` ship here in the first place. On macOS it is right
    /// and on Linux it answered `super-s`, so a test that ran only on macOS
    /// agreed with it.
    #[test]
    fn the_platform_modifier_is_spelled_cmd_everywhere() {
        let chord = |modifiers: Modifiers, key: &str| {
            super::script_keystroke(&Keystroke {
                modifiers,
                key: key.to_owned(),
                key_char: None,
            })
        };

        assert_eq!(chord(Modifiers::command(), "s"), "cmd-s");
        assert_eq!(
            chord(Modifiers::command_shift(), "p"),
            "cmd-shift-p",
            "the modifier order is GPUI's own, so a chord round-trips through parse"
        );
        assert_eq!(chord(Modifiers::control(), "c"), "ctrl-c");
        assert_eq!(chord(Modifiers::alt(), "f"), "alt-f");
        assert_eq!(chord(Modifiers::none(), "escape"), "escape");
    }

    /// And it is a spelling `Keystroke::parse` accepts, on every platform.
    ///
    /// This is what makes a binding and the event it produces agree: the same
    /// text a script passes to `cx.bind_keys` is the text it will be handed
    /// back. Asserted rather than assumed, because `parse` accepting `cmd`
    /// away from macOS is the property the choice rests on.
    #[test]
    fn the_spelling_round_trips_through_gpui_parse() {
        for chord in ["cmd-s", "cmd-shift-p", "ctrl-alt-delete", "escape"] {
            let parsed = Keystroke::parse(chord).expect("every spelling here must parse");
            assert_eq!(
                super::script_keystroke(&parsed),
                chord,
                "`{chord}` must come back as it went in"
            );
        }
    }
}

#[cfg(test)]
mod module_lifecycle_tests {
    use super::{AppModules, ShellRuntime};
    use crate::dependencies::{GitDependencyStore, MaterializedDependency};
    use std::collections::BTreeMap;
    use std::process::Command;

    #[test]
    fn gpui_module_exports_div() {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        runtime
            .load_source(
                "gpui-import.js",
                r#"
import { div, View } from "gpui";
export default class Panel extends View { render() { return div(); } }
"#,
            )
            .expect("gpui is an importable built-in module");
    }

    #[test]
    fn registrations_for_the_same_root_are_generation_scoped_and_leased() {
        let modules = AppModules::default();
        let root = std::env::temp_dir().join("gpui-shell-module-lifecycle");

        let first = modules.register(root.clone());
        let second = modules.register(root.clone());

        assert_ne!(first.generation(), second.generation());
        assert_eq!(modules.registration_count(), 2);

        let retained = first.clone();
        drop(first);
        assert_eq!(modules.registration_count(), 2);
        drop(retained);
        assert_eq!(modules.registration_count(), 1);
        drop(second);
        assert_eq!(modules.registration_count(), 0);
    }

    #[test]
    fn importer_tags_select_the_exact_same_root_generation() {
        let modules = AppModules::default();
        let root = std::env::temp_dir().join("gpui-shell-module-generation");
        let first = modules.register(root.clone());
        let second = modules.register(root.clone());

        let first_importer = format!("{}/main.js?v={}", root.display(), first.generation());
        let second_importer = format!("{}/main.js?v={}", root.display(), second.generation());

        assert_eq!(
            modules
                .application_for_base(&first_importer)
                .expect("first generation")
                .generation,
            first.generation()
        );
        assert_eq!(
            modules
                .application_for_base(&second_importer)
                .expect("second generation")
                .generation,
            second.generation()
        );
    }

    #[test]
    fn an_older_same_root_class_keeps_its_import_generation() {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let root = std::env::temp_dir().join(format!(
            "gpui-shell-same-root-generation-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("application directory");
        std::fs::write(
            root.join("main.js"),
            "import './feature.js';\n\
             export default class Panel {\n\
               static async label() { return (await import('./feature.js')).label; }\n\
             }",
        )
        .expect("entry module");
        std::fs::write(root.join("feature.js"), "export const label = 'first';")
            .expect("first feature");

        let first = runtime.load_app(&root, "main.js").expect("first load");
        std::fs::write(root.join("feature.js"), "export const label = 'second';")
            .expect("second feature");
        let second = runtime.load_app(&root, "main.js").expect("second load");

        let label = |view_type: &super::ViewType| {
            runtime
                .with_js(|ctx| {
                    let class = view_type.value.clone().restore(ctx)?;
                    let label: rquickjs::Function = class.get("label")?;
                    label.call::<_, rquickjs::Promise>(())?.finish::<String>()
                })
                .expect("dynamic import")
        };
        assert_eq!(label(&first), "first");
        assert_eq!(label(&second), "second");

        drop(first);
        assert_eq!(runtime.app_modules.registration_count(), 1);
        drop(second);
        assert_eq!(runtime.app_modules.registration_count(), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_failed_load_releases_its_module_registration() {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let root = std::env::temp_dir().join(format!(
            "gpui-shell-failed-module-generation-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("application directory");
        std::fs::write(
            root.join("main.js"),
            "import './missing.js'; export default class Panel {}",
        )
        .expect("entry module");

        runtime
            .load_app(&root, "main.js")
            .expect_err("missing import must reject the load");
        assert_eq!(runtime.app_modules.registration_count(), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_bare_dependency_import_loads_its_entry_and_relative_modules() {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let root = std::env::temp_dir().join(format!(
            "gpui-shell-third-party-module-{}",
            std::process::id()
        ));
        let application = root.join("application");
        let dependency = root.join("dependency");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&application).expect("application directory");
        std::fs::create_dir_all(&dependency).expect("dependency directory");
        std::fs::write(
            dependency.join("index.js"),
            "export { label } from './theme.js';",
        )
        .expect("dependency entry");
        std::fs::write(
            dependency.join("theme.js"),
            "export const label = 'third party'; export const tone = 'dark';",
        )
        .expect("dependency relative module");

        let application = application.canonicalize().expect("application root");
        let dependency = dependency.canonicalize().expect("dependency root");
        let mut dependencies = BTreeMap::new();
        dependencies.insert(
            "omarchy-ui".to_owned(),
            MaterializedDependency {
                entry: dependency.join("index.js"),
                root: dependency,
            },
        );
        let lease = runtime
            .app_modules
            .register_with_dependencies(application.clone(), dependencies);
        let generation = lease.generation();
        let view_type = runtime
            .load_source_with_lease(
                &format!("{}/main.js?v={generation}", application.display()),
                "import { label } from 'omarchy-ui'; import { tone } from 'omarchy-ui/theme.js'; export default class Panel { static label() { return `${label}:${tone}`; } }",
                Some(lease),
                None,
            )
            .expect("third-party module graph");

        let label = runtime
            .with_js(|ctx| {
                let class = view_type.value.clone().restore(ctx)?;
                let label: rquickjs::Function = class.get("label")?;
                label.call::<_, String>(())
            })
            .expect("dependency export");
        assert_eq!(label, "third party:dark");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_dependency_relative_import_cannot_escape_its_checkout() {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let root = std::env::temp_dir().join(format!(
            "gpui-shell-third-party-escape-{}",
            std::process::id()
        ));
        let application = root.join("application");
        let dependency = root.join("dependency");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&application).expect("application directory");
        std::fs::create_dir_all(&dependency).expect("dependency directory");
        std::fs::write(root.join("secret.js"), "export const secret = true;")
            .expect("outside module");
        std::fs::write(
            dependency.join("index.js"),
            "export { secret } from '../secret.js';",
        )
        .expect("dependency entry");

        let application = application.canonicalize().expect("application root");
        let dependency = dependency.canonicalize().expect("dependency root");
        let mut dependencies = BTreeMap::new();
        dependencies.insert(
            "third-party".to_owned(),
            MaterializedDependency {
                entry: dependency.join("index.js"),
                root: dependency,
            },
        );
        let lease = runtime
            .app_modules
            .register_with_dependencies(application.clone(), dependencies);
        let generation = lease.generation();
        let error = runtime
            .load_source_with_lease(
                &format!("{}/main.js?v={generation}", application.display()),
                "import 'third-party'; export default class Panel {}",
                Some(lease),
                None,
            )
            .expect_err("dependency traversal must be refused");

        assert!(error.to_string().contains("outside"), "{error:#}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn loading_an_application_fetches_its_manifest_git_dependencies() {
        let root =
            std::env::temp_dir().join(format!("gpui-shell-fetch-module-{}", std::process::id()));
        let application = root.join("application");
        let remote = root.join("remote");
        let cache = root.join("cache");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&application).expect("application directory");
        std::fs::create_dir_all(&remote).expect("remote directory");
        let git = |arguments: &[&str]| {
            let output = Command::new("git")
                .args(arguments)
                .current_dir(&remote)
                .output()
                .expect("git fixture command");
            assert!(
                output.status.success(),
                "git {} failed: {}",
                arguments.join(" "),
                String::from_utf8_lossy(&output.stderr)
            );
        };
        git(&["init", "--initial-branch=main"]);
        git(&["config", "user.name", "gpui-shell test"]);
        git(&["config", "user.email", "gpui-shell@example.invalid"]);
        std::fs::create_dir_all(remote.join("dist")).expect("dependency dist directory");
        std::fs::write(
            remote.join("dist/public.js"),
            "export const label = 'downloaded from package main';",
        )
        .expect("dependency source");
        std::fs::write(
            remote.join("package.json"),
            r#"{ "main": "dist/public.js" }"#,
        )
        .expect("dependency package manifest");
        git(&["add", "."]);
        git(&["commit", "-m", "dependency"]);

        std::fs::write(
            application.join(crate::plugin::MANIFEST_FILE),
            format!(
                r#"{{
                    "id": "com.example.fetch",
                    "name": "Fetch",
                    "entry": "main.js",
                    "dependencies": {{ "omarchy-ui": {} }}
                }}"#,
                serde_json::to_string(&format!("file://{}#main", remote.display()))
                    .expect("remote URL")
            ),
        )
        .expect("application manifest");
        std::fs::write(
            application.join("main.js"),
            "import { label } from 'omarchy-ui'; export default class Panel { static label() { return label; } }",
        )
        .expect("application source");

        let runtime =
            ShellRuntime::new_isolated_with_dependency_store(GitDependencyStore::new(cache))
                .expect("runtime");
        let view_type = runtime
            .load_app(&application, "main.js")
            .expect("application load");
        let label = runtime
            .with_js(|ctx| {
                let class = view_type.value.clone().restore(ctx)?;
                let label: rquickjs::Function = class.get("label")?;
                label.call::<_, String>(())
            })
            .expect("dependency export");
        assert_eq!(label, "downloaded from package main");
        let _ = std::fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod nested_view_lifecycle_tests {
    use super::*;
    use gpui::{ClickEvent, TestAppContext, VisualTestContext};
    use rquickjs::{Object, Persistent};

    struct ChildMount(Entity<ScriptView>);

    impl gpui::Render for ChildMount {
        fn render(
            &mut self,
            _: &mut Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            self.0.clone()
        }
    }

    fn child_type(runtime: &Rc<ShellRuntime>, source: &str) -> ViewType {
        let mut view_type = runtime
            .load_source("nested-child.js", source)
            .expect("load child view");
        view_type.application = Some(ApplicationGeneration::new(7));
        view_type
    }

    #[gpui::test]
    fn foreign_release_cannot_probe_or_remove_a_dead_nested_alias(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let owner_application = ApplicationGeneration::new(71);
        let foreign_application = ApplicationGeneration::new(72);
        let owner_policy = Rc::new(Policy::default());
        let foreign_policy = Rc::new(Policy::default());
        let mut view_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui-kit";
export default class Child extends View { render(cx) { return "child"; } }
"#,
        );
        view_type.application = Some(owner_application.clone());
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let handle = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(&view_type, owner_policy.clone(), None, window, cx)
            })
            .expect("child");
        let token = 91;
        runtime.nested_view_handles.borrow_mut().insert(
            token,
            NestedViewAlias {
                handle,
                provenance: NestedViewProvenance {
                    application: Some(owner_application.clone()),
                    policy: owner_policy.clone(),
                },
            },
        );
        let release = runtime
            .entities()
            .release_view(handle)
            .expect("typed child release");
        context.update(|_, cx| release.retire(cx));
        assert!(runtime.entities().view(handle).is_none());

        let foreign = context.update(|window, cx| {
            let (_scope, _) = scope::enter_with_application(
                &runtime,
                window,
                cx,
                ScopePhase::Event,
                None,
                foreign_policy,
                Some(foreign_application),
            );
            runtime.with_js(|ctx| runtime.queue_nested_view_release(ctx, token))
        });
        assert!(foreign.is_err(), "foreign authority must be rejected");
        assert!(
            runtime.nested_view_handles.borrow().contains_key(&token),
            "foreign release observed liveness and removed the dead alias"
        );

        let owner = context.update(|window, cx| {
            let (_scope, _) = scope::enter_with_application(
                &runtime,
                window,
                cx,
                ScopePhase::Event,
                None,
                owner_policy,
                Some(owner_application),
            );
            runtime.with_js(|ctx| runtime.queue_nested_view_release(ctx, token))
        });
        assert_eq!(owner.expect("owner call"), false);
        assert!(!runtime.nested_view_handles.borrow().contains_key(&token));
    }

    #[gpui::test]
    fn targeted_notify_rejects_an_entity_from_another_application(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let owner_application = ApplicationGeneration::new(81);
        let foreign_application = ApplicationGeneration::new(82);
        let owner_policy = Rc::new(Policy::default());
        let foreign_policy = Rc::new(Policy::default());
        let mut view_type = child_type(
            &runtime,
            r#"
import { View } from "gpui-kit";
export default class Child extends View { render(cx) { return "child"; } }
"#,
        );
        view_type.application = Some(owner_application.clone());
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let handle = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(&view_type, owner_policy.clone(), None, window, cx)
            })
            .expect("child");
        let token = 92;
        runtime.nested_view_handles.borrow_mut().insert(
            token,
            NestedViewAlias {
                handle,
                provenance: NestedViewProvenance {
                    application: Some(owner_application.clone()),
                    policy: owner_policy.clone(),
                },
            },
        );

        let foreign = context.update(|window, cx| {
            let (_scope, _) = scope::enter_with_application(
                &runtime,
                window,
                cx,
                ScopePhase::Event,
                None,
                foreign_policy,
                Some(foreign_application),
            );
            runtime.with_js(|ctx| runtime.queue_nested_view_notify(ctx, token))
        });
        assert!(foreign.is_err(), "foreign application must be rejected");
        assert!(
            runtime.nested_view_handles.borrow().contains_key(&token),
            "a foreign notify must not invalidate the owner's alias"
        );

        let owner = context.update(|window, cx| {
            let (_scope, _) = scope::enter_with_application(
                &runtime,
                window,
                cx,
                ScopePhase::Event,
                None,
                owner_policy,
                Some(owner_application),
            );
            runtime.with_js(|ctx| runtime.queue_nested_view_notify(ctx, token))
        });
        owner.expect("owner application can notify its live child");
    }

    #[gpui::test]
    fn releasing_a_rendered_child_retires_callbacks_while_a_frame_retains_it(
        cx: &mut TestAppContext,
    ) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let view_type = child_type(
            &runtime,
            r#"
import { View, div } from "gpui-kit";
globalThis.child_hits = 0;

export default class Child extends View {
  render(cx) {
    return div()
      .on_click(() => { globalThis.child_hits += 1; })
      .child("child");
  }
}
"#,
        );
        let runtime_for_window = runtime.clone();
        let handle_slot = Rc::new(Cell::new(None));
        let handle_for_window = handle_slot.clone();
        let window = cx.add_window(move |window, cx| {
            let handle = runtime_for_window
                .instantiate_nested_view(&view_type, crate::policy::default(), None, window, cx)
                .expect("child");
            handle_for_window.set(Some(handle));
            ChildMount(
                runtime_for_window
                    .entities()
                    .view(handle)
                    .expect("retained child"),
            )
        });
        let mut context = VisualTestContext::from_window(*window, cx);
        context.update(|window, cx| window.draw(cx).clear(cx));

        let handle = handle_slot.get().expect("child handle");
        let retained_frame = runtime.entities().view(handle).expect("frame entity clone");
        let callback = runtime
            .live_callback_ids()
            .into_iter()
            .next()
            .expect("rendered click callback");
        assert_eq!(runtime.live_callbacks(), 1);

        assert!(context.update(|_, cx| runtime.release_view_handle(handle, cx)));
        assert_eq!(
            runtime.live_callbacks(),
            0,
            "release must retire current and previous callback generations immediately"
        );
        context.update(|window, cx| {
            runtime.dispatch_click(callback, &ClickEvent::default(), window, cx)
        });
        let hits = runtime
            .with_js(|ctx| ctx.globals().get::<_, usize>("child_hits"))
            .expect("child hit count");
        assert_eq!(hits, 0, "a released callback must be inert");
        assert!(runtime.entities().view(handle).is_none());
        assert!(
            !context.update(|_, cx| runtime.release_view_handle(handle, cx)),
            "typed release must reject a stale view handle"
        );

        drop(retained_frame);
        context.update(|window, _| window.remove_window());
        context.run_until_parked();
        drop(context);
        let weak_runtime = Rc::downgrade(&runtime);
        drop(runtime);
        assert!(
            weak_runtime.upgrade().is_none(),
            "retired callbacks must not keep the child and runtime in a cycle"
        );
    }

    #[gpui::test]
    fn nested_init_receives_props_after_the_final_entity_exists(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let view_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui-kit";
import { InputState } from "gpui-base";

export default class Child extends View {
  init(props, cx) {
    this.label = props.label;
    this.tick = cx.timer.every(60_000, () => {});
    Promise.resolve().then(() => {
      this.continuation_input = InputState.new({ value: "continued" });
      this.continued = true;
    });
  }
  render(cx) { return this.label; }
}
"#,
        );
        let props = runtime
            .with_js(|ctx| {
                let props = Object::new(ctx.clone())?;
                props.set("label", "from props")?;
                Ok(Persistent::save(ctx, props.into_value()))
            })
            .expect("props");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let tasks_before = task_count();
        let records_before = runtime.entities().len();

        let handle = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &view_type,
                    crate::policy::default(),
                    Some(props),
                    window,
                    cx,
                )
            })
            .expect("instantiate nested child");

        let view = runtime
            .entities()
            .view(handle)
            .expect("the returned handle retains the child entity");
        assert!(
            runtime.entities().focus(handle).is_none(),
            "a view handle must never resolve as another retained type"
        );
        let object = context.update(|_, cx| view.read(cx).object().clone());
        let label = runtime
            .with_js(|ctx| object.clone().restore(ctx)?.get::<_, String>("label"))
            .expect("read initialized label");
        assert_eq!(label, "from props");
        let continued = runtime
            .with_js(|ctx| object.clone().restore(ctx)?.get::<_, bool>("continued"))
            .expect("read init continuation marker");
        assert!(
            continued,
            "successful init promise jobs must drain before the child scope exits"
        );
        assert_eq!(
            runtime.entities().len(),
            records_before + 2,
            "the continuation's retained state must be owned beside its child view"
        );
        assert_eq!(
            task_count(),
            tasks_before + 1,
            "init work must be registered under the final child owner"
        );

        assert!(context.update(|_, cx| runtime.release_view_handle(handle, cx)));
        assert_eq!(
            task_count(),
            tasks_before,
            "releasing the handle must cancel its exact-owner task even while a frame retains the entity"
        );
        assert_eq!(runtime.entities().len(), records_before);
        drop(view);
        context.update(|_, _| {});
    }

    #[gpui::test]
    fn successful_child_init_does_not_claim_a_preexisting_parent_job(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let parent_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui-kit";
import { InputState } from "gpui-base";
globalThis.parent_continuations = 0;
// Takes a context, because module scope has none: the caller is a live host
// call and hands one in, and the async flavour is still usable when the drain
// runs this `.then` later.
globalThis.queue_parent_job = (cx) => Promise.resolve().then(() => {
  globalThis.parent_continuations += 1;
  globalThis.parent_input = InputState.new({ value: "parent" });
  globalThis.parent_tick = cx.timer.every(60_000, () => {});
});
export default class Parent extends View {
  render() { return "parent"; }
}
"#,
        );
        let child_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui-kit";
import { InputState } from "gpui-base";
globalThis.child_continuations = 0;
export default class Child extends View {
  init(_props, cx) {
    Promise.resolve().then(() => {
      globalThis.child_continuations += 1;
      this.input = InputState.new({ value: "child" });
      this.tick = cx.timer.every(60_000, () => {});
    });
  }
  render(cx) { return "child"; }
}
"#,
        );
        let application = parent_type.application.clone();
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let records_before = runtime.entities().len();
        let tasks_before = task_count();
        let parent = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &parent_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("parent");
        let parent_entity = runtime.entities().view(parent).expect("parent entity");
        let child = context
            .update(|window, cx| {
                let (_scope, _) = scope::enter_with_application(
                    &runtime,
                    window,
                    cx,
                    ScopePhase::Event,
                    Some(parent_entity.clone()),
                    crate::policy::default(),
                    application.clone(),
                );
                runtime
                    .with_js(|ctx| {
                        ctx.globals()
                            .get::<_, Function>("queue_parent_job")?
                            .call::<_, ()>((context_object(ctx, ContextBinding::Ambient)?,))
                    })
                    .expect("queue parent continuation");
                runtime.instantiate_nested_view(
                    &child_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("child");

        assert_eq!(
            runtime
                .with_js(|ctx| ctx.globals().get::<_, usize>("parent_continuations"))
                .expect("parent count"),
            1
        );
        assert_eq!(
            runtime
                .with_js(|ctx| ctx.globals().get::<_, usize>("child_continuations"))
                .expect("child count"),
            1
        );
        assert_eq!(runtime.entities().len(), records_before + 4);
        assert_eq!(task_count(), tasks_before + 2);

        assert!(context.update(|_, cx| runtime.release_view_handle(child, cx)));
        assert_eq!(
            runtime.entities().len(),
            records_before + 2,
            "child release must preserve the parent view and its continuation-owned input"
        );
        assert_eq!(
            task_count(),
            tasks_before + 1,
            "child release must preserve the parent continuation-owned timer"
        );

        assert!(context.update(|_, cx| runtime.release_view_handle(parent, cx)));
        assert_eq!(runtime.entities().len(), records_before);
        assert_eq!(task_count(), tasks_before);
        drop(parent_entity);
        context.update(|_, _| {});
    }

    #[gpui::test]
    fn throwing_child_init_rolls_back_its_job_but_not_a_preexisting_parent_job(
        cx: &mut TestAppContext,
    ) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let parent_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui-kit";
import { InputState } from "gpui-base";
globalThis.parent_continuations = 0;
// Takes a context, because module scope has none: the caller is a live host
// call and hands one in, and the async flavour is still usable when the drain
// runs this `.then` later.
globalThis.queue_parent_job = (cx) => Promise.resolve().then(() => {
  globalThis.parent_continuations += 1;
  globalThis.parent_input = InputState.new({ value: "parent" });
  globalThis.parent_tick = cx.timer.every(60_000, () => {});
});
export default class Parent extends View {
  render() { return "parent"; }
}
"#,
        );
        let child_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui-kit";
import { InputState } from "gpui-base";
globalThis.child_continuations = 0;
export default class BrokenChild extends View {
  init(_props, cx) {
    Promise.resolve().then(() => {
      globalThis.child_continuations += 1;
      this.input = InputState.new({ value: "child" });
      this.tick = cx.timer.every(60_000, () => {});
    });
    throw new Error("mixed init failed");
  }
  render(cx) { return "unreachable"; }
}
"#,
        );
        let application = parent_type.application.clone();
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let records_before = runtime.entities().len();
        let tasks_before = task_count();
        let parent = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &parent_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("parent");
        let parent_entity = runtime.entities().view(parent).expect("parent entity");
        let error = context
            .update(|window, cx| {
                let (_scope, _) = scope::enter_with_application(
                    &runtime,
                    window,
                    cx,
                    ScopePhase::Event,
                    Some(parent_entity.clone()),
                    crate::policy::default(),
                    application.clone(),
                );
                runtime
                    .with_js(|ctx| {
                        ctx.globals()
                            .get::<_, Function>("queue_parent_job")?
                            .call::<_, ()>((context_object(ctx, ContextBinding::Ambient)?,))
                    })
                    .expect("queue parent continuation");
                runtime.instantiate_nested_view(
                    &child_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect_err("child init must fail");

        assert!(error.to_string().contains("mixed init failed"), "{error}");
        assert_eq!(
            runtime
                .with_js(|ctx| ctx.globals().get::<_, usize>("parent_continuations"))
                .expect("parent count"),
            1
        );
        assert_eq!(
            runtime
                .with_js(|ctx| ctx.globals().get::<_, usize>("child_continuations"))
                .expect("child count"),
            1
        );
        assert_eq!(
            runtime.entities().len(),
            records_before + 2,
            "failed child rollback must preserve the parent view and continuation-owned input"
        );
        assert_eq!(
            task_count(),
            tasks_before + 1,
            "failed child rollback must preserve the parent continuation-owned timer"
        );

        assert!(context.update(|_, cx| runtime.release_view_handle(parent, cx)));
        assert_eq!(runtime.entities().len(), records_before);
        assert_eq!(task_count(), tasks_before);
        drop(parent_entity);
        context.update(|_, _| {});
    }

    #[gpui::test]
    fn successor_first_init_chain_fails_the_runtime_and_rolls_back_the_child(
        cx: &mut TestAppContext,
    ) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let parent_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui-kit";
export default class Parent extends View {
  render() { return "parent"; }
}
"#,
        );
        let child_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui-kit";
import { InputState } from "gpui-base";
globalThis.successor_runs = 0;
export default class BrokenChild extends View {
  init(_props, cx) {
    this.input = InputState.new({ value: "candidate" });
    this.tick = cx.timer.every(60_000, () => {});
    const again = () => {
      Promise.resolve().then(again);
      Promise.resolve().then(again);
      globalThis.successor_runs += 1;
    };
    Promise.resolve().then(again);
  }
  render(cx) { return "unreachable"; }
}
"#,
        );
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let parent = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &parent_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("parent");
        let parent_entity = runtime.entities().view(parent).expect("parent entity");
        let records_before = runtime.entities().len();
        let tasks_before = task_count();

        let error = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &child_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect_err("a non-quiescing init wave must fail within the host job bound");

        assert!(error.to_string().contains("job queue"), "{error}");
        assert_eq!(
            runtime.entities().len(),
            records_before,
            "terminal job failure must roll back the candidate child locally"
        );
        assert_eq!(task_count(), tasks_before);
        context.update(|window, cx| {
            scheduler::drain_after_render(
                &runtime,
                parent_entity.clone(),
                crate::policy::default(),
                window,
                cx,
            )
        });
        assert_eq!(
            task_count(),
            tasks_before,
            "terminal pending jobs must not register a later deferred drain"
        );
        let disabled = context.update(|window, cx| {
            runtime.instantiate_nested_view(&child_type, crate::policy::default(), None, window, cx)
        });
        assert!(
            disabled
                .expect_err("the failed runtime must refuse later script execution")
                .to_string()
                .contains("job queue")
        );
        assert!(context.update(|_, cx| runtime.release_view_handle(parent, cx)));
        drop(parent_entity);
    }

    #[gpui::test]
    fn nested_view_retains_the_real_loaded_application_lease(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let root = std::env::temp_dir().join(format!(
            "gpui-shell-nested-view-lease-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("application directory");
        std::fs::write(
            root.join("main.js"),
            r#"
import { div, View } from "gpui-kit";
export default class Child extends View {
  render(cx) { return "loaded child"; }
}
"#,
        )
        .expect("application source");
        let view_type = runtime.load_app(&root, "main.js").expect("loaded app");
        let application = view_type
            .application
            .clone()
            .expect("real application lease");
        assert_eq!(runtime.app_modules.registration_count(), 1);
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let handle = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &view_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("nested loaded child");
        let view = runtime.entities().view(handle).expect("retained child");
        let child_application = context.update(|_, cx| {
            view.read(cx)
                .application_generation()
                .expect("child application")
        });
        assert!(Rc::ptr_eq(&child_application, &application));

        drop(view_type);
        assert_eq!(
            runtime.app_modules.registration_count(),
            1,
            "the retained child object must keep its evaluated module lease"
        );
        context.update(|_, cx| runtime.release_application_generation(&application, cx));
        cancel_application_tasks(&application);
        assert!(
            runtime.entities().view(handle).is_none(),
            "application unload must remove the child's retained handle"
        );
        drop(view);
        drop(child_application);
        drop(application);
        context.update(|_, _| {});
        assert_eq!(runtime.app_modules.registration_count(), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[gpui::test]
    fn releasing_one_child_preserves_its_sibling_and_application_state(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let view_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui-kit";
import { InputState } from "gpui-base";

export default class Child extends View {
  init(_props, cx) {
    this.input = InputState.new();
    this.tick = cx.timer.every(60_000, () => {});
  }
  render(cx) { return "child"; }
}
"#,
        );
        let application = view_type.application.clone().expect("application");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let tasks_before = task_count();
        let application_focus = context
            .update(|_, cx| runtime.entities().create_focus(Some(application), cx))
            .expect("room for one focus handle");
        assert!(
            !context.update(|_, cx| runtime.release_view_handle(application_focus, cx)),
            "typed view release must reject a live handle of another retained type"
        );
        let first = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &view_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("first child");
        let second = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &view_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("second child");

        assert_eq!(task_count(), tasks_before + 2);
        assert!(context.update(|_, cx| runtime.release_view_handle(first, cx)));
        context.update(|_, _| {});

        assert!(runtime.entities().view(first).is_none());
        assert!(
            runtime.entities().view(second).is_some(),
            "releasing one child must preserve its sibling"
        );
        assert!(
            runtime.entities().focus(application_focus).is_some(),
            "nested cleanup must not release application-owned retained state"
        );
        assert_eq!(
            task_count(),
            tasks_before + 1,
            "nested cleanup must cancel only the released child's task"
        );

        assert!(context.update(|_, cx| runtime.release_view_handle(second, cx)));
        assert!(runtime.entities().release(application_focus));
        context.update(|_, _| {});
        assert_eq!(task_count(), tasks_before);
    }

    #[gpui::test]
    fn releasing_a_child_recursively_cancels_retained_descendant_tasks(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let view_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui-kit";

export default class Child extends View {
  init(_props, cx) { this.tick = cx.timer.every(60_000, () => {}); }
  render(cx) { return "child"; }
}
"#,
        );
        let application = view_type.application.clone();
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let tasks_before = task_count();
        let parent = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &view_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("parent child");
        let parent_entity = runtime.entities().view(parent).expect("parent entity");
        let descendant = context
            .update(|window, cx| {
                let (_scope, _) = scope::enter_with_application(
                    &runtime,
                    window,
                    cx,
                    ScopePhase::Event,
                    Some(parent_entity.clone()),
                    crate::policy::default(),
                    application.clone(),
                );
                runtime.instantiate_nested_view(
                    &view_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("descendant child");
        let retained_descendant = runtime
            .entities()
            .view(descendant)
            .expect("descendant entity clone");

        assert_eq!(task_count(), tasks_before + 2);
        assert!(context.update(|_, cx| runtime.release_view_handle(parent, cx)));
        assert!(runtime.entities().view(parent).is_none());
        assert!(runtime.entities().view(descendant).is_none());
        assert_eq!(
            task_count(),
            tasks_before,
            "subtree cleanup must cancel descendant tasks even while GPUI retains their entities"
        );

        drop(retained_descendant);
        drop(parent_entity);
        context.update(|_, _| {});
    }

    #[gpui::test]
    fn exact_view_cancellation_is_qualified_by_runtime_across_apps(cx: &mut TestAppContext) {
        let runtime_a = ShellRuntime::new_isolated().expect("first runtime");
        let runtime_b = ShellRuntime::new_isolated().expect("second runtime");
        let view_type_a = child_type(
            &runtime_a,
            r#"
import { div, View } from "gpui-kit";
export default class Child extends View {
  init(_props, cx) { this.tick = cx.timer.every(60_000, () => {}); }
  render() { return "a"; }
}
"#,
        );
        let view_type_b = child_type(
            &runtime_b,
            r#"
import { div, View } from "gpui-kit";
export default class Child extends View {
  init(_props, cx) { this.tick = cx.timer.every(60_000, () => {}); }
  render(cx) { return "b"; }
}
"#,
        );
        let mut other = cx.new_app();
        let window_a = cx.add_window(|_, _| gpui::Empty);
        let window_b = other.add_window(|_, _| gpui::Empty);
        let mut context_a = VisualTestContext::from_window(*window_a, cx);
        let mut context_b = VisualTestContext::from_window(*window_b, &mut other);
        let tasks_before = task_count();
        let handle_a = context_a
            .update(|window, cx| {
                runtime_a.instantiate_nested_view(
                    &view_type_a,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("first child");
        let handle_b = context_b
            .update(|window, cx| {
                runtime_b.instantiate_nested_view(
                    &view_type_b,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("second child");
        let entity_a = runtime_a.entities().view(handle_a).expect("first entity");
        let entity_b = runtime_b.entities().view(handle_b).expect("second entity");

        assert_eq!(
            entity_a.entity_id(),
            entity_b.entity_id(),
            "fresh Apps must reproduce the local EntityId collision exercised by this test"
        );
        assert!(
            !context_b.update(|_, cx| runtime_b.release_view_handle(handle_a, cx)),
            "typed release must reject a handle from another runtime's store"
        );
        assert!(runtime_b.entities().view(handle_b).is_some());
        assert_eq!(task_count(), tasks_before + 2);
        assert!(context_a.update(|_, cx| runtime_a.release_view_handle(handle_a, cx)));
        assert_eq!(
            task_count(),
            tasks_before + 1,
            "releasing one App's colliding EntityId must preserve the other runtime's task"
        );
        assert!(runtime_b.entities().view(handle_b).is_some());

        assert!(context_b.update(|_, cx| runtime_b.release_view_handle(handle_b, cx)));
        assert_eq!(task_count(), tasks_before);
        drop(entity_a);
        drop(entity_b);
    }

    #[gpui::test]
    fn failed_child_init_rolls_back_only_the_candidate_child(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let view_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui-kit";
import { InputState } from "gpui-base";
globalThis.failed_child_continuations = 0;

export default class BrokenChild extends View {
  init(props, cx) {
    this.input = InputState.new({ value: props.value });
    this.tick = cx.timer.every(60_000, () => {});
    Promise.resolve().then(() => {
      globalThis.failed_child_continuations += 1;
      globalThis.continuation_input = InputState.new({ value: "continued" });
    });
    throw new Error("child init failed");
  }
  render(cx) { return "unreachable"; }
}
"#,
        );
        let application = view_type.application.clone().expect("application");
        let props = runtime
            .with_js(|ctx| {
                let props = Object::new(ctx.clone())?;
                props.set("value", "candidate")?;
                Ok(Persistent::save(ctx, props.into_value()))
            })
            .expect("props");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let application_focus = context
            .update(|_, cx| runtime.entities().create_focus(Some(application), cx))
            .expect("room for one focus handle");
        let records_before = runtime.entities().len();
        let tasks_before = task_count();

        let error = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &view_type,
                    crate::policy::default(),
                    Some(props),
                    window,
                    cx,
                )
            })
            .expect_err("child init must fail");

        assert!(error.to_string().contains("child init failed"), "{error}");
        assert_eq!(
            runtime.entities().len(),
            records_before,
            "the child handle and retained state created by init must roll back"
        );
        assert!(
            runtime.entities().focus(application_focus).is_some(),
            "rollback must preserve application-owned state"
        );
        assert_eq!(
            task_count(),
            tasks_before,
            "rollback must cancel the candidate child's exact-owner task"
        );
        let continuations = runtime
            .with_js(|ctx| ctx.globals().get::<_, usize>("failed_child_continuations"))
            .expect("continuation count");
        assert_eq!(
            continuations, 1,
            "init promise jobs must drain while the candidate child still owns the scope"
        );

        context.update(|window, cx| scheduler::drain_runtime_jobs(&runtime, window, cx));
        assert_eq!(runtime.entities().len(), records_before);

        assert!(runtime.entities().release(application_focus));
    }
}

#[cfg(test)]
mod reserved_element_method_tests {
    #[cfg(debug_assertions)]
    #[test]
    fn debug_runtimes_stay_on_the_interpreter_tier() {
        let config = super::shell_jit_config().expect("JIT configuration");
        assert_eq!(config.call_threshold(), u32::MAX);
        assert_eq!(config.loop_threshold(), u32::MAX);
    }

    /// `typings.rs` withholds these from a registered component that does not
    /// declare them. If the engine started accepting a fourth, the declarations
    /// would keep offering it on every component and the call would throw.
    #[test]
    fn the_declarations_withhold_exactly_the_behaviors_the_engine_gates() {
        assert_eq!(
            crate::typings::REGISTERED_COMMON_BEHAVIORS,
            ["disabled", "selected", "on_click"]
        );
    }

    /// A descriptor method whose name the prelude also defines is unreachable:
    /// the prototype entry wins and validates against a different vocabulary.
    /// `RESERVED_ELEMENT_METHODS` is what stops one being registered, so it has
    /// to say exactly what the prelude actually defines.
    #[test]
    fn the_checked_vocabularies_match_what_the_prelude_enforces() {
        let found = super::prelude_checked_vocabularies();
        let declared = crate::component_registry::prelude_checked_vocabularies_for_test();
        assert_eq!(
            found
                .iter()
                .map(|(name, values)| (name.as_str(), values.len()))
                .collect::<Vec<_>>(),
            declared
                .iter()
                .map(|(name, values)| (*name, values.len()))
                .collect::<Vec<_>>(),
            "the prelude and PRELUDE_CHECKED_VOCABULARIES have drifted; a check \
             added to the prelude must be recorded there, or a registered \
             component can declare a vocabulary that never runs"
        );
        for ((found_name, found_values), (name, values)) in found.iter().zip(declared) {
            assert_eq!(found_name, name);
            assert_eq!(found_values, values, "vocabulary for `{name}`");
        }
    }

    /// The prelude has to keep defining these by hand for the built-in
    /// components, which is what makes the vocabulary check necessary at all.
    #[test]
    fn the_prelude_still_owns_the_names_the_check_covers() {
        let owned = super::prelude_owned_element_methods();
        for (name, _) in crate::component_registry::prelude_checked_vocabularies_for_test() {
            assert!(
                owned.contains(&name),
                "the prelude no longer defines `{name}`"
            );
        }
    }
}
