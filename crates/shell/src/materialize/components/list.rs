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
//!
//! # A `list` keeps the reader's place by key
//!
//! GPUI's `list` knows its items by index only. When the count changes, the
//! items it measured may now be other items, so the shell splices the whole
//! collection — and GPUI's splice moves a scroll position inside the spliced
//! range to its start. Left at that, every message appended to a transcript,
//! and every page of older ones put above it, would throw the reader to the
//! first item.
//!
//! The script's `get_key(index)` is what says which item is which, so the
//! position is carried by key: after each layout the list remembers the key of
//! the item at the top of the viewport, and when the count has changed it
//! looks for that key near the item's old index and scrolls back to it, the
//! same distance into it. [`KeyedList`] does both around GPUI's own layout,
//! because asking the script for a key is something only layout may do here.
//!
//! A list following its tail (`with_follow_mode("tail")`, GPUI's
//! `FollowMode::Tail`) is the exception: it is kept at the end, and the key
//! is not looked for.
//!
//! # A `list` reports what it shows after the frame
//!
//! A script that loads older items when the reader reaches the top has to be
//! told where the reader is, and its item renderer cannot be the one to tell
//! it: that runs inside layout, where state may not change. GPUI's own
//! `ListState::set_scroll_handler` is no answer either. It is called for a
//! wheel or trackpad scroll only — not for a scrollbar drag, a position set
//! from code, a list following its tail or one whose items changed — and it
//! reports the range from before the scroll it was called for.
//!
//! So `on_visible_range_change` is driven from the result rather than the
//! cause: after each layout the list works out which items its viewport
//! shows, and when that or the item count differs from what it last reported,
//! it reports again once the frame is done, as an event.

use std::{
    ops::Range,
    rc::{Rc, Weak},
};

use gpui::{
    AnyElement, App, DefiniteLength, Edges, Element, ElementId, Empty, Entity, FollowMode,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, ListAlignment, ListOffset,
    ListState, Pixels, Refineable as _, SharedString, StyleRefinement, Styled as _,
    UniformListScrollHandle, Window, list as gpui_list, px, uniform_list,
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
    spec::{CallbackId, ListKind, ListSpec},
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

/// The retained side of a `list`: GPUI's measurements, the count they were
/// taken for, the item the viewport started at when it was last laid out,
/// whether GPUI was told to follow the tail, and the visible range and count
/// last reported to the script.
struct MeasuredItems {
    state: ListState,
    item_count: usize,
    top: Option<TopItem>,
    follows_tail: bool,
    reported: Option<(Range<usize>, usize)>,
}

/// The item at the top of a `list`'s viewport after a layout, by key.
struct TopItem {
    index: usize,
    key: String,
    /// The `get_key` the key came from. Every script render registers a new
    /// one, so a different one means the collection may have changed since,
    /// and the key has to be asked for again.
    get_key: CallbackId,
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
            if behavior.follow_mode.is_some() {
                tracing::warn!(
                    "with_follow_mode is ignored on a uniform_list: GPUI's uniform list has no \
                     follow mode. It is list that follows its tail"
                );
            }
            if behavior.on_visible_range_change.is_some() {
                tracing::warn!(
                    "on_visible_range_change is ignored on a uniform_list: it is list that \
                     reports its visible range after the frame"
                );
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
                    top: None,
                    follows_tail: false,
                    reported: None,
                }
            });
            let follows_tail = behavior.follow_mode == Some(FollowMode::Tail);
            let state = retained.update(cx, |retained, _| {
                // Only on a change: setting `Tail` scrolls to the end, so
                // setting it on every frame would undo the scroll up that
                // pauses following.
                if retained.follows_tail != follows_tail {
                    retained.state.set_follow_mode(if follows_tail {
                        FollowMode::Tail
                    } else {
                        FollowMode::Normal
                    });
                    retained.follows_tail = follows_tail;
                }
                retained.state.clone()
            });
            shared_scroll_position(&identity.clone(), window, cx)
                .update(cx, |shared, _| *shared = SharedScroll::List(state.clone()));

            let runtime = weak.clone();
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
            let mut padding = Edges::<DefiniteLength>::default();
            padding.refine(&refinement.padding);
            KeyedList {
                list: list.into_any_element(),
                padding,
                retained,
                item_count,
                get_key,
                on_visible_range_change: behavior.on_visible_range_change,
                runtime,
            }
            .into_any_element()
        }
    }
}

