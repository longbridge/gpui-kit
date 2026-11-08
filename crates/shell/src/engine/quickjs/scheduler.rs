//! Promises, timers and task ownership on GPUI's foreground executor
//! (design doc §12).
//!
//! Script code is asynchronous in the ordinary JavaScript way: `async` functions
//! and native promises. What this module supplies is the half a bare QuickJS
//! runtime does not have — a clock, an owner for pending work, and somebody to
//! pump the job queue.
//!
//! # Which `cx` may be held across an `await`
//!
//! GPUI hands the host `&mut Window` and `&mut App` as borrows that live exactly
//! as long as one call, and offers `AsyncApp` for the code that has to outlive
//! one. The two script flavours mirror that. A call-scoped `cx` is a generation
//! token for one call ([`crate::scope`]); an `await` returns to the host, the
//! frame goes away, and the token is stale when the continuation runs — a clear
//! [`crate::scope::StaleContext`] error rather than undefined behaviour.
//!
//! ```js
//! cx.spawn(async (cx) => {
//!   const data = await load();
//!   cx.notify();              // the async cx: it names no frame to go stale
//! });
//! ```
//!
//! Every resumption this module drives opens a *fresh* [`ScopePhase::Task`]
//! scope, which is what the async flavour resolves against.
//!
//! # The job queue
//!
//! QuickJS keeps promise reactions in a job queue that only runs when the host
//! asks it to. Nothing — no `.then`, no continuation after an `await` — happens
//! until somebody calls [`drain_jobs`]. **The engine must call it after every
//! script entry point** (click, change, timer, load); see that function's
//! documentation for the exact placement, which is inside the entry point's
//! scope and outside `Context::with`.
//!
//! Building a render snapshot is the one entry point that does *not* drain
//! inline. A continuation is arbitrary application code with no bound on how
//! long it runs, and a render is the last place that belongs; see
//! [`drain_after_render`].
//!
//! # Ownership and cancellation
//!
//! Every task retains the application policy active at creation. Tasks may also
//! belong to a view through `opts.owner` or [`scope::current_view`]; that weak
//! owner prevents callbacks from writing into a view that has gone away. The
//! policy is the stable application identity used by load, reload, and unload,
//! including for module-top-level work created before a view exists. Explicit
//! `cancel()` stops work on demand. A cancelled promise is simply never settled.

use crate::engine::quickjs::js_error;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    future::Future,
    rc::Rc,
    time::Duration,
};

use async_channel;
use gpui::{
    AnyWindowHandle, AsyncApp, BackgroundExecutor, Entity, EntityId, ForegroundExecutor, WeakEntity,
};
#[cfg(test)]
use rquickjs::Runtime as JsRuntime;
use rquickjs::{
    Ctx, Exception, FromJs, Function, IntoJs, Object, Persistent, Promise, Result as JsResult,
    Value,
    function::{Func, Opt, This},
};

use crate::{
    policy::Policy,
    runtime::ApplicationGeneration,
    scope::{self, ScopePhase},
    view::ScriptView,
};

use super::{ContextBinding, ShellRuntime, context_object, describe, describe_value};

/// Installs the scheduling surface.
///
/// `spawn`, `sleep` and `timer` reach a script through `cx`, because that is
/// where GPUI keeps them — `App::spawn`, and a timer from the executor the
/// context hands out. These globals are the implementation the prelude composes
/// onto `cx`; nothing names them from script. `with_cx` stays a module member:
/// it is the one call for code holding no context at all, so there is no `cx`
/// to reach it through.
///
/// The context argument is the engine's; everything here is built from the
/// module's own [`Object::ctx`], because `Ctx` and `Object` are invariant in
/// `'js` and a value built from one cannot be set on the other.
pub fn install(_ctx: &Ctx<'_>, module: &Object<'_>) -> JsResult<()> {
    let globals = module.ctx().globals();
    globals.set("__sleep", Func::from(js_sleep))?;
    globals.set("__spawn", Func::from(js_spawn))?;
    globals.set("__timer_after", Func::from(js_timer_after))?;
    globals.set("__timer_every", Func::from(js_timer_every))?;
    let _ = module;
    Ok(())
}

/// Runs every promise reaction QuickJS has queued.
///
/// # The engine must call this after every entry point
///
/// A pending job is script code that has already started: the tail of an
/// `async` function after its `await`, or a `.then` callback. If nobody drains
/// the queue, `await` never resumes and the failure is silent — no error, no
/// log, just a promise that stays pending. So every place that calls into
/// JavaScript ends with a drain:
///
/// - `dispatch_click` / `dispatch_change`, after the handler;
/// - `build_snapshot`, on the next turn of the event loop rather than inline —
///   see [`drain_after_render`];
/// - `load_source` / `instantiate`, after top-level module evaluation;
/// - every resumption this module drives, which is handled here already.
///
/// Two placement rules, both load-bearing:
///
/// 1. **Outside `Context::with`.** [`JsRuntime::execute_pending_job`] takes the
///    runtime lock that `Context::with` is already holding, so calling it from
///    inside a `with_js` closure deadlocks (or, without the `parallel` feature,
///    panics on a re-entrant `RefCell` borrow).
/// 2. **Inside the entry point's [`crate::scope`] guard.** A continuation is
///    resumed script code and will call `gpui.with_cx`, which needs a live
///    scope. Draining after the guard drops leaves that code with no context at
///    all. For an entry point whose own phase is not `Task` — a render pass —
///    open a fresh `ScopePhase::Task` scope around the drain instead.
///
/// A job that throws is reported and the drain continues: one broken
/// continuation must not stop the others.
#[cfg(test)]
pub fn drain_jobs(runtime: &JsRuntime) {
    if drain_job_batch(runtime) {
        return;
    }

    tracing::error!(
        "stopped draining promise jobs after {MAX_JOBS_PER_DRAIN}: a continuation is queueing \
         work faster than it completes. The remaining jobs run at the next entry point."
    );
}

/// Drains the ordinary event-loop batch unless this shell runtime has entered
/// terminal job-queue quarantine.
pub(super) fn drain_runtime_jobs(
    runtime: &Rc<ShellRuntime>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) {
    if runtime.job_queue_error().is_none() {
        if let Err(error) = runtime.flush_pending_nested_views(window, cx) {
            tracing::error!("error applying a nested view operation: {error}");
            return;
        }
        if drain_runtime_job_batch(runtime, window, cx) {
            return;
        }
        tracing::error!(
            "stopped draining promise jobs after {MAX_JOBS_PER_DRAIN}: a continuation is queueing \
             work faster than it completes. The remaining jobs run at the next entry point."
        );
    }
}

