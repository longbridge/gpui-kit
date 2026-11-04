//! `Slider`, `SliderTrack`, `SliderIndicator` and `SliderThumb` — the four
//! parts of a slider, composed and styled by the script, positioned by Rust.
//!
//! Base's `Slider` draws nothing at all: "applications provide the track, range
//! and thumb presentation as children". Each part carries a different piece of
//! the behavior and none of them is optional. The root owns the role, the
//! announced value and the release; the track owns the press and the drag; the
//! **indicator owns the geometry** — its `on_prepaint` is the only thing that
//! ever writes `SliderState::set_bounds`, and every pointer position is mapped
//! through those bounds; the thumb owns its own drag.
//!
//! # Why the geometry is not the script's
//!
//! The value lives in the `SliderState`, and dragging writes it there straight
//! from a GPUI drag listener — no callback, no script, no new description. That
//! is what keeps a drag off the VM, and it is also what would make a
//! script-computed thumb position wrong: the position would be frozen into the
//! snapshot the last script render produced, so the user would drag, the state
//! would change, GPUI would repaint — and the thumb would not move. The
//! announced value *would* move, because base reads that off the state while
//! the root is being built, so the failure reads as "the screen reader says 60
//! and the knob is still at 20".
//!
//! So the two percentage-derived boxes are written here, read from
//! `SliderState::percentage()` on every frame:
//!
//! * the thumb's inset along the axis, and
//! * the box of the filled part — which base's own doc comment calls the range,
//!   and which the script describes through `range_style` rather than as an
//!   element of its own, for the reason below.
//!
//! Everything else — which parts are nested where, their size, color, radius
//! and the rest — is the script's, exactly as it is for a `Progress`. This is
//! the "geometry belongs to Rust, composition and styling belong to the script"
//! arrangement recorded in the implementation plan.
//!
//! # Why the filled part is a style rather than a child
//!
//! Base has no component for it: in `gpui-component`'s own slider the fill is a
//! plain `div` inside the indicator, positioned from the same percentage. A
//! plain child cannot be given that position from here — by the time a child
//! reaches this module it is an `AnyElement`, whose style is sealed — and a
//! child the script positions itself is the frozen thumb again. So the script
//! declares how the fill *looks* and the shell owns where it *is*, which is the
//! same split `Calendar`'s `item_style` makes for the same reason.
//!
//! # Why a missing `SliderIndicator` is reported
//!
//! `update_value_by_position` divides by the recorded bounds. With no indicator
//! in the tree those bounds stay `Bounds::default()`, the division is by zero,
//! and the value becomes `NaN` — a slider that cannot be moved, reports no
//! error, and looks exactly like one that can. So [`warn_without_indicator`]
//! walks the slider's subtree while the description is still addressable, and
//! says so once.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Axis, IntoElement as _, Position, Refineable as _, StyleRefinement,
    Styled as _, div, relative,
};
use gpui_base::{Slider, SliderIndicator, SliderThumb, SliderTrack};

use crate::{
    engine::ShellRuntime,
    entities::EntityHandle,
    materialize::{
        Behavior, Children, StateStyles, finish, warn_ignored_key, warn_unhonoured_a11y,
        with_active_and_focus, with_hover,
    },
    spec::{Component, SpecArena, SpecId, SpecNode},
};

/// Reports a `Slider` whose subtree records no geometry.
///
/// Called from `materialize_node` rather than from [`slider`] below, because it
/// needs the arena and the node's address in it — and the dispatch that reaches
/// the components has neither. See the module comment for what goes wrong.
pub(in crate::materialize) fn warn_without_indicator(
    arena: &SpecArena,
    node: SpecId,
    handle: EntityHandle,
) {
    if records_bounds(arena, node, handle) {
        return;
    }
    tracing::warn!(
        "this Slider has no SliderIndicator under it, so nothing records the box pointer \
         positions are measured against: pressing or dragging it divides by a zero-sized box \
         and leaves the value stuck, with nothing on screen to say so. Nest a SliderIndicator \
         built from the same state inside the SliderTrack"
    );
}

/// The behavior root. It announces the value and owns the release; it draws
/// nothing.
///
/// Its identity is the state's, so `id()` has nowhere to go.
pub(in crate::materialize) fn slider(
    runtime: &Rc<ShellRuntime>,
    handle: EntityHandle,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "Slider");
    // The root builds its own accessibility node — `Role::Slider`, the value,
    // the bounds of the value, the orientation and the increment and decrement
    // actions — and has no builder to override any of it.
    warn_unhonoured_a11y(&behavior, "Slider", &[]);
    let Some(state) = runtime.entities().slider(handle) else {
        tracing::error!("slider handle {handle} is no longer live");
        return div().into_any_element();
    };

    // `Slider` is a `RenderOnce` over a `Div` and implements neither
    // `InteractiveElement` nor `StatefulInteractiveElement`, so a state style
    // here has nowhere to land. As with `Switch`, saying so beats dropping it
    // without a word.
    warn_unstateful(&states, "Slider", "SliderTrack");

    let slider = Slider::new(&state)
        .axis(axis(&behavior))
        .disabled(behavior.disabled);
    finish(slider, refinement, children)
}

