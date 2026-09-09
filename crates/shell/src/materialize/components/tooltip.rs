//! `tooltip(text)` — the hover label, and the reason the trigger is written
//! here rather than taken from base.
//!
//! Base ships the two ends of a tooltip and nothing that joins them.
//! [`gpui_base::Tooltip`] is the popup box — `div().role(Role::Tooltip)`, with
//! no color, no box and no position of its own — and
//! [`gpui_base::TooltipOverlay`] is the per-window layer that shows one, with
//! the show delay, the grace period between two triggers, the deferred paint
//! above every other layer and the window-edge clamping already inside it.
//! What base has no part of is the *trigger*: `gpui-component` writes that
//! itself, in `crates/component/src/tooltip.rs`, against its own `Root`.
//! [`crate::root::ShellRoot`] is a different root, so the shell writes it once
//! more, here.
//!
//! The trigger is three listeners and no state. Prepaint records the box the
//! popup is anchored to — the overlay is a sibling of the whole content tree
//! and has no other way to learn where the trigger is. Hover asks the overlay
//! to show or to hide. A press dismisses: a label explaining a button should
//! not still be up while the button is being used.
//!
//! # Why the content is a Rust view rather than a script one
//!
//! `TooltipRequest`'s builder is `Rc<dyn Fn(&mut Window, &mut App) -> AnyView>`
//! and the overlay calls it on *every frame the tooltip is up*. A script
//! closure in that slot would be the first content in the shell to outlive the
//! render that described it and then be re-entered once a frame — the VM back
//! on the frame path, which is the one thing the snapshot exists to prevent.
//!
//! So the bound form takes a string, and what it puts on screen is
//! [`TooltipLabel`]: one small view, built when the pointer arrives and cloned
//! by every frame after it. That covers what tooltips are actually for — the
//! name of an icon-only button — for no script cost at all.
//!
//! A `tooltip(() => Element)` form is possible over the same overlay and is
//! deliberately left for later; `.scratch/bindings/tooltip.md` records what it
//! needs and the one trap in it.
//!
//! # Placement
//!
//! `TooltipRequest` offers `placement` and nothing else: `TooltipPositioner`
//! nails the window margin to 4px and never reaches the shared positioner's
//! `align` or `offset`. Nothing here invents the two base does not have, and
//! the first bound form does not expose the one it does — a script that needs
//! to choose a side is the case for adding it, not a reason to guess now.

use std::{cell::Cell, rc::Rc};

use gpui::{
    AppContext as _, Bounds, Context, IntoElement, MouseButton, ParentElement, Pixels, Render,
    SharedString, StatefulInteractiveElement, Styled as _, Window,
};
use gpui_base::{ElementExt as _, Theme, Tooltip, TooltipRequest};

use crate::{materialize::Behavior, root::ShellRoot, spec::Component};

/// The view a string tooltip is shown as.
///
/// Base's `Tooltip` carries the role and nothing else, which is the boundary
/// working as intended: the box, the palette and the spacing are the
/// application's, and for a script application the shell is the application.
/// The tokens are read in `render` rather than captured at construction so that
/// a tooltip left up across a theme switch repaints in the new palette.
struct TooltipLabel {
    text: SharedString,
}

impl Render for TooltipLabel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = Theme::global(cx).tokens;
        let (colors, radius, spacing) = (tokens.colors, tokens.radius, tokens.spacing);

        Tooltip::new("shell-tooltip")
            // The positioner puts the box against the trigger; this is the gap
            // between the two, and it belongs to the content because the
            // positioner has no spacing of its own.
            .m(spacing.xs)
            .px(spacing.sm)
            .py(spacing.xxs)
            .bg(colors.surface)
            .text_color(colors.surface_foreground)
            .border_1()
            .border_color(colors.border)
            .rounded(radius.md)
            .text_sm()
            .child(self.text.clone())
    }
}

/// Wires the hover trigger, when the script asked for one.
///
/// Takes the whole [`Behavior`] rather than the text so that an element with no
/// `tooltip` pays nothing: the listeners, the prepaint canvas and the shared
/// cell are all built only on the branch that has something to show.
pub(in crate::materialize) fn with_tooltip<E>(element: E, behavior: &Behavior) -> E
where
    E: StatefulInteractiveElement + ParentElement,
{
    let Some(text) = behavior.tooltip.clone() else {
        return element;
    };

    // The trigger's own box, written during prepaint and read when the pointer
    // arrives. A cell rather than a value because the two happen in different
    // passes: layout resolves the box, and the hover that needs it comes some
    // frames later.
    let trigger_bounds: Rc<Cell<Bounds<Pixels>>> = Rc::new(Cell::new(Bounds::default()));
    let writer = Rc::clone(&trigger_bounds);

    element
        .on_prepaint(move |bounds, _, _| writer.set(bounds))
        .on_hover(move |hovered, window, cx| {
            let Some(overlay) = ShellRoot::tooltip_overlay(window, cx) else {
                // No overlay layer means the window's first view is not a
                // `ShellRoot`, which is a host wiring mistake rather than a
                // script one — and one the window announces loudly elsewhere.
                return;
            };
            if !*hovered {
                overlay.update(cx, |overlay, cx| overlay.request_hide(window, cx));
                return;
            }

            // Built here, not in the builder closure: the overlay calls that
            // closure once per frame, so constructing the view inside it would
            // create — and strand — a fresh entity on every frame the tooltip
            // is up.
            let label = cx.new(|_| TooltipLabel { text: text.clone() });
            let request =
                TooltipRequest::new(trigger_bounds.get(), move |_, _| label.clone().into());
            overlay.update(cx, |overlay, cx| overlay.request_show(request, window, cx));
        })
        .on_mouse_down(MouseButton::Left, |_, window, cx| {
            if let Some(overlay) = ShellRoot::tooltip_overlay(window, cx) {
                // `hide`, not `request_hide`: the grace period exists so that
                // moving between two triggers does not flash, and a press is
                // not that. The label has served its purpose the moment the
                // control is used.
                overlay.update(cx, |overlay, cx| overlay.hide(cx));
            }
        })
}

/// Reports a `tooltip` on a component that cannot carry the listeners.
///
/// Called once from [`materialize_node`](crate::materialize), which is the last
/// place the component and its behavior are together, rather than from each
/// component that does not support it — the list of those is every component
/// there is, and it grows with each one bound.
pub(in crate::materialize) fn warn_unhonoured_tooltip(component: &Component, behavior: &Behavior) {
    if behavior.tooltip.is_none() || honours_tooltip(component) {
        return;
    }
    tracing::warn!(
        "`tooltip` is not wired on a {}: it needs an element the shell owns the hover \
         listeners of, which today is a plain `div`, `h_flex` or `v_flex` and a `Button`. \
         Wrap it and write `tooltip` on the wrapper",
        component.name()
    );
}

/// The components [`with_tooltip`] is applied to.
///
/// Kept to the two that cover the case: a `Button`, which is what an icon-only
/// control is, and a plain element, which is how a script gives one to anything
/// else. Widening this is adding one call per component, not a new mechanism.
fn honours_tooltip(component: &Component) -> bool {
    matches!(
        component,
        Component::Div | Component::HFlex | Component::VFlex | Component::Button(_)
    )
}