/// GPUI's `List`, with what the shell does on either side of laying it out.
///
/// Before: a changed item count is spliced into GPUI's measurements, and the
/// reading position is carried to wherever the item at the top went. After:
/// the item now at the top is remembered by key, for the next change, and a
/// changed visible range is reported. The first two ask the script for keys,
/// and [`crate::materialize`] never enters the script while an element is
/// built, so they run here, inside layout, where the item renderer already
/// does. The report is deferred to after the frame.
///
/// It has no layout node of its own: the list's is returned as its own, so the
/// refinement the script gave the list is what sizes it.
struct KeyedList {
    list: AnyElement,
    padding: Edges<DefiniteLength>,
    retained: Entity<MeasuredItems>,
    item_count: usize,
    get_key: CallbackId,
    on_visible_range_change: Option<CallbackId>,
    runtime: Weak<ShellRuntime>,
}

impl KeyedList {
    /// Splices a changed item count into GPUI's measurements and keeps the
    /// reader on the item they were reading.
    ///
    /// Every measurement is dropped, as before: the count says how many items
    /// there are, not which ones changed, so none of the old sizes can be
    /// trusted for the new items. What is kept is the position, which GPUI's
    /// splice would otherwise move to the first item.
    fn splice_item_count(&mut self, window: &mut Window, cx: &mut App) {
        let new_count = self.item_count;
        let retained = self.retained.read(cx);
        let old_count = retained.item_count;
        if old_count == new_count {
            return;
        }
        let state = retained.state.clone();
        let remembered = retained
            .top
            .as_ref()
            .map(|top| (top.index, top.key.clone()));
        let position = state.logical_scroll_top();

        // A position past the last item is the end of the list, and a splice
        // of everything leaves it at the new end. A list following its tail
        // is put at the end by its own layout; scrolling it to an item here
        // would stop it following.
        let target = (position.item_ix < old_count && !state.is_following_tail()).then(|| {
            let moved_to = remembered.and_then(|(index, key)| {
                let found =
                    self.find_key(&key, search_order(index, old_count, new_count), window, cx)?;
                // The remembered item was at the top when the list was last
                // laid out. A scroll since then has put the position some
                // items away from it, and it stays that many items away.
                Some(found as isize + position.item_ix as isize - index as isize)
            });
            ListOffset {
                // The item is gone, or was never seen: stay at the index.
                item_ix: moved_to.map_or(position.item_ix, |index| {
                    index.clamp(0, new_count as isize) as usize
                }),
                offset_in_item: position.offset_in_item,
            }
        });

        state.splice(0..old_count, new_count);
        if let Some(target) = target {
            state.scroll_to(target);
        }
        self.retained.update(cx, |retained, _| {
            retained.item_count = new_count;
            retained.top = None;
        });
    }

