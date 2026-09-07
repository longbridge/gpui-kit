//! Dialogs, the sheet and toasts, on the script-side `window`.
//!
//! An overlay is a *host* capability, not something a script draws. A dialog is
//! not a floating `div`: it is a place in the window's stacking order, a focus
//! trap, an Escape target, and a promise about what a backdrop press means —
//! all of which [`ShellRoot`] decides for the window as a whole, because only
//! something that sees every overlay at once can order them. A script that drew
//! its own dialog would own none of that, and two scripts drawing two dialogs
//! would own even less. So the script says *what* to put in front of the user,
//! and the root says where it goes and how it leaves.
//!
//! # Why `window` and not `cx`
//!
//! A dialog belongs to the **window**, not to the view that opened it.
//! `cx.notify()` re-renders this view; `window.open_dialog()` changes what the
//! window is showing. Hanging both off one object said they were the same kind
//! of thing. `gpui-component` draws exactly this line — `window.open_dialog`,
//! `window.push_notification` — and the script API spells it the same way, so a
//! reader moving between the two halves of an application is reading one
//! vocabulary rather than two.
//!
//! And `window` is somewhere to grow. Overlays are what it carries today;
//! `Window` in Rust also answers focus, size and appearance, and a script will
//! want some of that. Those land in a namespace that already exists. Flat
//! exports would either sprawl across the module or have to be gathered into a
//! namespace later, which is a rename every application would feel.
//!
//! It also removed a failure mode. On `cx` these calls carried a generation, so
//! a `cx` stashed in a closure and used later reached a dead stack frame and had
//! to be caught. `window` is ambient like `fs` and `store` — it reads the call
//! that is running *now* — so there is no stale handle to hold.
//!
//! # Why every entry point checks the phase first
//!
//! Opening or closing an overlay mutates the window, and the render pass is
//! reading it. GPUI's borrow model has no way to express "the script may notify
//! from here but not from there", so [`crate::scope`] carries the phase and each
//! entry point refuses `Render` and `Layout` (design doc §16.2). The check lives
//! here as well as in [`ShellRoot`] because the two refusals are different
//! things: the root logs and ignores, which is the right answer for host code
//! that got it wrong, while a script gets a thrown `TypeError` naming the phase
//! it called from — the same shape the style layer uses for an unknown method,
//! and the only shape an author can act on.
//!
//! # The script surface
//!
//! `window` is a global. There is nothing to import, and unlike `cx`, nothing
//! hands it to you either.
//!
//! ```js
//!
//! import { v_flex } from "gpui-base";
//!
//! const depth = window.open_dialog(() => v_flex().child("Delete?"), {
//!   escape_dismissable: false,
//!   backdrop_dismissable: false,
//! });
//! window.close_dialog();       // -> did anything close?
//! window.close_all_dialogs();  // -> how many closed
//! window.has_active_dialog();
//!
//! window.open_sheet(() => filters());          // right, the default placement
//! window.open_sheet_at("left", () => nav());
//! window.close_sheet();        // -> did anything close?
//! window.has_active_sheet();
//!
//! window.push_toast({ title: "Saved", description: "3 files", level: "success",
//!                     timeout: 4000, id: "save" });
//! window.remove_toast("save");
//! window.clear_toasts();
//! ```
//!
//! # Why the content is a function
//!
//! `open_dialog` takes a function returning an element, not an element. An
//! element belongs to the arena of the render pass that built it, and a dialog
//! outlives the call that opened it — so an element built at open time would
//! belong to the wrong pass. The function is called when the dialog renders, and
//! again whenever it re-renders, which is the same contract a view's `render`
//! has. Whatever it closes over is the dialog's state.
//!
//! What it closes over is usually another view's state, and this call answers a
//! depth rather than a handle — so there is nothing for a script to notify when
//! that state moves. `window.refresh()` is the call for it: the root redraws,
//! and an overlay built from a script rebuilds with it rather than
//! re-materializing the description it was opened with.

use std::time::Duration;

use gpui::{AnyView, App, AppContext as _, Context, Window};
use gpui_base::Placement;
use rquickjs::{
    Ctx, Exception, FromJs, Object, Persistent, Result as JsResult, Value,
    function::{Func, Opt},
};

