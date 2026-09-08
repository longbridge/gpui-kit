//! Headless interaction and frame snapshots using GPUI's public APIs.
//!
//! [`ElementSnapshot`] is immutable. Call [`TestWindowExt::render_frame`] after
//! external changes, or use [`TestAppContextExt::wait_for`] for asynchronous UI.
use gpui::{
    AnyWindowHandle, App, AppContext, ElementId, InputEvent, Keystroke, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollDelta, ScrollWheelEvent,
    TestAppContext, Window, point, px,
};
use std::time::Duration;

use gpui_base::test_support as observation;
pub use gpui_base::test_support::{ElementSnapshot, ObserveElement};

/// Testing operations on GPUI's existing window.
pub trait TestWindowExt {
    /// Requires a target from the last completed frame, with registered paths in errors.
    fn find(&self, id: impl Into<ElementId>) -> ElementSnapshot;
    /// Returns None for an absent target; ambiguous IDs still require a scope.
    fn try_find(&self, id: impl Into<ElementId>) -> Option<ElementSnapshot>;
    /// Restricts queries to a GPUI identity scope; no additional layout wrapper is needed.
    fn within(&mut self, id: impl Into<ElementId>) -> ScopedWindow<'_>;
    /// Invalidates cached facts and completes a frame.
    fn render_frame(&mut self, cx: &mut App);
    fn click(&mut self, id: impl Into<ElementId>, cx: &mut App);
    /// Clicks at a local offset from the target's top-left corner.
    fn click_at(&mut self, id: impl Into<ElementId>, offset: Point<Pixels>, cx: &mut App);
    fn right_click(&mut self, id: impl Into<ElementId>, cx: &mut App);
    fn double_click(&mut self, id: impl Into<ElementId>, cx: &mut App);
    fn hover(&mut self, id: impl Into<ElementId>, cx: &mut App);
    /// Dispatches a wheel event, preserving GPUI's delta sign and units.
    fn scroll(&mut self, id: impl Into<ElementId>, delta: ScrollDelta, cx: &mut App);
    /// Drags between window-local positions through native pointer dispatch.
    fn drag(&mut self, from: Point<Pixels>, to: Point<Pixels>, cx: &mut App);
    /// Sends a parsed GPUI keystroke, such as "backspace" or "cmd-a".
    fn press(&mut self, key: &str, cx: &mut App);
    /// Sends text to the current focus; does not focus a target or replace its whole value.
    fn input(&mut self, text: &str, cx: &mut App);
}

fn require(window: &Window, scope: &[ElementId], id: &ElementId) -> ElementSnapshot {
    observation::find(window, scope, id).unwrap_or_else(|| {
        panic!("missing ElementId {id:?} in scope {scope:?}. Registered paths: {}. Check the ID, observation, and completed frame.", observation::registered_paths(window))
    })
}

fn target_position(
    window: &Window,
    scope: &[ElementId],
    id: &ElementId,
    offset: Option<Point<Pixels>>,
) -> Point<Pixels> {
    let target = require(window, scope, id);
    assert!(
        target.visible(),
        "ElementId {id:?} is not visible (path {:?})",
        target.path()
    );
    if let Some(offset) = offset {
        let size = target.bounds().size;
        assert!(
            offset.x >= px(0.)
                && offset.y >= px(0.)
                && offset.x < size.width
                && offset.y < size.height,
            "click offset {offset:?} is outside ElementId {id:?} bounds {:?}",
            target.bounds()
        );
        target.bounds().origin + offset
    } else {
        target.bounds().center()
    }
}

