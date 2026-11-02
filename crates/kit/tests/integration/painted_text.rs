use crate::common;
use gpui_component::{avatar::Avatar, button::Button, kbd::Kbd, label::Label, text::TextView};
use gpui_kit::base::test_support::texts;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, Context, Keystroke, TestAppContext, Window, div, prelude::*, px, size};

struct Painted;
impl Render for Painted {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("painted")
            .flex()
            .flex_col()
            .child(Label::new("Hello"))
            .child(Kbd::new(Keystroke::parse("ctrl-s").unwrap()))
            .child(Avatar::new().name("Jane Doe"))
            .child(
                Button::new("save")
                    .label("Save")
                    .tooltip("Write the file to disk"),
            )
            .child(TextView::markdown("doc", "A painted paragraph."))
    }
}

#[gpui_kit::test]
fn painted_text_and_alternative_text_are_observed(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let (handle, _) = common::open_window(cx, Some(size(px(600.), px(500.))), |_, cx| {
        cx.new(|_| Painted)
    });
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        let texts = texts(window);
        let find = |text: &str| {
            texts
                .iter()
                .find(|snapshot| snapshot.text() == text)
                .unwrap_or_else(|| panic!("{text:?} was not painted: {texts:?}"))
        };

        let label = find("Hello");
        assert!(label.visible());
        assert!(!label.is_alternative());
        assert!(label.path().contains(&"painted".into()));
        find(&Kbd::format(&Keystroke::parse("ctrl-s").unwrap()));
        find("A painted paragraph.");
        assert!(find("Jane Doe").is_alternative());

        let save = window.find("save");
        assert_eq!(save.description(), Some("Write the file to disk"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn each_frame_replaces_the_text_of_the_last(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let (handle, _) = common::open_window(cx, Some(size(px(600.), px(500.))), |_, cx| {
        cx.new(|_| Painted)
    });
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        // GPUI Fast draws an unchanged Root again from its last frame without
        // rendering it, so it would not start the frame. Like a cached view,
        // a retained one records no text.
        #[cfg(feature = "gpui-fast")]
        window.set_view_retention(false);
        window.draw(cx).clear(cx);
        let first = texts(window).len();
        assert!(first > 0);
        window.refresh();
        window.draw(cx).clear(cx);
        assert_eq!(texts(window).len(), first);
    })
    .unwrap();
}