/// The production batch pairs every completed QuickJS job with any nested-view
/// operation it requested. Applying that operation here keeps all JavaScript
/// execution outside `Context::with` while the job's owner scope is still live.
fn drain_runtime_job_batch(
    runtime: &Rc<ShellRuntime>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> bool {
    for _ in 0..MAX_JOBS_PER_DRAIN {
        let pending_checkpoint = runtime.pending_nested.borrow().len();
        match runtime.js_runtime.execute_pending_job() {
            Ok(true) => {
                if let Err(error) = runtime.flush_pending_nested_views(window, cx) {
                    tracing::error!("error applying a nested view operation: {error}");
                    if runtime.job_queue_error().is_some() {
                        return true;
                    }
                }
            }
            Ok(false) => return true,
            Err(exception) => {
                runtime
                    .pending_nested
                    .borrow_mut()
                    .truncate(pending_checkpoint);
                let message = exception.0.with(|ctx| {
                    let thrown = ctx.catch();
                    describe_value(&ctx, &thrown)
                });
                tracing::error!("error in a promise continuation: {message}");
            }
        }
    }
    !runtime.js_runtime.is_job_pending()
}

/// Executes at most one event-loop-sized batch and reports queue quiescence.
#[cfg(test)]
fn drain_job_batch(runtime: &JsRuntime) -> bool {
    for _ in 0..MAX_JOBS_PER_DRAIN {
        match runtime.execute_pending_job() {
            Ok(true) => {}
            Ok(false) => return true,
            // `JobException` is not a nameable type outside rquickjs, but its
            // context — the one that threw — is a public field.
            Err(exception) => {
                let message = exception.0.with(|ctx| {
                    let thrown = ctx.catch();
                    describe_value(&ctx, &thrown)
                });
                tracing::error!("error in a promise continuation: {message}");
            }
        }
    }
    !runtime.is_job_pending()
}

/// Drains one causal job wave before an ownership boundary changes.
///
/// The ordinary event-loop drain is deliberately bounded so a busy application
/// yields. Nested view construction cannot yield across owners, but it also
/// cannot loop until quiescence: an interrupted job can leave a successor in
/// QuickJS's opaque queue forever. Stop at a hard total-job limit and quarantine
/// the whole runtime because QuickJS offers no selective pending-job removal.
pub(crate) fn drain_jobs_transactionally(
    runtime: &Rc<ShellRuntime>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> anyhow::Result<()> {
    if let Some(error) = runtime.job_queue_error() {
        return Err(error);
    }
    for _ in 0..MAX_TRANSACTIONAL_JOBS {
        let pending_checkpoint = runtime.pending_nested.borrow().len();
        match runtime.js_runtime.execute_pending_job() {
            Ok(true) => {
                if let Err(error) = runtime.flush_pending_nested_views(window, cx) {
                    if let Some(terminal) = runtime.job_queue_error() {
                        return Err(terminal);
                    }
                    tracing::error!("error applying a nested view operation: {error}");
                }
            }
            Ok(false) => return Ok(()),
            Err(exception) => {
                runtime
                    .pending_nested
                    .borrow_mut()
                    .truncate(pending_checkpoint);
                let message = exception.0.with(|ctx| {
                    let thrown = ctx.catch();
                    describe_value(&ctx, &thrown)
                });
                tracing::error!("error in a transactional promise continuation: {message}");
            }
        }
    }
    if runtime.js_runtime.is_job_pending() {
        Err(runtime.fail_job_queue())
    } else {
        Ok(())
    }
}

/// Queues a drain on GPUI's event loop instead of running one here.
///
/// A promise continuation is application code that has already started, and its
/// running time is unbounded from the renderer's point of view. Running it at
/// the end of a snapshot build would put that time on the path a render took —
/// the same coupling the snapshot lifecycle exists to remove, arriving through a
/// different door.
///
/// So the render path only *notices* that jobs are pending and hands them to the
/// foreground executor. The common case — a render that queued nothing, which is
/// most of them — costs one `is_job_pending` check and no task at all.
///
/// The deferred drain opens its own [`ScopePhase::Task`] scope carrying `view`,
/// so a continuation that calls `cx.notify()` still reaches the view that was
/// rendering when it was queued. A notify from there marks the view for the
/// *next* frame, which is exactly right: the snapshot just published is not
/// invalidated by work that had not finished when it was built.
pub fn drain_after_render(
    runtime: &Rc<ShellRuntime>,
    view: Entity<ScriptView>,
    policy: Rc<Policy>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) {
    // A quarantined runtime deliberately keeps its opaque unfinished queue
    // untouched until drop. Do not register a fresh scheduler task merely
    // because that terminal queue is still observably non-empty.
    if runtime.job_queue_error().is_some() || !runtime.js_runtime.is_job_pending() {
        return;
    }

    let mut task = TaskState::new("deferred render drain", Some(view.downgrade()), None);
    task.policy.replace(Some(policy));
    task.runtime = Rc::downgrade(runtime);
    let task = match try_register(task) {
        Ok(task) => task,
        Err(error) => {
            tracing::error!("deferred drain dropped: {error}");
            return;
        }
    };
    let handle = window.window_handle();
    let mut app = cx.to_async();
    cx.foreground_executor()
        .spawn(async move {
            let entered = handle.update(&mut app, |_, window, cx| {
                let Some(runtime) = task.runtime.upgrade() else {
                    tracing::debug!("deferred drain dropped: the shell runtime has shut down");
                    return;
                };
                let Readiness::Ready(Some(view)) = task.readiness() else {
                    tracing::debug!("deferred drain dropped: its owner or policy was cancelled");
                    return;
                };
                let (guard, _) = scope::enter_with_runtime(
                    &runtime,
                    window,
                    cx,
                    ScopePhase::Task,
                    Some(view),
                    task.policy(),
                );
                drain_runtime_jobs(&runtime, window, cx);
                drop(guard);
            });

            if let Err(error) = entered {
                tracing::debug!("deferred drain dropped: {error}");
            }
            finish(&task);
        })
        .detach();
}

/// Releases every pending task and the script functions they hold.
///
/// The engine should call this from `ShellRuntime::drop`, next to
/// `callbacks.clear()` and for the same reason: a `Persistent` released after
/// its runtime aborts the process.
// Unused until `ShellRuntime::drop` calls it; see the module header's wiring
// notes, which is the one change outside this file the scheduler needs.
#[allow(dead_code)]
pub fn shutdown(runtime: &ShellRuntime) {
    let runtime = runtime as *const ShellRuntime;
    TASKS.with_borrow_mut(|tasks| {
        let owned: Vec<_> = tasks
            .values()
            .filter(|task| task.runtime.as_ptr() == runtime)
            .cloned()
            .collect();
        for task in &owned {
            task.cancelled.set(true);
            task.cancel_work();
            task.callback.replace(None);
            task.rejection.replace(None);
            task.policy.replace(None);
        }
        tasks.retain(|_, task| task.runtime.as_ptr() != runtime);
    });
}

/// Cancels every pending task created under one application policy.
///
/// Plugin unload uses this before dropping its view so owner-less timers and
/// host operations cannot retain or exercise the unloaded plugin's authority.
pub(crate) fn cancel_policy(policy: &Rc<Policy>) {
    TASKS.with_borrow_mut(|tasks| {
        let owned: Vec<_> = tasks
            .values()
            .filter(|task| {
                task.policy
                    .borrow()
                    .as_ref()
                    .is_some_and(|candidate| Rc::ptr_eq(candidate, policy))
            })
            .cloned()
            .collect();
        tasks.retain(|_, task| !owned.iter().any(|owned| owned.id == task.id));
        for task in &owned {
            task.cancelled.set(true);
            task.cancel_work();
            task.callback.replace(None);
            task.rejection.replace(None);
            task.policy.replace(None);
        }
    });
}

/// Cancels work owned by exactly one evaluated application incarnation.
pub(crate) fn cancel_application_generation(generation: &Rc<ApplicationGeneration>) {
    generation.retire();
    cancel_where(|task| {
        task.application
            .as_ref()
            .is_some_and(|candidate| Rc::ptr_eq(candidate, generation))
    });
}

/// Cancels tasks owned by exactly one retained script view.
///
/// Entity identity is stored by the weak owner even after it can no longer be
/// upgraded. The runtime pointer supplies the namespace because GPUI entity ids
/// are only App-local while this registry is shared across the thread.
pub(crate) fn cancel_view(runtime: &Rc<ShellRuntime>, entity_id: EntityId) {
    let runtime = Rc::as_ptr(runtime);
    cancel_where(|task| {
        task.runtime.as_ptr() == runtime
            && task
                .owner
                .as_ref()
                .is_some_and(|owner| owner.entity_id() == entity_id)
    });
}

/// The exact task registry membership before a transactional host operation.
/// The UI and VM are main-thread bound, so no unrelated runtime work can race
/// this checkpoint while the operation is executing synchronously.
pub(super) struct TaskCheckpoint {
    runtime: *const ShellRuntime,
    ids: HashSet<TaskId>,
}

pub(super) fn checkpoint_runtime_tasks(runtime: &Rc<ShellRuntime>) -> TaskCheckpoint {
    let runtime = Rc::as_ptr(runtime);
    let ids = TASKS.with_borrow(|tasks| {
        tasks
            .values()
            .filter(|task| task.runtime.as_ptr() == runtime)
            .map(|task| task.id)
            .collect()
    });
    TaskCheckpoint { runtime, ids }
}

pub(super) fn rollback_runtime_tasks(checkpoint: TaskCheckpoint) {
    cancel_where(|task| {
        task.runtime.as_ptr() == checkpoint.runtime && !checkpoint.ids.contains(&task.id)
    });
}

fn cancel_where(mut predicate: impl FnMut(&TaskState) -> bool) {
    TASKS.with_borrow_mut(|tasks| {
        let cancelled: Vec<_> = tasks
            .values()
            .filter(|task| predicate(task))
            .cloned()
            .collect();
        tasks.retain(|_, task| !cancelled.iter().any(|cancelled| cancelled.id == task.id));
        for task in &cancelled {
            task.cancelled.set(true);
            task.cancel_work();
            task.callback.replace(None);
            task.rejection.replace(None);
            task.policy.replace(None);
        }
    });
}

#[cfg(test)]
pub(crate) fn task_count() -> usize {
    TASKS.with_borrow(|tasks| tasks.len())
}

/// A drain that is bounded so a `for(;;) Promise.resolve().then(f)` cannot wedge
/// the frame loop for ever. It is far above any legitimate burst.
const MAX_JOBS_PER_DRAIN: usize = 100_000;
/// Ownership transitions get a smaller hard wall. Legitimate init work is
/// expected to be tiny; reaching this is a terminal script-runtime failure.
const MAX_TRANSACTIONAL_JOBS: usize = 10_000;

// ---------------------------------------------------------------------------
// The JavaScript surface
// ---------------------------------------------------------------------------

/// `gpui.sleep(ms)` — a promise resolved after `ms` on the foreground executor.
///
/// The delay itself is a background timer, but nothing script-visible ever
/// leaves the main thread: the resolution runs in a fresh `Task` scope, and the
/// continuation it unblocks runs in the drain that follows, inside that same
/// scope.
///
/// A free function rather than a closure because the returned `Promise<'js>`
/// borrows the context lifetime; a closure cannot be inferred as polymorphic
/// over a lifetime that appears in both its parameter and its return type.
fn js_sleep<'js>(ctx: Ctx<'js>, ms: Opt<f64>) -> JsResult<Promise<'js>> {
    const API: &str = "cx.sleep(ms)";

    let delay = duration(&ctx, API, ms.0.unwrap_or_default())?;
    let host = host(&ctx, API)?;
    let (promise, resolve, _reject) = ctx.promise()?;

    let task = register(
        &ctx,
        TaskState::new(
            API,
            scope::current_view().map(|view| view.downgrade()),
            Some(Persistent::save(&ctx, resolve)),
        ),
    )?;

    let sleeping = task.clone();
    host.foreground
        .clone()
        .spawn(async move {
            host.background.timer(delay).await;

            // A cancelled sleep leaves its promise pending for ever, which is
            // what cancellation means for a promise: the continuation does not
            // run, and no error is invented for code that asked to stop.
            if let Readiness::Ready(owner) = sleeping.readiness()
                && let Some(resolve) = sleeping.take_callback()
            {
                resume(
                    &host,
                    &sleeping.policy(),
                    sleeping.application.clone(),
                    owner,
                    move |ctx, _| resolve.restore(ctx)?.call::<_, ()>(()),
                );
            }
            finish(&sleeping);
        })
        .detach();

    Ok(promise)
}