fn move_pointer(
    window: &mut Window,
    position: Point<Pixels>,
    pressed_button: Option<MouseButton>,
    cx: &mut App,
) {
    window.dispatch_event(
        MouseMoveEvent {
            position,
            pressed_button,
            modifiers: Default::default(),
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
}

fn mouse_down(
    window: &mut Window,
    position: Point<Pixels>,
    button: MouseButton,
    click_count: usize,
    cx: &mut App,
) {
    window.dispatch_event(
        MouseDownEvent {
            button,
            position,
            modifiers: Default::default(),
            click_count,
            first_mouse: false,
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
}

fn mouse_up(
    window: &mut Window,
    position: Point<Pixels>,
    button: MouseButton,
    click_count: usize,
    cx: &mut App,
) {
    window.dispatch_event(
        MouseUpEvent {
            button,
            position,
            modifiers: Default::default(),
            click_count,
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
}

fn click_target(
    window: &mut Window,
    scope: &[ElementId],
    id: ElementId,
    offset: Option<Point<Pixels>>,
    button: MouseButton,
    count: usize,
    cx: &mut App,
) {
    window.render_frame(cx);
    let position = target_position(window, scope, &id, offset);
    move_pointer(window, position, None, cx);
    for click_count in 1..=count {
        mouse_down(window, position, button, click_count, cx);
        mouse_up(window, position, button, click_count, cx);
    }
}

impl TestWindowExt for Window {
    fn find(&self, id: impl Into<ElementId>) -> ElementSnapshot {
        require(self, &[], &id.into())
    }
    fn try_find(&self, id: impl Into<ElementId>) -> Option<ElementSnapshot> {
        observation::find(self, &[], &id.into())
    }
    fn within(&mut self, id: impl Into<ElementId>) -> ScopedWindow<'_> {
        let scope = observation::scope(self, &[], &id.into());
        ScopedWindow {
            window: self,
            scope,
        }
    }
    fn render_frame(&mut self, cx: &mut App) {
        self.refresh();
        self.draw(cx).clear(cx);
    }
    fn click(&mut self, id: impl Into<ElementId>, cx: &mut App) {
        click_target(self, &[], id.into(), None, MouseButton::Left, 1, cx);
    }
    fn click_at(&mut self, id: impl Into<ElementId>, offset: Point<Pixels>, cx: &mut App) {
        click_target(self, &[], id.into(), Some(offset), MouseButton::Left, 1, cx);
    }
    fn right_click(&mut self, id: impl Into<ElementId>, cx: &mut App) {
        click_target(self, &[], id.into(), None, MouseButton::Right, 1, cx);
    }
    fn double_click(&mut self, id: impl Into<ElementId>, cx: &mut App) {
        click_target(self, &[], id.into(), None, MouseButton::Left, 2, cx);
    }
    fn hover(&mut self, id: impl Into<ElementId>, cx: &mut App) {
        self.render_frame(cx);
        let position = target_position(self, &[], &id.into(), None);
        move_pointer(self, position, None, cx);
    }
    fn scroll(&mut self, id: impl Into<ElementId>, delta: ScrollDelta, cx: &mut App) {
        self.render_frame(cx);
        let position = target_position(self, &[], &id.into(), None);
        move_pointer(self, position, None, cx);
        self.dispatch_event(
            ScrollWheelEvent {
                position,
                delta,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        self.render_frame(cx);
    }
    fn drag(&mut self, from: Point<Pixels>, to: Point<Pixels>, cx: &mut App) {
        self.render_frame(cx);
        move_pointer(self, from, None, cx);
        mouse_down(self, from, MouseButton::Left, 1, cx);
        for step in 1..=8 {
            let fraction = step as f32 / 8.;
            move_pointer(
                self,
                point(
                    from.x + (to.x - from.x) * fraction,
                    from.y + (to.y - from.y) * fraction,
                ),
                Some(MouseButton::Left),
                cx,
            );
        }
        mouse_up(self, to, MouseButton::Left, 1, cx);
    }
    fn press(&mut self, key: &str, cx: &mut App) {
        let key =
            Keystroke::parse(key).unwrap_or_else(|error| panic!("invalid test keystroke: {error}"));
        self.render_frame(cx);
        self.dispatch_keystroke(key, cx);
        self.render_frame(cx);
    }
    fn input(&mut self, text: &str, cx: &mut App) {
        self.render_frame(cx);
        for character in text.chars() {
            let text = character.to_string();
            let mut key =
                Keystroke::parse(&text).expect("a Unicode character is a valid GPUI keystroke");
            key.key_char = Some(text);
            self.dispatch_keystroke(key, cx);
            self.render_frame(cx);
        }
    }
}

/// A borrowed GPUI identity scope, not a new element or layout container.
pub struct ScopedWindow<'a> {
    window: &'a mut Window,
    scope: Vec<ElementId>,
}
impl ScopedWindow<'_> {
    pub fn find(&self, id: impl Into<ElementId>) -> ElementSnapshot {
        require(self.window, &self.scope, &id.into())
    }
    pub fn try_find(&self, id: impl Into<ElementId>) -> Option<ElementSnapshot> {
        observation::find(self.window, &self.scope, &id.into())
    }
    pub fn within(&mut self, id: impl Into<ElementId>) -> ScopedWindow<'_> {
        let scope = observation::scope(self.window, &self.scope, &id.into());
        ScopedWindow {
            window: self.window,
            scope,
        }
    }
    pub fn click(&mut self, id: impl Into<ElementId>, cx: &mut App) {
        click_target(
            self.window,
            &self.scope,
            id.into(),
            None,
            MouseButton::Left,
            1,
            cx,
        );
    }
    pub fn click_at(&mut self, id: impl Into<ElementId>, offset: Point<Pixels>, cx: &mut App) {
        click_target(
            self.window,
            &self.scope,
            id.into(),
            Some(offset),
            MouseButton::Left,
            1,
            cx,
        );
    }
}

/// Executor-aware operations which must run outside a borrowed window update.
pub trait TestAppContextExt {
    /// Refreshes frames until the predicate succeeds or the test-clock timeout expires.
    /// Panics with registered paths on timeout. Polls every 10 ms of GPUI test time.
    fn wait_for(
        &mut self,
        window: AnyWindowHandle,
        timeout: Duration,
        predicate: impl FnMut(&mut Window, &mut App) -> bool,
    ) -> impl Future<Output = ()>;
}
impl TestAppContextExt for TestAppContext {
    async fn wait_for(
        &mut self,
        handle: AnyWindowHandle,
        timeout: Duration,
        mut predicate: impl FnMut(&mut Window, &mut App) -> bool,
    ) {
        let mut elapsed = Duration::ZERO;
        loop {
            let (ready, paths) = self
                .update_window(handle, |_, window, cx| {
                    window.render_frame(cx);
                    {
                        let ready = predicate(window, cx);
                        let paths = if !ready && elapsed >= timeout {
                            observation::registered_paths(window)
                        } else {
                            String::new()
                        };
                        (ready, paths)
                    }
                })
                .expect("test window closed while waiting");
            if ready {
                return;
            }
            assert!(
                elapsed < timeout,
                "UI condition timed out after {timeout:?}. Registered paths: {paths}"
            );
            let interval = Duration::from_millis(10).min(timeout - elapsed);
            self.executor().timer(interval).await;
            elapsed += interval;
        }
    }
}
