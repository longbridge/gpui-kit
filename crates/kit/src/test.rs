//! Headless UI tests using GPUI identity, completed layout and native input.
//!
//! Import [`TestWindowExt`] alongside GPUI's prelude. Draw once before the first
//! query (`window.draw(cx).clear(cx)`). Interactions redraw before and after
//! dispatch. After external state changes, actions, or resizing, refresh and draw.
use gpui::{
    App, ElementId, InputEvent, Keystroke, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Window,
};

pub use gpui_base::test_support::{ObserveElement, TestElement};

/// Small testing extension to GPUI's existing Window.
pub trait TestWindowExt {
    /// Looks up an existing ElementId in the latest completed frame.
    /// Panics on ambiguous IDs in different scopes: give test targets unique IDs.
    fn find(&self, id: impl Into<ElementId>) -> Option<TestElement>;
    /// Sends mouse move/down/up through GPUI hit testing at the target's center.
    /// Panics for missing or geometrically invisible targets. Disabled controls
    /// still receive native events and decide whether to handle them themselves.
    fn click(&mut self, id: impl Into<ElementId>, cx: &mut App);
    /// Sends Unicode text to the current keyboard focus through GPUI's simulated
    /// input path. Does not focus a target or replace the control's whole value.
    fn input(&mut self, text: &str, cx: &mut App);
}

impl TestWindowExt for Window {
    fn find(&self, id: impl Into<ElementId>) -> Option<TestElement> {
        gpui_base::test_support::find(self, id.into())
    }

    fn click(&mut self, id: impl Into<ElementId>, cx: &mut App) {
        let id = id.into();
        redraw(self, cx);
        let element = self
            .find(id.clone())
            .unwrap_or_else(|| panic!("missing ElementId {id:?}"));
        assert!(element.visible(), "ElementId {id:?} is not visible");
        let position = element.bounds().center();
        self.dispatch_event(
            MouseMoveEvent {
                position,
                pressed_button: None,
                modifiers: Default::default(),
            }
            .to_platform_input(),
            cx,
        );
        redraw(self, cx);
        self.dispatch_event(
            MouseDownEvent {
                button: MouseButton::Left,
                position,
                modifiers: Default::default(),
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
            cx,
        );
        redraw(self, cx);
        self.dispatch_event(
            MouseUpEvent {
                button: MouseButton::Left,
                position,
                modifiers: Default::default(),
                click_count: 1,
            }
            .to_platform_input(),
            cx,
        );
        redraw(self, cx);
    }

    fn input(&mut self, text: &str, cx: &mut App) {
        redraw(self, cx);
        for character in text.chars() {
            let text = character.to_string();
            let mut keystroke = Keystroke::parse(&text)
                .expect("a single Unicode character is a valid GPUI keystroke");
            keystroke.key_char = Some(text);
            self.dispatch_keystroke(keystroke, cx);
            redraw(self, cx);
        }
    }
}

fn redraw(window: &mut Window, cx: &mut App) {
    window.refresh();
    window.draw(cx).clear(cx);
}