/// `gpui.spawn(asyncFn, opts?)` — calls `asyncFn(cx)` and adopts its promise.
///
/// Adoption is the point. A rejected promise nobody handles is JavaScript's most
/// common silent failure: the work stops, the interface keeps the state it had,
/// and nothing is written anywhere. Here it reaches `tracing::error!` with the
/// script's own stack.
///
/// The `cx` passed to `asyncFn` is valid until its first `await`, and only when
/// there is a host call in progress — at module top level there is no window
/// yet, and the argument is `undefined`.
fn js_spawn<'js>(
    ctx: Ctx<'js>,
    body: Function<'js>,
    opts: Opt<Value<'js>>,
) -> JsResult<Object<'js>> {
    let task = register(
        &ctx,
        TaskState::new(
            "cx.spawn(fn)",
            owner_from_options(&ctx, opts.0.as_ref())?,
            None,
        ),
    )?;

    // An async `cx`, always — including at module top level, where the old
    // call-scoped one had to be omitted and the body was handed `undefined`.
    // It names no frame, so there is nothing to be absent.
    let started = body.call::<_, Value>((context_object(&ctx, ContextBinding::Ambient)?,));

    match started {
        Ok(value) => match value.into_promise() {
            Some(promise) => adopt(&promise, &task)?,
            // A plain function, already finished. Nothing to wait for.
            None => finish(&task),
        },
        Err(error) => {
            tracing::error!("error in gpui.spawn: {}", describe(&ctx, error));
            finish(&task);
        }
    }

    handle_object(&ctx, task.id)
}

/// `gpui.timer.after(ms, fn, opts?)`.
fn js_timer_after<'js>(
    ctx: Ctx<'js>,
    ms: f64,
    handler: Function<'js>,
    opts: Opt<Value<'js>>,
) -> JsResult<Object<'js>> {
    schedule(
        ctx,
        "cx.timer.after(ms, fn)",
        ms,
        handler,
        opts,
        Repeat::Once,
    )
}

/// `gpui.timer.every(ms, fn, opts?)`.
///
/// The interval is measured between the end of one callback and the start of the
/// next wait, so a slow callback delays the following tick rather than piling
/// ticks up behind it. §12.5 leaves throttling an invisible window to host
/// policy; this implementation keeps ticking.
fn js_timer_every<'js>(
    ctx: Ctx<'js>,
    ms: f64,
    handler: Function<'js>,
    opts: Opt<Value<'js>>,
) -> JsResult<Object<'js>> {
    schedule(
        ctx,
        "cx.timer.every(ms, fn)",
        ms,
        handler,
        opts,
        Repeat::Forever,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Repeat {
    Once,
    Forever,
}

fn schedule<'js>(
    ctx: Ctx<'js>,
    api: &'static str,
    ms: f64,
    handler: Function<'js>,
    opts: Opt<Value<'js>>,
    repeat: Repeat,
) -> JsResult<Object<'js>> {
    let delay = duration(&ctx, api, ms)?;
    let host = host(&ctx, api)?;
    let task = register(
        &ctx,
        TaskState::new(
            api,
            owner_from_options(&ctx, opts.0.as_ref())?,
            Some(Persistent::save(&ctx, handler)),
        ),
    )?;

    let ticking = task.clone();
    host.foreground
        .clone()
        .spawn(async move {
            loop {
                host.background.timer(delay).await;

                let Readiness::Ready(owner) = ticking.readiness() else {
                    break;
                };
                let callback = match repeat {
                    Repeat::Once => ticking.take_callback(),
                    Repeat::Forever => ticking.clone_callback(),
                };
                let Some(callback) = callback else { break };

                resume(
                    &host,
                    &ticking.policy(),
                    ticking.application.clone(),
                    owner,
                    // A timer handler is resumed script code like any other, so
                    // its `cx` is the async flavor: a handler that awaits keeps
                    // a usable context instead of having to reach for `with_cx`.
                    move |ctx, _generation| {
                        callback
                            .restore(ctx)?
                            .call::<_, ()>((context_object(ctx, ContextBinding::Ambient)?,))
                    },
                );

                if repeat == Repeat::Once {
                    break;
                }
            }
            finish(&ticking);
        })
        .detach();

    handle_object(&ctx, task.id)
}

/// The script-side task handle: `cancel()` and `is_done()`.
///
/// It carries only the id, so it stays valid after the task has been reaped.
fn handle_object<'js>(ctx: &Ctx<'js>, id: TaskId) -> JsResult<Object<'js>> {
    let handle = Object::new(ctx.clone())?;
    handle.set("cancel", Func::from(move || cancel(id)))?;
    handle.set("is_done", Func::from(move || is_done(id)))?;
    Ok(handle)
}

/// Attaches the reporting handlers to a spawned task's promise.
fn adopt<'js>(promise: &Promise<'js>, task: &Rc<TaskState>) -> JsResult<()> {
    let settled = task.clone();
    let failed = task.clone();

    // `then` rather than `catch` so success also marks the task done; the
    // success handler takes no arguments because the resolved value is not ours.
    promise.then()?.call::<_, ()>((
        This(promise.clone()),
        Func::from(move || finish(&settled)),
        Func::from(move |failure: ScriptFailure| {
            if !failed.cancelled.get() {
                tracing::error!("unhandled rejection in gpui.spawn: {}", failure.0);
            }
            finish(&failed);
        }),
    ))
}

/// A rejection value, already rendered as a message.
///
/// The formatting has to happen in `FromJs`: a closure taking both `Ctx<'js>`
/// and `Value<'js>` cannot be inferred, because the two elided lifetimes are
/// separate parameters as far as inference is concerned. Inside `from_js` they
/// are one lifetime again. `Arguments` in the parent module exists for the same
/// reason.
struct ScriptFailure(String);

impl<'js> FromJs<'js> for ScriptFailure {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        Ok(Self(describe_value(ctx, &value)))
    }
}

// ---------------------------------------------------------------------------
// Resumption
// ---------------------------------------------------------------------------

/// Everything a deferred task needs to get back onto the main thread.
///
/// Captured while a host call is in progress, because that is the only time the
/// window handle and the executors are reachable.
#[derive(Clone)]
pub(super) struct Host {
    window: AnyWindowHandle,
    app: AsyncApp,
    foreground: ForegroundExecutor,
    background: BackgroundExecutor,
    runtime: std::rc::Weak<ShellRuntime>,
}

/// The only part of an actor-owned promise that crosses threads.
///
/// It deliberately contains no QuickJS value, foreground executor or task
/// state. Its receiver is checked by a foreground task created with the normal
/// task registry, so resumption retains the usual ownership and scope rules.
pub(super) struct ActorCompletion<T = ()>(async_channel::Sender<std::result::Result<T, String>>);

impl<T> ActorCompletion<T> {
    pub(super) fn settle(self, outcome: std::result::Result<T, String>) {
        let _ = self.0.try_send(outcome);
    }
}

