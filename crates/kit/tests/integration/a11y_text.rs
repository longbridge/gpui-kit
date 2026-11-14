use crate::common;
use gpui_kit::{AppContext, Context, TestAppContext, Window, component::label::Label, div, prelude::*};

/// The Quick Start renders its greeting with `Label`, which exposes its text in
/// the accessibility tree. GPUI only adds elements with an ID to the tree, so a
/// bare string child stays invisible to screen readers.
struct Greeting;
impl Render for Greeting {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(Label::new("Hello, World!"))
            .child("Bare string")
    }
}

#[gpui_kit::test]
fn quick_start_text_is_in_the_accessibility_tree(cx: &mut TestAppContext) {
    // GPUI Fast's test window builds accessibility data only when requested.
    // Re-run in a child so the cached flag is set before any frame, without
    // changing the process environment during parallel tests.
    if std::env::var("GPUI_A11Y_ACTIVE").as_deref() != Ok("1") {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "a11y_text::quick_start_text_is_in_the_accessibility_tree",
                "--nocapture",
            ])
            .env("GPUI_A11Y_ACTIVE", "1")
            .status()
            .unwrap();
        assert!(
            status.success(),
            "accessibility tree check failed: {status}"
        );
        return;
    }

    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, None, |_, cx| cx.new(|_| Greeting));
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        let tree = window.debug_a11y_tree_json().expect("accessibility tree");
        assert!(tree.contains("\"Hello, World!\""), "{tree}");
        assert!(!tree.contains("Bare string"), "{tree}");
    })
    .unwrap();
}
