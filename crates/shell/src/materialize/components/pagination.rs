//! `Pagination` — the landmark, and none of the buttons.
//!
//! Base's `Pagination` is a `div` announcing itself as a navigation landmark
//! with a label. It draws no page buttons, no arrows and no ellipsis: those are
//! the script's own elements, written with the style surface.
//!
//! The part a script genuinely cannot write for itself is which page numbers
//! to show — the layout that keeps the first page, the last page, a window
//! around the current one, and an ellipsis where the run is broken. That is a
//! calculation rather than an element, so it is exported as the free function
//! `pagination_items(...)` rather than folded into this component — and lives
//! beside that export in the engine, not here, because nothing in this
//! directory is reachable from outside the render path.
//!
//! `PaginationState`'s other members are deliberately not bound. `on_change`,
//! `request_page`, `previous_page` and `next_page` all answer questions a
//! script that already holds `current_page` and `total_pages` can answer with
//! arithmetic; binding them would move a script's own state into a Rust value
//! that has to be rebuilt every frame to stay in step with it.

use gpui::{
    AnyElement, IntoElement as _, ParentElement as _, Refineable as _, SharedString,
    StyleRefinement, Styled as _,
};
use gpui_base::{Pagination, PaginationState};

use crate::materialize::{Behavior, Children, StateStyles, warn_unhonoured_a11y};

/// The pagination root.
pub(in crate::materialize) fn pagination(
    id: &str,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_unhonoured_a11y(&behavior, "Pagination", &["accessibility_label"]);
    if !states.is_empty() {
        tracing::warn!(
            "state styles on a Pagination are ignored; put them on the page buttons inside it, \
             which are the interactive elements"
        );
    }

    // `Pagination::new` requires a state, and its `render` does not read one:
    // the root announces the label and lays out its children, and that is all.
    // So this is a placeholder to satisfy the constructor, not a number the
    // control shows — the page a script is on lives in the script, and reaches
    // the screen through the buttons it builds from `pagination_items`.
    //
    // If base ever announces the position from the state — "page 3 of 20" is
    // the obvious thing to add — this stops being adequate and the shell has
    // to carry the two numbers across. There is no announcement to regress
    // today, so there is nothing to carry yet.
    let mut element = Pagination::new(
        SharedString::from(id.to_owned()),
        PaginationState::new(1, 1),
    );
    if let Some(label) = behavior.accessibility_label.clone() {
        element = element.accessibility_label(label);
    }
    element.style().refine(&refinement);
    element.extend(children);
    element.into_any_element()
}