/// The executors and window handle a deferred call needs, taken from the scope
/// that is running now. Held rather than re-derived, so work that outlives the
/// call — a queue that drives itself — does not need a `Ctx` it cannot have.
pub(super) fn host_for(ctx: &Ctx<'_>, api: &str) -> JsResult<Host> {
    host(ctx, api)
}

fn host(ctx: &Ctx<'_>, api: &str) -> JsResult<Host> {
    let Some(generation) = scope::current_generation() else {
        return Err(outside_host_call(ctx, api));
    };

    let runtime = scope::current_runtime()
        .ok_or_else(|| js_error::type_error(ctx, &format!("{api} has no owning shell runtime")))?;
    scope::with_context(generation, |window, app| Host {
        window: window.window_handle(),
        app: app.to_async(),
        foreground: app.foreground_executor().clone(),
        background: app.background_executor().clone(),
        runtime: Rc::downgrade(&runtime),
    })
    .map_err(|error| js_error::type_error(ctx, &error.to_string()))
}

/// Re-enters script code in a fresh [`ScopePhase::Task`] scope.
///
/// The three steps are one unit and their order is the contract: open the scope,
/// call the script, drain the jobs the call queued — all before the guard drops,
/// because a continuation resumed by that drain is script code that will ask for
/// a `cx` of its own.
fn resume(
    host: &Host,
    policy: &Rc<Policy>,
    application: Option<Rc<ApplicationGeneration>>,
    owner: Option<Entity<ScriptView>>,
    body: impl FnOnce(&Ctx<'_>, u64) -> JsResult<()>,
) {
    let policy = policy.clone();
    let mut app = host.app.clone();
    let entered = host.window.update(&mut app, |_, window, cx| {
        let Some(runtime) = host.runtime.upgrade() else {
            tracing::debug!("script task dropped: the shell runtime has already shut down");
            return;
        };

        // `enter_with` and not `enter`: there is no enclosing frame to inherit
        // from out here, so `enter` would reach for the default policy.
        let (guard, generation) = scope::enter_with_application(
            &runtime,
            window,
            cx,
            ScopePhase::Task,
            owner,
            policy,
            application,
        );
        if let Err(error) = runtime.with_js(|ctx| body(ctx, generation)) {
            tracing::error!("error in script task: {error}");
        }
        drain_runtime_jobs(&runtime, window, cx);
        drop(guard);
    });

    if let Err(error) = entered {
        tracing::debug!("script task dropped: {error}");
    }
}

// ---------------------------------------------------------------------------
// The task registry
// ---------------------------------------------------------------------------

type TaskId = u64;

thread_local! {
    /// Live tasks, so `cancel()` can reach work that has already been handed to
    /// the executor, and so shutdown can release the script functions they hold.
    /// A thread-local and not a field on the runtime because the VM, GPUI's
    /// `App` and every task here are all main-thread only.
    static TASKS: RefCell<HashMap<TaskId, Rc<TaskState>>> = RefCell::new(HashMap::new());
    static NEXT_TASK_ID: Cell<TaskId> = const { Cell::new(1) };
}

struct TaskState {
    id: TaskId,
    /// The JavaScript spelling of whatever created this task, for diagnostics.
    api: &'static str,
    /// `None` means the task was created with `owner: null` and outlives every
    /// view deliberately.
    owner: Option<WeakEntity<ScriptView>>,
    /// The authority the task was created under.
    ///
    /// Captured here rather than derived from the owner when the task resumes,
    /// because an owner-less task has nothing to derive it from and would fall
    /// back to the default policy — which for a plugin means running its timer
    /// with the *host application's* permissions. A plugin's module top level
    /// and its `init` both create tasks before its view exists, so this is the
    /// ordinary case rather than an edge one.
    policy: RefCell<Option<Rc<Policy>>>,
    /// The evaluated application incarnation that created this work.
    application: Option<Rc<ApplicationGeneration>>,
    runtime: std::rc::Weak<ShellRuntime>,
    cancelled: Cell<bool>,
    done: Cell<bool>,
    /// The script function to resume with: a timer handler, or a promise's
    /// `resolve`. Held here rather than in the future so [`shutdown`] can
    /// release it while the QuickJS runtime is still alive.
    callback: RefCell<Option<Persistent<Function<'static>>>>,
    /// A promise's `reject`, for work that can fail. Held for the same reason
    /// and released at the same time.
    rejection: RefCell<Option<Persistent<Function<'static>>>>,
    cancellation: RefCell<Option<Rc<dyn Fn()>>>,
}

/// Whether a task may run now, and the owner its scope belongs to.
enum Readiness {
    Ready(Option<Entity<ScriptView>>),
    Cancelled,
    OwnerGone,
}

impl TaskState {
    fn new(
        api: &'static str,
        owner: Option<WeakEntity<ScriptView>>,
        callback: Option<Persistent<Function<'static>>>,
    ) -> Self {
        Self {
            id: NEXT_TASK_ID.with(|next| {
                let id = next.get();
                next.set(id + 1);
                id
            }),
            api,
            owner,
            policy: RefCell::new(Some(scope::policy())),
            application: scope::current_application_generation(),
            runtime: scope::current_runtime()
                .map_or_else(std::rc::Weak::new, |runtime| Rc::downgrade(&runtime)),
            cancelled: Cell::new(false),
            done: Cell::new(false),
            rejection: RefCell::new(None),
            cancellation: RefCell::new(None),
            callback: RefCell::new(callback),
        }
    }

    fn readiness(&self) -> Readiness {
        if self.cancelled.get()
            || self.done.get()
            || self.policy.borrow().is_none()
            || self
                .application
                .as_ref()
                .is_some_and(|generation| !generation.is_active())
        {
            return Readiness::Cancelled;
        }
        match &self.owner {
            None => Readiness::Ready(None),
            Some(owner) => match owner.upgrade() {
                Some(view) => Readiness::Ready(Some(view)),
                None => Readiness::OwnerGone,
            },
        }
    }

    fn policy(&self) -> Rc<Policy> {
        self.policy
            .borrow()
            .clone()
            .expect("a ready scheduler task must retain its policy")
    }

    fn take_callback(&self) -> Option<Persistent<Function<'static>>> {
        self.callback.borrow_mut().take()
    }

    fn clone_callback(&self) -> Option<Persistent<Function<'static>>> {
        self.callback.borrow().clone()
    }

    fn with_rejection(self, reject: Persistent<Function<'static>>) -> Self {
        self.rejection.replace(Some(reject));
        self
    }

    fn with_cancellation(self, cancel: impl Fn() + 'static) -> Self {
        self.cancellation.replace(Some(Rc::new(cancel)));
        self
    }

    #[cfg(test)]
    fn with_application(mut self, generation: Rc<ApplicationGeneration>) -> Self {
        self.application = Some(generation);
        self
    }

    #[cfg(test)]
    fn with_runtime(mut self, runtime: &Rc<ShellRuntime>) -> Self {
        self.runtime = Rc::downgrade(runtime);
        self
    }

    fn cancel_work(&self) {
        if let Some(cancel) = self.cancellation.borrow_mut().take() {
            cancel();
        }
    }

    fn take_rejection(&self) -> Option<Persistent<Function<'static>>> {
        self.rejection.borrow_mut().take()
    }
}

/// Work whose result nobody awaits, run off the main thread.
///
/// The store's write queue is what this exists for. Nobody asked for the write,
/// so there is no call to fail — a failure is logged, and a script that wants to
/// be told awaits `flush` instead.
///
/// Takes a [`Host`] rather than a `Ctx` because the completion drives the queue
/// again, and out there no script call is in progress to take one from.
pub(super) fn detached_on(
    host: &Host,
    work: impl FnOnce() -> Result<(), String> + Send + 'static,
    done: impl FnOnce(Result<(), String>) + 'static,
) -> bool {
    let host = host.clone();
    host.foreground
        .clone()
        .spawn(async move {
            done(host.background.spawn(async move { work() }).await);
        })
        .detach();
    true
}

/// How often a [`stream`] looks for events that have arrived.
const STREAM_POLL: Duration = Duration::from_millis(10);

/// One event a [`stream`] delivers to its script callback.
pub(super) trait StreamEvent: Send + 'static {
    /// The arguments the callback receives ahead of its `cx`.
    fn into_args<'js>(self, ctx: &Ctx<'js>) -> JsResult<(&'static str, Value<'js>)>;
    /// Whether nothing follows this event.
    fn is_last(&self) -> bool;
}

