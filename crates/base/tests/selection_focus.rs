//! Selection controls keep their place in the focus path when disabled by a
//! controlled update, so native Tab can still leave them in either direction.
#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;

use gpui::{
    AppContext as _, Context, FocusHandle, InteractiveElement as _, IntoElement, KeyDownEvent,
    KeyUpEvent, Keystroke, ParentElement as _, Render, Styled as _, TestAppContext,
    VisualTestContext, Window, div,
};
use gpui_base::{Button, Checkbox, Radio, Root, Switch, Toggle};

#[derive(Clone, Copy, Debug)]
enum Control {
    Toggle,
    Checkbox,
    Radio,
    Switch,
}

struct DisableDuringFocus {
    control: Control,
    handles: [FocusHandle; 3],
    disabled: bool,
    changes: usize,
}

impl Render for DisableDuringFocus {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus = &self.handles[1];
        let on_change = cx.listener(|this, _, _, cx| {
            this.disabled = true;
            this.changes += 1;
            cx.notify();
        });
        let control = match self.control {
            Control::Toggle => Toggle::new("control")
                .disabled(self.disabled)
                .track_focus(focus)
                .on_change(move |_, event, window, cx| on_change(event, window, cx))
                .into_any_element(),
            Control::Checkbox => Checkbox::new("control")
                .disabled(self.disabled)
                .track_focus(focus)
                .on_change(move |_, event, window, cx| on_change(event, window, cx))
                .into_any_element(),
            Control::Radio => Radio::new("control")
                .disabled(self.disabled)
                .track_focus(focus)
                .on_change(move |_, event, window, cx| on_change(event, window, cx))
                .into_any_element(),
            Control::Switch => Switch::new("control")
                .disabled(self.disabled)
                .track_focus(focus)
                .on_change(move |_, event, window, cx| on_change(event, window, cx))
                .into_any_element(),
        };
        div()
            .size_full()
            .flex()
            .tab_group()
            .child(Button::new("before").track_focus(&self.handles[0]))
            .child(control)
            .child(Button::new("after").track_focus(&self.handles[2]))
    }
}

fn press_key(cx: &mut VisualTestContext, key: &str) {
    let keystroke =
        cx.update(|_, cx| Keystroke::parse_with_os(key, cx.platform_info().os).unwrap());
    cx.simulate_event(KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(KeyUpEvent { keystroke });
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn assert_focus(cx: &mut VisualTestContext, expected: &FocusHandle) {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(window.focused(cx).as_ref(), Some(expected));
    });
}

fn check_disabled_focus(cx: &mut TestAppContext, control: Control) {
    cx.update(gpui_base::init);
    for (key, target, reverse, opposite) in [("tab", 2, "shift-tab", 0), ("shift-tab", 0, "tab", 2)]
    {
        let handles = cx.update(|cx| std::array::from_fn(|_| cx.focus_handle()));
        let controls = cx.new(|_| DisableDuringFocus {
            control,
            handles: handles.clone(),
            disabled: false,
            changes: 0,
        });
        let (_, cx) = cx.add_window_view(|window, cx| Root::new(controls.clone(), window, cx));
        cx.update(|window, _| window.activate_window());
        cx.executor().run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            handles[0].focus(window, cx);
            window.draw(cx).clear(cx);
        });
        press_key(cx, "tab");
        assert_focus(cx, &handles[1]);
        press_key(cx, "space");
        assert_focus(cx, &handles[1]);
        cx.update(|_, cx| {
            assert!(controls.read(cx).disabled);
            assert_eq!(controls.read(cx).changes, 1);
        });
        for activation in ["enter", "space"] {
            press_key(cx, activation);
        }
        cx.update(|_, cx| assert_eq!(controls.read(cx).changes, 1, "disabled stays inert"));

        // Leaving a just-disabled control retains its original neighbors.
        press_key(cx, key);
        assert_focus(cx, &handles[target]);
        // Returning skips the disabled control, even though its handle survives.
        press_key(cx, reverse);
        assert_focus(cx, &handles[opposite]);
        controls.update(cx, |this, cx| {
            this.disabled = false;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        press_key(cx, key);
        assert_focus(cx, &handles[1]);
    }
}

#[gpui::test]
fn disabling_focused_toggle_preserves_tab_exit(cx: &mut TestAppContext) {
    check_disabled_focus(cx, Control::Toggle);
}

#[gpui::test]
fn disabling_focused_checkbox_preserves_tab_exit(cx: &mut TestAppContext) {
    check_disabled_focus(cx, Control::Checkbox);
}

#[gpui::test]
fn disabling_focused_radio_preserves_tab_exit(cx: &mut TestAppContext) {
    check_disabled_focus(cx, Control::Radio);
}

#[gpui::test]
fn disabling_focused_switch_preserves_tab_exit(cx: &mut TestAppContext) {
    check_disabled_focus(cx, Control::Switch);
}
