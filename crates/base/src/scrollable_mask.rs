use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    App, Axis, BorderStyle, Bounds, ContentMask, Edges, Element, ElementId, GlobalElementId,
    Hitbox, Hsla, InteractiveElement as _, IntoElement, IsZero as _, LayoutId, OngoingScroll,
    PaintQuad, ParentElement as _, Point, Position, ScrollHandle, ScrollWheelEvent,
    StatefulInteractiveElement as _, Style, StyleRefinement, Styled as _, Window, div, px,
    relative,
};
use gpui::{Corners, Pixels};

use crate::{AxisExt, OngoingScrollExt as _, ScrollbarHandle, StyledExt as _};

/// Default element id for a mask: the call site, so two masks built in
/// different places never share their per-gesture axis lock.
#[inline]
#[track_caller]
fn caller_id() -> ElementId {
    ElementId::CodeLocation(*std::panic::Location::caller())
}

/// A horizontal scroll viewport that only consumes horizontal wheel deltas.
///
/// GPUI's native `overflow_x_scroll` maps vertical wheel input onto horizontal
/// scrolling when there is no vertical overflow. This wrapper keeps the visual
/// clipping and scroll offset, while delegating wheel input to [`ScrollableMask`]
/// so vertical wheel events can continue bubbling to the parent scroller.
pub(crate) fn horizontal_scroll_area(
    id: impl Into<ElementId>,
    scroll_handle: &ScrollHandle,
    style: &StyleRefinement,
    child: impl IntoElement,
) -> impl IntoElement {
    let id = id.into();

    // The mask must be a sibling of the scrolled element (like in Table), not
    // a child of it: children are prepainted with the scroll offset applied,
    // which would slide the mask away from the viewport as the content
    // scrolls, leaving the uncovered part to the parent scroller.
    //
    // The frame cover is a sibling for the same reason: it keeps the rounded
    // corners clean at every scroll offset (see [`RoundedFrameCover`]).
    let frame = RoundedFrameCover {
        corner_radii: Corners {
            top_left: style.corner_radii.top_left,
            top_right: style.corner_radii.top_right,
            bottom_left: style.corner_radii.bottom_left,
            bottom_right: style.corner_radii.bottom_right,
        },
        backdrop: None,
        frame_border: px(0.),
    };

    div()
        .w_full()
        .relative()
        .child(
            div()
                .id(id.clone())
                .w_full()
                .refine_style(style)
                .overflow_hidden()
                .track_scroll(scroll_handle)
                .child(child),
        )
        .child(ScrollableMask::new(Axis::Horizontal, scroll_handle).id(id))
        .child(frame)
}

/// Keeps a rounded scroll viewport's corners clean at every scroll offset.
///
/// GPUI clips children with a rectangular content mask, so content scrolled
/// under a rounded frame paints square into the corner notches — the regions
/// inside the frame's rect corners but outside its border arcs. Rounding the
/// content itself (e.g. a table's first row) only helps while its own corners
/// sit at the viewport corners; once the content scrolls, the clipped edge
/// meets the notch square again. The cover fills each notch with the backdrop
/// color, which reads as a rounded clip. The frame's own border needs no help:
/// gpui paints an element's border after its children, so it already draws
/// over both the scrolled content and these covers.
///
/// Like [`ScrollableMask`], it must be a sibling of the scrolled element so it
/// does not move with the scroll offset.
pub struct RoundedFrameCover {
    corner_radii: Corners<Option<gpui::AbsoluteLength>>,
    backdrop: Option<Hsla>,
    frame_border: Pixels,
}

impl RoundedFrameCover {
    /// A frame with the same radius on all four corners.
    pub fn uniform(radius: Pixels) -> Self {
        let radius = Some(radius.into());
        Self {
            corner_radii: Corners {
                top_left: radius,
                top_right: radius,
                bottom_left: radius,
                bottom_right: radius,
            },
            backdrop: None,
            frame_border: px(0.),
        }
    }

    /// The frame's border width, when the cover is a child of the bordered
    /// element itself: the cover's bounds are then the content box, so the
    /// painted notches are grown outward by this width to reach the border
    /// box. The border itself needs no repaint — gpui paints an element's
    /// border after its children, so the frame's own border already draws
    /// over the covers.
    pub fn frame_border(mut self, width: Pixels) -> Self {
        self.frame_border = width;
        self
    }

