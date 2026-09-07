//! `v_virtual_list` / `h_virtual_list` — the one component that enters the VM
//! on a frame's budget.
//!
//! Every other binding in this directory turns a finished description into
//! elements and never calls script again. A virtualized list cannot: which rows
//! exist depends on where the list has been scrolled, and GPUI only knows that
//! during layout. So base's `render_items` closure runs from inside layout and
//! prepaint — once to measure a representative item, once to place the visible
//! window — and each of those calls reaches the script.
//!
//! The exception is stated in full in [`crate::materialize`]'s module comment,
//! along with what confines it. This file is about the two decisions the
//! confinement forced.
//!
//! # A row cannot register a handler
//!
//! The natural script for a list is `row.on_click(...)`, one handler per row.
//! It cannot work here, and the reason is worth writing down because it is not
//! obvious from either side.
//!
//! Callbacks belong to the snapshot that registered them: a generation is
//! opened when a script render begins, committed when it succeeds, and retired
//! when the snapshot drops. That lifetime is exactly right for a description
//! that stands until script state changes — and exactly wrong for a row, which
//! is rebuilt on every frame the list is on screen. Twenty visible rows over a
//! thousand frames of scrolling would leave twenty thousand persistent
//! JavaScript functions in the arena, unreachable and unreleased, for as long
//! as the view stood.
//!
//! So registering one is refused where it is written, and the list carries a
//! single `on_item_click((key, cx) => …)` instead — registered from the view's
//! `render()`, in the ordinary way, with the ordinary lifetime. The key comes
//! from the list's required `get_key(index)` and is captured with the hit box.
//! An event delivered from an older frame therefore still identifies the item
//! that owned that box rather than whichever item later moved into its index.
//!
//! What it does not cover is a row with *several* independently clickable
//! parts. Doing that properly needs a callback generation scoped to a batch of
//! items — opened when the batch is described, retired when the next batch
//! replaces it — which is a lifetime `CallbackArena` does not have today. The
//! shape is recorded in the implementation plan as P9; nothing here forecloses
//! it, and the refusal above becomes an ordinary registration against the
//! batch's generation when it exists.
//!
//! # The geometry is base's, the pairing is by name
//!
//! Base's `VirtualList` paints no scrollbar of its own — its showcase puts one
//! beside it — so the bar is an explicit [`Scrollbar`] the script places, paired
//! with the list by the id both were given. See
//! [`crate::materialize::components::scrollbar`] for how that pairing works;
//! all this file does is make sure the position the list scrolls is the one
//! filed under its name.
//!
//! [`Scrollbar`]: gpui_base::Scrollbar

use std::{
    cell::Cell,
    ops::Range,
    rc::{Rc, Weak},
};

use gpui::{
    AnyElement, App, Axis, Bounds, Context, ElementId, Empty, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, Pixels, Refineable as _, Render, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled as _, Window, canvas, div,
};
use gpui_base::{VirtualListScrollHandle, h_virtual_list, v_virtual_list};

use crate::{
    engine::ShellRuntime,
    materialize::{
        Behavior, Children, StateStyles,
        components::scrollbar::{SharedScroll, shared_scroll_position},
        materialize_subtree, warn_ignored_key, warn_unhonoured_a11y,
    },
    spec::{CallbackId, VirtualListSpec},
};

/// The entity base's constructor takes.
///
/// `v_virtual_list(view, …)` calls `view.update(cx, …)` for one reason: to hand
/// the item renderer a `Context`. The shell has nothing to put there — its
/// renderer reads no Rust state at all, only a runtime and a callback id, and
/// it captures both — so this is a placeholder, kept in window element state
/// under the list's own name so that it is the same entity every frame.
///
/// Deliberately *not* the `ScriptView`. That would mean taking a mutable borrow
/// of the view from inside GPUI's layout pass, on behalf of a closure that
/// never reads it — a borrow whose safety the shell would then have to keep
/// arguing for every time a list ended up nested inside something.
/// The two things a script can put on any lazy list that it cannot honour.
///
/// Shared with [`super::list`], which has the same two: what a list draws is
/// whatever its item renderer returns, and the list itself has no hit state.
pub(in crate::materialize) fn warn_lazy_list_misuse(
    name: &str,
    children: &Children,
    states: &StateStyles,
) {
    if !children.is_empty() {
        tracing::warn!(
            "children are dropped on a {name}: its contents are whatever the item renderer \
             returns"
        );
    }
    if states.hover.is_some() || states.active.is_some() || states.focus.is_some() {
        tracing::warn!(
            "state styles are ignored on a {name}: it has no interactive state of its own. \
             Put them on the rows the item renderer returns, or on an element around the list"
        );
    }
}