/// Delivers events produced off the main thread to a script callback, for as
/// long as they keep coming.
///
/// The callback is called as `dispatch(name, payload, cx)` on the UI thread,
/// with the async `cx` a timer handler gets, so it can `cx.notify()` and keep
/// the context across an `await`. Events that arrived together are delivered
/// in one host call, in order.
///
/// The stream is a task like any other: owned by the view that started it,
/// cancelled when that view is released, when its application is reloaded or
/// with the window — and `cancel` is how the producer is told to stop.
pub(super) fn stream<'js, E: StreamEvent>(
    ctx: &Ctx<'js>,
    api: &'static str,
    dispatch: Function<'js>,
    events: async_channel::Receiver<E>,
    cancel: impl Fn() + 'static,
) -> JsResult<()> {
    let host = host(ctx, api)?;
    let task = register(
        ctx,
        TaskState::new(
            api,
            scope::current_view().map(|view| view.downgrade()),
            Some(Persistent::save(ctx, dispatch)),
        )
        .with_cancellation(cancel),
    )?;

    let running = task.clone();
    let foreground = host.foreground.scheduler_executor();
    host.foreground
        .clone()
        .spawn(async move {
            let mut ended = false;
            loop {
                // Polled like an actor's completion rather than awaited: the
                // producer is an OS thread, and a wake from one is not a
                // foreground schedule GPUI's deterministic test executor
                // accepts.
                let mut batch = Vec::new();
                loop {
                    match events.try_recv() {
                        Ok(event) => batch.push(event),
                        Err(async_channel::TryRecvError::Empty) => break,
                        Err(async_channel::TryRecvError::Closed) => {
                            ended = true;
                            break;
                        }
                    }
                }
                if batch.is_empty() {
                    if ended {
                        break;
                    }
                    foreground.timer(STREAM_POLL).await;
                    continue;
                }
                ended = ended || batch.iter().any(StreamEvent::is_last);

                let Readiness::Ready(owner) = running.readiness() else {
                    break;
                };
                let Some(callback) = running.clone_callback() else {
                    break;
                };
                resume(
                    &host,
                    &running.policy(),
                    running.application.clone(),
                    owner,
                    move |ctx, _generation| {
                        let dispatch = callback.restore(ctx)?;
                        for event in batch {
                            let (name, payload) = event.into_args(ctx)?;
                            // One handler that throws is reported and does
                            // not cost the events after it.
                            if let Err(error) = dispatch.call::<_, ()>((
                                name,
                                payload,
                                context_object(ctx, ContextBinding::Ambient)?,
                            )) {
                                tracing::error!(
                                    "error in {api} {name} handler: {}",
                                    describe(ctx, error)
                                );
                            }
                        }
                        Ok(())
                    },
                );
                if ended {
                    break;
                }
            }
            // Stopped for any reason but the end — the owner went away, the
            // task was cancelled — stops the producer too.
            if !ended {
                running.cancel_work();
            }
            finish(&running);
        })
        .detach();
    Ok(())
}

/// A promise something else will settle later.
///
/// [`blocking`] settles when its own work finishes. This is for a promise whose
/// outcome is decided elsewhere — a `flush` waiting on a write another call
/// started — so the resolver comes back as a closure the deciding code calls,
/// once, from the main thread.
pub(super) fn deferred<'js>(
    ctx: &Ctx<'js>,
    api: &'static str,
    resolve: Function<'js>,
    reject: Function<'js>,
) -> JsResult<crate::storage::Settle> {
    let host = host(ctx, api)?;
    let task = register(
        ctx,
        TaskState::new(
            api,
            scope::current_view().map(|view| view.downgrade()),
            Some(Persistent::save(ctx, resolve)),
        )
        .with_rejection(Persistent::save(ctx, reject)),
    )?;
    Ok(Box::new(move |outcome| {
        if let Readiness::Ready(owner) = task.readiness() {
            let resolve = task.take_callback();
            let reject = task.take_rejection();
            let policy = task.policy();
            resume(
                &host,
                &policy,
                task.application.clone(),
                owner,
                move |ctx, _| match outcome {
                    Ok(()) => match resolve {
                        Some(resolve) => resolve.restore(ctx)?.call::<_, ()>(()),
                        None => Ok(()),
                    },
                    Err(message) => match reject {
                        Some(reject) => {
                            let error = Exception::from_message(ctx.clone(), &message)?;
                            reject.restore(ctx)?.call::<_, ()>((error,))
                        }
                        None => Ok(()),
                    },
                },
            );
        }
        finish(&task);
    }))
}

/// Creates a promise settled by an actor thread without moving any QuickJS or
/// foreground-executor state off the main thread.
pub(super) fn actor_deferred<'js>(
    ctx: &Ctx<'js>,
    api: &'static str,
) -> JsResult<(Promise<'js>, ActorCompletion)> {
    let host = host(ctx, api)?;
    let (promise, resolve, reject) = ctx.promise()?;
    let task = register(
        ctx,
        TaskState::new(
            api,
            scope::current_view().map(|view| view.downgrade()),
            Some(Persistent::save(ctx, resolve)),
        )
        .with_rejection(Persistent::save(ctx, reject)),
    )?;
    let (completion, receiver) = async_channel::bounded(1);
    let running = task.clone();
    let foreground = host.foreground.scheduler_executor();
    host.foreground
        .clone()
        .spawn(async move {
            let outcome = loop {
                match receiver.try_recv() {
                    Ok(outcome) => break outcome,
                    Err(async_channel::TryRecvError::Closed) => {
                        break Err("actor stopped before completing the operation".to_owned());
                    }
                    Err(async_channel::TryRecvError::Empty) => {
                        foreground.timer(Duration::from_millis(10)).await;
                    }
                }
            };
            settle_actor_task(&running, &host, outcome);
        })
        .detach();
    Ok((promise, ActorCompletion(completion)))
}

fn settle_actor_task(task: &Rc<TaskState>, host: &Host, outcome: std::result::Result<(), String>) {
    if let Readiness::Ready(owner) = task.readiness() {
        let resolve = task.take_callback();
        let reject = task.take_rejection();
        resume(
            &host,
            &task.policy(),
            task.application.clone(),
            owner,
            move |ctx, _| match outcome {
                Ok(()) => match resolve {
                    Some(resolve) => resolve.restore(ctx)?.call::<_, ()>(()),
                    None => Ok(()),
                },
                Err(message) => match reject {
                    Some(reject) => {
                        let error = Exception::from_message(ctx.clone(), &message)?;
                        reject.restore(ctx)?.call::<_, ()>((error,))
                    }
                    None => Ok(()),
                },
            },
        );
    }
    finish(&task);
}

/// Creates a promise whose Send value is supplied by a socket or other actor.
///
/// The actor sends exactly one outcome. Its foreground receiver checks the
/// channel on a short timer, never occupying a shared background worker, so a
/// pending read cannot prevent a write or close command from being submitted to
/// the actor.
pub(super) fn actor_blocking<'js, T>(
    ctx: &Ctx<'js>,
    api: &'static str,
) -> JsResult<(Promise<'js>, ActorCompletion<T>)>
where
    T: for<'a> IntoJs<'a> + Send + 'static,
{
    let host = host(ctx, api)?;
    let (promise, resolve, reject) = ctx.promise()?;
    let task = register(
        ctx,
        TaskState::new(
            api,
            scope::current_view().map(|view| view.downgrade()),
            Some(Persistent::save(ctx, resolve)),
        )
        .with_rejection(Persistent::save(ctx, reject)),
    )?;
    let (completion, receiver) = async_channel::bounded(1);
    let running = task.clone();
    let foreground = host.foreground.scheduler_executor();
    host.foreground
        .clone()
        .spawn(async move {
            let outcome = loop {
                match receiver.try_recv() {
                    Ok(outcome) => break outcome,
                    Err(async_channel::TryRecvError::Closed) => {
                        break Err("actor stopped before completing the operation".to_owned());
                    }
                    Err(async_channel::TryRecvError::Empty) => {
                        foreground.timer(Duration::from_millis(10)).await;
                    }
                }
            };
            if let Readiness::Ready(owner) = running.readiness() {
                let resolve = running.take_callback();
                let reject = running.take_rejection();
                resume(
                    &host,
                    &running.policy(),
                    running.application.clone(),
                    owner,
                    move |ctx, _| match outcome {
                        Ok(value) => match resolve {
                            Some(resolve) => resolve.restore(ctx)?.call::<_, ()>((value,)),
                            None => Ok(()),
                        },
                        Err(message) => match reject {
                            Some(reject) => {
                                let error = Exception::from_message(ctx.clone(), &message)?;
                                reject.restore(ctx)?.call::<_, ()>((error,))
                            }
                            None => Ok(()),
                        },
                    },
                );
            }
            finish(&running);
        })
        .detach();
    Ok((promise, ActorCompletion(completion)))
}

/// A promise driven by a future on the background executor.
///
/// [`blocking`] is this with a synchronous closure in front; this one takes the
/// future directly, which is what an asynchronous HostModule function hands
/// over. Ownership, cancellation and scope restoration are identical — a call
/// whose view has gone away leaves its promise pending rather than inventing an
/// error for code that was asked to stop.
pub(super) fn awaiting<'js, T>(
    ctx: &Ctx<'js>,
    api: &'static str,
    work: impl Future<Output = Result<T, String>> + Send + 'static,
) -> JsResult<Promise<'js>>
where
    T: for<'a> IntoJs<'a> + Send + 'static,
{
    let host = host(ctx, api)?;
    let (promise, resolve, reject) = ctx.promise()?;

    let task = register(
        ctx,
        TaskState::new(
            api,
            scope::current_view().map(|view| view.downgrade()),
            Some(Persistent::save(ctx, resolve)),
        )
        .with_rejection(Persistent::save(ctx, reject)),
    )?;

    let running = task.clone();
    host.foreground
        .clone()
        .spawn(async move {
            let outcome = host.background.spawn(work).await;

            if let Readiness::Ready(owner) = running.readiness() {
                let resolve = running.take_callback();
                let reject = running.take_rejection();
                resume(
                    &host,
                    &running.policy(),
                    running.application.clone(),
                    owner,
                    move |ctx, _| match outcome {
                        Ok(value) => match resolve {
                            Some(resolve) => resolve.restore(ctx)?.call::<_, ()>((value,)),
                            None => Ok(()),
                        },
                        Err(message) => match reject {
                            Some(reject) => {
                                let error = Exception::from_message(ctx.clone(), &message)?;
                                reject.restore(ctx)?.call::<_, ()>((error,))
                            }
                            None => Ok(()),
                        },
                    },
                );
            }
            finish(&running);
        })
        .detach();

    Ok(promise)
}

