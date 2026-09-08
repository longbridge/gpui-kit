use gpui::{
    AppContext, Context, Entity, TestAppContext, Window, WindowHandle, div, prelude::*, px,
};
use gpui_component::input::{Input, InputState};
use gpui_kit::test::TestWindowExt;

struct Inputs {
    first: Entity<InputState>,
    second: Entity<InputState>,
    guarded: Entity<InputState>,
    secret: Entity<InputState>,
    disabled: bool,
    readonly: bool,
}
impl Render for Inputs {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .child(Input::new(&self.first).id("first").w(px(240.)))
            .child(Input::new(&self.second).id("second").w(px(240.)))
            .child(
                Input::new(&self.guarded)
                    .id("guarded")
                    .w(px(240.))
                    .disabled(self.disabled)
                    .readonly(self.readonly),
            )
            .child(Input::new(&self.secret).id("secret").w(px(240.)))
    }
}
fn inputs(cx: &mut TestAppContext) -> WindowHandle<Inputs> {
    cx.update(gpui_component::init);
    cx.add_window(|window, cx| Inputs {
        first: cx.new(|cx| InputState::new(window, cx)),
        second: cx.new(|cx| InputState::new(window, cx)),
        guarded: cx.new(|cx| InputState::new(window, cx).default_value("fixed")),
        secret: cx.new(|cx| InputState::new(window, cx).masked(true)),
        disabled: false,
        readonly: false,
    })
}

#[gpui::test]
fn text_goes_only_to_the_focused_input(cx: &mut TestAppContext) {
    let handle = inputs(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("first", cx);
        window.input("A🦀", cx);
        let previous = window.find("first");
        window.click("second", cx);
        window.input("中文", cx);
        assert!(!window.find("first").focused());
        assert!(window.find("second").focused());
        assert_eq!(window.find("first").text(), Some("A🦀"));
        assert_eq!(window.find("second").text(), Some("中文"));
        assert!(previous.focused());
    })
    .unwrap();
    handle
        .update(cx, |view, _, cx| {
            assert_eq!(view.first.read(cx).value(), "A🦀");
            assert_eq!(view.second.read(cx).value(), "中文");
        })
        .unwrap();
}

#[gpui::test]
fn readonly_and_disabled_inputs_reject_native_typing(cx: &mut TestAppContext) {
    let handle = inputs(cx);
    for disabled in [false, true] {
        handle
            .update(cx, |view, _, cx| {
                view.disabled = disabled;
                view.readonly = !disabled;
                cx.notify();
            })
            .unwrap();
        cx.update_window(handle.into(), |_, window, cx| {
            window.click("guarded", cx);
            window.input("ignored", cx);
            let input = window.find("guarded");
            assert_eq!(input.disabled(), disabled);
            assert_eq!(input.text(), Some("fixed"));
        })
        .unwrap();
        handle
            .update(cx, |view, _, cx| {
                assert_eq!(view.guarded.read(cx).value(), "fixed")
            })
            .unwrap();
    }
}

#[gpui::test]
fn masked_input_handles_typing_without_reporting_secret_value(cx: &mut TestAppContext) {
    let handle = inputs(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("secret", cx);
        window.input("secret", cx);
        let secret = window.find("secret");
        assert!(secret.focused());
        assert_eq!(secret.text(), None);
        assert_eq!(secret.value(), None);
    })
    .unwrap();
    handle
        .update(cx, |view, _, cx| {
            assert_eq!(view.secret.read(cx).value(), "secret")
        })
        .unwrap();
}

#[gpui::test]
fn existing_gpui_keyboard_editing_updates_observed_value(cx: &mut TestAppContext) {
    let handle = inputs(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("first", cx);
        window.input("ab", cx);
    })
    .unwrap();
    cx.simulate_keystrokes(handle.into(), "backspace");
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        assert_eq!(window.find("first").text(), Some("a"));
    })
    .unwrap();
    handle
        .update(cx, |view, _, cx| {
            assert_eq!(view.first.read(cx).value(), "a")
        })
        .unwrap();
}
