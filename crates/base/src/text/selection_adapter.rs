use std::{cell::RefCell, ops::RangeInclusive, rc::Rc};

use crate::{
    TextSelectionContentKey, TextSelectionCoverage, TextSelectionEndpoint, TextSelectionEvent,
    TextSelectionHandle, TextSelectionRegistration, TextSelectionSnapshot,
};
use gpui::{App, Bounds, EntityId, Hitbox, Pixels, Point, WeakEntity, Window};

use super::TextViewState;

#[derive(Clone, Copy, Debug, PartialEq)]
struct CachedBlockEndpoint {
    endpoint: TextSelectionEndpoint,
    block_ix: Option<usize>,
}

#[derive(Default)]
struct VirtualBlockSelection {
    anchor: Option<CachedBlockEndpoint>,
    cursor: Option<CachedBlockEndpoint>,
    coverage: TextSelectionCoverage,
}

impl VirtualBlockSelection {
    fn update(&mut self, snapshot: Option<TextSelectionSnapshot>, entity_id: EntityId) {
        let Some(snapshot) = snapshot else {
            *self = Self::default();
            return;
        };

        self.coverage = snapshot.coverage();

        Self::update_endpoint(&mut self.anchor, snapshot.anchor(), entity_id);
        Self::update_endpoint(&mut self.cursor, snapshot.cursor(), entity_id);
    }

    fn update_endpoint(
        cached: &mut Option<CachedBlockEndpoint>,
        endpoint: TextSelectionEndpoint,
        entity_id: EntityId,
    ) {
        if cached.is_some_and(|cached| cached.endpoint == endpoint) {
            return;
        }
        let block_ix = (endpoint.entity_id() == Some(entity_id))
            .then(|| endpoint.content_key().map(|key| key.value() as usize))
            .flatten();
        *cached = Some(CachedBlockEndpoint { endpoint, block_ix });
    }

    fn block_range(&self, entity_id: EntityId, last: usize) -> Option<RangeInclusive<usize>> {
        let anchor = self.anchor?;
        let cursor = self.cursor?;
        match self.coverage {
            TextSelectionCoverage::Full => Some(0..=last),
            TextSelectionCoverage::FromStart => Some(0..=anchor.block_ix.or(cursor.block_ix)?),
            TextSelectionCoverage::ToEnd => Some(anchor.block_ix.or(cursor.block_ix)?..=last),
            TextSelectionCoverage::Bounded => {
                if anchor.endpoint.entity_id() != Some(entity_id)
                    || cursor.endpoint.entity_id() != Some(entity_id)
                {
                    return None;
                }
                let (anchor, cursor) = (anchor.block_ix?, cursor.block_ix?);
                Some(anchor.min(cursor)..=anchor.max(cursor))
            }
        }
    }
}

/// TextView's renderer-specific bridge to base-owned window selection.
#[derive(Clone)]
pub(super) struct TextViewSelectionAdapter {
    selection: TextSelectionHandle,
    text_bounds: Vec<Bounds<Pixels>>,
    layout_revision: Option<usize>,
}