use crate::{
    materialize::{ANCHOR_NAMES, anchor_from_name},
    root::{DialogOptions, FpsHudRequest, ShellRoot, ToastLevel, ToastRequest},
    scope::{self, ScopePhase},
    view::ScriptView,
};

use super::{ShellRuntime, ViewObject};

/// The names an error message uses, so a refusal reads like the call that
/// caused it rather than like the Rust function that answered it.
const OPEN_DIALOG: &str = "window.open_dialog(content, options)";
const CLOSE_DIALOG: &str = "window.close_dialog()";
const CLOSE_ALL_DIALOGS: &str = "window.close_all_dialogs()";
const OPEN_SHEET: &str = "window.open_sheet(content)";
const OPEN_SHEET_AT: &str = "window.open_sheet_at(placement, content)";
const CLOSE_SHEET: &str = "window.close_sheet()";
const PUSH_TOAST: &str = "window.push_toast(options)";
const REMOVE_TOAST: &str = "window.remove_toast(id)";
const CLEAR_TOASTS: &str = "window.clear_toasts()";
const SHOW_FPS_MONITOR: &str = "show_fps_monitor(options)";
const HIDE_FPS_MONITOR: &str = "hide_fps_monitor()";
const FPS_MONITOR_VISIBLE: &str = "fps_monitor_visible()";

