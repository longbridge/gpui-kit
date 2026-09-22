use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, App, Axis, Element, ElementId, Entity, GlobalElementId, InteractiveElement,
    IntoElement, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement as _, Pixels, Point,
    Render, StatefulInteractiveElement, Styled as _, Window, div, prelude::FluentBuilder as _, px,
};

use crate::{AxisExt as _, theme::ActiveTheme as _};

pub(crate) const HANDLE_PADDING: Pixels = px(4.);
pub(crate) const HANDLE_SIZE: Pixels = px(1.);
/// How far a hugging handle's hairline sits from the boundary it marks.
///
/// It is room for a renderer to draw something thicker than the line and have
/// it overhang evenly without crossing back outside the container, where
/// [`HandleEdge`] explains what would happen to it. Expressed as padding
/// rather than as an inset on the handle's own box, because an inset on the
/// side a handle is pinned to does not survive the box's own sizing.
pub(crate) const EDGE_CLEARANCE: Pixels = px(1.);

/// Create a resize handle for a resizable panel.
#[doc(hidden)]
pub fn resize_handle<T: 'static, E: 'static + Render>(
    id: impl Into<ElementId>,
    axis: Axis,
) -> ResizeHandle<T, E> {
    ResizeHandle::new(id, axis)
}

/// Draws the visible part of a resize handle.
///
/// Returning `None` keeps the built-in line, so a renderer can override some
/// handles and leave the rest alone.
pub type ResizeHandleRenderer =
    Rc<dyn Fn(&ResizeHandleContext, &mut Window, &mut App) -> Option<AnyElement>>;

/// What a [`ResizeHandleRenderer`] is told about the handle it is drawing.
///
/// The hit area, the cursor and the drag itself stay with the handle; a
/// renderer only supplies what is painted inside it.
pub struct ResizeHandleContext {
    axis: Axis,
    state: ResizeHandleState,
}

impl ResizeHandleContext {
    /// The axis the handle resizes along: `Horizontal` for a vertical divider
    /// between two side-by-side panels.
    pub fn axis(&self) -> Axis {
        self.axis
    }

    /// Whether the pointer currently owns this handle.
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// How far the pointer has gone with this handle.
    pub fn state(&self) -> ResizeHandleState {
        self.state
    }
}

/// How far the pointer has gone with a resize handle.
///
/// A drag takes the pointer out of the handle's own band within a pixel or
/// two, so GPUI's hover reads false for most of a drag and cannot stand in for
/// `Dragging`. Base tracks the progression instead, and a renderer reads it
/// through [`ResizeHandleContext::state`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResizeHandleState {
    /// The pointer is somewhere else.
    #[default]
    Idle,
    /// The pointer is over the handle's band.
    Hovered,
    /// The pointer went down on the handle and has not moved since.
    Pressed,
    /// The handle is being dragged.
    Dragging,
}

impl ResizeHandleState {
    /// Whether the pointer owns the handle -- pressed on it, or dragging it.
    pub fn is_active(self) -> bool {
        matches!(self, Self::Pressed | Self::Dragging)
    }
}

/// Which edge of its own container a handle hugs.
///
/// A handle named no edge straddles the boundary it resizes, half its band on
/// either side, which is what a divider between two panels of a group wants.
/// A dock's own edge handle cannot: [`dock_frame`] clips to the dock's box, so
/// the half hanging outside is cut away -- what it paints and what it
/// hit-tests alike, which is why the outer half of a dock's grab band has
/// never actually been grabbable. Naming the edge moves the whole handle
/// inside, one pixel clear of the boundary so that an indicator thicker than
/// the hairline still sits centred on it without crossing back out.
///
/// [`dock_frame`]: crate::dock::dock_frame
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleEdge {
    /// Where the axis starts: the left edge of a horizontal handle's
    /// container, the top edge of a vertical one's.
    Leading,
    /// Where the axis ends.
    Trailing,
}

