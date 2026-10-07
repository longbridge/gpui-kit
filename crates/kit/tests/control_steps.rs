mod common;

use gpui_kit::component::{Disableable, button::Button, checkbox::Checkbox, switch::Switch};
use gpui_kit::test::{TestEventExt, TestQueryExt, TestWindowExt};
use gpui_kit::{
    AppContext, Context, MouseButton, Role, TestAppContext, Window, div, prelude::*, px, size,
};

#[derive(Default)]
struct Controls {
    button_clicks: usize,
    checked: bool,
    changes: Vec<bool>,
    disabled: bool,
}

impl Render for Controls {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("controls")
            .tab_group()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                Button::new("save")
                    .label("Save")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.button_clicks += 1;
                        cx.notify();
                    })),
            )
            .child(
                Checkbox::new("remember")
                    .label("Remember account")
                    .checked(self.checked)
                    .on_change(cx.listener(|this, next, _, cx| {
                        this.changes.push(*next);
                        this.checked = *next;
                        cx.notify();
                    })),
            )
            .child(
                Switch::new("notifications")
                    .label("Notifications")
                    .checked(self.checked)
                    .disabled(self.disabled)
                    .on_change(cx.listener(|this, next, _, cx| {
                        this.changes.push(*next);
                        this.checked = *next;
                        cx.notify();
                    })),
            )
    }
}

#[gpui_kit::test]
fn button_pointer_release_outside_cancels_activation_and_next_click_recovers(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (handle, content) = common::open_window(cx, Some(size(px(400.), px(300.))), |_, cx| {
        cx.new(|_| Controls::default())
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let button = window.find_by_label("Save");
        assert_eq!(button.role(), Some(Role::Button));
        let inside = button.bounds().center();
        let outside = window.find_by_label("Notifications").bounds().center();
        window.pointer_down(inside, MouseButton::Left, cx);
        assert_eq!(content.read(cx).button_clicks, 0);
        window.pointer_move(outside, Some(MouseButton::Left), cx);
        window.pointer_up(outside, MouseButton::Left, cx);
        assert_eq!(content.read(cx).button_clicks, 0);

        let inside = window.find_by_label("Save").bounds().center();
        window.pointer_down(inside, MouseButton::Left, cx);
        assert_eq!(content.read(cx).button_clicks, 0);
        window.pointer_up(inside, MouseButton::Left, cx);
        assert_eq!(content.read(cx).button_clicks, 1);
    })
    .unwrap();
}

#[gpui_kit::test]
fn checkbox_space_repeat_changes_controlled_value_once_on_release(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, content) = common::open_window(cx, None, |_, cx| cx.new(|_| Controls::default()));
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        // Root's Tab binding requires an existing focus path. Start at the
        // first stop, then traverse to the checkbox using the actual binding.
        window.focus_next(cx);
        window.render_frame(cx);
        assert_eq!(window.find_by_label("Save").focused(), Some(true));
        window.press("tab", cx);
        assert_eq!(
            window.find_by_label("Remember account").focused(),
            Some(true)
        );
        assert_eq!(
            window.find_by_label("Remember account").checked(),
            Some(false)
        );
        window.key_down("space", false, cx);
        window.key_down("space", true, cx);
        window.key_down("space", true, cx);
        assert!(content.read(cx).changes.is_empty());
        assert_eq!(
            window.find_by_label("Remember account").checked(),
            Some(false)
        );
        window.key_up("space", cx);
        assert_eq!(content.read(cx).changes, vec![true]);
        assert_eq!(
            window.find_by_label("Remember account").checked(),
            Some(true)
        );
        window.key_up("space", cx);
        assert_eq!(content.read(cx).changes, vec![true]);

        window.key_down("space", false, cx);
        window.key_up("space", cx);
        assert_eq!(content.read(cx).changes, vec![true, false]);
        assert_eq!(
            window.find_by_label("Remember account").checked(),
            Some(false)
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn switch_disabled_before_keyboard_release_cannot_request_a_value(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, content) = common::open_window(cx, None, |_, cx| cx.new(|_| Controls::default()));
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("notifications", cx);
        assert_eq!(content.read(cx).changes, vec![true]);
        assert_eq!(window.find_by_label("Notifications").focused(), Some(true));
        window.key_down("space", false, cx);
        assert_eq!(content.read(cx).changes, vec![true]);
        content.update(cx, |this, cx| {
            this.disabled = true;
            cx.notify();
        });
        window.render_frame(cx);
        window.key_up("space", cx);
        assert_eq!(content.read(cx).changes, vec![true]);
        assert_eq!(window.find_by_label("Notifications").checked(), Some(true));

        content.update(cx, |this, cx| {
            this.disabled = false;
            cx.notify();
        });
        window.render_frame(cx);
        window.key_down("space", false, cx);
        window.key_up("space", cx);
        assert_eq!(content.read(cx).changes, vec![true, false]);
        assert_eq!(window.find_by_label("Notifications").checked(), Some(false));
    })
    .unwrap();
}