/// Installs the host half of the script-side `window`.
///
/// The prelude builds the object; these are the functions behind it. They take
/// a view instance rather than a class, because the prelude has already wrapped
/// the author's content function into `{ render }` — which is the whole of what
/// a script view is.
///
/// `ctx` is unused — every value installed here is built by `Object::set` from
/// the target object's own context, because `Ctx` and `Object` are invariant in
/// their lifetime and a value built from one cannot be set on the other. It
/// stays in the signature to match the other installers.
pub fn install(_ctx: &Ctx<'_>, globals: &Object<'_>) -> JsResult<()> {
    // Returns the new depth of the dialog stack rather than a handle: the root
    // addresses dialogs by position, never by identity, so a handle would have
    // to promise "close *this* dialog", which is not an operation that exists.
    // The depth is what a script can actually use — to assert it opened one, or
    // to unwind to a known level.
    globals.set(
        "__open_dialog",
        Func::from(
            |ctx: Ctx<'_>, content: ViewInstance, options: Opt<DialogRequest>| -> JsResult<u32> {
                guard(&ctx, OPEN_DIALOG)?;
                let options = options.0.unwrap_or_default().options;

                with_root(&ctx, OPEN_DIALOG, |root, window, cx| {
                    let view = mount(&ctx, content.0, cx)?;
                    root.open_dialog_with(view, options, window, cx);
                    Ok(root.dialog_count() as u32)
                })
            },
        ),
    )?;

    globals.set(
        "__close_dialog",
        Func::from(|ctx: Ctx<'_>| -> JsResult<bool> {
            guard(&ctx, CLOSE_DIALOG)?;
            with_root(&ctx, CLOSE_DIALOG, |root, window, cx| {
                Ok(root.close_dialog(window, cx))
            })
        }),
    )?;

    globals.set(
        "__close_all_dialogs",
        Func::from(|ctx: Ctx<'_>| -> JsResult<u32> {
            guard(&ctx, CLOSE_ALL_DIALOGS)?;
            with_root(&ctx, CLOSE_ALL_DIALOGS, |root, window, cx| {
                // Read before clearing: the root reports nothing, and "how many
                // did I close?" is the same question `close_dialog`'s `bool`
                // answers for one.
                let closed = root.dialog_count() as u32;
                root.close_all_dialogs(window, cx);
                Ok(closed)
            })
        }),
    )?;

    // Reading the window is not mutating it, so this one is legal from `render`
    // — a view that draws itself differently while a dialog is up needs to ask
    // during the pass that draws it.
    globals.set(
        "__has_active_dialog",
        Func::from(|ctx: Ctx<'_>| -> JsResult<bool> {
            with_root(&ctx, "window.has_active_dialog()", |root, _, _| {
                Ok(root.dialog_count() > 0)
            })
        }),
    )?;

    globals.set(
        "__open_sheet",
        Func::from(
            |ctx: Ctx<'_>, placement: Opt<String>, content: ViewInstance| -> JsResult<()> {
                let api = if placement.0.is_some() {
                    OPEN_SHEET_AT
                } else {
                    OPEN_SHEET
                };
                guard(&ctx, api)?;
                let placement = match placement.0 {
                    Some(name) => parse_placement(&ctx, &name)?,
                    None => Placement::Right,
                };

                with_root(&ctx, api, |root, window, cx| {
                    let view = mount(&ctx, content.0, cx)?;
                    root.open_sheet(placement, view, window, cx);
                    Ok(())
                })
            },
        ),
    )?;

    globals.set(
        "__close_sheet",
        Func::from(|ctx: Ctx<'_>| -> JsResult<bool> {
            guard(&ctx, CLOSE_SHEET)?;
            with_root(&ctx, CLOSE_SHEET, |root, window, cx| {
                Ok(root.close_sheet(window, cx))
            })
        }),
    )?;

    globals.set(
        "__has_active_sheet",
        Func::from(|ctx: Ctx<'_>| -> JsResult<bool> {
            with_root(&ctx, "window.has_active_sheet()", |root, _, _| {
                Ok(root.sheet().is_some())
            })
        }),
    )?;

    // A toast is data, not a view: no function, no instance, nothing for the
    // script to render. That is why it is the one overlay whose whole content
    // crosses the boundary as an options object.
    globals.set(
        "__push_toast",
        Func::from(|ctx: Ctx<'_>, toast: ToastArgument| -> JsResult<()> {
            guard(&ctx, PUSH_TOAST)?;
            with_root(&ctx, PUSH_TOAST, |root, window, cx| {
                root.push_toast(toast.0, window, cx);
                Ok(())
            })
        }),
    )?;

    globals.set(
        "__remove_toast",
        Func::from(|ctx: Ctx<'_>, id: String| -> JsResult<bool> {
            guard(&ctx, REMOVE_TOAST)?;
            with_root(&ctx, REMOVE_TOAST, |root, _, cx| {
                Ok(root.remove_toast(id, cx))
            })
        }),
    )?;

    globals.set(
        "__clear_toasts",
        Func::from(|ctx: Ctx<'_>| -> JsResult<()> {
            guard(&ctx, CLEAR_TOASTS)?;
            with_root(&ctx, CLEAR_TOASTS, |root, _, cx| {
                root.clear_toasts(cx);
                Ok(())
            })
        }),
    )?;

    // The performance HUD is the root's, not the script's: the script says
    // whether and where, the host draws it above everything and keeps its
    // history across a hide and a show. Nothing crosses back but a flag.
    globals.set(
        "__show_fps_monitor",
        Func::from(
            |ctx: Ctx<'_>, options: Opt<FpsHudArgument>| -> JsResult<()> {
                guard(&ctx, SHOW_FPS_MONITOR)?;
                let request = options.0.map(|argument| argument.0).unwrap_or_default();
                with_root(&ctx, SHOW_FPS_MONITOR, |root, _, cx| {
                    root.show_fps_monitor(request, cx);
                    Ok(())
                })
            },
        ),
    )?;

    globals.set(
        "__hide_fps_monitor",
        Func::from(|ctx: Ctx<'_>| -> JsResult<bool> {
            guard(&ctx, HIDE_FPS_MONITOR)?;
            with_root(&ctx, HIDE_FPS_MONITOR, |root, _, cx| {
                Ok(root.hide_fps_monitor(cx))
            })
        }),
    )?;

    globals.set(
        "__fps_monitor_visible",
        Func::from(|ctx: Ctx<'_>| -> JsResult<bool> {
            with_root(&ctx, FPS_MONITOR_VISIBLE, |root, _, _| {
                Ok(root.fps_monitor_visible())
            })
        }),
    )
}

/// Refuses an overlay change that is not being made from an event or a task.
///
/// Outside any scope there is no window to reach either, so `none` is refused
/// with the same message: both cases mean the call has no live host frame.
fn guard(ctx: &Ctx<'_>, api: &str) -> JsResult<()> {
    let phase = scope::current_phase();
    if phase.is_some_and(ScopePhase::allows_notify) {
        return Ok(());
    }

    Err(Exception::throw_type(
        ctx,
        &format!(
            "{api} is not allowed during the `{}` phase; overlays may only be opened or \
             closed while handling an event or a task",
            phase.map(ScopePhase::as_str).unwrap_or("none")
        ),
    ))
}