/// A promise settled by work that must not run on this thread.
///
/// The filesystem surface is what this exists for. A capability check is cheap
/// and stays here, on the calling thread, so a denial is still a thrown error at
/// the call site rather than a rejected promise nobody awaited. The syscall
/// behind it is not cheap and has no bound: a network volume, a cold disk or a
/// large file blocks for as long as it likes, and blocking here blocks the frame
/// *and* the VM — the interrupt budget cannot even see it, because the time is
/// spent in the kernel rather than in script.
///
/// So `work` runs on the background executor and the promise settles back on the
/// main thread, in a scope of its own like any other resumption. A cancelled
/// task leaves its promise pending for ever, which is what cancellation means.
pub(super) fn blocking<'js, T>(
    ctx: &Ctx<'js>,
    api: &'static str,
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> JsResult<Promise<'js>>
where
    T: for<'a> IntoJs<'a> + Send + 'static,
{
    let host = host(ctx, api)?;
    let (promise, resolve, reject) = ctx.promise()?;

    let task = register(
        ctx,
        TaskState::new(
            api,
            scope::current_view().map(|view| view.downgrade()),
            Some(Persistent::save(ctx, resolve)),
        )
        .with_rejection(Persistent::save(ctx, reject)),
    )?;

    let running = task.clone();
    host.foreground
        .clone()
        .spawn(async move {
            let outcome = host.background.spawn(async move { work() }).await;

            if let Readiness::Ready(owner) = running.readiness() {
                let resolve = running.take_callback();
                let reject = running.take_rejection();
                resume(
                    &host,
                    &running.policy(),
                    running.application.clone(),
                    owner,
                    move |ctx, _| match outcome {
                        Ok(value) => match resolve {
                            Some(resolve) => resolve.restore(ctx)?.call::<_, ()>((value,)),
                            None => Ok(()),
                        },
                        Err(message) => match reject {
                            Some(reject) => {
                                let error = Exception::from_message(ctx.clone(), &message)?;
                                reject.restore(ctx)?.call::<_, ()>((error,))
                            }
                            None => Ok(()),
                        },
                    },
                );
            }
            finish(&running);
        })
        .detach();

    Ok(promise)
}

/// [`blocking`] with a hook that interrupts the underlying host operation.
pub(super) fn blocking_cancellable<'js, T>(
    ctx: &Ctx<'js>,
    api: &'static str,
    cancel: impl Fn() + 'static,
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> JsResult<Promise<'js>>
where
    T: for<'a> IntoJs<'a> + Send + 'static,
{
    let host = host(ctx, api)?;
    let (promise, resolve, reject) = ctx.promise()?;
    let task = register(
        ctx,
        TaskState::new(
            api,
            scope::current_view().map(|view| view.downgrade()),
            Some(Persistent::save(ctx, resolve)),
        )
        .with_rejection(Persistent::save(ctx, reject))
        .with_cancellation(cancel),
    )?;

    let running = task.clone();
    let watching = task.clone();
    let monitor_background = host.background.clone();
    host.foreground
        .clone()
        .spawn(async move {
            loop {
                monitor_background.timer(Duration::from_millis(25)).await;
                match watching.readiness() {
                    Readiness::Ready(_) => {}
                    Readiness::OwnerGone => {
                        watching.cancelled.set(true);
                        watching.cancel_work();
                        watching.callback.replace(None);
                        watching.rejection.replace(None);
                        break;
                    }
                    Readiness::Cancelled => break,
                }
            }
        })
        .detach();
    host.foreground
        .clone()
        .spawn(async move {
            let outcome = host.background.spawn(async move { work() }).await;
            if let Readiness::Ready(owner) = running.readiness() {
                let resolve = running.take_callback();
                let reject = running.take_rejection();
                resume(
                    &host,
                    &running.policy(),
                    running.application.clone(),
                    owner,
                    move |ctx, _| match outcome {
                        Ok(value) => match resolve {
                            Some(resolve) => resolve.restore(ctx)?.call::<_, ()>((value,)),
                            None => Ok(()),
                        },
                        Err(message) => match reject {
                            Some(reject) => {
                                let error = Exception::from_message(ctx.clone(), &message)?;
                                reject.restore(ctx)?.call::<_, ()>((error,))
                            }
                            None => Ok(()),
                        },
                    },
                );
            }
            finish(&running);
        })
        .detach();

    Ok(promise)
}

const MAX_OUTSTANDING_TASKS_PER_RUNTIME: usize = 1024;

fn register<'js>(ctx: &Ctx<'js>, task: TaskState) -> JsResult<Rc<TaskState>> {
    try_register(task).map_err(|message| js_error::range_error(ctx, &message))
}

fn try_register(task: TaskState) -> std::result::Result<Rc<TaskState>, String> {
    let runtime = task.runtime.as_ptr();
    let outstanding = TASKS.with_borrow(|tasks| {
        tasks
            .values()
            .filter(|running| running.runtime.as_ptr() == runtime)
            .count()
    });
    if outstanding >= MAX_OUTSTANDING_TASKS_PER_RUNTIME {
        return Err(format!(
            "{} exceeded the per-runtime outstanding host task limit of {MAX_OUTSTANDING_TASKS_PER_RUNTIME}",
            task.api
        ));
    }
    Ok(register_unchecked(task))
}

fn register_unchecked(task: TaskState) -> Rc<TaskState> {
    let task = Rc::new(task);
    TASKS.with_borrow_mut(|tasks| tasks.insert(task.id, task.clone()));
    task
}

/// Marks a task finished and drops it from the registry.
///
/// Reaping here rather than sweeping later keeps a long-lived application from
/// accumulating one entry per elapsed timer.
fn finish(task: &Rc<TaskState>) {
    task.done.set(true);
    task.callback.replace(None);
    task.cancellation.replace(None);
    task.policy.replace(None);
    TASKS.with_borrow_mut(|tasks| tasks.remove(&task.id));
}

/// `handle.cancel()`.
///
/// A timer stops before its next tick and a sleep's promise is never settled.
/// JavaScript has no way to interrupt a promise chain that is already running,
/// so cancelling a `gpui.spawn` means the runtime stops adopting its outcome —
/// the chain itself runs on until its next host-owned await point, where the
/// owner and cancellation checks apply again.
fn cancel(id: TaskId) {
    let Some(task) = TASKS.with_borrow_mut(|tasks| tasks.remove(&id)) else {
        return;
    };
    tracing::trace!("cancelled {} (task {id})", task.api);
    task.cancelled.set(true);
    task.done.set(true);
    task.cancel_work();
    task.callback.replace(None);
    task.rejection.replace(None);
    task.policy.replace(None);
}

/// `handle.is_done()`. A task the registry has forgotten has run or been
/// cancelled; either way there is nothing more to wait for.
fn is_done(id: TaskId) -> bool {
    TASKS.with_borrow(|tasks| tasks.get(&id).is_none_or(|task| task.done.get()))
}

