//! Boundary displacement only: the list keeps its clamped logical position.

use crate::{OngoingScrollExt as _, ScrollbarHandle};
use gpui::{
    AnyElement, App, Bounds, ContentMask, DispatchPhase, Element, ElementId, GlobalElementId,
    Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId, OngoingScroll, Pixels,
    ScrollDelta, ScrollWheelEvent, TouchPhase, Window, point, px,
};
#[cfg(not(target_family = "wasm"))]
use std::time::Instant;
use std::{cell::RefCell, rc::Rc, time::Duration};
#[cfg(target_family = "wasm")]
use web_time::Instant;

/// Motion tokens for [`ScrollBounce`]: how far a drag stretches the viewport
/// past an edge, and how quickly a released edge returns.
///
/// Base plays the stretch and the return; the feel belongs to the caller.
/// The default is tuned to feel like a `UIScrollView` bounce.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollBounceMotion {
    tracking: f32,
    response: Duration,
}

impl Default for ScrollBounceMotion {
    /// Tuned to feel like a `UIScrollView` bounce; these are not UIKit constants.
    fn default() -> Self {
        Self {
            tracking: 0.55,
            response: Duration::from_millis(524),
        }
    }
}

impl ScrollBounceMotion {
    /// Fraction of finger travel the stretched edge follows at first.
    ///
    /// The edge follows less and less as it approaches the viewport height,
    /// which it never reaches. Tracking is independent of the return, so
    /// slowing the return does not change how the finger feels.
    ///
    /// # Panics
    ///
    /// Panics when `tracking` is not finite or not positive.
    pub fn with_tracking(mut self, tracking: f32) -> Self {
        assert!(
            tracking.is_finite() && tracking > 0.,
            "scroll bounce tracking must be finite and positive"
        );
        self.tracking = tracking;
        self
    }

    /// Time scale of the return once the finger lifts.
    ///
    /// Read the way [`crate::Spring::new`] reads its response: the period one
    /// full oscillation would take without damping, which is the scale the
    /// return is felt at rather than the moment it stops. The return is
    /// critically damped, so it never crosses the edge. A zero response snaps
    /// the edge back on the spot.
    pub fn with_response(mut self, response: Duration) -> Self {
        self.response = response;
        self
    }

    /// Fraction of finger travel the stretched edge follows at first.
    pub fn tracking(&self) -> f32 {
        self.tracking
    }

    /// Time scale of the return once the finger lifts.
    pub fn response(&self) -> Duration {
        self.response
    }

    /// Undamped angular frequency of the return, or `None` when it snaps.
    fn omega(&self) -> Option<f32> {
        let seconds = self.response.as_secs_f32();
        (seconds > 0.).then(|| std::f32::consts::TAU / seconds)
    }
}

