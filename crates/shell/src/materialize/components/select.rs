//! `Select`, `Combobox` and `DatePicker` — three headless combobox roots.
//!
//! None of the three holds a value, a list of options or a date, and none of
//! them draws anything. What they own is the combobox role, the controlled
//! `open` state, and the transfer of the keyboard between the trigger and the
//! popup content. Everything on screen — the trigger, the list, the highlight,
//! the calendar — is the script's own, put inside the root as ordinary
//! children, usually wrapped in a [`Popup`](super::popover::popup) so that it
//! floats above the window instead of pushing the layout around.
//!
//! # What the shell does not give them, and what a script can
//!
//! Base opens the surface on ↑ / ↓ / Enter, moves the keyboard to the content
//! handle, and then calls `cx.propagate()` — it expects whatever is inside the
//! surface to take over and run the highlight from there. In a Rust
//! application that "whatever" is a list with key bindings of its own.
//!
//! Nothing here supplies one, so out of the box the control is complete with a
//! pointer, opens and closes from the keyboard, and cannot be *navigated* from
//! the keyboard once open. That is written into `gpui-kit.d.ts` rather than left
//! for a script author to discover.
//!
//! What changed since this was written is that the script can now supply the
//! missing half itself: `on_key_down` on the content element, or ↑ / ↓ bound
//! to actions under its own `key_context` (§10.5, §10.6). Nothing in this file
//! had to change for that, which is what `content_focus_handle` was bound for
//! — the focus does move, so a script drawing its highlight from
//! `handle.is_focused()` already got the right answer.
//!
//! `DatePicker` loses more than the other two. It handles Confirm and Cancel
//! but sets no key context of its own, and every binding base installs is
//! scoped to one — `crates/component` supplies both halves for its own picker, and the
//! shell can supply neither. So its `on_open_change` never fires from the
//! keyboard at all, and a script opens the picker from a press on the trigger
//! it drew. What is left still earns the binding: the combobox role, the
//! announced expanded state, and a trigger that is really in the Tab order.
//!
//! # Why the active option is the script's job
//!
//! GPUI marks the active descendant on the *option element* rather than on the
//! container, so a root that never sees the options cannot mark one — base's
//! own doc comment says as much. A script highlights an option by calling
//! `aria_active_descendant()` on it. That is the whole protocol, and no wiring
//! here can stand in for it.

use std::rc::Rc;

use gpui::{
    AnyElement, App, FocusHandle, IntoElement, ParentElement, SharedString, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _,
};
use gpui_base::{Combobox, DatePicker, Select};

use crate::{
    engine::ShellRuntime,
    entities::EntityHandle,
    materialize::{
        Behavior, Children, StateStyles, content_focus, dispatch_change, dispatch_signal, finish,
        tracked_focus, warn_ignored_key, warn_unhonoured_a11y, warn_unsupported,
        with_active_and_focus, with_hover,
    },
};