// ---------------------------------------------------------------------------
// Arguments and diagnostics
// ---------------------------------------------------------------------------

/// Reads `opts.owner`, defaulting to the view whose call is in progress.
///
/// `null` is the deliberate opt-out for work that must outlive its view. Any
/// other view is refused rather than silently ignored: the engine can only
/// resolve the current view's script instance back to its entity, and a task
/// that quietly took the wrong owner would keep running after the panel it was
/// meant to follow had closed — exactly the bug ownership exists to prevent.
fn owner_from_options<'js>(
    ctx: &Ctx<'js>,
    opts: Option<&Value<'js>>,
) -> JsResult<Option<WeakEntity<ScriptView>>> {
    let current = scope::current_view();
    let default = || Ok(current.as_ref().map(Entity::downgrade));

    let Some(options) = opts.and_then(Value::as_object) else {
        return default();
    };
    let owner: Value = options.get("owner")?;
    if owner.is_undefined() {
        return default();
    }
    if owner.is_null() {
        return Ok(None);
    }

    let Some(owner) = owner.as_object() else {
        return Err(js_error::type_error(
            ctx,
            "opts.owner must be a view instance, or null for a task that outlives every view",
        ));
    };

    let is_current = current
        .as_ref()
        .and_then(|view| scope::with_current_app(|app| view.read(app).object().clone()))
        .and_then(|object| object.restore(ctx).ok())
        .is_some_and(|object| &object == owner);

    if is_current {
        default()
    } else {
        Err(js_error::type_error(
            ctx,
            "opts.owner must be the view that is running, or null; a task cannot yet be owned \
             by another view",
        ))
    }
}

fn duration(ctx: &Ctx<'_>, api: &str, ms: f64) -> JsResult<Duration> {
    if !ms.is_finite() || ms < 0.0 {
        return Err(js_error::type_error(
            ctx,
            &format!("{api} expects a non-negative number of milliseconds, got {ms}"),
        ));
    }
    Duration::try_from_secs_f64(ms / 1000.0).map_err(|_| {
        js_error::range_error(
            ctx,
            &format!("{api} milliseconds are outside the supported duration range"),
        )
    })
}