#[doc(hidden)]
pub struct ResizeHandle<T: 'static, E: 'static + Render> {
    id: ElementId,
    axis: Axis,
    drag_value: Option<Rc<T>>,
    edge: Option<HandleEdge>,
    on_drag: Option<Rc<dyn Fn(&Point<Pixels>, &mut Window, &mut App) -> Entity<E>>>,
    appearance: Option<ResizeHandleRenderer>,
}

impl<T: 'static, E: 'static + Render> ResizeHandle<T, E> {
    fn new(id: impl Into<ElementId>, axis: Axis) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            on_drag: None,
            drag_value: None,
            edge: None,
            appearance: None,
            axis,
        }
    }

    /// Hand the painted part of this handle to `appearance`.
    pub fn with_appearance(mut self, appearance: ResizeHandleRenderer) -> Self {
        self.appearance = Some(appearance);
        self
    }

    pub fn on_drag(
        mut self,
        value: T,
        f: impl Fn(Rc<T>, &Point<Pixels>, &mut Window, &mut App) -> Entity<E> + 'static,
    ) -> Self {
        let value = Rc::new(value);
        self.drag_value = Some(value.clone());
        self.on_drag = Some(Rc::new(move |p, window, cx| {
            f(value.clone(), p, window, cx)
        }));
        self
    }

    /// Keep the whole handle inside its container, hugging `edge`, instead of
    /// straddling the boundary it resizes.
    pub fn inside(mut self, edge: HandleEdge) -> Self {
        self.edge = Some(edge);
        self
    }
}

/// One handle's [`ResizeHandleState`], shared between the element and the
/// mouse listeners it registers.
///
/// The `Rc` is load-bearing. `with_element_state` hands back a clone and a
/// bare `Cell` clones by value, so a listener holding one wrote its progress
/// into a copy that died with the event: the stored state never left `Idle`
/// and `is_active` never once read true.
#[derive(Default, Debug, Clone)]
struct SharedHandleState {
    state: Rc<Cell<ResizeHandleState>>,
}

impl SharedHandleState {
    fn get(&self) -> ResizeHandleState {
        self.state.get()
    }

    /// Reports whether the state actually changed, so a listener repaints the
    /// window only when there is something new to paint.
    fn set(&self, state: ResizeHandleState) -> bool {
        let changed = self.state.get() != state;
        self.state.set(state);
        changed
    }
}