/// Adds vertical touch overscroll to an existing scroll viewport.
///
/// The child owns layout, content, and ordinary scrolling; `handle` must be the
/// child's scroll handle. Only unused vertical deltas stretch the viewport.
/// The stable `id` owns gesture and spring state. Change it when replacing the
/// document. Put fixed chrome (scrollbars, toolbars) outside this wrapper.
///
/// Enabled by default on iOS and Android. Other platforms pass through unless
/// explicitly enabled; their input must emit `Ended` at finger release, before momentum.
/// Reduced motion disables displacement. Keyboard, focus, and line-wheel input
/// remain owned by the child. No colors, padding, or dimensions are imposed.
pub struct ScrollBounce {
    id: ElementId,
    handle: Rc<dyn ScrollbarHandle>,
    child: AnyElement,
    enabled: bool,
    motion: ScrollBounceMotion,
    on_scroll: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl ScrollBounce {
    pub fn new<H: ScrollbarHandle + Clone>(
        id: impl Into<ElementId>,
        handle: &H,
        child: impl IntoElement,
    ) -> Self {
        Self {
            id: id.into(),
            handle: Rc::new(handle.clone()),
            child: child.into_any_element(),
            enabled: cfg!(any(target_os = "ios", target_os = "android")),
            motion: ScrollBounceMotion::default(),
            on_scroll: None,
        }
    }

    /// Opt in on a platform with compatible touch phase semantics.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set how far a drag stretches past an edge and how the edge returns.
    pub fn motion(mut self, motion: ScrollBounceMotion) -> Self {
        self.motion = motion;
        self
    }

    /// Observe a logical scroll performed when a reverse drag leaves the stretched
    /// region. Runs after the handle update, with no internal state borrowed.
    /// Ordinary child scrolling continues to use the child's own notifications.
    pub fn on_scroll(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_scroll = Some(Rc::new(handler));
        self
    }
}

#[derive(Default)]
struct State {
    physics: Physics,
    sampled_at: Option<Instant>,
    ongoing_scroll: OngoingScroll,
}

/// `ScrollbarHandle` has no `max_offset`; recover it from the definition
/// `content_size = viewport + max_offset`. Both dispatch phases clamp against
/// this bound and must agree on it.
fn max_scroll_extent(handle: &dyn ScrollbarHandle) -> Pixels {
    (handle.content_size().height - handle.viewport_bounds().size.height).max(px(0.))
}

#[doc(hidden)]
pub struct ScrollBouncePrepaintState {
    state: Rc<RefCell<State>>,
    hitbox: Hitbox,
}

impl IntoElement for ScrollBounce {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for ScrollBounce {
    type RequestLayoutState = ();
    type PrepaintState = ScrollBouncePrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let state = window.with_element_state(
            id.expect("ScrollBounce has an id"),
            |state: Option<Rc<RefCell<State>>>, _| {
                let state = state.unwrap_or_default();
                (state.clone(), state)
            },
        );
        let offset = {
            let mut state = state.borrow_mut();
            if !self.enabled || cx.reduce_motion() {
                *state = State::default();
            }
            state.physics.motion = self.motion;
            let now = Instant::now();
            let elapsed = state
                .sampled_at
                .replace(now)
                .map_or(0., |at| now.duration_since(at).as_secs_f32());
            if state.physics.step(elapsed) {
                window.request_animation_frame();
            }
            state.physics.offset()
        };
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            window.with_element_offset(point(px(0.), px(offset)), |window| {
                self.child.prepaint(window, cx);
            });
        });
        ScrollBouncePrepaintState { state, hitbox }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.enabled && !cx.reduce_motion() {
            let state = prepaint.state.clone();
            let hitbox = prepaint.hitbox.id;
            let handle = self.handle.clone();
            let view = window.current_view();
            let on_scroll = self.on_scroll.clone();
            let mut before = 0.;
            window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
                let ScrollDelta::Pixels(mut delta) = event.delta else {
                    return;
                };
                if !hitbox.should_handle_scroll(window) {
                    return;
                }
                let mut state = state.borrow_mut();
                // Lock the gesture to the axis it started on, so a diagonal
                // swipe cannot wobble out of the stretch from one packet to
                // the next. Both dispatch phases see the same packet, and the
                // lock gives both the same answer.
                state
                    .ongoing_scroll
                    .lock_axis(&mut delta, event.touch_phase);
                if delta.x.abs() > delta.y.abs() {
                    return;
                }
                let ended = matches!(event.touch_phase, TouchPhase::Ended | TouchPhase::Cancelled);
                let mut scrolled = false;
                let mut changed = false;
                if phase == DispatchPhase::Capture {
                    before = handle.offset().y.as_f32();
                    if event.touch_phase == TouchPhase::Started {
                        state.physics.begin(bounds.size.height.as_f32());
                    }
                    if state.physics.suppress_momentum {
                        cx.stop_propagation();
                        return;
                    }
                    if state.physics.offset() != 0. {
                        let remainder = state.physics.pull(delta.y.as_f32());
                        if remainder != 0. {
                            let max = max_scroll_extent(handle.as_ref());
                            let mut offset = handle.offset();
                            offset.y = px(before + remainder).clamp(-max, px(0.));
                            handle.set_offset(offset);
                            scrolled = true;
                        }
                        if ended {
                            state.physics.release();
                        }
                        changed = true;
                        cx.stop_propagation();
                    } else if ended {
                        state.physics.release();
                    }
                } else {
                    // Div applies deltas immediately but clamps during its next
                    // prepaint. Clamp here so that boundary deltas are not
                    // mistaken for consumed scrolling (ListState clamps eagerly).
                    let mut offset = handle.offset();
                    let max = max_scroll_extent(handle.as_ref());
                    let clamped = offset.y.clamp(-max, px(0.));
                    if clamped != offset.y {
                        offset.y = clamped;
                        handle.set_offset(offset);
                    }
                    let after = offset.y.as_f32();
                    let requested = delta.y.as_f32();
                    // A List can coalesce several packets against one painted
                    // scroll position. Their offset difference alone does not
                    // prove overscroll, especially after direction changes or
                    // a zero-delta Ended packet from a trackpad.
                    let at_outward_edge = (requested > 0. && offset.y == px(0.))
                        || (requested < 0. && offset.y == -max);
                    let residual =
                        (requested - (after - before)).clamp(requested.min(0.), requested.max(0.));
                    if at_outward_edge && residual.abs() > 0.01 && !state.physics.suppress_momentum
                    {
                        let dragging = state.physics.dragging;
                        if !dragging {
                            state.physics.begin(bounds.size.height.as_f32());
                        }
                        state.physics.pull(residual);
                        if !dragging || ended {
                            state.physics.release();
                        }
                        changed = true;
                    }
                }
                if changed {
                    state.sampled_at = Some(Instant::now());
                }
                drop(state);
                if changed {
                    cx.notify(view);
                }
                if scrolled && let Some(handler) = &on_scroll {
                    handler(window, cx);
                }
            });
        }
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            self.child.paint(window, cx)
        });
    }
}

