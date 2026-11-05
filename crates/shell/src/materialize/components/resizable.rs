//! `h_resizable` / `v_resizable` / `resizable_panel` — panes a user drags apart.
//!
//! Two things make this the first component that cannot use the ordinary
//! children path.
//!
//! `ResizablePanelGroup::child` takes a `ResizablePanel`, not an element. Base
//! does offer `impl From<T: Into<AnyElement>> for ResizablePanel`, so any
//! element *can* be wrapped — but a panel made that way has no `size`, no
//! `size_range` and no `visible`, which is most of what a script came here to
//! say. So the group reads its children as descriptions and builds the typed
//! value itself; see [`ChildSpecs`](crate::materialize::ChildSpecs).
//!
//! And the group is not an element the way the rest are: it implements neither
//! `Styled` nor `InteractiveElement`, and its own render is `size_full()`. A
//! group with nothing around it therefore has no size at all, which is why a
//! Rust caller always writes `div().w(400).h(100).child(h_resizable(…))`. The
//! script's styles land on that frame, built here.
//!
//! Sizes survive a repaint without the script holding them: base keeps them in
//! window element state, keyed by the group's own id (`use_keyed_state` in
//! `panel.rs`). A drag therefore stays put across frames that never enter the
//! VM, and `on_resize` is a notification rather than the thing that makes
//! resizing work — a group that ignores it still resizes.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Axis, ElementId, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Refineable as _, SharedString, StyleRefinement, Styled as _, Window, div,
};
use gpui_base::{ResizablePanel, h_resizable, resizable_panel, v_resizable};

use crate::{
    materialize::{
        Behavior, ChildSpecs, Children, NodeParts, StateStyles, dispatch_resize, finish,
        warn_ignored_key, warn_unhonoured_a11y, warn_unsupported, with_active_and_focus,
        with_hover,
    },
    spec::Component,
};

/// The group. Its identity comes from the constructor's id, so `id()` is
/// ignored — and that identity is also where base files the panel sizes, which
/// is why it has to be the script's stable name rather than a tree position.
#[allow(clippy::too_many_arguments)]
pub(in crate::materialize) fn panel_group(
    specs: ChildSpecs<'_>,
    id: String,
    axis: Axis,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let component = match axis {
        Axis::Horizontal => "h_resizable",
        Axis::Vertical => "v_resizable",
    };
    warn_ignored_key(&behavior, component);
    warn_unhonoured_a11y(&behavior, component, &[]);
    // The orientation is the constructor, exactly as in Rust: there is one
    // `axis` word in the whole script API, and on a group it names something
    // already decided.
    warn_unsupported(component, &[("axis", behavior.axis.is_some())]);

    let name = SharedString::from(id);
    let mut group = match axis {
        Axis::Horizontal => h_resizable(ElementId::Name(name.clone())),
        Axis::Vertical => v_resizable(ElementId::Name(name.clone())),
    };

    for child in specs.ids() {
        group = match specs.component(*child) {
            // The one case the flattening would have destroyed. Everything
            // else keeps base's own wrapping, which is what lets a script put
            // a plain `div()` in a group and get a panel with default
            // constraints.
            Some(Component::ResizablePanel) => match specs.parts(*child, window, cx) {
                Some(parts) => group.child(panel(parts)),
                None => group,
            },
            _ => group.child(specs.element(*child, window, cx)),
        };
    }

    if let Some(callback) = behavior.on_resize {
        let runtime = Rc::downgrade(specs.runtime());
        group = group.on_resize(move |state, window, cx| {
            // The script is handed the sizes rather than the state entity: it
            // has no handle for a `ResizableState`, and the sizes are the whole
            // of what the Rust callback is read for. Pixels, in the group's
            // child order.
            let sizes = state
                .read(cx)
                .sizes()
                .iter()
                .map(|size| size.as_f32())
                .collect::<Vec<_>>();
            dispatch_resize(&runtime, callback, sizes, window, cx);
        });
    }

    // The frame the group has no way to be. Named apart from the group's own
    // id so the two never share an element-state slot.
    let frame = div().id(ElementId::Name(SharedString::from(format!(
        "gpui-shell-resizable:{name}"
    ))));
    let frame = with_hover(frame, &states);
    let frame = with_active_and_focus(frame, &states);
    let mut children = Children::new();
    children.push(group.into_any_element());
    finish(frame, refinement, children)
}

/// One panel, built from its description rather than from a finished element.
///
/// `size`, `size_range` and `visible` exist on `ResizablePanel` and nowhere on
/// `AnyElement`, so a panel that arrived through base's `From` impl has already
/// lost them by the time anything here could read them.
fn panel(parts: NodeParts) -> ResizablePanel {
    let NodeParts {
        refinement,
        behavior,
        states,
        children,
    } = parts;
    warn_ignored_key(&behavior, "resizable_panel");
    warn_unhonoured_a11y(&behavior, "resizable_panel", &[]);
    // Like `Switch`, the panel is not an interactive element — it is a sized
    // box around one — so a state style on it has nowhere to land.
    if !states.is_empty() {
        tracing::warn!(
            "state styles on a resizable_panel are ignored; style an element inside it instead"
        );
    }

    let mut panel = resizable_panel().visible(behavior.visible.unwrap_or(true));
    if let Some(size) = behavior.panel_size {
        panel = panel.size(size);
    }
    if let Some(range) = behavior.size_range.clone() {
        panel = panel.size_range(range);
    }

    // Not `finish`: the group needs the panel, not an `AnyElement`, which is
    // the whole reason this component reads descriptions.
    panel.style().refine(&refinement);
    panel.extend(children);
    panel
}

/// A `resizable_panel()` that never reached a group.
///
/// Base's panel reads the group's `ResizableState` in its own `render` and
/// panics outright when there is none, so this cannot be built as a panel. The
/// content is kept — a script that lost its resizing should still see its
/// interface — and the mistake is reported rather than crashing the process.
pub(in crate::materialize) fn orphan_panel(
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_unhonoured_a11y(&behavior, "resizable_panel", &[]);
    tracing::error!(
        "a resizable_panel() outside an h_resizable()/v_resizable() cannot resize: its size and \
         its drag handle both belong to the group. Rendered as a plain element instead"
    );
    let element = with_hover(div(), &states);
    finish(element, refinement, children)
}