impl<T: 'static, E: 'static + Render> IntoElement for ResizeHandle<T, E> {
    type Element = ResizeHandle<T, E>;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl<T: 'static, E: 'static + Render> Element for ResizeHandle<T, E> {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let neg_offset = -HANDLE_PADDING;
        let axis = self.axis;
        // Sizes are border-box: the extent has to name the whole band, padding
        // included, or the content box resolves to zero and the hairline
        // overflows into the padding -- which is how a handle pinned to an edge
        // ended up drawing its line flush against the boundary it was supposed
        // to stay clear of.
        let hug_extent = HANDLE_SIZE + HANDLE_PADDING + EDGE_CLEARANCE;
        let straddle_extent = HANDLE_SIZE + HANDLE_PADDING * 2.;

        window.with_element_state(id.unwrap(), |state, window| {
            let state: SharedHandleState = state.unwrap_or_default();

            let bg_color = handle_color(&cx.theme(), state.get().is_active());

            let mut el = div()
                .id(self.id.clone())
                .occlude()
                .absolute()
                .flex_shrink_0()
                .group("handle")
                .when_some(self.on_drag.clone(), |this, on_drag| {
                    this.on_drag(
                        self.drag_value.clone().unwrap(),
                        move |_, position, window, cx| on_drag(&position, window, cx),
                    )
                })
                .map(|this| match (self.edge, axis) {
                    // Hugging an edge: the whole band is inside the container,
                    // and the hairline sits a pixel clear of the boundary.
                    // FIXME: Improve this to let the scroll bar have px(HANDLE_PADDING)
                    (Some(HandleEdge::Trailing), Axis::Horizontal) => this
                        .cursor_col_resize()
                        .top_0()
                        .right_0()
                        .h_full()
                        .w(hug_extent)
                        .pl(HANDLE_PADDING)
                        .pr(EDGE_CLEARANCE),
                    (Some(HandleEdge::Leading), Axis::Horizontal) => this
                        .cursor_col_resize()
                        .top_0()
                        .left_0()
                        .h_full()
                        .w(hug_extent)
                        .pr(HANDLE_PADDING)
                        .pl(EDGE_CLEARANCE),
                    (Some(HandleEdge::Trailing), Axis::Vertical) => this
                        .cursor_row_resize()
                        .bottom_0()
                        .left_0()
                        .w_full()
                        .h(hug_extent)
                        .pt(HANDLE_PADDING)
                        .pb(EDGE_CLEARANCE),
                    (Some(HandleEdge::Leading), Axis::Vertical) => this
                        .cursor_row_resize()
                        .top_0()
                        .left_0()
                        .w_full()
                        .h(hug_extent)
                        .pb(HANDLE_PADDING)
                        .pt(EDGE_CLEARANCE),
                    // Straddling the boundary: half the band on either side.
                    (None, Axis::Horizontal) => this
                        .cursor_col_resize()
                        .top_0()
                        .left(neg_offset)
                        .h_full()
                        .w(straddle_extent)
                        .px(HANDLE_PADDING),
                    (None, Axis::Vertical) => this
                        .cursor_row_resize()
                        .top(neg_offset)
                        .left_0()
                        .w_full()
                        .h(straddle_extent)
                        .py(HANDLE_PADDING),
                })
                .child(
                    // A renderer that declines — or is absent — leaves the
                    // built-in line, so overriding one handle never obliges a
                    // caller to redraw them all.
                    self.appearance
                        .as_ref()
                        .and_then(|appearance| {
                            appearance(
                                &ResizeHandleContext {
                                    axis,
                                    state: state.get(),
                                },
                                window,
                                cx,
                            )
                        })
                        .unwrap_or_else(|| {
                            div()
                                // The handle's border box is HANDLE_SIZE wide but
                                // padded by HANDLE_PADDING, so its content area is
                                // zero and a shrinkable child collapses with it.
                                .flex_none()
                                .bg(bg_color)
                                .group_hover("handle", |this| this.bg(bg_color))
                                .when(axis.is_horizontal(), |this| this.h_full().w(HANDLE_SIZE))
                                .when(axis.is_vertical(), |this| this.w_full().h(HANDLE_SIZE))
                                .into_any_element()
                        }),
                )
                .into_any_element();

            let layout_id = el.request_layout(window, cx);

            ((layout_id, el), state)
        })
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        request_layout.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: gpui::Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        request_layout.paint(window, cx);

        window.with_element_state(id.unwrap(), |state: Option<SharedHandleState>, window| {
            let state = state.unwrap_or_default();

            window.on_mouse_event({
                let state = state.clone();
                move |ev: &MouseDownEvent, phase, window, _| {
                    if bounds.contains(&ev.position)
                        && phase.bubble()
                        && state.set(ResizeHandleState::Pressed)
                    {
                        window.refresh();
                    }
                }
            });

            window.on_mouse_event({
                let state = state.clone();
                move |ev: &MouseMoveEvent, phase, window, _| {
                    if !phase.bubble() {
                        return;
                    }

                    // A press that moves is a drag, and stays one until the
                    // button comes back up: by the second frame the pointer is
                    // outside this nine-pixel band, so where it is says nothing
                    // about whether the handle is still being dragged.
                    let next = match state.get() {
                        engaged if engaged.is_active() => ResizeHandleState::Dragging,
                        _ if bounds.contains(&ev.position) => ResizeHandleState::Hovered,
                        _ => ResizeHandleState::Idle,
                    };
                    if state.set(next) {
                        window.refresh();
                    }
                }
            });

            window.on_mouse_event({
                let state = state.clone();
                move |ev: &MouseUpEvent, _, window, _| {
                    if !state.get().is_active() {
                        return;
                    }

                    // Releasing over the handle leaves it hovered. Going
                    // straight to idle there would drop the indicator for one
                    // frame and bring it back under a pointer that never left.
                    let next = if bounds.contains(&ev.position) {
                        ResizeHandleState::Hovered
                    } else {
                        ResizeHandleState::Idle
                    };
                    if state.set(next) {
                        window.refresh();
                    }
                }
            });

            ((), state)
        });
    }
}

