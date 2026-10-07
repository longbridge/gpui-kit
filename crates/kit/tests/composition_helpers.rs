mod common;

use gpui_kit::{
    AppContext, Context, ElementInputHandler, Entity, Render, TestAppContext, Window,
    component::input::{Input, InputState},
    prelude::*,
    test::{TestInput, TestWindowExt},
};

struct Field(Entity<InputState>);

impl Render for Field {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.0).id("field")
    }
}

#[gpui_kit::test]
fn preedit_uses_utf16_and_whole_text_commit_replaces_it(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, field) = common::open_window(cx, None, |window, cx| {
        cx.new(|cx| Field(cx.new(|cx| InputState::new(window, cx))))
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("field", cx);
        let handler =
            ElementInputHandler::new(window.find("field").bounds(), field.read(cx).0.clone());
        let mut input = TestInput::new(handler, window);
        input.commit_text("A🦀Z", cx);
        assert_eq!(input.selected_text_range(cx).unwrap().range, 4..4);
        input.compose_text(Some(1..3), "に🦀", Some(1..3), cx);
        assert_eq!(input.marked_text_range(cx), Some(1..4));
        assert_eq!(input.selected_text_range(cx).unwrap().range, 2..4);
        input.compose_text(None, "日本", Some(2..2), cx);
        input.commit_text("日本語", cx);
        assert_eq!(input.marked_text_range(cx), None);
        assert_eq!(input.selected_text_range(cx).unwrap().range, 4..4);
        drop(input);
        assert_eq!(window.find("field").value(), Some("A日本語Z"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn unmark_retains_text_and_empty_preedit_cancels(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, field) = common::open_window(cx, None, |window, cx| {
        cx.new(|cx| Field(cx.new(|cx| InputState::new(window, cx))))
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("field", cx);
        let handler =
            ElementInputHandler::new(window.find("field").bounds(), field.read(cx).0.clone());
        let mut input = TestInput::new(handler, window);
        input.compose_text(None, "日本", Some(2..2), cx);
        input.unmark_text(cx);
        assert_eq!(input.marked_text_range(cx), None);
        input.compose_text(None, "ni", Some(2..2), cx);
        input.compose_text(None, "", None, cx);
        assert_eq!(input.marked_text_range(cx), None);
        input.commit_text("語", cx);
        drop(input);
        assert_eq!(window.find("field").value(), Some("日本語"));
    })
    .unwrap();
}