impl TextViewSelectionAdapter {
    pub(super) fn new(view: WeakEntity<TextViewState>, cx: &mut App) -> Self {
        let selection = TextSelectionHandle::new("", cx);
        let selection_id = selection.entity_id();
        let virtual_blocks = Rc::new(RefCell::new(VirtualBlockSelection::default()));

        let view_for_events = view.clone();
        let blocks_for_events = virtual_blocks.clone();
        selection
            .subscribe(
                move |event, cx| match event {
                    TextSelectionEvent::SelectionChanged(snapshot) => {
                        let snapshot = *snapshot;
                        let _ = view_for_events.update(cx, |state, cx| {
                            blocks_for_events
                                .borrow_mut()
                                .update(snapshot, selection_id);
                            state.is_selecting =
                                snapshot.is_some_and(|snapshot| snapshot.is_selecting());
                            cx.notify();
                        });
                    }
                    TextSelectionEvent::AutoScroll(delta) => {
                        let delta = *delta;
                        let _ = view_for_events.update(cx, |state, cx| {
                            if state.scrollable {
                                state.set_auto_scroll(delta, cx);
                            } else if delta.is_none() {
                                state.stop_auto_scroll();
                            }
                        });
                    }
                    TextSelectionEvent::Cleared => {}
                },
                cx,
            )
            .detach();

        let view_for_clear = view.clone();
        let blocks_for_clear = virtual_blocks.clone();
        selection.clear_with(
            move |cx| {
                blocks_for_clear.replace(VirtualBlockSelection::default());
                let _ = view_for_clear.update(cx, |state, cx| {
                    state.reset_selection();
                    cx.notify();
                });
            },
            cx,
        );

        let view_for_copy = view.clone();
        let blocks_for_copy = virtual_blocks.clone();
        selection.copy_with(
            move |cx| {
                let Some(view) = view_for_copy.upgrade() else {
                    return String::new();
                };
                let state = view.read(cx);
                let last = state.parsed_content.document.blocks.len().saturating_sub(1);
                let blocks = blocks_for_copy.borrow().block_range(selection_id, last);
                state.selected_text_in(blocks)
            },
            cx,
        );

        let view_for_content_key = view.clone();
        selection.resolve_content_key_with(
            move |point, cx| {
                let view = view_for_content_key.upgrade()?;
                view.read(cx)
                    .block_ix_at(point.y)
                    .map(|block| TextSelectionContentKey::new(block as u64))
            },
            cx,
        );

        selection.focus_with(
            move |window, cx| {
                let Some(view) = view.upgrade() else {
                    return;
                };
                let focus_handle = view.read(cx).focus_handle.clone();
                view.update(cx, |state, _| state.is_selecting = true);
                focus_handle.focus(window, cx);
            },
            cx,
        );

        Self {
            selection,
            text_bounds: Vec::new(),
            layout_revision: None,
        }
    }

    pub(super) fn update_layout_revision(&mut self, revision: usize, is_selecting: bool) -> bool {
        let changed = self
            .layout_revision
            .is_some_and(|previous| previous != revision);
        if !changed || !is_selecting {
            self.layout_revision = Some(revision);
        }
        changed && !is_selecting
    }

    pub(super) fn begin_frame(&mut self) {
        self.text_bounds.clear();
    }

    pub(super) fn register_inline(&mut self, bounds: Vec<Bounds<Pixels>>) {
        self.text_bounds.extend(bounds);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn register(
        &self,
        hitbox: Hitbox,
        bounds: Bounds<Pixels>,
        scroll_offset: Point<Pixels>,
        document_order: u64,
        self_scroll: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.selection.register(
            TextSelectionRegistration::new(hitbox, bounds)
                .with_scroll_offset(scroll_offset)
                .with_document_order(document_order)
                .with_text_bounds(self.text_bounds.clone())
                .with_self_scroll(self_scroll),
            window,
            cx,
        );
    }

    pub(super) fn selection_points(&self, cx: &App) -> Option<(Point<Pixels>, Point<Pixels>)> {
        let points = self.selection.snapshot(cx)?.window_points()?;
        Some((points.anchor(), points.cursor()))
    }

    pub(super) fn set_local_selection(&self, active: bool, cx: &mut App) {
        self.selection.set_local_selection(active, cx);
    }

    pub(super) fn is_part_of_window_selection(&self, cx: &App) -> bool {
        let id = self.selection.entity_id();
        self.selection.snapshot(cx).is_some_and(|snapshot| {
            snapshot.anchor().entity_id() == Some(id) || snapshot.cursor().entity_id() == Some(id)
        })
    }

    pub(super) fn has_selection_snapshot(&self, cx: &App) -> bool {
        self.selection.snapshot(cx).is_some()
    }

    #[cfg(test)]
    pub(super) fn text_bounds(&self) -> Vec<Bounds<Pixels>> {
        self.text_bounds.clone()
    }
}
