//! Presentation hook for an editor's existing viewport, without a second scroll system.
use crate::{ScrollbarAxis, ScrollbarHandle};
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
    pub(crate) content_height_f64: f64,
    pub(crate) scroll_y_f64: f64,
    pub(crate) axis: ScrollbarAxis,
}

impl EditorScrollbarContext {
    pub fn axis(&self) -> ScrollbarAxis {
        self.axis
    }

    /// Authoritative vertical content height in `f64` pixels (including padding
    /// when combined via `scroll_viewport_size`).
    pub fn content_height_f64(&self) -> f64 {
        self.content_height_f64
    }

    /// Authoritative vertical scroll offset in `f64` pixels (`<= 0.0`).
    pub fn scroll_y_f64(&self) -> f64 {
        self.scroll_y_f64
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