    /// The color behind the frame, used to fill the corner notches.
    /// Defaults to the theme background.
    pub fn backdrop(mut self, color: Hsla) -> Self {
        self.backdrop = Some(color);
        self
    }
}

impl IntoElement for RoundedFrameCover {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for RoundedFrameCover {
    type RequestLayoutState = ();
    type PrepaintState = Bounds<Pixels>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        // Same layout trick as `ScrollableMask`: absolute and parent-sized.
        let mut style = Style::default();
        style.position = Position::Absolute;
        style.flex_grow = 1.0;
        style.flex_shrink = 1.0;
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();

        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        // Shift up by one height to cover the viewport, like `ScrollableMask`.
        Bounds {
            origin: Point {
                x: bounds.origin.x,
                y: bounds.origin.y - bounds.size.height,
            },
            size: bounds.size,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        cover_bounds: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        use crate::theme::ActiveTheme as _;

        let mut bounds = *cover_bounds;
        if !self.frame_border.is_zero() {
            bounds.origin.x -= self.frame_border;
            bounds.origin.y -= self.frame_border;
            bounds.size.width += self.frame_border * 2.;
            bounds.size.height += self.frame_border * 2.;
        }
        let rem_size = window.rem_size();
        let resolve = |corner: Option<gpui::AbsoluteLength>| {
            corner
                .map(|radius| radius.to_pixels(rem_size))
                .filter(|radius| !radius.is_zero())
        };
        let radii = Corners {
            top_left: resolve(self.corner_radii.top_left),
            top_right: resolve(self.corner_radii.top_right),
            bottom_left: resolve(self.corner_radii.bottom_left),
            bottom_right: resolve(self.corner_radii.bottom_right),
        };
        if radii.top_left.is_none()
            && radii.top_right.is_none()
            && radii.bottom_left.is_none()
            && radii.bottom_right.is_none()
        {
            return;
        }

        let backdrop = self.backdrop.unwrap_or(cx.theme().tokens.colors.background);
        let corners = [
            (bounds.origin, 1., 1., radii.top_left),
            (bounds.top_right(), -1., 1., radii.top_right),
            (bounds.bottom_right(), -1., -1., radii.bottom_right),
            (
                Point {
                    x: bounds.origin.x,
                    y: bounds.bottom_right().y,
                },
                1.,
                -1.,
                radii.bottom_left,
            ),
        ];
        for (corner, x_dir, y_dir, radius) in corners {
            let Some(radius) = radius else { continue };
            if let Some(path) = corner_notch_path(corner, x_dir, y_dir, radius) {
                window.paint_path(path, backdrop);
            }
        }
    }
}

/// The notch at one frame corner: the square of side `radius` anchored at
/// `corner`, minus the quarter disc of the border arc. `x_dir`/`y_dir` point
/// from the corner toward the frame's inside (`±1`).
fn corner_notch_path(
    corner: Point<Pixels>,
    x_dir: f32,
    y_dir: f32,
    radius: Pixels,
) -> Option<gpui::Path<Pixels>> {
    let mut builder = gpui::PathBuilder::fill();
    builder.move_to(corner);
    builder.line_to(Point {
        x: corner.x + radius * x_dir,
        y: corner.y,
    });
    builder.arc_to(
        Point {
            x: radius,
            y: radius,
        },
        px(0.),
        false,
        // The quarter arc nearest the corner: counterclockwise when the two
        // inward directions agree in sign, clockwise otherwise.
        x_dir * y_dir < 0.,
        Point {
            x: corner.x,
            y: corner.y + radius * y_dir,
        },
    );
    builder.close();
    builder.build().ok()
}

/// Make a scrollable mask element to cover the parent view with the mouse wheel event listening.
///
/// When the mouse wheel is scrolled, will move the `scroll_handle` scrolling with the `axis` direction.
/// You can use this `scroll_handle` to control what you want to scroll.
/// This is only can handle once axis scrolling.
///
/// Axis-dominant wheel events are consumed in the capture phase, so the mask
/// wins over ancestor scrollers (e.g. `gpui::list`) that register their
/// listeners after their children; events dominated by the other axis keep
/// propagating. The mask stays inert while occluded.
///
/// Dominance is decided per gesture, not per event: a precise (trackpad) delta
/// locks onto the axis its gesture started on, so mid-swipe wobble cannot flip
/// which mask consumes the events. Line deltas keep the per-event comparison.
///
/// At the scroll edge the two axes differ, matching platform scrollers:
/// a vertical mask hands the event over to the ancestor scroller (CSS
/// `overscroll-behavior: auto` chaining), while a horizontal mask keeps
/// consuming it — a bubbled horizontal delta would get mapped onto a
/// vertical-only ancestor by gpui's own wheel listener (see #2468).
pub struct ScrollableMask<H: ScrollbarHandle + Clone = ScrollHandle> {
    axis: Axis,
    id: ElementId,
    scroll_handle: H,
    debug: Option<Hsla>,
}

impl<H: ScrollbarHandle + Clone> ScrollableMask<H> {
    /// Create a new scrollable mask element.
    #[track_caller]
    pub fn new(axis: Axis, scroll_handle: &H) -> Self {
        Self {
            scroll_handle: scroll_handle.clone(),
            axis,
            id: caller_id(),
            debug: None,
        }
    }

