//! The native `gpui-fps` performance overlay.

use gpui::{AnyElement, IntoElement as _, StyleRefinement, prelude::FluentBuilder as _};

use crate::materialize::{
    Behavior, Children, StateStyles, warn_unhonoured_a11y, warn_without_surface,
};

pub(in crate::materialize) fn fps_monitor(
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> AnyElement {
    warn_unhonoured_a11y(&behavior, "fps_monitor", &[]);
    warn_without_surface("fps_monitor", &refinement, &states, &children);

    gpui_fps::fps_monitor(window, cx)
        .when_some(behavior.anchor, |monitor, anchor| monitor.anchor(anchor))
        .when_some(behavior.frame_budget, |monitor, budget| {
            monitor.frame_budget(budget)
        })
        .into_any_element()
}
