//! `list` and `uniform_list`: GPUI's own lazy lists, driven from script.
//!
//! A [`VirtualList`](crate::spec::Component::VirtualList) is base's: the script
//! states every item's extent and base places the items by the table. These
//! two are GPUI's, and the difference is who measures. `uniform_list` measures
//! one item and places every row by it; `list` measures each item it draws and
//! keeps the sizes, so rows of unequal, unstated height still scroll as one
//! collection. Both draw only what is on screen, and both reach the script the
//! way the virtual list does — one renderer, called with the visible range from
//! inside layout — so the confinement recorded in [`crate::materialize`] holds
//! for them unchanged.
//!
//! Neither takes a `VirtualListScrollHandle`: the position is GPUI's own state,
//! kept under the id the list was built with. That id is also the name a
//! `Scrollbar` pairs with, through the same shared slot a scroll area uses.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Empty, IntoElement, ListAlignment, ListState, Refineable as _,
    SharedString, StyleRefinement, Styled as _, UniformListScrollHandle, Window, list as gpui_list,
    px, uniform_list,
};

use crate::{
    engine::ShellRuntime,
    materialize::{
        Behavior, Children, StateStyles,
        components::{
            scrollbar::{SharedScroll, shared_scroll_position},
            virtual_list::{ItemHandlers, render_range, warn_lazy_list_misuse},
        },
        warn_ignored_key, warn_unhonoured_a11y,
    },
    spec::{ListKind, ListSpec},
};

/// How far past the viewport a `list` draws and measures, in pixels.
///
/// GPUI's list can only scroll into what it has measured, and it measures by
/// drawing: with nothing drawn past the viewport, a list whose last drawn row
/// ends exactly at the bottom edge has nowhere to scroll to and never asks
/// for more. A band below the fold keeps a wheel notch or a bar drag inside
/// measured ground, and each frame it moves measures the next band. Kept to
/// a few rows rather than the screenful GPUI's own callers use: every item in
/// the band is a script render per frame, and the whole point of the list is
/// to leave an item a screen away undrawn.
const LIST_OVERDRAW: gpui::Pixels = px(160.);

/// The retained side of a `list`: GPUI's measurements, and the count they were
/// taken for, so a collection that grew or shrank is spliced rather than
/// re-measured from nothing.
struct MeasuredItems {
    state: ListState,
    item_count: usize,
}

#[allow(clippy::too_many_arguments)]
pub(in crate::materialize) fn list(
    runtime: &Rc<ShellRuntime>,
    spec: &ListSpec,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let name = match spec.kind() {
        ListKind::Measured => "list",
        ListKind::Uniform => "uniform_list",
    };
    warn_ignored_key(&behavior, name);
    warn_unhonoured_a11y(&behavior, name, &[]);
    warn_lazy_list_misuse(name, &children, &states);
    if behavior.virtual_scroll.is_some() {
        tracing::warn!(
            "track_scroll is ignored on a {name}: its scroll position is GPUI's own, filed \
             under the id it was built with, which is where a Scrollbar of that name finds it"
        );
    }

    let identity = ElementId::Name(SharedString::from(spec.id().to_owned()));
    let weak = Rc::downgrade(runtime);
    let get_key = spec.get_key();
    let render_items = spec.render_items();
    let handlers = ItemHandlers {
        click: behavior.on_item_click,
        secondary_click: behavior.on_item_secondary_click,
    };
    let item_count = spec.item_count();

    match spec.kind() {
        ListKind::Uniform => {
            let scroll = window
                .use_keyed_state((identity.clone(), "uniform-list-scroll"), cx, |_, _| {
                    UniformListScrollHandle::new()
                })
                .read(cx)
                .clone();
            shared_scroll_position(&identity.clone(), window, cx).update(cx, |shared, _| {
                *shared = SharedScroll::Uniform(scroll.clone())
            });

            let mut list = uniform_list(identity.clone(), item_count, move |range, window, cx| {
                render_range(&weak, get_key, render_items, handlers, range, window, cx)
            })
            .track_scroll(&scroll)
            // Base's virtual list fills its box unless told otherwise; the
            // same default here, so a list dropped into a sized column shows
            // rows rather than a zero-height strip. The refinement may say
            // otherwise.
            .size_full();
            if let Some(index) = behavior.item_to_measure_index {
                list = list.with_width_from_item(Some(index));
            }
            list.style().refine(&refinement);
            list.into_any_element()
        }
        ListKind::Measured => {
            if behavior.item_to_measure_index.is_some() {
                tracing::warn!(
                    "with_item_to_measure_index is ignored on a list: it measures every item \
                     it draws, so there is no one item the rest are sized from. It is \
                     uniform_list that takes one"
                );
            }
            let retained = window.use_keyed_state((identity.clone(), "list-state"), cx, |_, _| {
                MeasuredItems {
                    state: ListState::new(item_count, ListAlignment::Top, LIST_OVERDRAW),
                    item_count,
                }
            });
            let state = retained.update(cx, |retained, _| {
                if retained.item_count != item_count {
                    // Every item is a new one as far as the measurements go;
                    // what survives is the scroll position, which `reset`
                    // would throw away.
                    retained.state.splice(0..retained.item_count, item_count);
                    retained.item_count = item_count;
                }
                retained.state.clone()
            });
            shared_scroll_position(&identity.clone(), window, cx)
                .update(cx, |shared, _| *shared = SharedScroll::List(state.clone()));

            let mut list = gpui_list(state, move |index, window, cx| {
                render_range(
                    &weak,
                    get_key,
                    render_items,
                    handlers,
                    index..index + 1,
                    window,
                    cx,
                )
                .into_iter()
                .next()
                .unwrap_or_else(|| Empty.into_any_element())
            })
            .size_full();
            list.style().refine(&refinement);
            list.into_any_element()
        }
    }
}
