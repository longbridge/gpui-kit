//! A window with nothing focused still answers Tab and Shift-Tab: they reach
//! its controls before anything has been clicked, and again after the focused
//! control goes away. Nothing is focused until then.

use crate::common;
use gpui_kit::component::button::Button;
use gpui_kit::test::{TestQueryExt as _, TestWindowExt as _};
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, KeyBinding, ParentElement as _, Render,
    TestAppContext, Window, WindowHandle, actions, base::Root, div, prelude::FluentBuilder as _,
};
use std::{cell::Cell, rc::Rc};

actions!(root_focus_test, [AppTab]);

struct Controls {
    show_middle: bool,
}

impl Render for Controls {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(Button::new("first").label("First"))
            .when(self.show_middle, |this| {
                this.child(Button::new("middle").label("Middle"))
            })
            .child(Button::new("last").label("Last"))
    }
}

fn open(cx: &mut TestAppContext) -> (WindowHandle<Root>, Entity<Controls>) {
    cx.update(gpui_kit::init);
    let opened = common::open_window(cx, None, |_, cx| cx.new(|_| Controls { show_middle: true }));
    cx.update_window(opened.0.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.focused(cx).is_none());
    })
    .unwrap();
    opened
}

#[gpui_kit::test]
fn tab_in_a_window_with_nothing_focused_reaches_the_first_control(cx: &mut TestAppContext) {
    let (handle, _) = open(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.press("tab", cx);
        window.render_frame(cx);
        assert_eq!(window.find_by_label("First").focused(), Some(true));
    })
    .unwrap();
}

#[gpui_kit::test]
fn shift_tab_in_a_window_with_nothing_focused_reaches_the_last_control(cx: &mut TestAppContext) {
    let (handle, _) = open(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.press("shift-tab", cx);
        window.render_frame(cx);
        assert_eq!(window.find_by_label("Last").focused(), Some(true));
    })
    .unwrap();
}

#[gpui_kit::test]
fn tab_reaches_the_controls_again_after_the_focused_one_goes_away(cx: &mut TestAppContext) {
    let (handle, controls) = open(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.focus_next(cx);
        window.focus_next(cx);
        window.render_frame(cx);
        assert_eq!(window.find_by_label("Middle").focused(), Some(true));
    })
    .unwrap();
    controls.update(cx, |controls, cx| {
        controls.show_middle = false;
        cx.notify();
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.render_frame(cx);
        assert!(window.focused(cx).is_none());
        window.press("tab", cx);
        window.render_frame(cx);
        assert_eq!(window.find_by_label("First").focused(), Some(true));
    })
    .unwrap();
}

#[gpui_kit::test]
fn an_app_binding_for_tab_outside_any_context_still_takes_tab(cx: &mut TestAppContext) {
    let (handle, _) = open(cx);
    let ran = Rc::new(Cell::new(false));
    cx.update({
        let ran = ran.clone();
        move |cx| {
            cx.bind_keys([KeyBinding::new("tab", AppTab, None)]);
            cx.on_action(move |_: &AppTab, _| ran.set(true));
        }
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.press("tab", cx);
        window.render_frame(cx);
        assert!(window.focused(cx).is_none());
    })
    .unwrap();
    assert!(ran.get());
}
