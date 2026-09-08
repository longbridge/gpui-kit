use gpui::{AppContext, Context, Entity, TestAppContext, Window, div, prelude::*, px, size};
use gpui_component::{
    Disableable,
    button::Button,
    input::{Input, InputState},
    popover::Popover,
};
use gpui_kit::test::{TestPropsExt, TestWindowExt};

struct Controls {
    input: Entity<InputState>,
    clicks: usize,
}
impl Render for Controls {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                Button::new("disabled")
                    .label("Disabled")
                    .disabled(true)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.clicks += 1;
                        cx.notify();
                    })),
            )
            .child(Input::new(&self.input).id("search").w(px(240.)))
            .child(
                Popover::new("popover-host")
                    .trigger(Button::new("open").label("Open"))
                    .content(|_, _, _| {
                        div()
                            .id("popover-content")
                            .test_props(|props| props)
                            .w(px(120.))
                            .h(px(50.))
                            .child("Details")
                    }),
            )
    }
}

#[gpui::test]
fn kit_controls_use_native_events_and_report_state(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let handle = cx.open_window(size(px(600.), px(500.)), |window, cx| Controls {
        input: cx.new(|cx| InputState::new(window, cx)),
        clicks: 0,
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert!(window.find("disabled").disabled());
        window.click("disabled", cx);
        window.click("search", cx);
        assert!(window.find("search").focused());
        window.input("GPUI 中文 🦀", cx);
        assert_eq!(window.find("search").text(), Some("GPUI 中文 🦀"));
        assert!(window.try_find("popover-content").is_none());
        window.click("open", cx);
        let content = window.find("popover-content");
        assert!(content.visible());
        assert!(content.bounds().top() >= window.find("open").bounds().bottom());
    })
    .unwrap();
    handle
        .update(cx, |view, _, _| assert_eq!(view.clicks, 0))
        .unwrap();
}

struct NamedButton;
impl Render for NamedButton {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Button::new("save")
            .label("Save")
            .accessibility_label("Save this document")
    }
}

#[gpui::test]
fn button_reports_its_label_instead_of_accessibility_name(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let handle = cx.add_window(|_, _| NamedButton);
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(window.find("save").text(), Some("Save"));
    })
    .unwrap();
}