/// What a resize handle paints, given the active theme.
///
/// Projected colors win; without them the handle resolves from the tokens that
/// already mean these two states everywhere else -- `border` for a divider at
/// rest, `ring` for the thing the pointer currently owns. Before this the
/// unprojected answer was `Hsla::default()`, which is transparent, so a
/// consumer with no styled façade had no divider at all.
pub(crate) fn handle_color(theme: &crate::Theme, active: bool) -> gpui::Hsla {
    if active {
        theme
            .resizable
            .active_handle
            .unwrap_or(theme.tokens.colors.ring)
    } else {
        theme.resizable.handle.unwrap_or(theme.tokens.colors.border)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{TestAppContext, hsla};

    use super::{ResizeHandleState, SharedHandleState, handle_color};
    use crate::{ResizableTheme, Theme};

    #[test]
    fn a_listener_writes_its_progress_back_into_the_stored_state() {
        let stored = SharedHandleState::default();
        // What `paint` hands each mouse listener.
        let listener = stored.clone();

        assert!(listener.set(ResizeHandleState::Pressed));

        // The regression this pins down: the state used to be a bare `Cell`,
        // which clones by value, so a listener wrote into a copy that died
        // with the event and the handle never left `Idle`.
        assert_eq!(stored.get(), ResizeHandleState::Pressed);
        assert!(stored.get().is_active());
    }

    #[test]
    fn setting_the_state_it_already_has_asks_for_no_repaint() {
        let state = SharedHandleState::default();

        assert!(state.set(ResizeHandleState::Hovered));
        assert!(!state.set(ResizeHandleState::Hovered));
    }

    #[test]
    fn only_a_held_handle_is_active() {
        assert!(!ResizeHandleState::Idle.is_active());
        assert!(!ResizeHandleState::Hovered.is_active());
        assert!(ResizeHandleState::Pressed.is_active());
        assert!(ResizeHandleState::Dragging.is_active());
    }

    #[gpui::test]
    fn an_unprojected_handle_resolves_from_the_theme_tokens(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let border = hsla(0., 0., 0.5, 1.0);
            let ring = hsla(0.6, 0.5, 0.5, 1.0);
            let theme = Theme::global_mut(cx);
            theme.tokens.colors.border = border;
            theme.tokens.colors.ring = ring;
            theme.resizable = ResizableTheme::default();

            let theme = Theme::global(cx);
            assert_eq!(handle_color(&theme, false), border);
            assert_eq!(handle_color(&theme, true), ring);
            // The point of the change: the default used to be transparent, so
            // a divider with nothing projected onto it was not drawn at all.
            assert_ne!(handle_color(&theme, false), gpui::Hsla::default());
        });
    }

    #[gpui::test]
    fn a_projected_handle_still_wins(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let projected = hsla(0.3, 0.4, 0.5, 1.0);
            let active = hsla(0.9, 0.4, 0.5, 1.0);
            let theme = Theme::global_mut(cx);
            theme.tokens.colors.border = hsla(0., 0., 0.5, 1.0);
            theme.resizable = ResizableTheme {
                handle: Some(projected),
                active_handle: Some(active),
            };

            let theme = Theme::global(cx);
            assert_eq!(handle_color(&theme, false), projected);
            assert_eq!(handle_color(&theme, true), active);
        });
    }
}
