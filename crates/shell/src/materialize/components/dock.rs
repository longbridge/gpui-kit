//! `dock_area` — the one element whose contents the description does not
//! contain.
//!
//! Every other component here is the whole of what it draws. A dock area is
//! not, and cannot be: the layout is what the *user* changed. A drag, a resize,
//! a closed tab and a collapsed dock all happen without a script render, so a
//! dock rebuilt from a description would put every one of them back the way the
//! last render described it. The layout therefore lives in a retained entity —
//! `gpui_base::dock::DockArea` — and this node mounts it.
//!
//! Three things do cross from the description into that entity, and this file
//! is where all three happen.
//!
//! # The chrome handlers, once per frame
//!
//! Base draws no chrome at all, so every tab bar, dock frame and drag bar comes
//! back through a renderer — and the renderer is installed when the area is
//! *created*, while the handlers belong to whichever snapshot is currently
//! published. [`DockChromeSlots`] is the join: this writes the current
//! handlers as the description is replayed, and the skin reads them when base
//! asks it to draw, which is later in the same frame.
//!
//! Writing them every frame rather than only when they change is deliberate.
//! A callback id is only meaningful while the snapshot that registered it
//! lives, and materialization is the one place that always runs against the
//! live snapshot.
//!
//! # The dock's own content
//!
//! Base hands a dock's content to the chrome as a finished `AnyElement` and
//! keeps whatever the chrome hands back, so a chrome that wants both has to
//! place the content itself. An element cannot cross into script, so the script
//! writes `dock_content()` where the content belongs and this file resolves it:
//! the engine installs the real element in a slot for the length of one chrome
//! callback, and the placeholder takes it.
//!
//! Taking, not cloning — an `AnyElement` is a value that is consumed when used.
//! A description with two `dock_content()`s draws the content once and says so.
//! The slot itself lives in [`crate::dock`], because the engine installs it and
//! this file only reads it back.
//!
//! # The commands a chrome element carries
//!
//! A chrome handler may not register a callback — one created there would pile
//! up for as long as the dock stood — so its elements say what they do with a
//! [`DockCommand`], which names a container and what to ask it. [`with_commands`]
//! is where those become GPUI listeners, resolved against the contexts the last
//! drawn frame recorded.

use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext as _, Div, DragMoveEvent, Empty, InteractiveElement as _,
    IntoElement, ParentElement as _, Refineable as _, Stateful, StatefulInteractiveElement as _,
    Styled as _, Window, div,
};
use gpui_base::dock::{DragPanel, PanelId};

use crate::{
    dock::{DockChromeHooks, DockCommand, DockContexts, ResizingDock},
    engine::ShellRuntime,
    entities::EntityHandle,
    materialize::{Behavior, Children, StateStyles, warn_ignored_key, warn_unhonoured_a11y},
};

/// The area itself: a retained entity, mounted as an ordinary child.
#[allow(clippy::too_many_arguments)]
pub(in crate::materialize) fn dock_area(
    runtime: &Rc<ShellRuntime>,
    handle: EntityHandle,
    hooks: DockChromeHooks,
    refinement: gpui::StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
    _window: &mut Window,
    _cx: &mut App,
) -> AnyElement {
    warn_ignored_key(&behavior, "dock_area");
    // The area is a container base tracks its own focus and bounds on; the
    // element here only positions it.
    warn_unhonoured_a11y(&behavior, "dock_area", &[]);
    if !children.is_empty() {
        tracing::warn!(
            "children are dropped on a dock_area: what it draws is the panels in its layout, \
             which are added with add_panel(...) rather than described"
        );
    }
    if states.hover.is_some() || states.active.is_some() || states.focus.is_some() {
        tracing::warn!(
            "state styles are ignored on a dock_area: it has no interactive state of its own. \
             Put them on the chrome you draw for it, or on an element around the area"
        );
    }

    // The store borrow ends before anything else runs: mounting the area and
    // drawing it both reach back into the runtime.
    let (area, contexts, slots) = {
        let entities = runtime.entities();
        (
            entities.dock(handle),
            entities.dock_contexts(handle),
            entities.dock_slots(handle),
        )
    };
    let Some(area) = area else {
        tracing::error!("dock area handle {handle} is no longer live");
        return div().into_any_element();
    };

    // The frame about to be drawn records its own contexts, so the ones the
    // last frame left are dropped here — this runs before base walks the area.
    if let Some(contexts) = contexts {
        contexts.clear();
    }
    if let Some(slots) = slots {
        slots.set(hooks);
    }

    // `size_full` before the script's own refinement, so it is a default rather
    // than an override: an area with no size draws nothing at all, which is a
    // failure with no visible cause on screen.
    let mut frame = div().size_full();
    frame.style().refine(&refinement);
    frame.child(area).into_any_element()
}