    /// The first of `candidates` whose key, as `get_key` answers now, is `key`.
    fn find_key(
        &self,
        key: &str,
        mut candidates: impl Iterator<Item = usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<usize> {
        self.runtime
            .upgrade()?
            .item_keys(self.get_key, window, cx, |key_of| {
                candidates.find(|candidate| key_of(*candidate).as_deref() == Some(key))
            })
            .flatten()
    }

    /// Remembers the key of the item at the top of the viewport, as this
    /// layout left it.
    ///
    /// The key is asked for only when that item or the render has changed:
    /// nothing on a frame that only repaints, one call on a frame that
    /// scrolled to another item, on top of the rows the list draws.
    fn remember_top(&mut self, window: &mut Window, cx: &mut App) {
        let retained = self.retained.read(cx);
        let index = retained.state.logical_scroll_top().item_ix;
        if index >= self.item_count {
            self.retained.update(cx, |retained, _| retained.top = None);
            return;
        }
        if retained
            .top
            .as_ref()
            .is_some_and(|top| top.index == index && top.get_key == self.get_key)
        {
            return;
        }
        let key = self.runtime.upgrade().and_then(|runtime| {
            runtime
                .item_keys(self.get_key, window, cx, |key_of| key_of(index))
                .flatten()
        });
        let get_key = self.get_key;
        self.retained.update(cx, |retained, _| {
            retained.top = key.map(|key| TopItem {
                index,
                key,
                get_key,
            });
        });
    }

    /// Reports the visible range and the item count, once this frame is done,
    /// when either differs from the last report.
    ///
    /// Deferred to the end of the effect cycle, because the handler is meant
    /// to change state — load the page above, mark items read — and nothing
    /// inside layout may. The comparison is with the last report, not with
    /// what moved the list, so every way the range can change is reported —
    /// a wheel or trackpad scroll, a scrollbar drag, a new item count, a
    /// resize — and a frame that changed neither is not.
    fn report_visible_range(&mut self, window: &mut Window, cx: &mut App) {
        let retained = self.retained.read(cx);
        let Some(callback) = self.on_visible_range_change else {
            // A handler added later reports the list as it is then.
            if retained.reported.is_some() {
                self.retained
                    .update(cx, |retained, _| retained.reported = None);
            }
            return;
        };
        let padding = self.padding.to_pixels(
            retained.state.viewport_bounds().size.into(),
            window.rem_size(),
        );
        let shown = (visible_range(&retained.state, padding), self.item_count);
        if retained.reported.as_ref() == Some(&shown) {
            return;
        }
        self.retained
            .update(cx, |retained, _| retained.reported = Some(shown.clone()));
        let runtime = self.runtime.clone();
        let (range, count) = shown;
        window.defer(cx, move |window, cx| {
            if let Some(runtime) = runtime.upgrade() {
                runtime.dispatch_visible_range(callback, range, count, window, cx);
            }
        });
    }
}

/// The items a list's viewport shows after its last layout: from the item at
/// the top to the last one whose top edge is above the viewport's bottom.
fn visible_range(state: &ListState, padding: Edges<Pixels>) -> Range<usize> {
    let count = state.item_count();
    let start = state.logical_scroll_top().item_ix.min(count);
    let viewport = state.viewport_bounds();
    // GPUI does not prepaint any rows when padding fills the viewport.
    if viewport.size.height <= padding.top + padding.bottom {
        return start..start;
    }
    let bottom = viewport.bottom();
    let mut end = start;
    while end < count
        && state
            .bounds_for_item(end)
            // GPUI's item bounds omit the padding added during prepaint.
            .is_some_and(|bounds| bounds.top() + padding.top < bottom)
    {
        end += 1;
    }
    start..end
}

/// The order in which a remembered item is looked for after the count went
/// from `old_count` to `new_count`: where it was, then where a change wholly
/// above it would have moved it, then outward from where it was, one step
/// each way. Every index below `new_count` comes up exactly once.
fn search_order(index: usize, old_count: usize, new_count: usize) -> impl Iterator<Item = usize> {
    let shifted = (index + new_count)
        .checked_sub(old_count)
        .filter(|shifted| *shifted != index);
    let outward = (1..=index.max(new_count))
        .flat_map(move |distance| [index.checked_add(distance), index.checked_sub(distance)])
        .flatten()
        .filter(move |candidate| Some(*candidate) != shifted);
    std::iter::once(index)
        .chain(shifted)
        .chain(outward)
        .filter(move |candidate| *candidate < new_count)
}

impl IntoElement for KeyedList {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for KeyedList {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        self.splice_item_count(window, cx);
        (self.list.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        _request_layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.list.prepaint(window, cx);
        self.remember_top(window, cx);
        self.report_visible_range(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        _request_layout: &mut (),
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.list.paint(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::search_order;

    #[test]
    fn the_search_tries_the_likely_places_first_and_every_index_once() {
        // Ten items put above the item at 4 of 20 move it to 14.
        let order: Vec<usize> = search_order(4, 20, 30).collect();
        assert_eq!(order[..2], [4, 14]);
        let mut visited = order.clone();
        visited.sort_unstable();
        assert_eq!(visited, (0..30).collect::<Vec<_>>());

        // A collection that shrank below the old index is still covered.
        let mut visited: Vec<usize> = search_order(50, 60, 10).collect();
        visited.sort_unstable();
        assert_eq!(visited, (0..10).collect::<Vec<_>>());
    }
}
