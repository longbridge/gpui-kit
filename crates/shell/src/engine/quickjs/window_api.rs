//! The window's own measurements and controls.
//!
//! `Window` in GPUI answers three things a script has had no way to ask: how
//! big it is, what the type metrics under it are, and what state the platform
//! window is in. Every one of them is a method on `Window`, so every one of
//! them is a member of the `window` global — the same mapping the overlays
//! already follow.
//!
//! The split between the two halves here is the same one `render` enforces
//! everywhere else. Reading a measurement during a render pass is not only
//! legal but the point: a view that sizes itself from the viewport has to ask
//! while it is drawing. Changing the window — the rem size, the focus, the
//! platform state — is a mutation, and a mutation from inside `render` is a
//! frame arguing with itself, so those take the same guard the overlays do.

use crate::engine::quickjs::js_error;
use gpui::{Window, WindowAppearance};
use rquickjs::{Ctx, Object, Result as JsResult, function::Func};

use crate::action::ShellAction;
use crate::scope::{self, ScopePhase};

/// Installs the host halves of the `window` measurement and control members.
pub(super) fn install(ctx: &Ctx<'_>) -> JsResult<()> {
    let globals = ctx.globals();

    // The reads. Legal from `render`, because a view that sizes itself from
    // the window has to ask during the pass that draws it.
    globals.set(
        "__window_rem_size",
        Func::from(|ctx: Ctx<'_>| -> JsResult<f32> {
            read(&ctx, "window.rem_size()", |window| {
                f32::from(window.rem_size())
            })
        }),
    )?;
    globals.set(
        "__window_line_height",
        Func::from(|ctx: Ctx<'_>| -> JsResult<f32> {
            read(&ctx, "window.line_height()", |window| {
                f32::from(window.line_height())
            })
        }),
    )?;
    globals.set("__window_viewport_size", Func::from(viewport_size))?;
    globals.set("__window_bounds", Func::from(bounds))?;
    globals.set("__window_mouse_position", Func::from(mouse_position))?;
    globals.set(
        "__window_appearance",
        Func::from(|ctx: Ctx<'_>| -> JsResult<String> {
            read(&ctx, "window.appearance()", |window| {
                appearance_name(window.appearance()).to_owned()
            })
        }),
    )?;
    // Three host functions rather than one answering three flags. An earlier
    // shape returned them together on the theory that they are read together;
    // the prelude then spelled each reader as its own method, so every one of
    // them paid for all three and for the object carrying them. `Window` has
    // three methods, the script has three methods, and now so does this.
    globals.set(
        "__window_is_active",
        Func::from(|ctx: Ctx<'_>| -> JsResult<bool> {
            read(&ctx, "window.is_window_active()", Window::is_window_active)
        }),
    )?;
    globals.set(
        "__window_is_fullscreen",
        Func::from(|ctx: Ctx<'_>| -> JsResult<bool> {
            read(&ctx, "window.is_fullscreen()", Window::is_fullscreen)
        }),
    )?;
    globals.set(
        "__window_is_maximized",
        Func::from(|ctx: Ctx<'_>| -> JsResult<bool> {
            read(&ctx, "window.is_maximized()", Window::is_maximized)
        }),
    )?;

    // The mutations. Refused from `render` for the reason `cx.notify()` is:
    // a frame that changes the window it is drawing into is a frame arguing
    // with itself.
    globals.set(
        "__window_set_rem_size",
        Func::from(|ctx: Ctx<'_>, size: f32| -> JsResult<()> {
            write(&ctx, "window.set_rem_size()", |window, _| {
                window.set_rem_size(gpui::px(size));
            })
        }),
    )?;
    globals.set(
        "__window_refresh",
        Func::from(|ctx: Ctx<'_>| -> JsResult<()> {
            write(&ctx, "window.refresh()", |window, _| window.refresh())
        }),
    )?;
    globals.set(
        "__window_focus_next",
        Func::from(|ctx: Ctx<'_>| -> JsResult<()> {
            write(&ctx, "window.focus_next()", |window, app| {
                window.focus_next(app)
            })
        }),
    )?;
    globals.set(
        "__window_focus_prev",
        Func::from(|ctx: Ctx<'_>| -> JsResult<()> {
            write(&ctx, "window.focus_prev()", |window, app| {
                window.focus_prev(app)
            })
        }),
    )?;
    globals.set(
        "__window_activate",
        Func::from(|ctx: Ctx<'_>| -> JsResult<()> {
            write(&ctx, "window.activate_window()", |window, _| {
                window.activate_window()
            })
        }),
    )?;
    globals.set(
        "__window_minimize",
        Func::from(|ctx: Ctx<'_>| -> JsResult<()> {
            write(&ctx, "window.minimize_window()", |window, _| {
                window.minimize_window()
            })
        }),
    )?;
    globals.set(
        "__window_zoom",
        Func::from(|ctx: Ctx<'_>| -> JsResult<()> {
            write(&ctx, "window.zoom_window()", |window, _| {
                window.zoom_window()
            })
        }),
    )?;
    globals.set(
        "__window_start_move",
        Func::from(|ctx: Ctx<'_>| -> JsResult<()> {
            write(&ctx, "window.start_window_move()", |window, _| {
                window.start_window_move()
            })
        }),
    )?;
    globals.set(
        "__window_toggle_fullscreen",
        Func::from(|ctx: Ctx<'_>| -> JsResult<()> {
            write(&ctx, "window.toggle_fullscreen()", |window, _| {
                window.toggle_fullscreen()
            })
        }),
    )?;

    globals.set("__bind_keys", Func::from(bind_keys))?;
    globals.set(
        "__dispatch_action",
        Func::from(|ctx: Ctx<'_>, action: String| -> JsResult<()> {
            if action.is_empty() {
                return Err(js_error::type_error(
                    &ctx,
                    "window.dispatch_action(action) expects a non-empty action name",
                ));
            }
            write(&ctx, "window.dispatch_action()", |window, app| {
                window.dispatch_action(Box::new(ShellAction::new(action)), app);
            })
        }),
    )?;

    Ok(())
}

