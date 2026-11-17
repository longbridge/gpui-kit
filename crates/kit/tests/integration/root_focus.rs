//! A window with nothing focused still answers the keyboard: Tab and
//! Shift-Tab reach its controls before anything has been clicked, and again
//! after the focused control goes away.

use crate::common;
use gpui_kit::component::button::Button;
use gpui_kit::test::{TestEventExt as _, TestQueryExt as _, TestWindowExt as _};
use gpui_kit::{
    AppContext as _, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, Render, TestAppContext, Window, WindowHandle, base::Root, div,
    point, prelude::FluentBuilder as _, px,
};

struct Controls {
    show_middle: bool,
    /// Focused when the window opens if nothing else is, as the story
    /// gallery focuses its root.
    own_focus: Option<FocusHandle>,
}

impl Render for Controls {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("controls")
            .when_some(self.own_focus.as_ref(), |this, focus| {
                this.track_focus(focus)
            })
            .child(Button::new("first").label("First"))
            .when(self.show_middle, |this| {
                this.child(Button::new("middle").label("Middle"))
            })
            .child(Button::new("last").label("Last"))
    }
}

fn open(cx: &mut TestAppContext, focus_itself: bool) -> (WindowHandle<Root>, Entity<Controls>) {
    cx.update(gpui_kit::init);
    common::open_window(cx, None, move |window, cx| {
        let controls = cx.new(|cx| Controls {
            show_middle: true,
            own_focus: focus_itself.then(|| cx.focus_handle()),
        });
        if let Some(focus) = controls.read(cx).own_focus.clone() {
            window.defer(cx, move |window, cx| {
                if window.focused(cx).is_none() {
                    focus.focus(window, cx);
                }
            });
        }
        controls
    })
}

#[gpui_kit::test]
fn tab_in_a_window_with_nothing_focused_reaches_the_first_control(cx: &mut TestAppContext) {
    let (handle, _) = open(cx, false);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.press("tab", cx);
        window.render_frame(cx);
        assert_eq!(window.find_by_label("First").focused(), Some(true));
    })
    .unwrap();
}

#[gpui_kit::test]
fn shift_tab_in_a_window_with_nothing_focused_reaches_the_last_control(cx: &mut TestAppContext) {
    let (handle, _) = open(cx, false);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.press("shift-tab", cx);
        window.render_frame(cx);
        assert_eq!(window.find_by_label("Last").focused(), Some(true));
    })
    .unwrap();
}

#[gpui_kit::test]
fn tab_reaches_the_controls_again_after_the_focused_one_goes_away(cx: &mut TestAppContext) {
    let (handle, controls) = open(cx, false);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
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
        window.press("tab", cx);
        window.render_frame(cx);
        assert_eq!(window.find_by_label("First").focused(), Some(true));
    })
    .unwrap();
}

#[gpui_kit::test]
fn content_that_focuses_itself_when_the_window_opens_keeps_that_focus(cx: &mut TestAppContext) {
    let (handle, controls) = open(cx, true);
    let focus = controls.read_with(cx, |controls, _| controls.own_focus.clone().unwrap());
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(focus.is_focused(window));
    })
    .unwrap();
}

#[gpui_kit::test]
fn pressing_the_background_leaves_the_focused_control_focused(cx: &mut TestAppContext) {
    let (handle, _) = open(cx, false);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.press("tab", cx);
        window.render_frame(cx);
        assert_eq!(window.find_by_label("First").focused(), Some(true));
        let background = point(px(600.), px(600.));
        window.pointer_down(background, MouseButton::Left, cx);
        window.pointer_up(background, MouseButton::Left, cx);
        window.render_frame(cx);
        assert_eq!(window.find_by_label("First").focused(), Some(true));
    })
    .unwrap();
}