/// Where the dock's own content goes inside the chrome drawn around it.
pub(in crate::materialize) fn dock_content(
    refinement: gpui::StyleRefinement,
    behavior: Behavior,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "dock_content");
    if !children.is_empty() {
        tracing::warn!(
            "children are dropped on a dock_content(): it stands in for the dock's own content, \
             which base supplies"
        );
    }

    let Some(content) = crate::dock::take_dock_content() else {
        tracing::warn!(
            "dock_content() was used outside a dock's chrome handler, or twice inside one; the \
             dock's content is a single element and can only be placed once"
        );
        return Empty.into_any_element();
    };

    let mut frame = div();
    frame.style().refine(&refinement);
    frame.child(content).into_any_element()
}

/// Wires the dock commands a chrome element carries.
///
/// Only a `div`, `h_flex` or `v_flex` gets them, which is the same set that
/// gets the generic pointer and keyboard handlers: a command needs
/// `on_click`, `on_drag` and `on_drop`, and those are `StatefulInteractiveElement`
/// methods that a `Button` or a `Checkbox` — which build their own interior —
/// do not expose. A command on anything else is reported rather than dropped.
pub(in crate::materialize) fn with_commands(
    element: Stateful<Div>,
    behavior: &Behavior,
    runtime: &Rc<ShellRuntime>,
    cx: &mut App,
) -> Stateful<Div> {
    let mut element = element;
    for action in behavior.dock_commands.iter() {
        let contexts = { runtime.entities().dock_contexts(action.dock()) };
        let Some(contexts) = contexts else {
            tracing::error!(
                "a dock command names dock area {}, which is no longer live",
                action.dock()
            );
            continue;
        };
        element = wire(element, contexts, action.command(), cx);
    }
    element
}

fn wire(
    element: Stateful<Div>,
    contexts: Rc<DockContexts>,
    command: DockCommand,
    cx: &mut App,
) -> Stateful<Div> {
    match command {
        DockCommand::SelectTab { node, index } => element.on_click(move |_, window, cx| {
            if let Some(group) = contexts.tab_group(node) {
                group.select_tab(index, window, cx);
            }
        }),
        DockCommand::ClosePanel { node, panel } => element.on_click(move |_, window, cx| {
            if let Some(group) = contexts.tab_group(node) {
                group.close(PanelId::from_u64(panel), window, cx);
            }
        }),
        DockCommand::ToggleGroupZoom { node } => element.on_click(move |_, window, cx| {
            if let Some(group) = contexts.tab_group(node) {
                group.toggle_zoom(window, cx);
            }
        }),
        // The payload is base's own, so a drop anywhere base listens — its
        // content frame, another group's tab bar — already knows what to do
        // with it. `drag_panel` answers `None` for a position that names no
        // panel, which is a tab bar drawn from a stale frame.
        DockCommand::DragTab { node, index } => {
            // The payload is built here rather than when the drag starts,
            // because `on_drag` takes the value: base wants a `DragPanel` and
            // the group is the only thing that can name one. The group is the
            // one this frame's chrome was drawn for, and the description this
            // element belongs to is that same frame's.
            let Some(drag) = contexts
                .tab_group(node)
                .and_then(|group| group.drag_panel(index, cx))
            else {
                tracing::warn!(
                    "drag_tab(group, {index}) names no panel in group {node}; the tab bar is \
                     being drawn from a layout that has already changed"
                );
                return element;
            };
            element.on_drag(drag, |drag, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            })
        }
        DockCommand::DropTab { node, index } => {
            element.on_drop(move |drag: &DragPanel, window, cx| {
                if let Some(group) = contexts.tab_group(node) {
                    group.drop_panel(drag.clone(), index, true, window, cx);
                }
            })
        }
        DockCommand::ToggleDock { placement } => element.on_click(move |_, window, cx| {
            if let Some(dock) = contexts.dock(placement) {
                dock.toggle(window, cx);
            }
        }),
        // Base clamps every position it is given against the area and the
        // opposite dock, so the drag is only a stream of pointer positions.
        DockCommand::ResizeDock { placement } => element
            .on_drag(ResizingDock(placement), |drag, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| *drag)
            })
            .on_drag_move(move |event: &DragMoveEvent<ResizingDock>, window, cx| {
                if event.drag(cx).0 != placement {
                    return;
                }
                if let Some(dock) = contexts.dock(placement) {
                    dock.resize_to(event.event.position, window, cx);
                }
            }),
    }
}