/// Runs `body` against the overlay host of the window the call belongs to.
///
/// Ambient, like `fs` and `store`: the window comes from the call that is
/// running now rather than from a handle the script is holding. On `cx` these
/// calls carried a generation, so a `cx` stashed in a closure and used later
/// reached a dead stack frame — a failure mode that simply does not exist once
/// there is nothing to stash.
///
/// Two ways this fails, and they are different mistakes. Outside any host call
/// there is no window to reach, which is the script calling from somewhere it
/// cannot. A window whose root view is not a [`ShellRoot`] is the *host's*
/// mistake — it opened the window with something else — and the message says so
/// rather than blaming the script.
fn with_root<R>(
    ctx: &Ctx<'_>,
    api: &str,
    body: impl FnOnce(&mut ShellRoot, &mut Window, &mut Context<ShellRoot>) -> JsResult<R>,
) -> JsResult<R> {
    let Some(reached) = scope::with_current(|window, app| ShellRoot::update(window, app, body))
    else {
        return Err(Exception::throw_type(
            ctx,
            &format!(
                "{api} needs a live host call; call it from init(), an event handler or a task"
            ),
        ));
    };

    reached.unwrap_or_else(|| {
        Err(Exception::throw_type(
            ctx,
            &format!(
                "{api} needs a ShellRoot as the window's first view; this window was \
                 opened with another view"
            ),
        ))
    })
}

/// Wraps a freshly constructed script instance as a view the root can mount.
fn mount(ctx: &Ctx<'_>, object: ViewObject, cx: &mut App) -> JsResult<AnyView> {
    let Some(runtime) = scope::current_runtime().or_else(|| ShellRuntime::global(cx)) else {
        return Err(Exception::throw_type(
            ctx,
            "the shell runtime is not installed on this application",
        ));
    };
    let policy = scope::policy();
    Ok(cx
        .new(|_| ScriptView::with_policy(runtime, object, policy))
        .into())
}

/// A view instance, kept alive across the argument conversion.
///
/// Its own type because a JS closure cannot unify the lifetime of a `Ctx<'js>`
/// parameter with that of a `Value<'js>` one — the two elided lifetimes are
/// independent as far as inference is concerned. Converting inside [`FromJs`],
/// where both are the same lifetime again, is the pattern `Arguments` in the
/// spec layer uses for the same reason.
struct ViewInstance(ViewObject);

impl<'js> FromJs<'js> for ViewInstance {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        // The prelude wraps the author's function; anything else reaching here
        // means the wrapper was bypassed, which is worth saying plainly rather
        // than failing later on a missing `render`.
        let Some(object) = value.into_object() else {
            return Err(Exception::throw_type(
                ctx,
                "expected a function returning an element; open_dialog and open_sheet take \
                 a function, not an element and not a view class",
            ));
        };
        Ok(Self(ViewObject::unscoped(Persistent::save(ctx, object))))
    }
}

/// `{ escape_dismissable, backdrop_dismissable }`.
///
/// No `props`. The content function closes over whatever it needs, so a dialog's
/// starting state comes from the same place every other value in the script
/// does, rather than through a channel that existed only for overlays.
#[derive(Default)]
struct DialogRequest {
    options: DialogOptions,
}

const DIALOG_KEYS: &[&str] = &["escape_dismissable", "backdrop_dismissable"];

impl<'js> FromJs<'js> for DialogRequest {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let Some(object) = options_object(ctx, &value, OPEN_DIALOG)? else {
            return Ok(Self::default());
        };
        reject_unknown_keys(ctx, object, DIALOG_KEYS, OPEN_DIALOG)?;

        let mut options = DialogOptions::default();
        if let Some(dismissable) = object.get::<_, Option<bool>>("escape_dismissable")? {
            options = options.escape_dismissable(dismissable);
        }
        if let Some(dismissable) = object.get::<_, Option<bool>>("backdrop_dismissable")? {
            options = options.backdrop_dismissable(dismissable);
        }

        Ok(Self { options })
    }
}

/// `{ anchor, frame_budget }`, every key optional; `undefined` or `null` is
/// the default HUD.
struct FpsHudArgument(FpsHudRequest);

const FPS_HUD_KEYS: &[&str] = &["anchor", "frame_budget"];