fn outside_host_call(ctx: &Ctx<'_>, api: &str) -> rquickjs::Error {
    js_error::type_error(
        ctx,
        &format!(
            "{api} was called with no host call in progress. It needs the window and app of \
             a live call, so call it from render, from an event handler, or from a task the \
             scheduler resumed — not from a bare promise callback."
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ViewObject;
    use gpui::{AppContext as _, TestAppContext, VisualTestContext};
    use rquickjs::{Context as JsContext, Object};
    use std::ops::Deref as _;

    fn context() -> (JsRuntime, JsContext) {
        let runtime = JsRuntime::new().expect("runtime");
        let context = rquickjs::Context::full(&runtime).expect("context");
        (runtime, context)
    }

    /// The scheduler alone on `globalThis.gpui`, which is all these tests need.
    fn install_module(ctx: &Ctx<'_>) {
        let module = Object::new(ctx.clone()).expect("module");
        install(ctx, &module).expect("install");
        ctx.globals().set("gpui", module).expect("globals");

        // `cx` composes its members in the prelude, which these tests do not
        // load. The factory is the one piece of it `context_object` reaches
        // for, so stand in for it rather than teaching the runtime to cope
        // with a half-built module.
        ctx.eval::<Value, _>(
            "globalThis.__gpui = { __context_members: (check) => ({ theme: () => (check(), null) }) };",
        )
        .expect("context members");
    }

    /// The failure this module exists to prevent: without a drain, a resolved
    /// promise's `.then` never runs and nothing says so.
    #[test]
    fn drain_jobs_runs_pending_promise_callbacks() {
        let (runtime, context) = context();

        context.with(|ctx| {
            ctx.eval::<Value, _>(
                r#"
                globalThis.resumed = false;
                globalThis.settle = null;
                globalThis.promise = new Promise((resolve) => { globalThis.settle = resolve; })
                    .then(() => { globalThis.resumed = true; });
                "#,
            )
            .expect("eval");
            ctx.eval::<Value, _>("settle()").expect("settle");

            assert!(
                !ctx.globals().get::<_, bool>("resumed").expect("resumed"),
                "a reaction must not run before the host drains the queue"
            );
        });

        drain_jobs(&runtime);

        context.with(|ctx| {
            assert!(
                ctx.globals().get::<_, bool>("resumed").expect("resumed"),
                "draining must run the queued reaction"
            );
        });
    }

    /// A drain with nothing queued is a no-op, not a hang.
    #[test]
    fn drain_jobs_returns_when_the_queue_is_empty() {
        let (runtime, context) = context();
        context.with(|ctx| {
            ctx.eval::<Value, _>("globalThis.x = 1").expect("eval");
        });
        drain_jobs(&runtime);
    }

    #[test]
    fn actor_completion_can_cross_an_actor_thread() {
        fn assert_send<T: Send>() {}
        assert_send::<ActorCompletion>();
    }

    /// Scheduling needs a window and an executor, both of which only exist
    /// during a host call. Saying so beats scheduling against a dead frame.
    #[test]
    fn sleep_outside_a_host_call_reports_clearly() {
        let (_runtime, context) = context();

        let message = context.with(|ctx| {
            install_module(&ctx);

            let error = ctx
                .eval::<Value, _>("__sleep(10)")
                .expect_err("sleep must refuse to schedule");
            describe(&ctx, error)
        });

        assert!(message.contains("cx.sleep(ms)"), "{message}");
    }

    #[test]
    fn a_negative_delay_is_refused() {
        let (_runtime, context) = context();

        let message = context.with(|ctx| {
            install_module(&ctx);

            let error = ctx
                .eval::<Value, _>("__sleep(-1)")
                .expect_err("a negative delay must be refused");
            describe(&ctx, error)
        });

        assert!(message.contains("non-negative"), "{message}");
    }

    /// `gpui.spawn` needs no scope — an application starts work at module top
    /// level, before there is a window — and a rejection nobody handled must
    /// still settle the task rather than vanish.
    #[test]
    fn spawn_adopts_a_rejected_promise() {
        let (runtime, context) = context();

        context.with(|ctx| {
            install_module(&ctx);
            ctx.eval::<Value, _>(
                "globalThis.handle = __spawn(async () => { throw new Error(\"boom\"); });",
            )
            .expect("spawn");

            assert!(
                !ctx.eval::<bool, _>("handle.is_done()").expect("is_done"),
                "an adopted promise is still pending until the queue is drained"
            );
        });

        drain_jobs(&runtime);

        context.with(|ctx| {
            assert!(
                ctx.eval::<bool, _>("handle.is_done()").expect("is_done"),
                "the rejection handler must settle the task"
            );
        });
    }

    #[test]
    fn spawn_settles_on_a_resolved_promise() {
        let (runtime, context) = context();

        context.with(|ctx| {
            install_module(&ctx);
            ctx.eval::<Value, _>(
                r#"
                globalThis.ran = false;
                globalThis.handle = __spawn(async () => { globalThis.ran = true; });
                "#,
            )
            .expect("spawn");

            assert!(
                ctx.globals().get::<_, bool>("ran").expect("ran"),
                "the body runs synchronously up to its first await"
            );
            assert!(!ctx.eval::<bool, _>("handle.is_done()").expect("is_done"));
        });

        drain_jobs(&runtime);

        context.with(|ctx| {
            assert!(ctx.eval::<bool, _>("handle.is_done()").expect("is_done"));
        });
    }

    /// A synchronous throw is reported the same way a rejection is, and does not
    /// escape into the caller as a live exception.
    #[test]
    fn spawn_survives_a_body_that_throws_at_once() {
        let (_runtime, context) = context();

        context.with(|ctx| {
            install_module(&ctx);
            ctx.eval::<Value, _>(
                "globalThis.handle = __spawn(() => { throw new Error(\"boom\"); });",
            )
            .expect("spawn must absorb the throw");

            assert!(ctx.eval::<bool, _>("handle.is_done()").expect("is_done"));
        });
    }

    #[test]
    fn a_cancelled_spawn_reports_itself_done() {
        let (_runtime, context) = context();

        context.with(|ctx| {
            install_module(&ctx);
            ctx.eval::<Value, _>(
                "globalThis.handle = __spawn(async () => { await Promise.resolve(); });",
            )
            .expect("spawn");

            assert!(!ctx.eval::<bool, _>("handle.is_done()").expect("is_done"));
            ctx.eval::<Value, _>("handle.cancel()").expect("cancel");
            assert!(ctx.eval::<bool, _>("handle.is_done()").expect("is_done"));
        });
    }

    /// An owner that is not the view in progress is refused rather than quietly
    /// downgraded to "no owner", which would defeat the point of ownership.
    #[test]
    fn an_unknown_owner_is_refused() {
        let (_runtime, context) = context();

        let message = context.with(|ctx| {
            install_module(&ctx);
            let error = ctx
                .eval::<Value, _>("__spawn(async () => {}, { owner: {} })")
                .expect_err("an unrelated owner must be refused");
            describe(&ctx, error)
        });

        assert!(message.contains("opts.owner"), "{message}");
    }

    #[test]
    fn a_cancelled_task_never_becomes_ready() {
        let task = register_unchecked(TaskState::new("test", None, None));
        assert!(matches!(task.readiness(), Readiness::Ready(None)));
        assert!(!is_done(task.id));

        cancel(task.id);

        assert!(
            matches!(task.readiness(), Readiness::Cancelled),
            "a cancelled timer must not fire on its next tick"
        );
        assert!(is_done(task.id), "a cancelled task reports itself done");
        assert!(
            TASKS.with_borrow(|tasks| !tasks.contains_key(&task.id)),
            "cancelling reaps the registry entry"
        );
    }

    #[test]
    fn a_finished_task_is_reaped() {
        let task = register_unchecked(TaskState::new("test", None, None));
        finish(&task);

        assert!(is_done(task.id));
        assert!(TASKS.with_borrow(|tasks| !tasks.contains_key(&task.id)));
        // The handle keeps answering after the entry is gone.
        assert!(is_done(task.id + 10_000));
    }

    #[test]
    fn shutting_down_one_runtime_keeps_another_runtimes_tasks() {
        let first = ShellRuntime::new_isolated().expect("first runtime");
        let second = ShellRuntime::new_isolated().expect("second runtime");
        let first_task =
            register_unchecked(TaskState::new("first", None, None).with_runtime(&first));
        let second_task =
            register_unchecked(TaskState::new("second", None, None).with_runtime(&second));

        shutdown(&first);

        assert!(is_done(first_task.id));
        assert!(
            !is_done(second_task.id),
            "dropping one runtime cancelled another runtime's task"
        );
        finish(&second_task);
    }

    #[test]
    fn cancelling_one_policy_keeps_other_applications_tasks() {
        let first_policy = Rc::new(Policy::default());
        let second_policy = Rc::new(Policy::default());
        let first = TaskState::new("first", None, None);
        first.policy.replace(Some(first_policy.clone()));
        let first = register_unchecked(first);
        let second = TaskState::new("second", None, None);
        second.policy.replace(Some(second_policy.clone()));
        let second = register_unchecked(second);

        cancel_policy(&first_policy);

        assert!(matches!(first.readiness(), Readiness::Cancelled));
        assert!(matches!(second.readiness(), Readiness::Ready(None)));
        finish(&second);
    }

    #[test]
    fn reload_ownership_follows_generation_not_task_creation_order() {
        let reloaded_policy = Rc::new(Policy::default());
        let other_policy = Rc::new(Policy::default());
        let old_generation = ApplicationGeneration::new(1);
        let replacement_generation = ApplicationGeneration::new(2);

        // The replacement can create work before an old event re-enters during
        // reload. A task-id checkpoint would put these on the wrong sides.
        let replacement = TaskState::new("replacement", None, None)
            .with_application(replacement_generation.clone());
        replacement.policy.replace(Some(reloaded_policy.clone()));
        let replacement = register_unchecked(replacement);

        let old =
            TaskState::new("reentrant old", None, None).with_application(old_generation.clone());
        old.policy.replace(Some(reloaded_policy.clone()));
        let old = register_unchecked(old);
        let unrelated_generation = ApplicationGeneration::new(3);
        let unrelated =
            TaskState::new("unrelated", None, None).with_application(unrelated_generation);
        unrelated.policy.replace(Some(other_policy));
        let unrelated = register_unchecked(unrelated);

        cancel_application_generation(&old_generation);
        assert!(matches!(old.readiness(), Readiness::Cancelled));
        assert!(matches!(replacement.readiness(), Readiness::Ready(None)));
        assert!(matches!(unrelated.readiness(), Readiness::Ready(None)));

        cancel_application_generation(&replacement_generation);
        assert!(matches!(replacement.readiness(), Readiness::Cancelled));
        assert!(matches!(unrelated.readiness(), Readiness::Ready(None)));
        finish(&unrelated);
    }

    #[gpui::test]
    fn cancelling_a_policy_drops_its_queued_render_drain(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        cx.update(|cx| runtime.set_global(cx));
        runtime
            .with_js(|ctx| {
                ctx.globals().set("deferred_drain_ran", false)?;
                ctx.eval::<(), _>(
                    "Promise.resolve().then(() => { globalThis.deferred_drain_ran = true; });",
                )
            })
            .expect("queue promise job");
        assert!(runtime.js_runtime.is_job_pending());

        let object = runtime.context.with(|ctx| {
            ViewObject::unscoped(Persistent::save(
                &ctx,
                Object::new(ctx.clone()).expect("object"),
            ))
        });
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime.clone(), object)));
        let weak_view = view.downgrade();
        let policy = Rc::new(Policy::default());

        context.update(|window, cx| {
            drain_after_render(&runtime, view.clone(), policy.clone(), window, cx)
        });
        cancel_policy(&policy);
        drop(view);
        context.update(|_, _| {});

        assert_eq!(
            Rc::strong_count(&policy),
            1,
            "a cancelled render drain retained the unloaded plugin policy"
        );
        assert!(
            weak_view.upgrade().is_none(),
            "a queued render drain retained the unloaded plugin view"
        );
        context.run_until_parked();
        let ran = runtime
            .with_js(|ctx| ctx.globals().get::<_, bool>("deferred_drain_ran"))
            .expect("read marker");
        assert!(!ran, "a cancelled policy resumed its queued promise job");

        std::mem::forget(runtime);
    }

    /// The rule from §12.3: work whose panel has closed must not run. Note it is
    /// skipped, not cancelled — nobody called `cancel`, the owner simply left.
    #[gpui::test]
    async fn a_task_whose_owner_is_gone_is_skipped(cx: &mut gpui::TestAppContext) {
        use gpui::AppContext as _;

        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let object = runtime.context.with(|ctx| {
            ViewObject::unscoped(Persistent::save(
                &ctx,
                Object::new(ctx.clone()).expect("object"),
            ))
        });

        let view = cx.update(|cx| cx.new(|_| ScriptView::new(runtime.clone(), object)));
        let task = register_unchecked(TaskState::new("test", Some(view.downgrade()), None));
        assert!(matches!(task.readiness(), Readiness::Ready(Some(_))));

        drop(view);
        cx.update(|_| {});

        assert!(matches!(task.readiness(), Readiness::OwnerGone));
        finish(&task);

        // The QuickJS runtime is leaked on purpose: `ScriptView` holds a
        // `Persistent`, test teardown order is not ours to choose, and a
        // `Persistent` released after its runtime aborts the process.
        std::mem::forget(runtime);
    }
}

#[cfg(test)]
struct Empty;

#[cfg(test)]
impl gpui::Render for Empty {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
    }
}

#[cfg(test)]
mod policy_capture_tests {
    use super::*;
    use crate::capability::Capabilities;

    /// The P0 this closed: a task created before its view exists.
    ///
    /// A plugin's module top level and its `init` both run with no view — the
    /// view is what they are building — so a timer, a `spawn` or an awaited
    /// `fs` call there has no owner to take a policy from. Deriving one at
    /// resume time therefore fell back to the *default* policy, which for a
    /// plugin host is the grant of the application it is embedded in. Capturing
    /// at creation is what makes the task keep its own.
    #[test]
    fn a_task_records_the_policy_it_was_created_under() {
        let plugin = crate::policy::Policy::new().with_capabilities(
            Capabilities::new().read_roots([std::path::PathBuf::from("/tmp/p")]),
        );
        crate::policy::set_default(plugin);
        let expected = crate::policy::default();

        // No owner: exactly the case that used to lose the policy.
        let task = TaskState::new("test", None, None);
        assert!(Rc::ptr_eq(&task.policy(), &expected));

        // The host reconfigures for something else. The task keeps its own.
        crate::policy::set_default(crate::policy::Policy::new());
        assert!(task.policy().capabilities().has_read_access());
        assert!(!crate::policy::default().capabilities().has_read_access());
    }

    #[test]
    fn an_unrepresentable_duration_is_a_js_error_not_a_host_panic() {
        let runtime = JsRuntime::new().expect("runtime");
        let context = rquickjs::Context::full(&runtime).expect("context");

        context.with(|ctx| {
            duration(&ctx, "timer", 1e308).expect_err("duration must be bounded");
        });
    }
}
