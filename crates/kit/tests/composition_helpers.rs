mod common;

use gpui_kit::{
    App, AppContext, Context, ElementId, ElementInputHandler, Entity, Render, TestAppContext,
    Window,
    component::input::{Editor, EditorState, Input, InputState, Textarea, TextareaState},
    div,
    prelude::*,
    px,
    test::{TestInput, TestWindowExt},
};

struct Field(Entity<InputState>);

impl Render for Field {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.0).id("field")
    }
}

enum MultilineState {
    Textarea(Entity<TextareaState>),
    Editor(Entity<EditorState>),
}

impl MultilineState {
    fn id(&self) -> ElementId {
        match self {
            Self::Textarea(state) => ("input", state.entity_id()).into(),
            Self::Editor(state) => ("input", state.entity_id()).into(),
        }
    }

    fn input<'a>(&self, window: &'a mut Window) -> TestInput<'a> {
        let bounds = window.find(self.id()).bounds();
        match self {
            Self::Textarea(state) => {
                TestInput::new(ElementInputHandler::new(bounds, state.clone()), window)
            }
            Self::Editor(state) => {
                TestInput::new(ElementInputHandler::new(bounds, state.clone()), window)
            }
        }
    }
}

struct MultilineField {
    state: MultilineState,
    readonly: bool,
}

impl Render for MultilineField {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(match &self.state {
            MultilineState::Textarea(state) => Textarea::new(state)
                .readonly(self.readonly)
                .h(px(160.))
                .into_any_element(),
            MultilineState::Editor(state) => Editor::new(state)
                .readonly(self.readonly)
                .h(px(160.))
                .into_any_element(),
        })
    }
}

fn undo(window: &mut Window, cx: &mut App) {
    window.press(
        if cfg!(target_os = "macos") {
            "cmd-z"
        } else {
            "ctrl-z"
        },
        cx,
    );
}

#[gpui_kit::test]
fn multiline_composition_undo_restores_backward_unicode_selection(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    for editor in [false, true] {
        let (handle, field) = common::open_window(cx, None, |window, cx| {
            cx.new(|cx| MultilineField {
                state: if editor {
                    MultilineState::Editor(
                        cx.new(|cx| EditorState::new(window, cx).default_value("first\nA🦀")),
                    )
                } else {
                    MultilineState::Textarea(
                        cx.new(|cx| TextareaState::new(window, cx).default_value("first\nA🦀")),
                    )
                },
                readonly: false,
            })
        });
        cx.update_window(handle.into(), |_, window, cx| {
            let id = field.read(cx).state.id();
            window.click(id.clone(), cx);
            window.press(
                if cfg!(target_os = "macos") {
                    "cmd-down"
                } else {
                    "ctrl-end"
                },
                cx,
            );
            window.press("shift-left", cx);
            let mut input = field.read(cx).state.input(window);
            let before = input.selected_text_range(cx).unwrap();
            assert_eq!(before.range, 7..9);
            assert!(before.reversed);
            input.compose_text(None, "に🦀", Some(1..3), cx);
            assert_eq!(input.marked_text_range(cx), Some(7..10));
            input.commit_text("日本語", cx);
            assert_eq!(input.marked_text_range(cx), None);
            drop(input);
            assert_eq!(window.find(id.clone()).value(), Some("first\nA日本語"));
            undo(window, cx);
            assert_eq!(window.find(id.clone()).value(), Some("first\nA🦀"));
            let mut input = field.read(cx).state.input(window);
            let restored = input.selected_text_range(cx).unwrap();
            assert_eq!(restored.range, before.range);
            assert_eq!(restored.reversed, before.reversed);
            input.commit_text("語", cx);
            drop(input);
            assert_eq!(window.find(id).value(), Some("first\nA語"));
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn readonly_multiline_rejects_composition_without_consuming_committed_history(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    for editor in [false, true] {
        let (handle, field) = common::open_window(cx, None, |window, cx| {
            cx.new(|cx| MultilineField {
                state: if editor {
                    MultilineState::Editor(cx.new(|cx| EditorState::new(window, cx)))
                } else {
                    MultilineState::Textarea(cx.new(|cx| TextareaState::new(window, cx)))
                },
                readonly: false,
            })
        });
        cx.update_window(handle.into(), |_, window, cx| {
            let id = field.read(cx).state.id();
            window.click(id.clone(), cx);
            field
                .read(cx)
                .state
                .input(window)
                .commit_text("first\n🦀", cx);
            field.update(cx, |field, cx| {
                field.readonly = true;
                cx.notify();
            });
            window.render_frame(cx);
            let mut input = field.read(cx).state.input(window);
            let before = input.selected_text_range(cx).unwrap();
            input.compose_text(Some(6..8), "日本", Some(2..2), cx);
            assert_eq!(input.marked_text_range(cx), None);
            input.commit_text("語", cx);
            let after = input.selected_text_range(cx).unwrap();
            assert_eq!(after.range, before.range);
            assert_eq!(after.reversed, before.reversed);
            drop(input);
            undo(window, cx);
            assert_eq!(window.find(id.clone()).value(), Some("first\n🦀"));
            field.update(cx, |field, cx| {
                field.readonly = false;
                cx.notify();
            });
            window.render_frame(cx);
            undo(window, cx);
            assert_eq!(window.find(id).value(), Some(""));
        })
        .unwrap();
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