#[derive(Default)]
struct Physics {
    position: f32,
    velocity: f32,
    dragging: bool,
    suppress_momentum: bool,
    extent: f32,
    motion: ScrollBounceMotion,
}

impl Physics {
    fn offset(&self) -> f32 {
        if self.dragging {
            let d = self.extent.max(1.);
            let tracking = self.motion.tracking;
            self.position * tracking / (1. + tracking * self.position.abs() / d)
        } else {
            self.position
        }
    }

    fn begin(&mut self, extent: f32) {
        let offset = self.offset();
        // A displaced edge keeps the extent it was stretched under. The
        // rubber-band curve saturates at the extent, so re-reading a viewport
        // that shrank mid-return (rotation, keyboard) could not place the
        // finger where the edge is: it would snap, then need a long pull back.
        if offset == 0. {
            self.extent = extent.max(1.);
        }
        // Invert the rubber-band curve so grabbing a returning edge is continuous.
        let tracking = self.motion.tracking;
        self.position = offset / (tracking * (1. - offset.abs() / self.extent).max(0.01));
        self.velocity = 0.;
        self.dragging = true;
        self.suppress_momentum = false;
    }

    /// Apply finger displacement. Return the part that crosses back into the list.
    fn pull(&mut self, delta: f32) -> f32 {
        let previous = self.position;
        let next = previous + delta;
        if previous != 0. && previous.signum() != next.signum() {
            self.position = 0.;
            next
        } else {
            self.position = next;
            0.
        }
    }

    fn release(&mut self) {
        self.position = self.offset();
        self.dragging = false;
        if self.position != 0. {
            self.suppress_momentum = true;
        }
    }

    /// Exact critically damped spring integration, independent of refresh rate.
    fn step(&mut self, seconds: f32) -> bool {
        if self.dragging || self.position == 0. {
            return false;
        }
        let Some(omega) = self.motion.omega() else {
            self.position = 0.;
            self.velocity = 0.;
            return false;
        };
        let decay = (-omega * seconds).exp();
        let c = self.velocity + omega * self.position;
        self.position = (self.position + c * seconds) * decay;
        self.velocity = (self.velocity - omega * c * seconds) * decay;
        if self.position.abs() < 0.1 && self.velocity.abs() < 1. {
            self.position = 0.;
            self.velocity = 0.;
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Context, InteractiveElement as _, ParentElement as _, Render, ScrollHandle,
        StatefulInteractiveElement as _, Styled as _, TestAppContext, VisualTestContext, div,
    };

    struct ScrollTest {
        handle: ScrollHandle,
        enabled: bool,
    }

    impl Render for ScrollTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().p(px(20.)).child(
                ScrollBounce::new(
                    "bounce",
                    &self.handle,
                    div()
                        .id("viewport")
                        .w(px(200.))
                        .h(px(200.))
                        .overflow_y_scroll()
                        .track_scroll(&self.handle)
                        .child(div().h(px(600.)).w_full()),
                )
                .enabled(self.enabled),
            )
        }
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn scroll(cx: &mut VisualTestContext, delta: f32, phase: TouchPhase) {
        cx.simulate_event(ScrollWheelEvent {
            position: point(px(100.), px(100.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(delta))),
            touch_phase: phase,
            ..Default::default()
        });
    }

    struct ListTest(gpui::ListState);

