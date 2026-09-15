//! Touch selection: the grab handles and the edit menu a long press leaves
//! behind, drawn over an [`crate::input::Input`] or over the window text
//! selection that [`crate::text::TextView`] takes part in.
//!
//! Base owns the gesture and the drag; see [`gpui_base::TouchSelectionSnapshot`].
//! This module draws what Base laid out: a handle at each end of the
//! selection, and a row of commands above it.

mod edit_menu;
mod handle;
mod window_overlay;

use std::rc::Rc;

use gpui::{AnyElement, App, Bounds, ElementId, IntoElement, Pixels, Point, TouchPhase, Window};
use gpui_base::{SelectionEdge, TouchSelectionSnapshot};

pub(crate) use edit_menu::{EditMenu, EditMenuItem};
pub(crate) use handle::{DragHandler, SelectionHandles, SnapshotSource, SurfaceHandler};
pub(crate) use window_overlay::WindowTouchSelectionOverlay;

use handle::KNOB_EXTENT;

/// Draws one touch selection: its handles and, when open, its edit menu.
///
/// The handles read the selection's geometry as they paint, so they follow
/// text that scrolls in the same frame. The menu is placed from the snapshot
/// read here; it steps aside while the text scrolls, so a frame's lag in its
/// anchor never shows.
pub(crate) struct TouchSelectionOverlay {
    id: ElementId,
    source: SnapshotSource,
    items: Vec<EditMenuItem>,
    on_drag: DragHandler,
    on_paint: Option<SurfaceHandler>,
}

impl TouchSelectionOverlay {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        source: impl Fn(&Window, &App) -> Option<TouchSelectionSnapshot> + 'static,
        on_drag: impl Fn(SelectionEdge, TouchPhase, Point<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            source: Rc::new(source),
            items: Vec::new(),
            on_drag: Rc::new(on_drag),
            on_paint: None,
        }
    }

    /// The commands the edit menu offers. With none, no menu is drawn.
    pub(crate) fn items(mut self, items: impl IntoIterator<Item = EditMenuItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Called with every surface's bounds as it paints, for an owner whose
    /// press handling must leave those surfaces alone.
    pub(crate) fn on_paint(
        mut self,
        on_paint: impl Fn(Bounds<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_paint = Some(Rc::new(on_paint));
        self
    }

    /// The elements to add to the owner: the handles, and the menu when it
    /// is open. Each floats in window coordinates.
    pub(crate) fn into_elements(self, window: &Window, cx: &App) -> Vec<AnyElement> {
        let Some(snapshot) = (self.source)(window, cx) else {
            return Vec::new();
        };
        let mut elements = Vec::with_capacity(2);
        let handles = SelectionHandles::new(self.source, self.on_drag);
        let handles = match self.on_paint.clone() {
            Some(on_paint) => handles.on_paint(on_paint),
            None => handles,
        };
        elements.push(handles.into_any_element());

        if let Some(mut anchor) = snapshot
            .bounds()
            .filter(|_| snapshot.is_menu_open() && !self.items.is_empty())
        {
            // Leave the knobs uncovered: the menu anchors to the selection
            // plus the room its handles take above and below.
            if !snapshot.is_empty() {
                anchor.origin.y -= KNOB_EXTENT;
                anchor.size.height += KNOB_EXTENT * 2.;
            }
            let menu = EditMenu::new(self.id, anchor).items(self.items);
            let menu = match self.on_paint {
                Some(on_paint) => menu.on_paint(on_paint),
                None => menu,
            };
            elements.push(menu.into_any_element());
        }
        elements
    }
}