/// The press and drag surface. It records nothing and draws nothing.
pub(in crate::materialize) fn slider_track(
    runtime: &Rc<ShellRuntime>,
    handle: EntityHandle,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "SliderTrack");
    warn_unhonoured_a11y(&behavior, "SliderTrack", &[]);
    let Some(state) = runtime.entities().slider(handle) else {
        tracing::error!("slider handle {handle} is no longer live");
        return div().into_any_element();
    };

    // `SliderTrack` is interactive but not stateful: hover lands on it, the
    // other two have no element identity to hang on.
    if states.active.is_some() || states.focus.is_some() {
        tracing::warn!(
            "`active` and `focus` styles on a SliderTrack are ignored: it has no stable element \
             identity of its own. Put them on the SliderIndicator inside it"
        );
    }

    let track = SliderTrack::new(&state)
        .axis(axis(&behavior))
        .disabled(behavior.disabled);
    let track = with_hover(track, &states);
    finish(track, refinement, children)
}

/// The groove, and the one part that records the geometry.
///
/// It must span the whole travel of the slider: the bounds it records are what
/// every pointer position is divided by, so an indicator sized to the value
/// would make the value its own scale.
pub(in crate::materialize) fn slider_indicator(
    runtime: &Rc<ShellRuntime>,
    handle: EntityHandle,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
    cx: &App,
) -> AnyElement {
    warn_ignored_key(&behavior, "SliderIndicator");
    warn_unhonoured_a11y(&behavior, "SliderIndicator", &[]);
    let Some(state) = runtime.entities().slider(handle) else {
        tracing::error!("slider handle {handle} is no longer live");
        return div().into_any_element();
    };

    let percentage = state.read(cx).percentage();
    let indicator = SliderIndicator::new(&state);
    let indicator = with_hover(indicator, &states);
    let indicator = with_active_and_focus(indicator, &states);

    // The fill goes in before the ordinary children, so a thumb nested here
    // paints over it rather than under it. The description cannot say where
    // among its children the script wanted it, and this is the only order a
    // slider is ever drawn in.
    let mut all: Children = Children::new();
    if let Some(fill) = behavior.range_style.clone() {
        let mut element = div();
        element.style().refine(&fill);
        span(
            element.style(),
            axis(&behavior),
            percentage.start,
            percentage.end,
        );
        all.push(element.into_any_element());
    }
    all.extend(children);
    finish(indicator, refinement, all)
}

/// The knob. Its position along the axis is written here, from the state.
///
/// Unlike the other three, `id()` is honoured: the two thumbs of a range slider
/// are built from one state, so the handle cannot tell them apart, and a script
/// animating one of them needs a name to hang the motion on.
pub(in crate::materialize) fn slider_thumb(
    runtime: &Rc<ShellRuntime>,
    handle: EntityHandle,
    mut refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
    cx: &App,
) -> AnyElement {
    warn_unhonoured_a11y(&behavior, "SliderThumb", &[]);
    let Some(state) = runtime.entities().slider(handle) else {
        tracing::error!("slider handle {handle} is no longer live");
        return div().into_any_element();
    };

    let percentage = state.read(cx).percentage();
    let along = if behavior.start {
        percentage.start
    } else {
        percentage.end
    };
    pin(&mut refinement, axis(&behavior), along);

    let thumb = SliderThumb::new(&state)
        .axis(axis(&behavior))
        .start(behavior.start)
        .disabled(behavior.disabled);
    let thumb = with_hover(thumb, &states);
    let thumb = with_active_and_focus(thumb, &states);
    finish(thumb, refinement, children)
}

/// The axis a part was told it is on.
///
/// Each part is told separately, as it is in Rust: the axis is a builder on the
/// root, the track and the thumb rather than something the root hands down. All
/// four default to horizontal, so a vertical slider says so on each of them.
fn axis(behavior: &Behavior) -> Axis {
    behavior.axis.unwrap_or(Axis::Horizontal)
}

/// Spans a box between two percentages of its parent, along one axis.
///
/// The cross axis is filled only when the declaration left it open, so a script
/// that wants a fill thinner than its groove can still say so — but the axis
/// the value runs along is not negotiable.
///
/// Vertically the range grows from the bottom, which is what makes a vertical
/// slider read as "more is up", and is why the start of the range is the bottom
/// inset rather than the top one.
fn span(refinement: &mut StyleRefinement, axis: Axis, start: f32, end: f32) {
    refinement.position = Some(Position::Absolute);
    match axis {
        Axis::Horizontal => {
            refinement.inset.left = Some(relative(start).into());
            refinement.inset.right = Some(relative(1. - end).into());
            if refinement.size.height.is_none() {
                refinement.size.height = Some(relative(1.).into());
            }
        }
        Axis::Vertical => {
            refinement.inset.bottom = Some(relative(start).into());
            refinement.inset.top = Some(relative(1. - end).into());
            if refinement.size.width.is_none() {
                refinement.size.width = Some(relative(1.).into());
            }
        }
    }
}

