//! Boundary displacement only: the list keeps its clamped logical position.

use crate::ScrollbarHandle;
use gpui::{
    AnyElement, App, Bounds, ContentMask, DispatchPhase, Element, ElementId, GlobalElementId,
    Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId, Pixels, ScrollDelta,
    ScrollWheelEvent, TouchPhase, Window, point, px,
};
use std::{cell::RefCell, rc::Rc};
use web_time::Instant;

/// Adds vertical touch overscroll to an existing scroll viewport.
///
/// The child owns layout, content, and ordinary scrolling; `handle` must be the
/// child's scroll handle. Only unused vertical deltas stretch the viewport.
/// The stable `id` owns gesture and spring state. Change it when replacing the
/// document. Put fixed chrome (scrollbars, toolbars) outside this wrapper.
///
/// Enabled by default on iOS. Other platforms pass through unless explicitly
/// enabled; their input must emit `Ended` at finger release, before momentum.
/// Reduced motion disables displacement. Keyboard, focus, and line-wheel input
/// remain owned by the child. No colors, padding, or dimensions are imposed.
pub struct ElasticScroll<H: ScrollbarHandle + Clone> {
    id: ElementId,
    handle: H,
    child: AnyElement,
    enabled: bool,
    on_scroll: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl<H: ScrollbarHandle + Clone> ElasticScroll<H> {
    pub fn new(id: impl Into<ElementId>, handle: &H, child: impl IntoElement) -> Self {
        Self {
            id: id.into(),
            handle: handle.clone(),
            child: child.into_any_element(),
            enabled: cfg!(target_os = "ios"),
            on_scroll: None,
        }
    }

    /// Opt in on a platform with compatible touch phase semantics.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Observe a logical scroll performed when a reverse drag leaves the elastic
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
}

#[doc(hidden)]
pub struct ElasticScrollPrepaintState {
    state: Rc<RefCell<State>>,
    hitbox: Hitbox,
}

impl<H: ScrollbarHandle + Clone> IntoElement for ElasticScroll<H> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl<H: ScrollbarHandle + Clone> Element for ElasticScroll<H> {
    type RequestLayoutState = ();
    type PrepaintState = ElasticScrollPrepaintState;

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
            id.expect("ElasticScroll has an id"),
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
        ElasticScrollPrepaintState { state, hitbox }
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
                let ScrollDelta::Pixels(delta) = event.delta else {
                    return;
                };
                if !hitbox.should_handle_scroll(window) || delta.x.abs() > delta.y.abs() {
                    return;
                }
                let mut state = state.borrow_mut();
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
                            let max = (handle.content_size().height
                                - handle.viewport_bounds().size.height)
                                .max(px(0.));
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
                    let max = (handle.content_size().height - handle.viewport_bounds().size.height)
                        .max(px(0.));
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
    pub dragging: bool,
    pub suppress_momentum: bool,
    extent: f32,
}

impl Physics {
    pub fn offset(&self) -> f32 {
        if self.dragging {
            let d = self.extent.max(1.);
            self.position * 0.55 / (1. + 0.55 * self.position.abs() / d)
        } else {
            self.position
        }
    }

    pub fn begin(&mut self, extent: f32) {
        let offset = self.offset();
        self.extent = extent.max(1.);
        // Invert the rubber-band curve so grabbing a returning edge is continuous.
        self.position = offset / (0.55 * (1. - offset.abs() / self.extent).max(0.01));
        self.velocity = 0.;
        self.dragging = true;
        self.suppress_momentum = false;
    }

    /// Apply finger displacement. Return the part that crosses back into the list.
    pub fn pull(&mut self, delta: f32) -> f32 {
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

    pub fn release(&mut self) {
        self.position = self.offset();
        self.dragging = false;
        if self.position != 0. {
            self.suppress_momentum = true;
        }
    }

    /// Exact critically damped spring integration, independent of refresh rate.
    pub fn step(&mut self, seconds: f32) -> bool {
        if self.dragging || self.position == 0. {
            return false;
        }
        // Tuned return speed, not a UIKit constant. Keep the drag resistance
        // independent so slowing the return does not change finger tracking.
        let omega = 12.;
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
                ElasticScroll::new(
                    "elastic",
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
            ElasticScroll::new(
                "elastic-list",
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