    /// Set a specific element id, default is the [`std::panic::Location::caller`].
    ///
    /// Only needed when one call site creates several masks of the same axis,
    /// which would otherwise share their gesture axis lock.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Enable the debug border, to show the mask bounds.
    #[allow(dead_code)]
    pub fn debug(mut self) -> Self {
        self.debug = Some(gpui::yellow());
        self
    }
}

impl<H: ScrollbarHandle + Clone> IntoElement for ScrollableMask<H> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<H: ScrollbarHandle + Clone> Element for ScrollableMask<H> {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    // An id is needed to keep the gesture's axis lock across frames. The axis
    // suffix keeps both masks of one scroller apart when they share an id.
    fn id(&self) -> Option<ElementId> {
        let axis = match self.axis {
            Axis::Horizontal => "horizontal",
            Axis::Vertical => "vertical",
        };

        Some((self.id.clone(), axis).into())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        // Set the layout style relative to the table view to get same size.
        style.position = Position::Absolute;
        style.flex_grow = 1.0;
        style.flex_shrink = 1.0;
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();

        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        // Move y to bounds height to cover the parent view.
        let cover_bounds = Bounds {
            origin: Point {
                x: bounds.origin.x,
                y: bounds.origin.y - bounds.size.height,
            },
            size: bounds.size,
        };

        window.insert_hitbox(cover_bounds, gpui::HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        let is_horizontal = self.axis.is_horizontal();
        let line_height = window.line_height();
        let bounds = hitbox.bounds;
        let ongoing_scroll = global_id
            .map(|global_id| {
                window.with_element_state::<Rc<RefCell<OngoingScroll>>, _>(global_id, |state, _| {
                    let state = state.unwrap_or_default();
                    (state.clone(), state)
                })
            })
            .unwrap_or_default();

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            if let Some(color) = self.debug {
                window.paint_quad(PaintQuad {
                    bounds,
                    border_widths: Edges::all(px(1.0)),
                    border_color: color,
                    background: gpui::transparent_white().into(),
                    corner_radii: Corners::all(px(0.)),
                    border_style: BorderStyle::default(),
                });
            }

            window.on_mouse_event({
                let view_id = window.current_view();
                let scroll_handle = self.scroll_handle.clone();
                let hitbox_id = hitbox.id;
                let ongoing_scroll = ongoing_scroll.clone();

                move |event: &ScrollWheelEvent, phase, window, cx| {
                    // Handle in the capture phase: ancestor scrollers such as
                    // `gpui::list` register their wheel listeners after their
                    // children paint, so in the bubble phase (reverse
                    // registration order) they run first and would consume the
                    // vertical component of a trackpad swipe before this mask
                    // could stop the propagation.
                    //
                    // `should_handle_scroll` (instead of a raw bounds check)
                    // keeps the mask inert when it is occluded, e.g. below an
                    // open dialog or context menu.
                    if !(phase.capture() && hitbox_id.should_handle_scroll(window)) {
                        return;
                    }

                    let mut offset = scroll_handle.offset();
                    let mut delta = event.delta.pixel_delta(line_height);

                    // Lock the gesture to the axis it started on, so a diagonal
                    // trackpad swipe cannot flip which mask consumes it from one
                    // event to the next. Line deltas carry no touch phase.
                    if event.delta.precise() {
                        ongoing_scroll
                            .borrow_mut()
                            .lock_axis(&mut delta, event.touch_phase);
                    }

                    // Limit for only one way scrolling at same time.
                    // When use MacBook touchpad we may get both x and y delta,
                    // only allows the one that more to scroll.
                    if !delta.x.is_zero() && !delta.y.is_zero() {
                        if delta.x.abs() > delta.y.abs() {
                            delta.y = px(0.);
                        } else {
                            delta.x = px(0.);
                        }
                    }

                    if !is_horizontal {
                        // The current offset must be clamped too: after a
                        // bubbled event, the scrolled element's own listener
                        // pushes the shared offset beyond the edge unclamped
                        // (the div only clamps on prepaint), and that
                        // transient overscroll would read as "room to scroll".
                        // `ScrollbarHandle` has no `max_offset`; recover it from
                        // the definition `content_size = viewport + max_offset`.
                        let axis_max = (scroll_handle.content_size().height
                            - scroll_handle.viewport_bounds().size.height)
                            .max(px(0.));
                        let current = offset.y.clamp(-axis_max, px(0.));
                        let new_offset = (current + delta.y).clamp(-axis_max, px(0.));
                        if new_offset == current {
                            // At the edge or no overflow: bubble to the parent.
                            return;
                        }

                        offset.y = new_offset;
                        scroll_handle.set_offset(offset);
                        cx.notify(view_id);
                        cx.stop_propagation();
                        return;
                    }

                    offset.x += delta.x;

                    // NOTE: `set_offset` does not clamp (clamping happens in
                    // the div's prepaint), so any non-zero horizontal-dominant
                    // delta passes this guard — even at the scroll edge the
                    // event is consumed rather than turned into a parent
                    // scroll.
                    if offset != scroll_handle.offset() {
                        scroll_handle.set_offset(offset);
                        cx.notify(view_id);
                        cx.stop_propagation();
                    }
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Context, IntoElement, ListAlignment, ListState, Render, ScrollDelta, ScrollWheelEvent,
        TestAppContext, VisualTestContext, Window, div, list, point, px,
    };

    #[test]
    fn corner_notch_paths_stay_inside_their_corner_squares() {
        let width = px(200.);
        let height = px(100.);
        let radius = px(8.);
        let cases = [
            ("top left", point(px(0.), px(0.)), 1., 1.),
            ("top right", point(width, px(0.)), -1., 1.),
            ("bottom right", point(width, height), -1., -1.),
            ("bottom left", point(px(0.), height), 1., -1.),
        ];
        for (name, corner, x_dir, y_dir) in cases {
            let path =
                corner_notch_path(corner, x_dir, y_dir, radius).expect("the notch tessellates");
            let bounds = path.bounds;
            // Curve tessellation may inset fractionally; one pixel of slack.
            let tolerance = 1.0;
            assert!(
                (f32::from(bounds.size.width) - f32::from(radius)).abs() <= tolerance
                    && (f32::from(bounds.size.height) - f32::from(radius)).abs() <= tolerance,
                "{name}: notch size {:?} must stay a {radius:?} square",
                bounds.size,
            );
            let expected_origin = point(
                if x_dir > 0. {
                    corner.x
                } else {
                    corner.x - radius
                },
                if y_dir > 0. {
                    corner.y
                } else {
                    corner.y - radius
                },
            );
            assert!(
                (f32::from(bounds.origin.x) - f32::from(expected_origin.x)).abs() <= tolerance
                    && (f32::from(bounds.origin.y) - f32::from(expected_origin.y)).abs()
                        <= tolerance,
                "{name}: notch bounds {bounds:?} must anchor at the frame corner",
            );
        }
    }

    struct HorizontalScrollAreaTest {
        scroll_handle: ScrollHandle,
    }

    impl Render for HorizontalScrollAreaTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(100.)).h(px(40.)).child(horizontal_scroll_area(
                "horizontal-scroll-area",
                &self.scroll_handle,
                &Default::default(),
                div().w(px(300.)).h(px(40.)),
            ))
        }
    }

    #[gpui::test]
    fn horizontal_scroll_area_ignores_vertical_wheel(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let (_, cx) = cx.add_window_view({
            let scroll_handle = scroll_handle.clone();
            move |_, _| HorizontalScrollAreaTest {
                scroll_handle: scroll_handle.clone(),
            }
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
            ..Default::default()
        });

        assert_eq!(scroll_handle.offset().x, px(0.));
    }

    /// Reproduces the markdown table case: the scroll area lives inside a
    /// `gpui::list` item. The list registers its wheel listener after its
    /// items paint, so in the bubble phase (reverse registration order) the
    /// list runs first and consumes `delta.y` of every trackpad swipe.
    struct ListWithHorizontalAreaTest {
        scroll_handle: ScrollHandle,
        list_state: ListState,
        occluded: bool,
    }

    impl Render for ListWithHorizontalAreaTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let scroll_handle = self.scroll_handle.clone();
            let mut root = div().w(px(100.)).h(px(100.)).child(
                list(self.list_state.clone(), move |ix, _, _| {
                    if ix == 0 {
                        horizontal_scroll_area(
                            "horizontal-scroll-area",
                            &scroll_handle,
                            &Default::default(),
                            div().w(px(300.)).h(px(40.)),
                        )
                        .into_any_element()
                    } else {
                        div().w(px(100.)).h(px(40.)).into_any_element()
                    }
                })
                .w_full()
                .h_full(),
            );
            if self.occluded {
                // An overlay above the list, like an open dialog or menu.
                root = root.child(div().absolute().top_0().left_0().size_full().occlude());
            }
            root
        }
    }

