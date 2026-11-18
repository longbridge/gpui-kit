//! A controlled popover can open inside an element that lays out its children
//! during prepaint, such as `MotionReveal`, while an accessibility tree is
//! being built (#3705).

use crate::common;
use gpui_kit::base::MotionReveal;
use gpui_kit::component::{button::Button, popover::Popover};
use gpui_kit::{
    AppContext as _, Context, FocusHandle, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Role, StatefulInteractiveElement as _, TestAppContext, Window, div,
};

struct Revealed {
    open: bool,
    controller: FocusHandle,
}

impl Render for Revealed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let popover = Popover::new("controlled-popover")
            .open(self.open)
            .trigger(Button::new("popover-trigger").label("Popover trigger"))
            .child(div().id("popover-content").child("Popover content"));
        div()
            .child(
                div()
                    .id("controller")
                    .role(Role::Button)
                    .aria_label("Toggle")
                    .track_focus(&self.controller)
                    .child("Toggle"),
            )
            .child(MotionReveal::new("reveal", 1.0, popover.into_any_element()))
    }
}

#[gpui_kit::test]
fn a_controlled_popover_inside_a_motion_reveal_opens_and_closes_with_accessibility_on(
    cx: &mut TestAppContext,
) {
    // GPUI Fast's test window builds accessibility data only when asked to.
    // Re-run in a child so the flag is set before any frame, without changing
    // the process environment during parallel tests.
    if std::env::var("GPUI_A11Y_ACTIVE").as_deref() != Ok("1") {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "popover_a11y::a_controlled_popover_inside_a_motion_reveal_opens_and_closes_with_accessibility_on",
                "--nocapture",
            ])
            .env("GPUI_A11Y_ACTIVE", "1")
            .status()
            .unwrap();
        assert!(status.success(), "accessibility check failed: {status}");
        return;
    }

    cx.update(gpui_kit::init);
    let (handle, view) = common::open_window(cx, None, |_, cx| {
        cx.new(|cx| Revealed {
            open: false,
            controller: cx.focus_handle(),
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert!(window.is_a11y_active());
        // Focused, so the controller reports focus to the accessibility tree
        // in the same frame the popover opens.
        let controller = view.read(cx).controller.clone();
        controller.focus(window, cx);
        view.update(cx, |view, cx| {
            view.open = true;
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        let controller = view.read(cx).controller.clone();
        assert!(
            window.focused(cx).is_some() && !controller.is_focused(window),
            "opening the popover must move focus into it"
        );
        view.update(cx, |view, cx| {
            view.open = false;
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert!(
            view.read(cx).controller.is_focused(window),
            "closing the popover must give focus back to the controller"
        );
    })
    .unwrap();
}
