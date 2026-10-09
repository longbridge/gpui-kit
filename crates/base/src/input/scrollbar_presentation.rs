//! Presentation hook for an editor's existing viewport, without a second scroll system.
use crate::{ScrollbarAxis, ScrollbarHandle, ScrollbarPlacement};
use gpui::{AnyElement, App, Bounds, Pixels, Point, ScrollHandle, Size, Window};
use std::rc::Rc;

/// Current-frame editor geometry and its retained scroll offset.
/// The editor applies caret scrolling before invoking the presentation callback.
/// Like other [`ScrollbarHandle`] implementations, the content size includes
/// the viewport's padding, so subtracting the track size gives the scroll range.
#[derive(Clone)]
pub struct EditorScrollbarContext {
    pub(crate) handle: ScrollHandle,
    pub(crate) viewport: Bounds<Pixels>,
    pub(crate) scroll_viewport: Size<Pixels>,
    pub(crate) content: Size<Pixels>,
    pub(crate) axis: ScrollbarAxis,
    pub(crate) placement: ScrollbarPlacement,
}

impl EditorScrollbarContext {
    pub fn axis(&self) -> ScrollbarAxis {
        self.axis
    }

    /// Where the editor places its scrollbars, as set with
    /// `scrollbar_placement`, for a supplied scrollbar to follow.
    pub fn placement(&self) -> ScrollbarPlacement {
        self.placement
    }

    /// The editor's viewport without padding, for consumers that need its
    /// text layout size. Scrollbars should use [`ScrollbarHandle::content_size`]
    /// and [`ScrollbarHandle::viewport_bounds`] to calculate the scroll range;
    /// both include the padding around this viewport.
    pub fn scroll_viewport_size(&self) -> Size<Pixels> {
        self.scroll_viewport
    }
}

impl ScrollbarHandle for EditorScrollbarContext {
    fn viewport_bounds(&self) -> Bounds<Pixels> {
        self.viewport
    }
    fn content_size(&self) -> Size<Pixels> {
        self.content + (self.viewport.size - self.scroll_viewport)
    }
    fn offset(&self) -> Point<Pixels> {
        self.handle.offset()
    }
    fn set_offset(&self, offset: Point<Pixels>) {
        self.handle.set_offset(offset);
    }
}

/// Supply a scrollbar from the consuming UI library. None retains Base's default.
pub type EditorScrollbarRenderer =
    Rc<dyn Fn(EditorScrollbarContext, &mut Window, &mut App) -> AnyElement>;