/// Installs key bindings, from a list of `{ keystroke, action, context? }`.
///
/// Whole-list rather than one at a time, and validated before any of it is
/// installed: a keymap half applied because the fourth entry had a typo is a
/// worse state than one not applied at all, and the script has no way to see
/// which half made it.
///
/// The bindings are the application's, not the element's — `App::bind_keys` is
/// what this mirrors — so a chord bound here is live in every window. Which
/// element it reaches is the `context` predicate's job, matched against the
/// `key_context(...)` an element declares.
fn bind_keys(ctx: Ctx<'_>, bindings: Vec<Object<'_>>) -> JsResult<u32> {
    let phase = scope::current_phase();
    if !phase.is_some_and(ScopePhase::allows_notify) {
        return Err(js_error::type_error(
            &ctx,
            &format!(
                "cx.bind_keys() is not allowed during the `{}` phase; bind keys from \
                 init(), an event handler or a task",
                phase.map(ScopePhase::as_str).unwrap_or("none")
            ),
        ));
    }

    let os = scope::with_current_app(|cx| cx.platform_info().os)
        .ok_or_else(|| js_error::type_error(&ctx, "cx.bind_keys() needs a live host call"))?;
    let mut parsed = Vec::with_capacity(bindings.len());
    for (index, binding) in bindings.iter().enumerate() {
        let keystroke: String = binding.get("keystroke").map_err(|_| {
            js_error::type_error(
                &ctx,
                &format!("binding {index} needs a `keystroke`, such as \"cmd-s\""),
            )
        })?;
        let action: String = binding.get("action").map_err(|_| {
            js_error::type_error(
                &ctx,
                &format!("binding {index} needs an `action`, such as \"save\""),
            )
        })?;
        if action.is_empty() {
            return Err(js_error::type_error(
                &ctx,
                &format!("binding {index} has an empty `action`"),
            ));
        }
        let context: Option<String> = binding.get("context").ok();
        // `KeyBinding::new` panics on a bad keystroke or a bad predicate, and a
        // script typo must not take the process down, so both are parsed here
        // where the failure is a thrown error naming the entry.
        for source in keystroke.split_whitespace() {
            gpui::Keystroke::parse_with_os(source, os).map_err(|error| {
                js_error::type_error(
                    &ctx,
                    &format!("binding {index} has an unparsable keystroke `{source}`: {error:?}"),
                )
            })?;
        }
        if keystroke.split_whitespace().next().is_none() {
            return Err(js_error::type_error(
                &ctx,
                &format!("binding {index} has an empty `keystroke`"),
            ));
        }
        if let Some(context) = context.as_deref() {
            gpui::KeyBindingContextPredicate::parse(context).map_err(|error| {
                js_error::type_error(
                    &ctx,
                    &format!("binding {index} has an unparsable `context` `{context}`: {error}"),
                )
            })?;
        }
        parsed.push((keystroke, action, context));
    }

    let installed = parsed.len() as u32;
    scope::with_current(|_, app| {
        let bindings = parsed
            .into_iter()
            .map(|(keystroke, action, context)| {
                let context = context.as_deref().map(|context| {
                    gpui::KeyBindingContextPredicate::parse(context)
                        .expect("binding contexts were validated before installation")
                        .into()
                });
                gpui::KeyBinding::load(
                    &keystroke,
                    Box::new(ShellAction::new(action)),
                    context,
                    false,
                    None,
                    app,
                )
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        app.bind_keys(bindings);
        Ok::<_, gpui::InvalidKeystrokeError>(())
    })
    .ok_or_else(|| {
        js_error::type_error(
            &ctx,
            "cx.bind_keys() needs a live host call; call it from init(), an event \
             handler or a task",
        )
    })?
    .map_err(|error| js_error::type_error(&ctx, &format!("invalid keybinding: {error}")))?;
    Ok(installed)
}

/// The three measurements that answer an object.
///
/// Free functions rather than closures because each has to name one `'js` for
/// both the `Ctx` it takes and the `Object` it answers, and a closure cannot
/// unify the two.
fn viewport_size<'js>(ctx: Ctx<'js>) -> JsResult<Object<'js>> {
    let size = read(&ctx, "window.viewport_size()", |window| {
        window.viewport_size()
    })?;
    let object = Object::new(ctx)?;
    object.set("width", f32::from(size.width))?;
    object.set("height", f32::from(size.height))?;
    Ok(object)
}

