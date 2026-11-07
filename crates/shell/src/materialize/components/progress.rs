//! `Progress`, `ProgressTrack` and `ProgressIndicator` — a progress bar
//! assembled from three parts, none of which draws anything.
//!
//! `Progress` is the announcement, not the bar: it carries the progress role
//! and the `0..=100` numeric value a screen reader reads out, and otherwise
//! paints exactly what any other empty element paints — nothing. The track and
//! the indicator carry no semantics at all; each is a plain element with the
//! script's own styles on it. So the visible bar is whatever the script builds:
//! a track sized and filled by the script, holding an indicator whose width the
//! script sets from the same number it passed to `value`.
//!
//! `value` is clamped to `0..=100` by base, and `indeterminate(true)` withdraws
//! the value from the accessibility tree rather than changing it — "still
//! working, no idea how far" is a different announcement from "at 40%". It does
//! not animate anything: a barber-pole or a sliding indicator is the script's to
//! draw, and `transition`/`spring` on the indicator is how it moves.

use gpui::{AnyElement, SharedString, StyleRefinement};
use gpui_base::{Progress, ProgressIndicator, ProgressTrack};

use crate::materialize::{
    Behavior, Children, StateStyles, finish, warn_ignored_key, warn_unhonoured_a11y,
    with_active_and_focus, with_hover,
};

/// The progress root. Its identity comes from `new(id)`, so `id()` is ignored.
pub(in crate::materialize) fn progress(
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "Progress");
    warn_unhonoured_a11y(&behavior, "Progress", &[]);
    let mut progress = Progress::new(SharedString::from(id)).indeterminate(behavior.indeterminate);

    if let Some(label) = behavior.accessibility_label.clone() {
        progress = progress.accessibility_label(label);
    }

    if let Some(value) = behavior.value {
        progress = progress.value(value);
    }

    let progress = with_hover(progress, &states);
    let progress = with_active_and_focus(progress, &states);
    finish(progress, refinement, children)
}

/// The groove the indicator moves in. Styling is the whole of it.
pub(in crate::materialize) fn progress_track(
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_ops(&behavior, &states, "ProgressTrack");
    finish(ProgressTrack::new(), refinement, children)
}

/// The filled part. The script sizes it from the same number it gave `value`.
pub(in crate::materialize) fn progress_indicator(
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_ops(&behavior, &states, "ProgressIndicator");
    finish(ProgressIndicator::new(), refinement, children)
}

/// Neither part is interactive — they are `Div` with a style and children and
/// nothing else — so a state style on one has nowhere to land. Saying so is
/// better than dropping it without a word.
fn warn_ignored_ops(behavior: &Behavior, states: &StateStyles, component: &str) {
    if let Some(key) = &behavior.key {
        tracing::warn!(
            "id(\"{key}\") is ignored on a {component}: it is not interactive, so it has no \
             state for an identity to keep"
        );
    }
    if !states.is_empty() {
        tracing::warn!(
            "state styles on a {component} are ignored; put them on the Progress around it"
        );
    }
}
