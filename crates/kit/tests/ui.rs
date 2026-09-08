use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{
    AppContext, Context, Entity, SharedString, TestAppContext, Window,
    component::{
        Root,
        button::Button,
        input::{Input, InputState},
    },
    div,
    prelude::*,
    px, size,
};

struct Profile {
    name: Entity<InputState>,
    submitted: Option<SharedString>,
}

impl Render for Profile {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.submitted.as_ref().map_or_else(
            || SharedString::from("Not saved"),
            |name| SharedString::from(format!("Saved: {name}")),
        );
        div()
            .size_full()
            .flex()
            .flex_col()
            .p_4()
            .gap_4()
            .child(Input::new(&self.name).id("name").w(px(240.)))
            .child(
                Button::new("save")
                    .label("Save")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.submitted = Some(this.name.read(cx).value());
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("status")
                    .role(gpui_kit::Role::Status)
                    .test_support()
                    .aria_label(status.clone())
                    .child(status),
            )
    }
}

#[gpui_kit::test]
fn saves_a_profile_through_the_ui(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut profile = None;
    let handle = cx.open_window(size(px(640.), px(480.)), |window, cx| {
        let view = cx.new(|cx| Profile {
            name: cx.new(|cx| InputState::new(window, cx)),
            submitted: None,
        });
        profile = Some(view.clone());
        Root::new(view, window, cx)
    });
    let profile = profile.unwrap();

    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("status").label(), Some("Not saved"));

        window.click("name", cx);
        window.input("Ada 中文", cx);
        let name = window.find("name");
        assert_eq!(name.focused(), Some(true));
        assert_eq!(name.value(), Some("Ada 中文"));
        assert!(name.bounds().size.width > px(0.));
        // Named keys share the same Window API and refresh the resulting frame.
        window.press("backspace", cx);
        assert_eq!(window.find("name").value(), Some("Ada 中"));

        window.click("save", cx);
        let status = window.find("status");
        assert!(status.visible());
        assert_eq!(status.label(), Some("Saved: Ada 中"));
        assert!(status.bounds().top() >= window.find("save").bounds().bottom());
    })
    .unwrap();

    // Verify the application result as well as the native properties.
    cx.update(|cx| {
        assert_eq!(profile.read(cx).submitted.as_deref(), Some("Ada 中"));
    });
}