impl<'js> FromJs<'js> for FpsHudArgument {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        if value.is_undefined() || value.is_null() {
            return Ok(Self(FpsHudRequest::default()));
        }
        let Some(object) = value.as_object() else {
            return Err(Exception::throw_type(
                ctx,
                &format!(
                    "{SHOW_FPS_MONITOR} expects an object, such as {{ anchor: \"bottom_left\" }}"
                ),
            ));
        };
        reject_unknown_keys(ctx, object, FPS_HUD_KEYS, SHOW_FPS_MONITOR)?;

        let mut request = FpsHudRequest::default();
        if let Some(anchor) = object.get::<_, Option<String>>("anchor")? {
            request.anchor = anchor_from_name(&anchor).ok_or_else(|| {
                Exception::throw_type(
                    ctx,
                    &format!(
                        "{SHOW_FPS_MONITOR}: unknown anchor `{anchor}`; expected one of {}",
                        ANCHOR_NAMES.join(", ")
                    ),
                )
            })?;
        }
        if let Some(millis) = object.get::<_, Option<f64>>("frame_budget")? {
            if !(millis.is_finite() && millis > 0.) {
                return Err(Exception::throw_type(
                    ctx,
                    &format!(
                        "{SHOW_FPS_MONITOR}: frame_budget must be a positive number of milliseconds"
                    ),
                ));
            }
            request.frame_budget = Some(std::time::Duration::from_secs_f64(millis / 1000.));
        }
        Ok(Self(request))
    }
}

/// `{ title, description, level, timeout, id }`.
struct ToastArgument(ToastRequest);

const TOAST_KEYS: &[&str] = &["title", "description", "level", "timeout", "id"];

impl<'js> FromJs<'js> for ToastArgument {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let Some(object) = value.as_object() else {
            return Err(Exception::throw_type(
                ctx,
                &format!("{PUSH_TOAST} expects an object, such as {{ title: \"Saved\" }}"),
            ));
        };
        reject_unknown_keys(ctx, object, TOAST_KEYS, PUSH_TOAST)?;

        let Some(title) = object.get::<_, Option<String>>("title")? else {
            return Err(Exception::throw_type(
                ctx,
                &format!("{PUSH_TOAST} requires a `title`; it is the sentence the user reads"),
            ));
        };

        let mut toast = ToastRequest::new(title);
        if let Some(description) = object.get::<_, Option<String>>("description")? {
            toast = toast.with_description(description);
        }
        if let Some(level) = object.get::<_, Option<String>>("level")? {
            toast = toast.with_level(parse_level(ctx, &level)?);
        }
        if let Some(id) = object.get::<_, Option<String>>("id")? {
            toast = toast.with_id(id);
        }

        // An absent `timeout` keeps the default; an explicit `null` is the way
        // to ask for a toast that stays until it is dismissed, so the two
        // cannot be collapsed into one `Option`.
        let timeout: Value = object.get("timeout")?;
        if !timeout.is_undefined() {
            toast = toast.with_timeout(parse_timeout(ctx, &timeout)?);
        }

        Ok(Self(toast))
    }
}

/// The object form of an optional trailing options argument.
///
/// `None` means "not given": an omitted argument, or an explicit `null` or
/// `undefined`, all of which mean the defaults. Anything that is not an object
/// is a mistake worth naming.
fn options_object<'a, 'js>(
    ctx: &Ctx<'js>,
    value: &'a Value<'js>,
    api: &str,
) -> JsResult<Option<&'a Object<'js>>> {
    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }
    match value.as_object() {
        Some(object) => Ok(Some(object)),
        None => Err(Exception::throw_type(
            ctx,
            &format!("{api} expects an options object"),
        )),
    }
}

/// Every placement a script may name. Also what an unknown one is told to use, so
/// the message cannot drift from the set.
const SHEET_PLACEMENTS: [Placement; 4] = [
    Placement::Left,
    Placement::Right,
    Placement::Top,
    Placement::Bottom,
];

const TOAST_LEVELS: [ToastLevel; 4] = [
    ToastLevel::Info,
    ToastLevel::Success,
    ToastLevel::Warning,
    ToastLevel::Error,
];

