mod common;
use gpui_kit::test::{TestEventExt, TestSupportExt, TestWindowExt};
use gpui_kit::{
    AppContext, Context, FocusHandle, Modifiers, MouseButton, ScrollDelta, TestAppContext, Window,
    div, point, prelude::*, px,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct Events {
    pointer: Vec<(&'static str, Modifiers, Option<MouseButton>)>,
    keys: Vec<(&'static str, bool)>,
    key_modifiers: Vec<Modifiers>,
    scroll_modifiers: Vec<Modifiers>,
}
struct Surface {
    focus: FocusHandle,
    events: Rc<RefCell<Events>>,
}
impl Render for Surface {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let moved = self.events.clone();
        let down = self.events.clone();
        let up = self.events.clone();
        let key_down = self.events.clone();
        let key_up = self.events.clone();
        let scrolled = self.events.clone();
        div()
            .id("surface")
            .test_support()
            .track_focus(&self.focus)
            .size(px(80.))
            .on_mouse_move(move |event, _, _| {
                moved
                    .borrow_mut()
                    .pointer
                    .push(("move", event.modifiers, event.pressed_button))
            })
            .on_mouse_down(MouseButton::Left, move |event, _, _| {
                down.borrow_mut()
                    .pointer
                    .push(("down", event.modifiers, Some(event.button)))
            })
            .on_mouse_up(MouseButton::Left, move |event, _, _| {
                up.borrow_mut()
                    .pointer
                    .push(("up", event.modifiers, Some(event.button)))
            })
            .on_scroll_wheel(move |event, _, _| {
                scrolled.borrow_mut().scroll_modifiers.push(event.modifiers)
            })
            .on_key_down(move |event, _, cx| {
                key_down.borrow_mut().keys.push(("down", event.is_held));
                key_down
                    .borrow_mut()
                    .key_modifiers
                    .push(event.keystroke.modifiers);
                cx.stop_propagation();
            })
            .on_key_up(move |event, _, _| {
                key_up.borrow_mut().keys.push(("up", false));
                key_up
                    .borrow_mut()
                    .key_modifiers
                    .push(event.keystroke.modifiers);
            })
    }
}

#[gpui_kit::test]
fn ordinary_pointer_helpers_retain_held_modifiers(cx: &mut TestAppContext) {
    let events = Rc::new(RefCell::new(Events::default()));
    let (handle, _) = common::open_window(cx, None, |_, cx| {
        cx.new(|cx| Surface {
            focus: cx.focus_handle(),
            events: events.clone(),
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.change_modifiers(Modifiers::alt(), cx);
        window.hover("surface", cx);
        window.click("surface", cx);
        window.scroll("surface", ScrollDelta::Pixels(point(px(0.), px(-10.))), cx);
        let center = window.find("surface").bounds().center();
        window.drag(center, center + point(px(10.), px(0.)), cx);
        assert_eq!(window.modifiers(), Modifiers::alt());
        let recorded = events.borrow();
        assert!(!recorded.pointer.is_empty());
        assert!(
            recorded
                .pointer
                .iter()
                .all(|(_, modifiers, _)| *modifiers == Modifiers::alt())
        );
        assert_eq!(recorded.scroll_modifiers, vec![Modifiers::alt()]);
        assert!(
            recorded
                .pointer
                .iter()
                .any(|(name, _, button)| *name == "move" && *button == Some(MouseButton::Left))
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn keyboard_steps_combine_held_and_parsed_modifiers(cx: &mut TestAppContext) {
    let events = Rc::new(RefCell::new(Events::default()));
    let (handle, content) = common::open_window(cx, None, |_, cx| {
        cx.new(|cx| Surface {
            focus: cx.focus_handle(),
            events: events.clone(),
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        let focus = content.read(cx).focus.clone();
        window.focus(&focus, cx);
        window.change_modifiers(Modifiers::shift(), cx);
        window.key_down("a", false, cx);
        window.key_up("a", cx);
        window.key_down("ctrl-a", false, cx);
        window.key_up("ctrl-a", cx);
        assert_eq!(window.modifiers(), Modifiers::shift());
        assert_eq!(
            events.borrow().key_modifiers,
            vec![
                Modifiers::shift(),
                Modifiers::shift(),
                Modifiers::shift() | Modifiers::control(),
                Modifiers::shift() | Modifiers::control(),
            ]
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn individual_pointer_steps_preserve_held_modifiers_and_button(cx: &mut TestAppContext) {
    let events = Rc::new(RefCell::new(Events::default()));
    let (handle, _) = common::open_window(cx, None, |_, cx| {
        cx.new(|cx| Surface {
            focus: cx.focus_handle(),
            events: events.clone(),
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let position = window.find("surface").bounds().center();
        let capslock = window.capslock();
        window.change_modifiers(Modifiers::shift(), cx);
        window.pointer_move(position, None, cx);
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_move(position, Some(MouseButton::Left), cx);
        window.pointer_up(position, MouseButton::Left, cx);
        assert_eq!(window.modifiers(), Modifiers::shift());
        assert_eq!(window.capslock(), capslock);
        window.change_modifiers(Modifiers::default(), cx);
    })
    .unwrap();
    assert_eq!(
        events.borrow().pointer,
        vec![
            ("move", Modifiers::shift(), None),
            ("down", Modifiers::shift(), Some(MouseButton::Left)),
            ("move", Modifiers::shift(), Some(MouseButton::Left)),
            ("up", Modifiers::shift(), Some(MouseButton::Left)),
        ]
    );
}

#[gpui_kit::test]
fn key_steps_expose_repeat_and_allow_updates_before_release(cx: &mut TestAppContext) {
    let events = Rc::new(RefCell::new(Events::default()));
    let (handle, content) = common::open_window(cx, None, |_, cx| {
        cx.new(|cx| Surface {
            focus: cx.focus_handle(),
            events: events.clone(),
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        let focus = content.read(cx).focus.clone();
        window.focus(&focus, cx);
        window.key_down("enter", false, cx);
        assert_eq!(events.borrow().keys, vec![("down", false)]);
        assert_eq!(window.find("surface").focused(), Some(true));
        window.key_down("enter", true, cx);
        assert_eq!(events.borrow().keys, vec![("down", false), ("down", true)]);
        window.key_up("enter", cx);
        assert_eq!(
            events.borrow().keys,
            vec![("down", false), ("down", true), ("up", false)]
        );
    })
    .unwrap();
}