/// Pins a box to one percentage of its parent, along one axis.
///
/// Only the near edge, because the thumb has a size of its own: setting both
/// insets and a width over-constrains the box, and which of the three wins is
/// not something a script should have to know. The far inset is dropped for the
/// same reason — the position along the axis is not the script's to set, so a
/// leftover `right(...)` would be a silent fight with this.
fn pin(refinement: &mut StyleRefinement, axis: Axis, along: f32) {
    refinement.position = Some(Position::Absolute);
    match axis {
        Axis::Horizontal => {
            refinement.inset.left = Some(relative(along).into());
            refinement.inset.right = None;
        }
        Axis::Vertical => {
            refinement.inset.bottom = Some(relative(along).into());
            refinement.inset.top = None;
        }
    }
}

/// Reports state styles on a part that cannot carry any.
fn warn_unstateful(states: &StateStyles, component: &str, instead: &str) {
    if !states.is_empty() {
        tracing::warn!(
            "state styles on a {component} are ignored: it is not an interactive element. Put \
             them on the {instead} inside it"
        );
    }
}

/// Whether anything under this node is the `SliderIndicator` that records the
/// bounds for this state.
///
/// Ordinary children only. A slot element is rendered somewhere of its
/// component's own choosing, and no slot on any bound component is a place a
/// slider part belongs — so a subtree reached only through one is a subtree
/// this slider does not contain.
///
/// The handle has to match: an indicator built from *another* slider's state
/// records that slider's bounds, which leaves this one exactly as stuck as
/// having none at all, and is the easier mistake of the two to make.
fn records_bounds(arena: &SpecArena, node: SpecId, handle: EntityHandle) -> bool {
    let Some(node) = arena.node(node) else {
        return false;
    };
    node.children().iter().any(|child| {
        matches!(
            arena.node(*child).and_then(SpecNode::component),
            Some(Component::SliderIndicator(found)) if *found == handle
        ) || records_bounds(arena, *child, handle)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A slider nests its indicator inside its track, so the check has to see
    /// through one level of nesting — and it has to answer for the state it
    /// was asked about rather than for any slider at all.
    #[test]
    fn a_slider_without_its_own_indicator_is_detected() {
        // Built bottom-up, the way a script builds one: a node that has been
        // attached can no longer be given children.
        let slider = |indicator: Option<EntityHandle>| {
            let mut arena = SpecArena::new();
            let track = arena.push(Component::SliderTrack(7));
            if let Some(handle) = indicator {
                let indicator = arena.push(Component::SliderIndicator(handle));
                arena.attach(track, indicator).unwrap();
            }
            let root = arena.push(Component::Slider(7));
            arena.attach(root, track).unwrap();
            records_bounds(&arena, root, 7)
        };

        assert!(!slider(None), "a track alone records nothing");
        assert!(
            !slider(Some(9)),
            "another slider's indicator records another slider's bounds, which \
             leaves this one exactly as stuck"
        );
        assert!(slider(Some(7)));
    }

    /// The fill spans the value; the thumb sits on one end of it.
    #[test]
    fn geometry_is_written_along_the_axis_the_part_was_given() {
        let mut horizontal = StyleRefinement::default();
        span(&mut horizontal, Axis::Horizontal, 0.25, 0.75);
        assert_eq!(horizontal.position, Some(Position::Absolute));
        assert_eq!(horizontal.inset.left, Some(relative(0.25).into()));
        assert_eq!(horizontal.inset.right, Some(relative(0.25).into()));
        assert_eq!(horizontal.size.height, Some(relative(1.).into()));
        assert_eq!(horizontal.inset.top, None);

        // A declared thickness is a style, so it survives; the two insets along
        // the axis are geometry, so they do not.
        let mut declared = StyleRefinement::default();
        declared.size.height = Some(gpui::px(4.).into());
        declared.inset.left = Some(relative(0.9).into());
        span(&mut declared, Axis::Horizontal, 0.0, 0.5);
        assert_eq!(declared.size.height, Some(gpui::px(4.).into()));
        assert_eq!(declared.inset.left, Some(relative(0.0).into()));

        // Vertically the range grows from the bottom.
        let mut vertical = StyleRefinement::default();
        span(&mut vertical, Axis::Vertical, 0.25, 0.75);
        assert_eq!(vertical.inset.bottom, Some(relative(0.25).into()));
        assert_eq!(vertical.inset.top, Some(relative(0.25).into()));
        assert_eq!(vertical.size.width, Some(relative(1.).into()));

        // The thumb takes the near edge alone, and gives up the far one: it has
        // a size of its own, so two insets and a width would over-constrain it.
        let mut thumb = StyleRefinement::default();
        thumb.inset.right = Some(relative(0.1).into());
        pin(&mut thumb, Axis::Horizontal, 0.4);
        assert_eq!(thumb.position, Some(Position::Absolute));
        assert_eq!(thumb.inset.left, Some(relative(0.4).into()));
        assert_eq!(thumb.inset.right, None);
    }
}