struct VirtualItems;

impl Render for VirtualItems {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[allow(clippy::too_many_arguments)]
pub(in crate::materialize) fn virtual_list(
    runtime: &Rc<ShellRuntime>,
    spec: &VirtualListSpec,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let name = match spec.axis() {
        Axis::Vertical => "v_virtual_list",
        Axis::Horizontal => "h_virtual_list",
    };
    warn_ignored_key(&behavior, name);
    // `VirtualList` is an `Element` with a `Styled` impl and nothing else: no
    // `Interactivity` of its own is reachable, so there is no focus handle,
    // role or hit state for any of this to land on.
    warn_unhonoured_a11y(&behavior, name, &[]);
    warn_lazy_list_misuse(name, &children, &states);

    let identity = ElementId::Name(SharedString::from(spec.id().to_owned()));
    let scroll = scroll_position(runtime, &behavior, &identity, window, cx);
    let host = window.use_keyed_state((identity.clone(), "virtual-list"), cx, |_, _| VirtualItems);

    // Weak, so an element outliving its runtime renders nothing rather than
    // keeping the VM alive for the length of a frame.
    let weak = Rc::downgrade(runtime);
    let get_key = spec.get_key();
    let render_items = spec.render_items();
    let handlers = ItemHandlers {
        click: behavior.on_item_click,
        secondary_click: behavior.on_item_secondary_click,
    };
    let describe = move |_: &mut VirtualItems,
                         range: Range<usize>,
                         window: &mut Window,
                         cx: &mut Context<VirtualItems>| {
        render_range(&weak, get_key, render_items, handlers, range, window, cx)
    };

    let sizes = spec.sizes().clone();
    let mut list = match spec.axis() {
        Axis::Vertical => v_virtual_list(host, identity, sizes, describe),
        Axis::Horizontal => h_virtual_list(host, identity, sizes, describe),
    }
    .track_scroll(&scroll);

    if let Some(index) = behavior.item_to_measure_index {
        list = list.with_item_to_measure_index(index);
    }

    list.style().refine(&refinement);
    list.into_any_element()
}

/// The scroll position this list drives, which is also the one a `Scrollbar`
/// naming the same id will find.
///
/// The wrapper is retained rather than rebuilt each frame because it carries
/// more than an offset: the item count, the content size measured during the
/// last prepaint, and any pending `scroll_to_item`. A fresh one every frame
/// would drop all three, and `scroll_to_item` would never fire — the request is
/// made between two frames and consumed by the next.
///
/// The write-through at the end is the pairing. A `Scrollbar` looks its
/// position up by name, and that lookup would otherwise create an unrelated
/// one: a bar laid out, painted, hit-tested and completely inert. A bar
/// materialized *before* the list in the very first frame reads the value it
/// created rather than this one, and picks this one up on the next frame —
/// which is also the first frame in which anything has a size to scroll.
fn scroll_position(
    runtime: &Rc<ShellRuntime>,
    behavior: &Behavior,
    identity: &ElementId,
    window: &mut Window,
    cx: &mut App,
) -> VirtualListScrollHandle {
    let owned = behavior.virtual_scroll.and_then(|handle| {
        let scroll = runtime.entities().virtual_scroll(handle);
        if scroll.is_none() {
            tracing::error!("virtual list scroll handle {handle} is no longer live");
        }
        scroll
    });

    let scroll = owned.unwrap_or_else(|| {
        window
            .use_keyed_state((identity.clone(), "virtual-list-scroll"), cx, |_, _| {
                VirtualListScrollHandle::new()
            })
            .read(cx)
            .clone()
    });

    shared_scroll_position(identity, window, cx).update(cx, |shared, _| {
        *shared = SharedScroll::Handle(scroll.base_handle().clone())
    });

    scroll
}

/// Describes one window of items and turns it into elements.
///
/// Both halves are timed together because from a frame's point of view they are
/// one cost, and both are the frame's: see [`crate::metrics`].
pub(in crate::materialize) fn render_range(
    runtime: &Weak<ShellRuntime>,
    get_key: CallbackId,
    render_items: CallbackId,
    handlers: ItemHandlers,
    range: Range<usize>,
    window: &mut Window,
    cx: &mut App,
) -> Vec<AnyElement> {
    let Some(runtime) = runtime.upgrade() else {
        return Vec::new();
    };

    runtime.metrics().time_frame_script(|| {
        let Some(described) =
            runtime.render_virtual_items(render_items, get_key, range.clone(), window, cx)
        else {
            return Vec::new();
        };

        // Base zips the returned elements against the range, so a short answer
        // silently leaves the tail of the window blank and a long one silently
        // drops the surplus. Neither reads as "the renderer returned the wrong
        // number of rows", which is what it is.
        if described.roots().len() != range.len() {
            tracing::warn!(
                "a virtual list's item renderer returned {} elements for items {}..{}; it is \
                 called once per visible range and must return one element per item in it",
                described.roots().len(),
                range.start,
                range.end
            );
        }

        described
            .roots()
            .iter()
            .zip(described.keys())
            .map(|(root, key)| {
                let item = materialize_subtree(&runtime, described.arena(), *root, window, cx);
                if handlers.is_empty() {
                    item
                } else {
                    clickable(item, key.clone(), handlers, &runtime)
                }
            })
            .collect()
    })
}

/// The list-level handlers a row reports through. Both are optional, and a
/// list that asked for neither gets its rows exactly as the renderer built
/// them.
#[derive(Clone, Copy, Default)]
pub(in crate::materialize) struct ItemHandlers {
    /// `on_item_click`: the key, on a click.
    pub(in crate::materialize) click: Option<CallbackId>,
    /// `on_item_secondary_click`: the key and the press, on a right press.
    pub(in crate::materialize) secondary_click: Option<CallbackId>,
}

impl ItemHandlers {
    fn is_empty(self) -> bool {
        self.click.is_none() && self.secondary_click.is_none()
    }
}

/// Wraps one row in the hit box that reports its stable domain key.
///
/// Only when the script asked for it: a list with no row handler gets its rows
/// exactly as the renderer built them.
///
/// The key also becomes the row's element identity; index remains only the
/// current coordinate used to ask the renderer for a window.
///
/// The box is `size_full` so that the whole row is clickable rather than only
/// the text in it. Under the definite space base gives each item during
/// prepaint that is the row's own box; under the min-content space it uses to
/// *measure* an item, a percentage has no parent size to resolve against and
/// falls back to the content, so the wrapper cannot inflate the size the list
/// infers for its cross axis.
///
/// A secondary press is reported with the row's own box, the way a press on
/// any other element is reported with that element's, so `local_position`
/// means the same thing in both handlers.
fn clickable(
    item: AnyElement,
    key: String,
    handlers: ItemHandlers,
    runtime: &Rc<ShellRuntime>,
) -> AnyElement {
    let mut row = div()
        .id(ElementId::Name(SharedString::from(format!(
            "gpui-shell-virtual-item:{key}"
        ))))
        .size_full();
    if let Some(callback) = handlers.secondary_click {
        let runtime = Rc::downgrade(runtime);
        let key = key.clone();
        let bounds = Rc::new(Cell::new(None::<Bounds<Pixels>>));
        let writer = Rc::clone(&bounds);
        row = row
            // Captured at paint rather than prepaint, and attached before the
            // row's content. Paint runs once, for the box that is actually on
            // screen, where a prepaint capture also sees the passes that only
            // measure. And an absolute child with no inset sits at its static
            // position -- where it would have been in flow -- so placed after
            // the content it would land one row down, and report a press ten
            // pixels into a row as thirty pixels above it.
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, _, _| writer.set(Some(bounds)),
                )
                .absolute()
                .size_full(),
            )
            .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                if let Some(runtime) = runtime.upgrade() {
                    runtime.dispatch_item_mouse_button(
                        callback,
                        &key,
                        event.button,
                        event.position,
                        event.click_count,
                        event.modifiers,
                        bounds.get(),
                        window,
                        cx,
                    );
                }
            });
    }
    row = row.child(item);
    if let Some(callback) = handlers.click {
        let runtime = Rc::downgrade(runtime);
        let key = key.clone();
        row = row.on_click(move |_, window, cx| {
            if let Some(runtime) = runtime.upgrade() {
                runtime.dispatch_item_key(callback, &key, window, cx);
            }
        });
    }
    row.into_any_element()
}