    fn setup_list_test<'a>(
        cx: &'a mut TestAppContext,
        scroll_handle: &ScrollHandle,
        list_state: &ListState,
        occluded: bool,
    ) -> &'a mut VisualTestContext {
        let (_, cx) = cx.add_window_view({
            let scroll_handle = scroll_handle.clone();
            let list_state = list_state.clone();
            move |_, _| ListWithHorizontalAreaTest {
                scroll_handle: scroll_handle.clone(),
                list_state: list_state.clone(),
                occluded,
            }
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        cx
    }

    #[gpui::test]
    fn horizontal_scroll_area_in_list_keeps_horizontal_dominant_wheel(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let list_state = ListState::new(10, ListAlignment::Top, px(0.));
        let cx = setup_list_test(cx, &scroll_handle, &list_state, false);

        // A trackpad swipe is rarely axis-pure: horizontal dominant with a
        // small vertical component.
        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-40.), px(-10.))),
            ..Default::default()
        });

        // The area consumes the horizontal delta...
        assert_eq!(scroll_handle.offset().x, px(-40.));
        // ...and the outer list must not scroll vertically.
        let scroll_top = list_state.logical_scroll_top();
        assert_eq!((scroll_top.item_ix, scroll_top.offset_in_item), (0, px(0.)));
    }

    #[gpui::test]
    fn horizontal_scroll_area_in_list_bubbles_vertical_dominant_wheel(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let list_state = ListState::new(10, ListAlignment::Top, px(0.));
        let cx = setup_list_test(cx, &scroll_handle, &list_state, false);

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-10.), px(-40.))),
            ..Default::default()
        });

        // Vertical dominant: the list scrolls, the area does not.
        assert_eq!(scroll_handle.offset().x, px(0.));
        let scroll_top = list_state.logical_scroll_top();
        assert_ne!((scroll_top.item_ix, scroll_top.offset_in_item), (0, px(0.)));
    }

    /// A `MessageScroller`-shaped nesting: a `gpui::list` with a vertical
    /// mask bound to its `ListState`, inside an ancestor vertical scroller.
    struct ListWithVerticalMaskTest {
        outer_handle: ScrollHandle,
        list_state: ListState,
    }

    impl Render for ListWithVerticalMaskTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .id("outer")
                .w(px(100.))
                .h(px(100.))
                .overflow_y_scroll()
                .track_scroll(&self.outer_handle)
                .child(
                    div().w_full().h(px(400.)).child(
                        div()
                            .relative()
                            .w_full()
                            .h(px(80.))
                            .overflow_hidden()
                            .child(
                                list(self.list_state.clone(), |_, _, _| {
                                    div().w_full().h(px(40.)).into_any_element()
                                })
                                .w_full()
                                .h_full(),
                            )
                            .child(ScrollableMask::new(Axis::Vertical, &self.list_state)),
                    ),
                )
        }
    }

    fn setup_vertical_mask_test<'a>(
        cx: &'a mut TestAppContext,
        outer_handle: &ScrollHandle,
        list_state: &ListState,
    ) -> &'a mut VisualTestContext {
        let (_, cx) = cx.add_window_view({
            let outer_handle = outer_handle.clone();
            let list_state = list_state.clone();
            move |_, _| ListWithVerticalMaskTest {
                outer_handle: outer_handle.clone(),
                list_state: list_state.clone(),
            }
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        cx
    }

    #[gpui::test]
    fn vertical_mask_contains_wheel_inside_the_list(cx: &mut TestAppContext) {
        let outer_handle = ScrollHandle::new();
        // A large overdraw measures every row up front, so the mask sees the
        // full content height.
        let list_state = ListState::new(10, ListAlignment::Top, px(1000.));
        let cx = setup_vertical_mask_test(cx, &outer_handle, &list_state);

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
            ..Default::default()
        });

        // The inner list consumes the wheel; the ancestor must not move.
        assert_eq!(list_state.scroll_px_offset_for_scrollbar().y, px(-40.));
        assert_eq!(outer_handle.offset().y, px(0.));
    }

    #[gpui::test]
    fn vertical_mask_chains_to_the_ancestor_at_the_edge(cx: &mut TestAppContext) {
        let outer_handle = ScrollHandle::new();
        let list_state = ListState::new(10, ListAlignment::Top, px(1000.));
        // Start the inner list at its bottom edge (400 - 80 = 320).
        list_state.scroll_to(gpui::ListOffset {
            item_ix: 8,
            offset_in_item: px(0.),
        });
        let cx = setup_vertical_mask_test(cx, &outer_handle, &list_state);

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
            ..Default::default()
        });

        // At the edge the mask lets the event bubble to the ancestor.
        assert_eq!(list_state.scroll_px_offset_for_scrollbar().y, px(-320.));
        assert_eq!(outer_handle.offset().y, px(-40.));
    }

    #[gpui::test]
    fn horizontal_scroll_area_covers_viewport_after_scrolled(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let list_state = ListState::new(10, ListAlignment::Top, px(0.));
        let cx = setup_list_test(cx, &scroll_handle, &list_state, false);

        // Scroll the area to the middle, then repaint.
        scroll_handle.set_offset(point(px(-150.), px(0.)));
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        // Swipe over the right side of the viewport. The mask must still
        // cover it — it must not slide away with the scrolled content.
        cx.simulate_event(ScrollWheelEvent {
            position: point(px(90.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-40.), px(-10.))),
            ..Default::default()
        });

        assert_eq!(scroll_handle.offset().x, px(-190.));
        let scroll_top = list_state.logical_scroll_top();
        assert_eq!((scroll_top.item_ix, scroll_top.offset_in_item), (0, px(0.)));
    }

    #[gpui::test]
    fn horizontal_scroll_area_traps_wheel_at_edge(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let list_state = ListState::new(10, ListAlignment::Top, px(0.));
        let cx = setup_list_test(cx, &scroll_handle, &list_state, false);

        // Scroll the area to its right edge (300 - 100 = 200).
        scroll_handle.set_offset(point(px(-200.), px(0.)));
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-40.), px(-10.))),
            ..Default::default()
        });

        // A horizontal mask consumes even at the edge: a bubbled horizontal
        // delta would be axis-mapped onto the vertical list (#2468).
        let scroll_top = list_state.logical_scroll_top();
        assert_eq!((scroll_top.item_ix, scroll_top.offset_in_item), (0, px(0.)));
    }

    #[gpui::test]
    fn horizontal_scroll_area_ignores_wheel_when_occluded(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let list_state = ListState::new(10, ListAlignment::Top, px(0.));
        let cx = setup_list_test(cx, &scroll_handle, &list_state, true);

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-40.), px(-10.))),
            ..Default::default()
        });

        // An overlay (dialog, context menu) occludes the area: the area
        // must not scroll underneath it.
        assert_eq!(scroll_handle.offset().x, px(0.));
    }

    #[gpui::test]
    fn horizontal_scroll_area_uses_horizontal_wheel(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let (_, cx) = cx.add_window_view({
            let scroll_handle = scroll_handle.clone();
            move |_, _| HorizontalScrollAreaTest {
                scroll_handle: scroll_handle.clone(),
            }
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-40.), px(0.))),
            ..Default::default()
        });

        assert_eq!(scroll_handle.offset().x, px(-40.));
    }

    fn setup_horizontal_area_test<'a>(
        cx: &'a mut TestAppContext,
        scroll_handle: &ScrollHandle,
    ) -> &'a mut VisualTestContext {
        let (_, cx) = cx.add_window_view({
            let scroll_handle = scroll_handle.clone();
            move |_, _| HorizontalScrollAreaTest {
                scroll_handle: scroll_handle.clone(),
            }
        });
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        cx
    }

    #[gpui::test]
    fn horizontal_mask_keeps_axis_lock_within_a_gesture(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let cx = setup_horizontal_area_test(cx, &scroll_handle);

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-40.), px(-10.))),
            ..Default::default()
        });
        assert_eq!(scroll_handle.offset().x, px(-40.));

        // Same gesture, now leaning vertical but under the unlock ratio: the
        // lock holds and the horizontal offset keeps moving. Comparing this
        // event alone would zero `delta.x` and stall the scroller at -40.
        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-10.), px(-15.))),
            ..Default::default()
        });
        assert_eq!(scroll_handle.offset().x, px(-50.));
    }

    #[gpui::test]
    fn horizontal_mask_releases_axis_lock_on_a_strong_turn(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let cx = setup_horizontal_area_test(cx, &scroll_handle);

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-40.), px(-10.))),
            ..Default::default()
        });
        assert_eq!(scroll_handle.offset().x, px(-40.));

        // Past the unlock ratio the gesture is no longer horizontal, so the
        // event stops driving this scroller.
        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(-10.), px(-25.))),
            ..Default::default()
        });
        assert_eq!(scroll_handle.offset().x, px(-40.));
    }

    /// Reproduces the DataTable case: a vertically scrollable element
    /// nested inside an outer vertical scroller.
    struct NestedVerticalScrollTest {
        outer_handle: ScrollHandle,
        inner_handle: ScrollHandle,
        inner_content_height: Pixels,
    }

    impl Render for NestedVerticalScrollTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .id("outer")
                .w(px(100.))
                .h(px(100.))
                .overflow_y_scroll()
                .track_scroll(&self.outer_handle)
                .child(
                    div()
                        .relative()
                        .w_full()
                        .h(px(60.))
                        .child(
                            div()
                                .id("inner")
                                .size_full()
                                .overflow_y_scroll()
                                .track_scroll(&self.inner_handle)
                                .child(div().w_full().h(self.inner_content_height)),
                        )
                        .child(ScrollableMask::new(Axis::Vertical, &self.inner_handle)),
                )
                .child(div().w_full().h(px(400.)))
        }
    }

    fn setup_nested_vertical_test<'a>(
        cx: &'a mut TestAppContext,
        outer_handle: &ScrollHandle,
        inner_handle: &ScrollHandle,
        inner_content_height: Pixels,
    ) -> &'a mut VisualTestContext {
        let (_, cx) = cx.add_window_view({
            let outer_handle = outer_handle.clone();
            let inner_handle = inner_handle.clone();
            move |_, _| NestedVerticalScrollTest {
                outer_handle: outer_handle.clone(),
                inner_handle: inner_handle.clone(),
                inner_content_height,
            }
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        cx
    }

    #[gpui::test]
    fn vertical_mask_consumes_wheel_when_scrollable(cx: &mut TestAppContext) {
        let outer_handle = ScrollHandle::new();
        let inner_handle = ScrollHandle::new();
        let cx = setup_nested_vertical_test(cx, &outer_handle, &inner_handle, px(300.));

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
            ..Default::default()
        });

        // The inner scroller consumes the event; the outer one must not move.
        assert_eq!(inner_handle.offset().y, px(-40.));
        assert_eq!(outer_handle.offset().y, px(0.));
    }

    #[gpui::test]
    fn vertical_mask_hands_off_to_parent_at_edge(cx: &mut TestAppContext) {
        let outer_handle = ScrollHandle::new();
        let inner_handle = ScrollHandle::new();
        let cx = setup_nested_vertical_test(cx, &outer_handle, &inner_handle, px(300.));

        // Scroll the inner element to its bottom edge (300 - 60 = 240).
        inner_handle.set_offset(point(px(0.), px(-240.)));
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
            ..Default::default()
        });

        // At the edge the event bubbles: the outer scroller takes over.
        assert_eq!(outer_handle.offset().y, px(-40.));
        // The inner offset is clamped back to the edge on the next prepaint.
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        assert_eq!(inner_handle.offset().y, px(-240.));
    }

    #[gpui::test]
    fn vertical_mask_bubbles_when_no_overflow(cx: &mut TestAppContext) {
        let outer_handle = ScrollHandle::new();
        let inner_handle = ScrollHandle::new();
        // Inner content (40) fits its 60px viewport: nothing to scroll.
        let cx = setup_nested_vertical_test(cx, &outer_handle, &inner_handle, px(40.));

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
            ..Default::default()
        });

        assert_eq!(outer_handle.offset().y, px(-40.));
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        assert_eq!(inner_handle.offset().y, px(0.));
    }

    #[gpui::test]
    fn vertical_mask_ignores_transient_overscroll(cx: &mut TestAppContext) {
        let outer_handle = ScrollHandle::new();
        let inner_handle = ScrollHandle::new();
        let cx = setup_nested_vertical_test(cx, &outer_handle, &inner_handle, px(300.));

        inner_handle.set_offset(point(px(0.), px(-240.)));
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        // Two wheel events at the edge with no redraw in between: the first
        // one leaves the inner offset beyond the edge unclamped, which must
        // not read as "room to scroll" and swallow the second event.
        for _ in 0..2 {
            cx.simulate_event(ScrollWheelEvent {
                position: point(px(10.), px(10.)),
                delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
                ..Default::default()
            });
        }

        assert_eq!(outer_handle.offset().y, px(-80.));
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
        assert_eq!(inner_handle.offset().y, px(-240.));
    }

    /// The vertical mask nested in a `gpui::list` ancestor: the list
    /// registers its wheel listener after its items paint, so only a
    /// capture-phase mask can stop it from scrolling on the same event.
    struct ListWithVerticalAreaTest {
        scroll_handle: ScrollHandle,
        list_state: ListState,
    }

    impl Render for ListWithVerticalAreaTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let scroll_handle = self.scroll_handle.clone();
            div().w(px(100.)).h(px(100.)).child(
                list(self.list_state.clone(), move |ix, _, _| {
                    if ix == 0 {
                        div()
                            .relative()
                            .w_full()
                            .h(px(60.))
                            .child(
                                div()
                                    .id("inner")
                                    .size_full()
                                    .overflow_y_scroll()
                                    .track_scroll(&scroll_handle)
                                    .child(div().w_full().h(px(300.))),
                            )
                            .child(ScrollableMask::new(Axis::Vertical, &scroll_handle))
                            .into_any_element()
                    } else {
                        div().w(px(100.)).h(px(40.)).into_any_element()
                    }
                })
                .w_full()
                .h_full(),
            )
        }
    }

    #[gpui::test]
    fn vertical_mask_in_list_consumes_wheel_when_scrollable(cx: &mut TestAppContext) {
        let scroll_handle = ScrollHandle::new();
        let list_state = ListState::new(10, ListAlignment::Top, px(0.));
        let (_, cx) = cx.add_window_view({
            let scroll_handle = scroll_handle.clone();
            let list_state = list_state.clone();
            move |_, _| ListWithVerticalAreaTest {
                scroll_handle: scroll_handle.clone(),
                list_state: list_state.clone(),
            }
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        cx.simulate_event(ScrollWheelEvent {
            position: point(px(10.), px(10.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
            ..Default::default()
        });

        // The inner scroller consumes the event; the list must not scroll.
        assert_eq!(scroll_handle.offset().y, px(-40.));
        let scroll_top = list_state.logical_scroll_top();
        assert_eq!((scroll_top.item_ix, scroll_top.offset_in_item), (0, px(0.)));
    }
}
