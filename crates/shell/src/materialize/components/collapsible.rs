//! `Collapsible` — an element with one gated slot, and nothing else.
//!
//! Base's `Collapsible` is a `div` that renders the element in its `content`
//! slot only while `open` is true. It has no role, announces no expanded state,
//! draws no chevron and animates nothing; ordinary children are rendered either
//! way, which is where a header or a trigger goes. The button that flips the
//! state, the look of it and any transition are the script's own, exactly as
//! they are for a `Checkbox`.
//!
//! It is the first user of [`SpecOp::Slot`](crate::spec::SpecOp::Slot) because
//! one `children` list cannot say "this one is different": the content has to
//! be renderable somewhere else, or not at all.

use gpui::{
    AnyElement, IntoElement as _, ParentElement as _, Refineable as _, StyleRefinement,
    Styled as _, prelude::FluentBuilder as _,
};
use gpui_base::Collapsible;

use crate::materialize::{
    Behavior, Children, Slots, StateStyles, take_slot, warn_unhonoured_a11y, warn_unread_slots,
};

/// The collapsible root. It has no `new(id)`, so `id()` is not ignored here:
/// it names the element for motion, which is the one thing an identity buys on
/// something neither interactive nor stateful.
pub(in crate::materialize) fn collapsible(
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    mut slots: Slots,
    children: Children,
) -> AnyElement {
    let content = take_slot(&mut slots, "content");
    warn_unread_slots(&slots, "Collapsible");
    warn_unhonoured_a11y(&behavior, "Collapsible", &[]);

    // `Collapsible` implements `Styled` and `ParentElement` and stops there —
    // it is a `RenderOnce` over a plain `div`, not an interactive element — so
    // hover, active and focus have nowhere to land. As with `Switch`, saying so
    // beats dropping them without a word.
    if !states.is_empty() {
        tracing::warn!(
            "state styles on a Collapsible are ignored; put them on the element around it"
        );
    }

    // Ordinary children first, then the content — base renders both in the
    // order the builder was called, and the description cannot say where among
    // its children the script wanted the slot. Below is the answer that matches
    // what a collapsible is: the always-visible header is a child, and the part
    // that appears is under it.
    // A collapsible has no uncontrolled mode, so a script that never said is a
    // script that meant closed.
    let mut element = Collapsible::new().open(behavior.open.unwrap_or(false));
    element.style().refine(&refinement);
    element.extend(children);
    element
        .when_some(content, |collapsible, content| collapsible.content(content))
        .into_any_element()
}
