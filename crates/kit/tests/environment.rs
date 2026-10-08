mod common;

use gpui_kit::{
    AppContext, Context, Entity, Render, TestAppContext, Window,
    component::{
        button::Button,
        input::{Input, InputState},
        link::Link,
    },
    div,
    prelude::*,
    px, size,
    test::{TestSupportExt, TestWindowExt},
};

struct Editor {
    input: Entity<InputState>,
    saves: usize,
}

impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .p_4()
            .gap_4()
            .child(Input::new(&self.input).id("editor").w_full())
            .child(
                Button::new("save")
                    .label("Save")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.saves += 1;
                        cx.notify();
                    })),
            )
    }
}

#[gpui_kit::test]
fn resize_and_display_scale_keep_input_state_and_pointer_targets(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, editor) = common::open_window(cx, Some(size(px(640.), px(360.))), |window, cx| {
        cx.new(|cx| Editor {
            input: cx.new(|cx| InputState::new(window, cx)),
            saves: 0,
        })
    });
    let original_width = cx
        .update_window(handle.into(), |_, window, cx| {
            window.click("editor", cx);
            window.input("草稿 🦀", cx);
            window.find("editor").bounds().size.width
        })
        .unwrap();

    for (width, scale) in [(320., 2.), (800., 1.25), (640., 1.)] {
        cx.simulate_window_resize(handle.into(), size(px(width), px(360.)));
        cx.simulate_window_scale_factor_change(handle.into(), scale);
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(window.viewport_size(), size(px(width), px(360.)));
            assert_eq!(window.scale_factor(), scale);
            let input = window.find("editor");
            assert_eq!(input.value(), Some("草稿 🦀"));
            assert_eq!(input.focused(), Some(true));
            assert_eq!(input.bounds().size.width, original_width + px(width - 640.));
            assert!(input.visible());
            window.click("save", cx);
            window.click("editor", cx);
        })
        .unwrap();
    }
    cx.update(|cx| assert_eq!(editor.read(cx).saves, 3));
}

#[gpui_kit::test]
fn clipboard_transfers_between_windows_without_sharing_editor_state(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (source, source_editor) =
        common::open_window(cx, Some(size(px(400.), px(200.))), |window, cx| {
            cx.new(|cx| Editor {
                input: cx.new(|cx| InputState::new(window, cx)),
                saves: 0,
            })
        });
    let (destination, destination_editor) =
        common::open_window(cx, Some(size(px(400.), px(200.))), |window, cx| {
            cx.new(|cx| Editor {
                input: cx.new(|cx| InputState::new(window, cx)),
                saves: 0,
            })
        });
    cx.update_window(source.into(), |_, window, cx| {
        window.click("editor", cx);
        window.input("共享文字 🦀", cx);
        window.press("secondary-a", cx);
        window.press("secondary-c", cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("共享文字 🦀")
        );
    })
    .unwrap();
    cx.update_window(destination.into(), |_, window, cx| {
        window.click("editor", cx);
        window.press("secondary-v", cx);
        assert_eq!(window.find("editor").value(), Some("共享文字 🦀"));
        window.press("secondary-a", cx);
        window.press("secondary-x", cx);
        assert_eq!(window.find("editor").value(), Some(""));
    })
    .unwrap();
    cx.update(|cx| {
        assert_eq!(source_editor.read(cx).input.read(cx).value(), "共享文字 🦀");
        assert_eq!(destination_editor.read(cx).input.read(cx).value(), "");
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("共享文字 🦀")
        );
    });
    cx.update_window(source.into(), |_, window, cx| {
        window.press("secondary-v", cx);
        assert_eq!(window.find("editor").value(), Some("共享文字 🦀"));
    })
    .unwrap();
}

struct DocumentationLink {
    activations: usize,
}

impl Render for DocumentationLink {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p_4().child(
            div()
                .id("documentation-target")
                .test_support()
                .w(px(160.))
                .h(px(32.))
                .child(
                    Link::new("documentation-link")
                        .href("https://gpui-kit.com/docs/testing")
                        .size_full()
                        .child("Testing documentation")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.activations += 1;
                            cx.notify();
                        })),
                ),
        )
    }
}

#[gpui_kit::test]
fn link_pointer_activation_opens_url_and_notifies_owner(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, link) = common::open_window(cx, Some(size(px(320.), px(120.))), |_, cx| {
        cx.new(|_| DocumentationLink { activations: 0 })
    });
    assert_eq!(cx.opened_url(), None);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("documentation-target", cx);
    })
    .unwrap();
    assert_eq!(
        cx.opened_url().as_deref(),
        Some("https://gpui-kit.com/docs/testing")
    );
    cx.update(|cx| assert_eq!(link.read(cx).activations, 1));
}
