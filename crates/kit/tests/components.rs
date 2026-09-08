use gpui::{AppContext, Context, Entity, TestAppContext, Window, div, prelude::*, px, size};
use gpui_component::{
    Disableable,
    button::Button,
    input::{Input, InputState},
    popover::Popover,
};
use gpui_kit::test::{TestSupportExt, TestWindowExt};

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
                            .test_support()
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
        assert_eq!(window.find("disabled").disabled(), None);
        window.click("disabled", cx);
        window.click("search", cx);
        assert_eq!(window.find("search").focused(), Some(true));
        window.input("GPUI 中文 🦀", cx);
        assert_eq!(window.find("search").value(), Some("GPUI 中文 🦀"));
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
fn button_reports_accessibility_name_without_claiming_visible_text(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let handle = cx.add_window(|_, _| NamedButton);
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(window.find("save").label(), Some("Save this document"));
    })
    .unwrap();
}

struct ScrollFocus {
    focus: gpui::FocusHandle,
}
impl Render for ScrollFocus {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        use gpui_component::scroll::ScrollableElement as _;
        div()
            .id("original")
            .test_support()
            .size(px(100.))
            .overflow_y_scrollbar()
            .id("scroll")
            .track_focus(&self.focus)
    }
}
#[gpui_kit::test]
fn scrollable_elements_forward_observed_focus_binding(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let handle = cx.add_window(|_, cx| ScrollFocus {
        focus: cx.focus_handle(),
    });
    let focus = handle.update(cx, |view, _, _| view.focus.clone()).unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        let content = (gpui::ElementId::from("scroll"), "content");
        window.render_frame(cx);
        assert_eq!(
            window.within("scroll").find(content.clone()).focused(),
            Some(false)
        );
        window.focus(&focus, cx);
        window.render_frame(cx);
        assert_eq!(
            window.within("scroll").find(content.clone()).focused(),
            Some(true)
        );
        window.blur(cx);
        window.render_frame(cx);
        assert_eq!(window.within("scroll").find(content).focused(), Some(false));
    })
    .unwrap();
}
