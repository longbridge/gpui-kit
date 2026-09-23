use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, App, Axis, Element, ElementId, Entity, GlobalElementId, Hitbox, HitboxBehavior,
    InteractiveElement, IntoElement, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement as _, Pixels, Point, Render, StatefulInteractiveElement, Styled as _, Window,
    deferred, div, prelude::FluentBuilder as _, px,
};

use crate::{AxisExt as _, theme::ActiveTheme as _};

pub(crate) const HANDLE_PADDING: Pixels = px(4.);
pub(crate) const HANDLE_SIZE: Pixels = px(1.);

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
/// never actually been grabbable. Naming the edge moves the whole band inside.
///
/// The hairline stays on the boundary itself: it is the container's outermost
/// pixel, the one the neighbour's content butts up against. Moving it inward
/// by even a pixel leaves that pixel of the container showing past the line
/// on one side, or a gap before it on the other, along the whole seam. What a
/// renderer paints on top of the line -- an indicator thicker than the line,
/// centred on it -- overhangs the boundary, and is drawn deferred so the
/// container's clip does not take the outer half off it.
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
    /// The band's own hitbox, so the listeners in `paint` can ask whether the
    /// pointer is really on the handle rather than merely within its bounds.
    type PrepaintState = Hitbox;

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
        let edge = self.edge;
        // Sizes are border-box: the extent has to name the whole band, padding
        // included, or the content box resolves to zero and the hairline
        // overflows into the padding, landing wherever the padding happens to
        // push it.
        let hug_extent = HANDLE_SIZE + HANDLE_PADDING;
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
                .map(|this| match (edge, axis) {
                    // Hugging an edge: the whole band is inside the container,
                    // padded on the inner side only, so the hairline is the
                    // container's outermost pixel -- the seam itself.
                    // FIXME: Improve this to let the scroll bar have px(HANDLE_PADDING)
                    (Some(HandleEdge::Trailing), Axis::Horizontal) => this
                        .cursor_col_resize()
                        .top_0()
                        .right_0()
                        .h_full()
                        .w(hug_extent)
                        .pl(HANDLE_PADDING),
                    (Some(HandleEdge::Leading), Axis::Horizontal) => this
                        .cursor_col_resize()
                        .top_0()
                        .left_0()
                        .h_full()
                        .w(hug_extent)
                        .pr(HANDLE_PADDING),
                    (Some(HandleEdge::Trailing), Axis::Vertical) => this
                        .cursor_row_resize()
                        .bottom_0()
                        .left_0()
                        .w_full()
                        .h(hug_extent)
                        .pt(HANDLE_PADDING),
                    (Some(HandleEdge::Leading), Axis::Vertical) => this
                        .cursor_row_resize()
                        .top_0()
                        .left_0()
                        .w_full()
                        .h(hug_extent)
                        .pb(HANDLE_PADDING),
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
                .child({
                    // A renderer that declines — or is absent — leaves the
                    // built-in line, so overriding one handle never obliges a
                    // caller to redraw them all.
                    let painted = self.appearance.as_ref().and_then(|appearance| {
                        appearance(
                            &ResizeHandleContext {
                                axis,
                                state: state.get(),
                            },
                            window,
                            cx,
                        )
                    });
                    match painted {
                        // A hugging handle's hairline is its container's
                        // outermost pixel, so anything a renderer centres on
                        // it overhangs the container, and the container clips.
                        // Deferring the appearance keeps its layout here and
                        // paints it after the tree, under the window's own
                        // mask, so the overhang survives. Nothing else moves:
                        // the appearance carries no hitbox, dialogs, popups
                        // and tooltips are deferred at higher priorities and
                        // still paint over it, and an overlay that is not --
                        // a sheet, a toast -- occludes the handle, which then
                        // never engages and has nothing to paint.
                        Some(painted) if edge.is_some() => deferred(painted).into_any_element(),
                        Some(painted) => painted,
                        None => div()
                            // The line fills the handle's content box exactly,
                            // so it has nothing to give: a shrinkable child
                            // collapses with it.
                            .flex_none()
                            .bg(bg_color)
                            .group_hover("handle", |this| this.bg(bg_color))
                            .when(axis.is_horizontal(), |this| this.h_full().w(HANDLE_SIZE))
                            .when(axis.is_vertical(), |this| this.w_full().h(HANDLE_SIZE))
                            .into_any_element(),
                    }
                })
                .into_any_element();

            let layout_id = el.request_layout(window, cx);

            ((layout_id, el), state)
        })
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: gpui::Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        request_layout.prepaint(window, cx);
        // After the child, deliberately. The band's own div occludes, and a
        // hit test stops at the first hitbox that does, so a hitbox inserted
        // before it would sit underneath and never read as hovered. Inserted
        // here it sits directly on top of the band and directly under whatever
        // is painted after this handle -- a sheet's overlay, a toast -- which
        // is exactly the ordering `is_hovered_at` should answer from.
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        request_layout.paint(window, cx);

        // Hovered and pressed are answered by the hitbox, not by the bounds.
        // Bounds containment reads true through anything painted over the
        // handle, so a divider under a sheet lit up as the pointer crossed
        // where it lay; the hitbox is occluded by that sheet and says no.
        let hitbox = hitbox.clone();

        window.with_element_state(id.unwrap(), |state: Option<SharedHandleState>, window| {
            let state = state.unwrap_or_default();

            window.on_mouse_event({
                let state = state.clone();
                let hitbox = hitbox.clone();
                move |ev: &MouseDownEvent, phase, window, _| {
                    if phase.bubble()
                        && hitbox.is_hovered_at(ev.position, window)
                        && state.set(ResizeHandleState::Pressed)
                    {
                        window.refresh();
                    }
                }
            });

            window.on_mouse_event({
                let state = state.clone();
                let hitbox = hitbox.clone();
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
                        _ if hitbox.is_hovered_at(ev.position, window) => {
                            ResizeHandleState::Hovered
                        }
                        _ => ResizeHandleState::Idle,
                    };
                    if state.set(next) {
                        window.refresh();
                    }
                }
            });

            window.on_mouse_event({
                let state = state.clone();
                let hitbox = hitbox.clone();
                move |ev: &MouseUpEvent, _, window, _| {
                    if !state.get().is_active() {
                        return;
                    }

                    // Releasing over the handle leaves it hovered. Going
                    // straight to idle there would drop the indicator for one
                    // frame and bring it back under a pointer that never left.
                    let next = if hitbox.is_hovered_at(ev.position, window) {
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
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        AnyElement, App, Axis, Bounds, Context, Empty, IntoElement, ParentElement as _, Pixels,
        Render, Styled as _, TestAppContext, Window, div, hsla, prelude::FluentBuilder as _, px,
    };

    use super::{
        HandleEdge, ResizeHandleContext, ResizeHandleState, SharedHandleState, handle_color,
        resize_handle,
    };
    use crate::{ElementExt as _, ResizableTheme, Theme};

    /// What a hugging handle's renderer drew, and under which mask.
    #[derive(Default)]
    struct Probe {
        line: Cell<Option<Bounds<Pixels>>>,
        indicator: Cell<Option<Bounds<Pixels>>>,
        mask: Cell<Option<Bounds<Pixels>>>,
    }

    /// The shape of a styled divider: a hairline filling the handle's content
    /// box, with a three-pixel indicator centred across it -- so it overhangs
    /// the line by one pixel on either side, the way `gpui-component` draws it.
    fn styled_divider(axis: Axis, probe: Rc<Probe>) -> AnyElement {
        let line_probe = probe.clone();
        div()
            .flex_none()
            .flex()
            .map(|line| match axis {
                Axis::Horizontal => line.w(px(1.)).h_full().items_center(),
                Axis::Vertical => line.h(px(1.)).w_full().items_start().justify_center(),
            })
            .on_prepaint(move |bounds, _, _| line_probe.line.set(Some(bounds)))
            .child(
                div()
                    .flex_none()
                    .map(|pill| match axis {
                        Axis::Horizontal => pill.w(px(3.)).h(px(20.)).ml(px(-1.)),
                        Axis::Vertical => pill.h(px(3.)).w(px(20.)).mt(px(-1.)),
                    })
                    .on_prepaint(move |bounds, window, _| {
                        probe.indicator.set(Some(bounds));
                        probe.mask.set(Some(window.content_mask().bounds));
                    }),
            )
            .into_any_element()
    }

    /// A drag payload for a handle nobody drags in these tests.
    struct NoDrag;

    impl Render for NoDrag {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Empty
        }
    }

    /// A dock-shaped box: 200px along the axis, clipped to itself the way
    /// `dock_frame` is, sitting between two 100px neighbours so both of its
    /// edges are seams. The handle hugs one of them.
    struct HuggingHarness {
        axis: Axis,
        edge: HandleEdge,
        probe: Rc<Probe>,
    }

    impl Render for HuggingHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let axis = self.axis;
            let probe = self.probe.clone();
            let neighbour = || match axis {
                Axis::Horizontal => div().w(px(100.)).h_full(),
                Axis::Vertical => div().h(px(100.)).w_full(),
            };
            div()
                .flex()
                .map(|row| match axis {
                    Axis::Horizontal => row.flex_row().w(px(400.)).h(px(100.)),
                    Axis::Vertical => row.flex_col().h(px(400.)).w(px(100.)),
                })
                .child(neighbour())
                .child(
                    div()
                        .relative()
                        .overflow_hidden()
                        .map(|dock| match axis {
                            Axis::Horizontal => dock.w(px(200.)).h_full(),
                            Axis::Vertical => dock.h(px(200.)).w_full(),
                        })
                        .child(
                            resize_handle::<(), NoDrag>("hugging", axis)
                                .inside(self.edge)
                                .with_appearance(Rc::new(
                                    move |handle: &ResizeHandleContext,
                                          _: &mut Window,
                                          _: &mut App| {
                                        Some(styled_divider(handle.axis(), probe.clone()))
                                    },
                                )),
                        ),
                )
                .child(neighbour())
        }
    }

    fn draw_hugging(cx: &mut TestAppContext, axis: Axis, edge: HandleEdge) -> Rc<Probe> {
        let probe = Rc::new(Probe::default());
        let (_, cx) = cx.add_window_view({
            let probe = probe.clone();
            move |_, _| HuggingHarness { axis, edge, probe }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        probe
    }

    /// The seam a hugging handle marks, along its axis.
    ///
    /// The dock spans 100..300 in the harness, so its leading seam is at 100
    /// and its trailing one at 300.
    fn seam(edge: HandleEdge) -> Pixels {
        match edge {
            HandleEdge::Leading => px(100.),
            HandleEdge::Trailing => px(300.),
        }
    }

    fn along(axis: Axis, bounds: Bounds<Pixels>) -> (Pixels, Pixels) {
        match axis {
            Axis::Horizontal => (bounds.left(), bounds.right()),
            Axis::Vertical => (bounds.top(), bounds.bottom()),
        }
    }

    /// A hugging handle's hairline is the container's outermost pixel.
    ///
    /// This is the regression. The line was set one pixel in from the
    /// boundary, to make room for an indicator to overhang it, and along the
    /// whole seam that pixel of the dock showed past the line on one side of
    /// the area and opened a gap before it on the other.
    #[gpui::test]
    fn a_hugging_handle_draws_its_line_on_the_seam(cx: &mut TestAppContext) {
        for axis in [Axis::Horizontal, Axis::Vertical] {
            for edge in [HandleEdge::Leading, HandleEdge::Trailing] {
                let probe = draw_hugging(cx, axis, edge);
                let line = probe.line.get().expect("the renderer was asked to draw");
                let (start, end) = along(axis, line);
                let expected = match edge {
                    HandleEdge::Leading => (seam(edge), seam(edge) + px(1.)),
                    HandleEdge::Trailing => (seam(edge) - px(1.), seam(edge)),
                };
                assert_eq!(
                    (start, end),
                    expected,
                    "{axis:?} {edge:?}: the hairline has to be the pixel against the seam"
                );
            }
        }
    }

    /// What a renderer centres on a hugging handle's line survives the
    /// container's clip.
    ///
    /// Centred on the outermost pixel, the indicator overhangs the container
    /// by a pixel, and `overflow_hidden` would take that pixel off it. The
    /// appearance is drawn deferred so the mask it is painted under is the
    /// window's, not the container's.
    #[gpui::test]
    fn a_hugging_handle_paints_its_indicator_unclipped(cx: &mut TestAppContext) {
        for axis in [Axis::Horizontal, Axis::Vertical] {
            for edge in [HandleEdge::Leading, HandleEdge::Trailing] {
                let probe = draw_hugging(cx, axis, edge);
                let line = probe.line.get().expect("the renderer was asked to draw");
                let indicator = probe.indicator.get().expect("the indicator was laid out");
                let mask = probe.mask.get().expect("the indicator was prepainted");

                let (line_start, line_end) = along(axis, line);
                assert_eq!(
                    along(axis, indicator),
                    (line_start - px(1.), line_end + px(1.)),
                    "{axis:?} {edge:?}: the indicator stays centred on the hairline"
                );
                assert_eq!(
                    mask.intersect(&indicator),
                    indicator,
                    "{axis:?} {edge:?}: the indicator {indicator:?} is clipped by {mask:?}"
                );
            }
        }
    }

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