fn bounds<'js>(ctx: Ctx<'js>) -> JsResult<Object<'js>> {
    let bounds = read(&ctx, "window.bounds()", |window| window.bounds())?;
    let object = Object::new(ctx)?;
    object.set("x", f32::from(bounds.origin.x))?;
    object.set("y", f32::from(bounds.origin.y))?;
    object.set("width", f32::from(bounds.size.width))?;
    object.set("height", f32::from(bounds.size.height))?;
    Ok(object)
}

fn mouse_position<'js>(ctx: Ctx<'js>) -> JsResult<Object<'js>> {
    let position = read(&ctx, "window.mouse_position()", |window| {
        window.mouse_position()
    })?;
    let object = Object::new(ctx)?;
    object.set("x", f32::from(position.x))?;
    object.set("y", f32::from(position.y))?;
    Ok(object)
}

/// The four platform appearances, as the two a script can act on.
///
/// The vibrancy variants differ in how the platform paints *behind* the
/// window, which a script cannot influence and does not need to branch on —
/// what it needs to know is whether it is drawing on light or on dark.
///
/// A free function so it can be tested against all four. The test window
/// reports `Light` and offers no way to change it from outside GPUI, so
/// exercising this through a rendered window would only ever prove one arm —
/// the same hole that let a platform-dependent keystroke spelling ship here.
fn appearance_name(appearance: WindowAppearance) -> &'static str {
    match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => "dark",
        WindowAppearance::Light | WindowAppearance::VibrantLight => "light",
    }
}

/// A measurement, legal from any host call including `render`.
fn read<R>(ctx: &Ctx<'_>, api: &str, body: impl FnOnce(&Window) -> R) -> JsResult<R> {
    scope::with_current(|window, _| body(window)).ok_or_else(|| {
        js_error::type_error(
            ctx,
            &format!(
                "{api} needs a live host call; call it from render(), init(), \
                 an event handler or a task"
            ),
        )
    })
}

/// A change to the window, refused during a render pass.
fn write(ctx: &Ctx<'_>, api: &str, body: impl FnOnce(&mut Window, &mut gpui::App)) -> JsResult<()> {
    let phase = scope::current_phase();
    if !phase.is_some_and(ScopePhase::allows_notify) {
        return Err(js_error::type_error(
            ctx,
            &format!(
                "{api} is not allowed during the `{}` phase; the window may only be \
                 changed while handling an event or a task",
                phase.map(ScopePhase::as_str).unwrap_or("none")
            ),
        ));
    }

    scope::with_current(|window, app| body(window, app)).ok_or_else(|| {
        js_error::type_error(
            ctx,
            &format!("{api} needs a live host call; call it from an event handler or a task"),
        )
    })
}

#[cfg(test)]
mod tests {
    use gpui::WindowAppearance;

    /// Every appearance reduces, and the vibrant pair reduces the same way as
    /// the plain pair.
    #[test]
    fn every_platform_appearance_reduces_to_light_or_dark() {
        assert_eq!(super::appearance_name(WindowAppearance::Light), "light");
        assert_eq!(
            super::appearance_name(WindowAppearance::VibrantLight),
            "light",
            "vibrancy is about what the platform paints behind the window, not about \
             what the script draws on"
        );
        assert_eq!(super::appearance_name(WindowAppearance::Dark), "dark");
        assert_eq!(
            super::appearance_name(WindowAppearance::VibrantDark),
            "dark"
        );
    }
}