/// The combobox surface `Select` and `Combobox` share.
///
/// The two are the same control twice over: base's `Combobox::render` forwards
/// every field to a `Select`, changing only the key context it announces
/// itself under. A trait rather than two copies of the builder chain, because
/// two chains would be two places for the next builder to be added to and only
/// one of them would get it.
trait ComboboxRoot: Sized {
    fn open(self, open: bool) -> Self;
    fn disabled(self, disabled: bool) -> Self;
    fn focus_handle(self, focus_handle: &FocusHandle) -> Self;
    fn content_focus_handle(self, focus_handle: &FocusHandle) -> Self;
    fn on_open_change(self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self;
    fn on_confirm(self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self;
    fn on_dismiss(self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self;
}

/// Both roots already have every one of these as an inherent builder of the
/// same name, so the impl is seven forwards and the macro is what keeps it
/// from being fourteen.
macro_rules! combobox_root {
    ($root:ty) => {
        impl ComboboxRoot for $root {
            fn open(self, open: bool) -> Self {
                <$root>::open(self, open)
            }
            fn disabled(self, disabled: bool) -> Self {
                <$root>::disabled(self, disabled)
            }
            fn focus_handle(self, focus_handle: &FocusHandle) -> Self {
                <$root>::focus_handle(self, focus_handle)
            }
            fn content_focus_handle(self, focus_handle: &FocusHandle) -> Self {
                <$root>::content_focus_handle(self, focus_handle)
            }
            fn on_open_change(
                self,
                handler: impl Fn(bool, &mut Window, &mut App) + 'static,
            ) -> Self {
                <$root>::on_open_change(self, handler)
            }
            fn on_confirm(self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
                <$root>::on_confirm(self, handler)
            }
            fn on_dismiss(self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
                <$root>::on_dismiss(self, handler)
            }
        }
    };
}

combobox_root!(Select);
combobox_root!(Combobox);

/// A select root. An accessible name is the one thing it has that a `Combobox`
/// does not.
pub(in crate::materialize) fn select(
    runtime: &Rc<ShellRuntime>,
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    let root = Select::new(SharedString::from(id))
        .when_some(behavior.accessibility_label.clone(), |root, label| {
            root.accessibility_label(label)
        });
    combobox_root(
        runtime, root, "Select", true, refinement, behavior, states, children,
    )
}

/// A combobox root: a `Select` under another key context, and nothing else.
pub(in crate::materialize) fn combobox(
    runtime: &Rc<ShellRuntime>,
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    combobox_root(
        runtime,
        Combobox::new(SharedString::from(id)),
        "Combobox",
        false,
        refinement,
        behavior,
        states,
        children,
    )
}

/// Everything the two share.
#[allow(clippy::too_many_arguments)]
fn combobox_root<R>(
    runtime: &Rc<ShellRuntime>,
    root: R,
    component: &'static str,
    names_itself: bool,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement
where
    R: ComboboxRoot + Styled + ParentElement + IntoElement + 'static,
{
    warn_ignored_key(&behavior, component);
    // The root builds its own combobox role and its own expanded state, and it
    // puts the trigger handle into tab traversal itself. The two focus calls
    // are what it honours; `track_focus` maps onto base's `focus_handle(...)`
    // because the shell has one word for "this handle is what the keyboard
    // means here", and a second spelling beside it would be a second thing to
    // learn for no difference in behavior.
    warn_unhonoured_a11y(
        &behavior,
        component,
        &["track_focus", "content_focus_handle"],
    );
    warn_unsupported(
        component,
        &[
            (
                "accessibility_label",
                !names_itself && behavior.accessibility_label.is_some(),
            ),
            ("default_open", behavior.default_open),
            ("overlay_closable", behavior.overlay_closable.is_some()),
            ("anchor", behavior.anchor.is_some()),
            ("mouse_button", behavior.mouse_button.is_some()),
            ("open_delay", behavior.open_delay.is_some()),
            ("close_delay", behavior.close_delay.is_some()),
        ],
    );
    // A `RenderOnce` over a plain `div`, like `Collapsible`: it refines the
    // style it was given and stops there, so there is no interactivity for a
    // hover or a pressed style to attach to.
    if states.hover.is_some() || states.active.is_some() || states.focus.is_some() {
        tracing::warn!(
            "state styles on a {component} are ignored; put them on the element you drew as its \
             trigger"
        );
    }

    let trigger = tracked_focus(runtime, &behavior, component);
    let content = content_focus(runtime, &behavior, component);
    if trigger.is_none() {
        tracing::warn!(
            "a {component} with no `track_focus` handle never takes the keyboard, so Escape, \
             Enter and the arrow keys reach nothing. Give it a FocusHandle and put the same \
             handle on the element you drew as its trigger"
        );
    }

    // A root that was never told an open state is one the script is not
    // controlling, and base has no uncontrolled mode here: `false` is where it
    // starts either way.
    let root = root
        .open(behavior.open.unwrap_or(false))
        .disabled(behavior.disabled)
        .when_some(trigger, |root, trigger| root.focus_handle(&trigger))
        .when_some(content, |root, content| root.content_focus_handle(&content))
        .when_some(behavior.on_open_change, |root, callback| {
            let runtime = Rc::downgrade(runtime);
            root.on_open_change(move |open, window, cx| {
                dispatch_change(&runtime, callback, open, window, cx);
            })
        })
        .when_some(behavior.on_confirm, |root, callback| {
            let runtime = Rc::downgrade(runtime);
            root.on_confirm(move |window, cx| {
                dispatch_signal(&runtime, callback, window, cx);
            })
        })
        .when_some(behavior.on_dismiss, |root, callback| {
            let runtime = Rc::downgrade(runtime);
            root.on_dismiss(move |window, cx| {
                dispatch_signal(&runtime, callback, window, cx);
            })
        });

    finish(root, refinement, children)
}

/// A date-picker root.
///
/// Its focus handle arrives from the constructor rather than from
/// `track_focus`, because base's `DatePicker::new` takes one: a picker with no
/// handle has no trigger the keyboard can reach, and there is no builder to
/// supply one afterwards.
pub(in crate::materialize) fn date_picker(
    runtime: &Rc<ShellRuntime>,
    id: String,
    focus: EntityHandle,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "DatePicker");
    // Nothing is honoured: the picker sets its own combobox role and tracks the
    // constructor's handle in its own `render`, so anything put here would be
    // overwritten a moment later.
    warn_unhonoured_a11y(&behavior, "DatePicker", &[]);
    warn_unsupported(
        "DatePicker",
        &[
            (
                "accessibility_label",
                behavior.accessibility_label.is_some(),
            ),
            ("on_confirm", behavior.on_confirm.is_some()),
            ("on_dismiss", behavior.on_dismiss.is_some()),
            ("default_open", behavior.default_open),
            ("overlay_closable", behavior.overlay_closable.is_some()),
            ("anchor", behavior.anchor.is_some()),
            ("mouse_button", behavior.mouse_button.is_some()),
        ],
    );

    let Some(focus) = runtime.entities().focus(focus) else {
        tracing::error!(
            "the focus handle given to DatePicker.new(\"{id}\", handle) has been released, so \
             there is no trigger left for the keyboard to reach and the picker is not rendered"
        );
        return div().into_any_element();
    };

    let picker = DatePicker::new(SharedString::from(id), &focus)
        .open(behavior.open.unwrap_or(false))
        .disabled(behavior.disabled);
    // Unlike the other two roots this one is a `Stateful<Div>` underneath and
    // implements both interactive traits, so the state styles land.
    let picker = with_hover(picker, &states);
    let picker = with_active_and_focus(picker, &states).when_some(
        behavior.on_open_change,
        |picker, callback| {
            let runtime = Rc::downgrade(runtime);
            picker.on_open_change(move |open, window, cx| {
                dispatch_change(&runtime, callback, open, window, cx);
            })
        },
    );

    finish(picker, refinement, children)
}