    impl Render for ListTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            ScrollBounce::new(
                "bounce-list",
                &self.0,
                gpui::list(self.0.clone(), |_, _, _| {
                    div().h(px(40.)).into_any_element()
                })
                .w(px(200.))
                .h(px(200.)),
            )
            .enabled(true)
        }
    }

    #[gpui::test]
    fn list_reverse_drag_preserves_events_before_the_next_frame(cx: &mut TestAppContext) {
        let handle = gpui::ListState::new(30, gpui::ListAlignment::Top, px(0.)).measure_all();
        let (_, cx) = cx.add_window_view({
            let handle = handle.clone();
            move |_, _| ListTest(handle)
        });
        draw(cx);
        scroll(cx, 100., TouchPhase::Started);
        scroll(cx, -140., TouchPhase::Moved);
        assert_eq!(handle.offset().y, px(-40.));
        scroll(cx, -10., TouchPhase::Moved);
        draw(cx);
        assert_eq!(handle.offset().y, px(-50.));
    }

    #[gpui::test]
    fn trackpad_release_between_frames_does_not_bounce_in_the_middle(cx: &mut TestAppContext) {
        let handle = gpui::ListState::new(30, gpui::ListAlignment::Top, px(0.)).measure_all();
        let (_, cx) = cx.add_window_view({
            let handle = handle.clone();
            move |_, _| ListTest(handle)
        });
        draw(cx);
        handle.set_offset(point(px(0.), px(-400.)));
        draw(cx);
        let origin = handle.viewport_bounds().origin.y;
        // Unlike simulate_event (which may draw after each event), dispatch
        // both packets within one update to exercise native input coalescing.
        cx.update(|window, cx| {
            for (delta, phase) in [(-30., TouchPhase::Started), (0., TouchPhase::Ended)] {
                window.dispatch_event(
                    gpui::PlatformInput::ScrollWheel(ScrollWheelEvent {
                        position: point(px(100.), px(100.)),
                        delta: ScrollDelta::Pixels(point(px(0.), px(delta))),
                        touch_phase: phase,
                        ..Default::default()
                    }),
                    cx,
                );
            }
        });
        draw(cx);
        assert_eq!(handle.viewport_bounds().origin.y, origin);
        assert!(handle.offset().y < px(-300.) && handle.offset().y > px(-500.));
    }

    #[gpui::test]
    fn trackpad_direction_change_between_frames_does_not_bounce_in_the_middle(
        cx: &mut TestAppContext,
    ) {
        let handle = gpui::ListState::new(30, gpui::ListAlignment::Top, px(0.)).measure_all();
        let (_, cx) = cx.add_window_view({
            let handle = handle.clone();
            move |_, _| ListTest(handle)
        });
        draw(cx);
        handle.set_offset(point(px(0.), px(-400.)));
        draw(cx);
        let origin = handle.viewport_bounds().origin.y;
        cx.update(|window, cx| {
            for (delta, phase) in [(-30., TouchPhase::Started), (5., TouchPhase::Moved)] {
                window.dispatch_event(
                    gpui::PlatformInput::ScrollWheel(ScrollWheelEvent {
                        position: point(px(100.), px(100.)),
                        delta: ScrollDelta::Pixels(point(px(0.), px(delta))),
                        touch_phase: phase,
                        ..Default::default()
                    }),
                    cx,
                );
            }
        });
        draw(cx);
        assert!(handle.offset().y < px(-300.) && handle.offset().y > px(-500.));
        assert_eq!(handle.viewport_bounds().origin.y, origin);
    }

    #[gpui::test]
    fn list_stretches_only_the_distance_past_either_edge(cx: &mut TestAppContext) {
        for (start, delta, end) in [(-30., 50., 0.), (-970., -50., -1000.)] {
            let mut app = cx.new_app();
            let handle = gpui::ListState::new(30, gpui::ListAlignment::Top, px(0.)).measure_all();
            let (_, cx) = app.add_window_view({
                let handle = handle.clone();
                move |_, _| ListTest(handle)
            });
            draw(cx);
            handle.set_offset(point(px(0.), px(start)));
            draw(cx);
            let origin = handle.viewport_bounds().origin.y;
            scroll(cx, delta, TouchPhase::Started);
            draw(cx);
            assert_eq!(handle.offset().y, px(end));
            let stretch = (handle.viewport_bounds().origin.y - origin).as_f32();
            assert_eq!(stretch.signum(), delta.signum());
            // Of the 50 px input, 30 px is ordinary scrolling. Only the
            // remaining 20 px may be rubber-banded (resistance reduces it).
            assert!(stretch.abs() > 0. && stretch.abs() < 20.);
        }
    }

    #[gpui::test]
    fn reverse_drag_consumes_stretch_before_scrolling_content(cx: &mut TestAppContext) {
        let handle = ScrollHandle::new();
        let (_, cx) = cx.add_window_view({
            let handle = handle.clone();
            move |_, _| ScrollTest {
                handle,
                enabled: true,
            }
        });
        draw(cx);
        let origin = handle.bounds().origin.y;
        assert_eq!(origin, px(20.));
        scroll(cx, 100., TouchPhase::Started);
        draw(cx);
        assert_eq!(handle.offset().y, px(0.));
        assert!(handle.bounds().origin.y > origin);
        scroll(cx, -140., TouchPhase::Moved);
        draw(cx);
        assert_eq!(handle.offset().y, px(-40.));
        assert_eq!(handle.bounds().origin.y, origin);
    }

    #[gpui::test]
    fn touch_release_ignores_momentum_until_a_new_touch_takes_over(cx: &mut TestAppContext) {
        let handle = ScrollHandle::new();
        let (_, cx) = cx.add_window_view({
            let handle = handle.clone();
            move |_, _| ScrollTest {
                handle,
                enabled: true,
            }
        });
        draw(cx);
        let origin = handle.bounds().origin.y;
        scroll(cx, 100., TouchPhase::Started);
        draw(cx);
        assert!(handle.bounds().origin.y > origin);
        scroll(cx, 0., TouchPhase::Ended);
        draw(cx);
        let released = handle.bounds().origin.y;
        // The iOS backend emits Moved packets for momentum after finger-up.
        // Even a large inward packet must not move the logical list while
        // the returning edge owns this gesture.
        scroll(cx, -400., TouchPhase::Moved);
        draw(cx);
        assert_eq!(handle.offset().y, px(0.));
        assert!(handle.bounds().origin.y <= released);
        // A new finger-down must end suppression and take over immediately.
        scroll(cx, 0., TouchPhase::Started);
        scroll(cx, -250., TouchPhase::Moved);
        draw(cx);
        assert!(handle.offset().y < px(0.));
        assert_eq!(handle.bounds().origin.y, origin);
    }

    #[gpui::test]
    fn disabled_and_reduced_motion_leave_the_viewport_fixed(cx: &mut TestAppContext) {
        for enabled in [false, true] {
            let mut app = cx.new_app();
            if enabled {
                app.update(|cx| cx.set_reduce_motion(true));
            }
            let handle = ScrollHandle::new();
            let (_, cx) = app.add_window_view({
                let handle = handle.clone();
                move |_, _| ScrollTest { handle, enabled }
            });
            draw(cx);
            let origin = handle.bounds().origin.y;
            scroll(cx, 100., TouchPhase::Started);
            draw(cx);
            assert_eq!(handle.bounds().origin.y, origin);
            scroll(cx, -40., TouchPhase::Moved);
            draw(cx);
            assert_eq!(handle.offset().y, px(-40.));
        }
    }

    #[test]
    fn resistance_and_reverse_preserve_unconsumed_distance() {
        let mut scroll = Physics::default();
        scroll.begin(600.);
        assert_eq!(scroll.pull(100.), 0.);
        assert!(scroll.offset() > 0. && scroll.offset() < 55.);
        assert_eq!(scroll.pull(-130.), -30.);
        assert_eq!(scroll.offset(), 0.);
    }

    #[gpui::test]
    fn diagonal_wobble_stays_with_the_stretch(cx: &mut TestAppContext) {
        let handle = ScrollHandle::new();
        let (_, cx) = cx.add_window_view({
            let handle = handle.clone();
            move |_, _| ScrollTest {
                handle,
                enabled: true,
            }
        });
        draw(cx);
        let origin = handle.bounds().origin.y;
        scroll(cx, 100., TouchPhase::Started);
        draw(cx);
        let stretched = handle.bounds().origin.y;
        assert!(stretched > origin);
        // A trackpad swipe that started vertical wobbles horizontal-dominant
        // for a packet. Dispatch within one update so the packets stay within
        // the axis lock's gesture separation.
        cx.update(|window, cx| {
            for delta in [point(px(30.), px(-20.)), point(px(0.), px(-20.))] {
                window.dispatch_event(
                    gpui::PlatformInput::ScrollWheel(ScrollWheelEvent {
                        position: point(px(100.), px(100.)),
                        delta: ScrollDelta::Pixels(delta),
                        touch_phase: TouchPhase::Moved,
                        ..Default::default()
                    }),
                    cx,
                );
            }
        });
        draw(cx);
        // Both packets shrink the stretch; neither scrolls the list.
        assert!(handle.bounds().origin.y < stretched);
        assert!(handle.bounds().origin.y > origin);
        assert_eq!(handle.offset(), point(px(0.), px(0.)));
    }

    #[test]
    fn motion_builder_configures_tracking_and_response() {
        let motion = ScrollBounceMotion::default()
            .with_tracking(0.4)
            .with_response(Duration::from_millis(300));
        assert_eq!(motion.tracking(), 0.4);
        assert_eq!(motion.response(), Duration::from_millis(300));
    }

    #[test]
    fn tracking_scales_the_first_stretch() {
        let stretch = |tracking: f32| {
            let mut scroll = Physics {
                motion: ScrollBounceMotion::default().with_tracking(tracking),
                ..Physics::default()
            };
            scroll.begin(600.);
            scroll.pull(100.);
            scroll.offset()
        };
        assert!(stretch(0.3) < stretch(0.55));
        assert!(stretch(0.55) < stretch(0.8));
    }

    #[test]
    fn response_scales_the_return_and_zero_snaps() {
        let remaining = |response: Duration| {
            let mut scroll = Physics {
                motion: ScrollBounceMotion::default().with_response(response),
                ..Physics::default()
            };
            scroll.begin(600.);
            scroll.pull(180.);
            scroll.release();
            scroll.step(0.25);
            scroll.offset()
        };
        assert!(remaining(Duration::from_secs(1)) > remaining(Duration::from_millis(524)));
        assert!(remaining(Duration::from_millis(524)) > remaining(Duration::from_millis(200)));
        assert_eq!(remaining(Duration::ZERO), 0.);
    }

    #[test]
    fn regrabbing_a_displaced_edge_keeps_its_extent() {
        let mut scroll = Physics::default();
        scroll.begin(600.);
        scroll.pull(-150.);
        scroll.release();
        scroll.step(0.08);
        let before = scroll.offset();
        // The viewport shrank below the displacement while the edge was
        // returning. The finger still lands on the edge where it is.
        scroll.begin(40.);
        assert!((scroll.offset() - before).abs() < 0.001);
        // And a short pull inward moves the edge right away.
        scroll.pull(10.);
        assert!(scroll.offset() > before);
        assert!(scroll.offset() < before + 10.);
    }

    #[test]
    fn a_gesture_from_rest_adopts_the_current_extent() {
        let mut scroll = Physics::default();
        scroll.begin(600.);
        scroll.begin(40.);
        scroll.pull(-100.);
        // The stretch saturates below the 40 px viewport, not the 600 px one.
        assert!(scroll.offset() > -40.);
    }

    #[test]
    fn grabbing_the_spring_does_not_jump() {
        let mut scroll = Physics::default();
        scroll.begin(600.);
        scroll.pull(-150.);
        scroll.release();
        scroll.step(0.08);
        let before = scroll.offset();
        scroll.begin(600.);
        assert!((scroll.offset() - before).abs() < 0.001);
        let held = scroll.offset();
        assert!(!scroll.step(0.1));
        assert_eq!(scroll.offset(), held);
    }

    #[test]
    fn spring_has_the_same_trajectory_at_60_and_120_hz() {
        let at = |hz: usize| {
            let mut scroll = Physics::default();
            scroll.begin(600.);
            scroll.pull(180.);
            scroll.release();
            for _ in 0..hz / 4 {
                scroll.step(1. / hz as f32);
            }
            scroll.offset()
        };
        assert!((at(60) - at(120)).abs() < 0.001);
    }

    #[test]
    fn return_keeps_a_visible_tail_after_a_quarter_second() {
        let mut scroll = Physics {
            position: 100.,
            ..Physics::default()
        };
        assert!(scroll.step(0.25));
        // A 100 px release should still have a visible, decelerating tail
        // after 250 ms instead of snapping almost completely back by then.
        assert!(scroll.offset() > 10. && scroll.offset() < 30.);
        assert!(scroll.velocity < 0.);
    }

    #[test]
    fn spring_settles_without_crossing_the_boundary() {
        let mut scroll = Physics::default();
        scroll.begin(600.);
        scroll.pull(-200.);
        scroll.release();
        let mut previous = scroll.offset();
        for _ in 0..120 {
            scroll.step(1. / 120.);
            assert!(scroll.offset() >= previous && scroll.offset() <= 0.);
            previous = scroll.offset();
        }
        assert_eq!(scroll.offset(), 0.);
        assert!(scroll.suppress_momentum);
        scroll.begin(600.);
        assert!(!scroll.suppress_momentum);
    }
}