fn parse_placement(ctx: &Ctx<'_>, name: &str) -> JsResult<Placement> {
    let placement = match name {
        "left" => Some(Placement::Left),
        "right" => Some(Placement::Right),
        "top" => Some(Placement::Top),
        "bottom" => Some(Placement::Bottom),
        _ => None,
    };
    placement.ok_or_else(|| {
        Exception::throw_type(
            ctx,
            &format!(
                "unknown sheet placement `{name}`; expected {}",
                listed_by(&SHEET_PLACEMENTS, placement_name)
            ),
        )
    })
}

fn placement_name(placement: Placement) -> &'static str {
    match placement {
        Placement::Left => "left",
        Placement::Right => "right",
        Placement::Top => "top",
        Placement::Bottom => "bottom",
    }
}

fn parse_level(ctx: &Ctx<'_>, name: &str) -> JsResult<ToastLevel> {
    ToastLevel::from_name(name).ok_or_else(|| {
        Exception::throw_type(
            ctx,
            &format!(
                "unknown toast level `{name}`; expected {}",
                listed_by(&TOAST_LEVELS, ToastLevel::as_str)
            ),
        )
    })
}
/// Refuses an option the surface does not have, rather than ignoring it.
///
/// A misspelled key that is silently dropped is a setting the author believes
/// they applied. Naming the valid ones turns the typo into a one-line fix.
fn reject_unknown_keys(
    ctx: &Ctx<'_>,
    object: &Object<'_>,
    known: &[&str],
    api: &str,
) -> JsResult<()> {
    for key in object.keys::<String>() {
        let key = key?;
        if !known.contains(&key.as_str()) {
            return Err(Exception::throw_type(
                ctx,
                &format!(
                    "unknown option `{key}` for {api}; expected {}",
                    listed(known)
                ),
            ));
        }
    }
    Ok(())
}

/// `null` is sticky; a number is milliseconds.
fn parse_timeout(ctx: &Ctx<'_>, value: &Value<'_>) -> JsResult<Option<Duration>> {
    if value.is_null() {
        return Ok(None);
    }

    let refused = || {
        Exception::throw_type(
            ctx,
            &format!(
                "{PUSH_TOAST} expects `timeout` to be a number of milliseconds, or null to keep \
                 the toast until it is dismissed"
            ),
        )
    };

    let ms = value.as_number().ok_or_else(refused)?;
    if !ms.is_finite() || ms < 0. {
        return Err(refused());
    }
    Ok(Some(Duration::from_millis(ms as u64)))
}

fn listed(names: &[&str]) -> String {
    match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        _ => names.concat(),
    }
}

fn listed_by<T: Copy>(values: &[T], name: fn(T) -> &'static str) -> String {
    listed(&values.iter().map(|value| name(*value)).collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::quickjs::context_object;
    use gpui::{Entity, IntoElement, Render, TestAppContext, VisualTestContext, div};
    use std::rc::Rc;

    struct Empty;

    impl Render for Empty {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    fn shell(
        cx: &mut TestAppContext,
    ) -> (Rc<ShellRuntime>, Entity<ShellRoot>, &mut VisualTestContext) {
        cx.update(crate::init);

        let runtime = cx.update(ShellRuntime::new).expect("runtime");
        // The views these tests open hold `Persistent` script values, and a
        // `Persistent` released after its runtime has gone aborts the process.
        // Teardown order is not ours to choose, so the runtime outlives the
        // test deliberately — the same trade the scheduler's tests make.
        std::mem::forget(runtime.clone());

        let (root, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(|_| Empty).into();
            ShellRoot::new(content, window, cx)
        });
        (runtime, root, cx)
    }

    /// Evaluates `source` with a script-side `cx` belonging to a fresh scope,
    /// which is what an overlay call needs and what the phase check reads.
    fn eval<T>(
        runtime: &Rc<ShellRuntime>,
        cx: &mut VisualTestContext,
        phase: ScopePhase,
        source: &str,
    ) -> anyhow::Result<T>
    where
        T: for<'js> FromJs<'js> + 'static,
    {
        cx.update(|window, app| {
            let (_guard, generation) = scope::enter_runtime(runtime, window, app, phase, None);
            runtime.with_js(|ctx| {
                ctx.globals().set(
                    "cx",
                    context_object(
                        ctx,
                        crate::engine::quickjs::ContextBinding::Call(generation),
                    )?,
                )?;
                ctx.eval::<T, _>(source)
            })
        })
    }

    #[gpui::test]
    fn a_script_opens_a_dialog_on_the_root(cx: &mut TestAppContext) {
        let (runtime, root, cx) = shell(cx);

        let depth: u32 = eval(
            &runtime,
            cx,
            ScopePhase::Event,
            "window.open_dialog(() => __gpui.text('confirm'))",
        )
        .expect("open_dialog");

        assert_eq!(depth, 1);
        assert_eq!(root.read_with(cx, |root, _| root.dialog_count()), 1);

        // A second dialog stacks rather than replacing, and closing reports
        // that it found something to close.
        let depth: u32 = eval(
            &runtime,
            cx,
            ScopePhase::Event,
            "window.open_dialog(() => __gpui.text('detail'), { escape_dismissable: false })",
        )
        .expect("open_dialog");
        assert_eq!(depth, 2);

        let closed: bool =
            eval(&runtime, cx, ScopePhase::Event, "window.close_dialog()").expect("close_dialog");
        assert!(closed);
        assert_eq!(root.read_with(cx, |root, _| root.dialog_count()), 1);
    }

    #[gpui::test]
    fn an_isolated_runtime_owns_the_overlay_view_it_constructs(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let default = cx.update(ShellRuntime::new).expect("default runtime");
        let isolated = ShellRuntime::new_isolated().expect("isolated runtime");
        std::mem::forget(default);
        std::mem::forget(isolated.clone());

        let (root, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(|_| Empty).into();
            ShellRoot::new(content, window, cx)
        });
        eval::<u32>(
            &isolated,
            cx,
            ScopePhase::Event,
            "window.open_dialog(() => __gpui.text('isolated'))",
        )
        .expect("open dialog from isolated runtime");

        let mounted_runtime = root.read_with(cx, |root, cx| {
            let view = root
                .topmost_dialog()
                .expect("dialog")
                .clone()
                .downcast::<ScriptView>()
                .expect("script dialog");
            view.read(cx).runtime()
        });
        assert!(Rc::ptr_eq(&mounted_runtime, &isolated));
    }

    #[gpui::test]
    fn closing_a_dialog_reports_when_none_was_open(cx: &mut TestAppContext) {
        let (runtime, root, cx) = shell(cx);

        let closed: bool =
            eval(&runtime, cx, ScopePhase::Event, "window.close_dialog()").expect("close_dialog");

        assert!(!closed);
        assert_eq!(root.read_with(cx, |root, _| root.dialog_count()), 0);
    }

    /// What replaced `props`: the content function closes over it.
    ///
    /// A dialog used to need a second channel to receive its starting state,
    /// because it was constructed from a class the script handed over. A
    /// function carries whatever it was written next to.
    #[gpui::test]
    fn the_content_function_closes_over_what_the_dialog_shows(cx: &mut TestAppContext) {
        let (runtime, _root, cx) = shell(cx);

        let name: String = eval(
            &runtime,
            cx,
            ScopePhase::Event,
            r#"
            const name = "notes.md";
            window.open_dialog(() => { globalThis.__seen = name; return __gpui.text(name); });
            globalThis.__seen ?? ""
            "#,
        )
        .expect("open_dialog");

        // Rendered lazily: the function runs when the dialog draws, not when it
        // is opened, because the element it builds belongs to that pass.
        assert_eq!(name, "");
    }

    #[gpui::test]
    fn an_unknown_sheet_placement_names_the_valid_ones(cx: &mut TestAppContext) {
        let (runtime, root, cx) = shell(cx);

        let error = eval::<()>(
            &runtime,
            cx,
            ScopePhase::Event,
            r#"window.open_sheet_at("middle", () => __gpui.text("filters"))"#,
        )
        .expect_err("an unknown placement must be refused");

        let message = error.to_string();
        assert!(message.contains("middle"), "{message}");
        assert!(message.contains("left, right, top or bottom"), "{message}");
        assert!(root.read_with(cx, |root, _| root.sheet().is_none()));
    }

    #[gpui::test]
    fn an_unknown_toast_level_names_the_valid_ones(cx: &mut TestAppContext) {
        let (runtime, root, cx) = shell(cx);

        let error = eval::<()>(
            &runtime,
            cx,
            ScopePhase::Event,
            r#"window.push_toast({ title: "Gone", level: "fatal" })"#,
        )
        .expect_err("an unknown level must be refused");

        let message = error.to_string();
        assert!(message.contains("fatal"), "{message}");
        assert!(
            message.contains("info, success, warning or error"),
            "{message}"
        );
        assert_eq!(root.read_with(cx, |root, _| root.toast_count()), 0);
    }

    #[gpui::test]
    fn a_toast_reaches_the_stack_and_can_be_dismissed_by_id(cx: &mut TestAppContext) {
        let (runtime, root, cx) = shell(cx);

        eval::<()>(
            &runtime,
            cx,
            ScopePhase::Event,
            r#"window.push_toast({ title: "Saved", description: "3 files",
                                       level: "success", timeout: 4000, id: "save" })"#,
        )
        .expect("toast");
        assert_eq!(root.read_with(cx, |root, _| root.toast_count()), 1);

        let dismissed: bool = eval(
            &runtime,
            cx,
            ScopePhase::Event,
            r#"window.remove_toast("save")"#,
        )
        .expect("remove_toast");
        assert!(dismissed);
    }

    #[gpui::test]
    fn a_script_puts_the_performance_hud_on_the_root_and_takes_it_down(cx: &mut TestAppContext) {
        let (runtime, root, cx) = shell(cx);
        assert!(!root.read_with(cx, |root, _| root.fps_monitor_visible()));

        eval::<()>(
            &runtime,
            cx,
            ScopePhase::Event,
            r#"__show_fps_monitor({ anchor: "bottom_left", frame_budget: 8.33 })"#,
        )
        .expect("show");
        assert!(root.read_with(cx, |root, _| root.fps_monitor_visible()));
        let visible: bool =
            eval(&runtime, cx, ScopePhase::Event, "__fps_monitor_visible()").expect("visible");
        assert!(visible);

        // Up means drawn: the root's own layer carries the HUD, not the script's tree.
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let hidden: bool =
            eval(&runtime, cx, ScopePhase::Event, "__hide_fps_monitor()").expect("hide");
        assert!(hidden);
        assert!(!root.read_with(cx, |root, _| root.fps_monitor_visible()));
        let hidden_again: bool =
            eval(&runtime, cx, ScopePhase::Event, "__hide_fps_monitor()").expect("hide again");
        assert!(!hidden_again, "nothing was up to take down");
    }

    #[gpui::test]
    fn an_unknown_hud_anchor_names_the_valid_ones(cx: &mut TestAppContext) {
        let (runtime, root, cx) = shell(cx);

        let error = eval::<()>(
            &runtime,
            cx,
            ScopePhase::Event,
            r#"__show_fps_monitor({ anchor: "middle" })"#,
        )
        .expect_err("an unknown anchor must be refused");
        let message = error.to_string();
        assert!(
            message.contains("middle") && message.contains("bottom_left"),
            "{message}"
        );
        assert!(!root.read_with(cx, |root, _| root.fps_monitor_visible()));
    }

    /// The render pass is reading the window an overlay would mutate, so the
    /// call is refused rather than deferred — and the message says which phase
    /// it came from, because that is the only clue the author has.
    #[gpui::test]
    fn overlays_are_refused_during_a_render(cx: &mut TestAppContext) {
        let (runtime, root, cx) = shell(cx);

        let error = eval::<u32>(
            &runtime,
            cx,
            ScopePhase::Render,
            "window.open_dialog(() => __gpui.text('confirm'))",
        )
        .expect_err("a render pass must not open a dialog");

        let message = error.to_string();
        assert!(message.contains("`render` phase"), "{message}");
        assert!(message.contains("event"), "{message}");
        assert_eq!(root.read_with(cx, |root, _| root.dialog_count()), 0);
    }

    /// A mistyped option is a silent no-op unless the host says otherwise.
    #[gpui::test]
    fn a_misspelled_option_is_refused(cx: &mut TestAppContext) {
        let (runtime, _root, cx) = shell(cx);

        let error = eval::<u32>(
            &runtime,
            cx,
            ScopePhase::Event,
            "window.open_dialog(() => __gpui.text('x'), { escapeDismissable: false })",
        )
        .expect_err("an unknown option must be refused");

        let message = error.to_string();
        assert!(message.contains("escapeDismissable"), "{message}");
        assert!(message.contains("escape_dismissable"), "{message}");
    }
}
