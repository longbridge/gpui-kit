mod common;

use gpui_kit::{
    AppContext, Context, Entity, Focusable, TestAppContext, Window, WindowHandle,
    base::Root,
    component::{
        button::Button,
        input::{AnyInputState, Editor, EditorState, Input, InputState, Textarea, TextareaState},
    },
    div, point,
    prelude::*,
    px, size,
    test::TestWindowExt,
};

const INPUT_IDS: [&str; 3] = ["first", "second", "third"];

struct InputFocus {
    inputs: [Entity<InputState>; 3],
    buttons: bool,
    prefix_clicks: usize,
    suffix_clicks: usize,
}

impl Render for InputFocus {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().flex().flex_col().p_4().gap_4().children(
            INPUT_IDS.into_iter().enumerate().map(|(ix, id)| {
                Input::new(&self.inputs[ix])
                    .id(id)
                    .w_96()
                    .prefix(div().child("Prefix"))
                    .suffix(div().child("Suffix"))
                    .when(self.buttons && id == "second", |input| {
                        input
                            .prefix(Button::new("prefix-button").label("Prefix").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.prefix_clicks += 1;
                                    cx.notify();
                                }),
                            ))
                            .suffix(Button::new("suffix-button").label("Suffix").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.suffix_clicks += 1;
                                    cx.notify();
                                }),
                            ))
                    })
            }),
        )
    }
}

fn open_inputs(cx: &mut TestAppContext, buttons: bool) -> (WindowHandle<Root>, Entity<InputFocus>) {
    cx.update(gpui_kit::init);
    common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
        cx.new(|cx| InputFocus {
            inputs: std::array::from_fn(|_| cx.new(|cx| InputState::new(window, cx))),
            buttons,
            prefix_clicks: 0,
            suffix_clicks: 0,
        })
    })
}

fn press(handle: WindowHandle<Root>, key: &str, cx: &mut TestAppContext) {
    cx.update_window(handle.into(), |_, window, cx| window.press(key, cx))
        .unwrap();
    // Focus subscriptions and deferred actions must settle before checking the
    // destination or typing; otherwise addon focus theft can go unnoticed.
    cx.run_until_parked();
}

fn assert_focus(
    handle: WindowHandle<Root>,
    view: &Entity<InputFocus>,
    expected: &'static str,
    cx: &mut TestAppContext,
) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find(expected).focused(), Some(true), "{expected}");
        for (ix, id) in INPUT_IDS.into_iter().enumerate() {
            assert_eq!(
                view.read(cx).inputs[ix].focus_handle(cx).is_focused(window),
                id == expected,
                "editor focus for {id} when {expected} should be focused",
            );
        }
        if view.read(cx).buttons {
            for id in ["prefix-button", "suffix-button"] {
                assert_eq!(
                    window.find(id).focused(),
                    Some(id == expected),
                    "addon focus for {id} when {expected} should be focused",
                );
            }
        } else {
            for id in INPUT_IDS {
                assert_eq!(window.find(id).focused(), Some(id == expected), "{id}");
            }
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn reverse_tab_cycles_three_inputs_with_passive_addons(cx: &mut TestAppContext) {
    let order = INPUT_IDS;
    let (handle, view) = open_inputs(cx, false);
    cx.update_window(handle.into(), |_, window, cx| window.click(order[2], cx))
        .unwrap();
    cx.run_until_parked();
    assert_focus(handle, &view, order[2], cx);

    // Start with reverse traversal: duplicate entries used to leave the last
    // editor focused on the very first Shift-Tab. Include wraparound and repeat
    // each cycle to catch registrations accumulating across frames.
    for (key, destinations) in [
        ("shift-tab", [order[1], order[0], order[2]]),
        ("tab", order),
    ] {
        for _ in 0..2 {
            for destination in destinations {
                press(handle, key, cx);
                assert_focus(handle, &view, destination, cx);
                cx.update_window(handle.into(), |_, window, cx| window.input("x", cx))
                    .unwrap();
                cx.run_until_parked();
            }
        }
    }

    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        for (ix, id) in INPUT_IDS.into_iter().enumerate() {
            assert_eq!(window.find(id).value(), Some("xxxx"), "{id}");
            assert_eq!(view.read(cx).inputs[ix].read(cx).value(), "xxxx", "{id}");
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn tab_cycles_keep_prefix_and_suffix_buttons_focused(cx: &mut TestAppContext) {
    let (handle, view) = open_inputs(cx, true);
    cx.update_window(handle.into(), |_, window, cx| window.click("third", cx))
        .unwrap();
    cx.run_until_parked();

    for (key, destinations) in [
        (
            "shift-tab",
            ["suffix-button", "second", "prefix-button", "first", "third"],
        ),
        (
            "tab",
            ["first", "prefix-button", "second", "suffix-button", "third"],
        ),
    ] {
        for destination in destinations {
            press(handle, key, cx);
            assert_focus(handle, &view, destination, cx);
            if destination.ends_with("-button") {
                press(handle, "enter", cx);
                assert_focus(handle, &view, destination, cx);
            } else {
                cx.update_window(handle.into(), |_, window, cx| window.input("x", cx))
                    .unwrap();
                cx.run_until_parked();
            }
        }
    }

    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let view = view.read(cx);
        assert_eq!(view.prefix_clicks, 2);
        assert_eq!(view.suffix_clicks, 2);
        for (ix, id) in INPUT_IDS.into_iter().enumerate() {
            assert_eq!(window.find(id).value(), Some("xx"), "{id}");
            assert_eq!(view.inputs[ix].read(cx).value(), "xx", "{id}");
        }
    })
    .unwrap();
}

enum MultilineField {
    Textarea(Entity<TextareaState>),
    Editor(Entity<EditorState>),
}

impl MultilineField {
    fn state(&self) -> AnyInputState {
        match self {
            Self::Textarea(state) => state.clone().into(),
            Self::Editor(state) => state.clone().into(),
        }
    }
}

struct MultilineFocus {
    previous: Entity<InputState>,
    field: MultilineField,
}

impl Render for MultilineFocus {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .p_4()
            .gap_4()
            .child(Input::new(&self.previous).id("previous").w_96())
            .child(match &self.field {
                MultilineField::Textarea(state) => {
                    Textarea::new(state).w_96().h(px(160.)).into_any_element()
                }
                MultilineField::Editor(state) => {
                    Editor::new(state).w_96().h(px(160.)).into_any_element()
                }
            })
    }
}

fn exercise_multiline_body_focus(cx: &mut TestAppContext, code_editor: bool) {
    cx.update(gpui_kit::init);
    let (handle, view) = common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
        cx.new(|cx| MultilineFocus {
            previous: cx.new(|cx| InputState::new(window, cx)),
            field: if code_editor {
                MultilineField::Editor(cx.new(|cx| EditorState::new(window, cx)))
            } else {
                MultilineField::Textarea(cx.new(|cx| TextareaState::new(window, cx)))
            },
        })
    });
    cx.update_window(handle.into(), |_, window, cx| window.click("previous", cx))
        .unwrap();
    cx.run_until_parked();

    let (state, id) = cx
        .update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            let view = view.read(cx);
            let state = view.field.state();
            assert!(view.previous.focus_handle(cx).is_focused(window));
            assert!(!state.focus_handle(cx).is_focused(window));
            let (entity_id, body) = match &view.field {
                MultilineField::Textarea(state) => {
                    (state.entity_id(), state.read(cx).text_bounds())
                }
                MultilineField::Editor(state) => (state.entity_id(), state.read(cx).text_bounds()),
            };
            let id = gpui_kit::ElementId::from(("input", entity_id));
            let body = body.expect("multiline text body must be laid out");
            assert!(body.size.width > px(8.) && body.size.height > px(8.));
            // Use measured text geometry, excluding the frame padding and the
            // code editor gutter. This hits the nested editor focus scope.
            let position = body.origin + point(px(4.), px(4.));
            let offset = position - window.find(id.clone()).bounds().origin;
            window.click_at(id.clone(), offset, cx);
            (state, id)
        })
        .unwrap();
    cx.run_until_parked();

    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(state.focus_handle(cx).is_focused(window));
        assert!(!view.read(cx).previous.focus_handle(cx).is_focused(window));
        assert_eq!(window.find(id.clone()).focused(), Some(true));
        window.input("abcd", cx);
        assert_eq!(state.value(cx), "abcd");
    })
    .unwrap();
    // Delete at the caret, move left, then extend the selection left. The
    // replacement proves that both movement and selection reached the editor.
    press(handle, "backspace", cx);
    cx.update_window(handle.into(), |_, _, cx| assert_eq!(state.value(cx), "abc"))
        .unwrap();
    press(handle, "left", cx);
    press(handle, "shift-left", cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.input("Z", cx);
        assert_eq!(state.value(cx), "aZc");
        assert_eq!(window.find(id.clone()).value(), Some("aZc"));
        assert!(state.focus_handle(cx).is_focused(window));
        // Multiline Shift-Tab is OutdentInline. Exercise the same public
        // traversal primitive Root uses without overriding that key binding.
        window.focus_prev(cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(view.read(cx).previous.focus_handle(cx).is_focused(window));
        assert!(!state.focus_handle(cx).is_focused(window));
        assert_eq!(window.find("previous").focused(), Some(true));
        assert_eq!(window.find(id).focused(), Some(false));
        window.input("previous", cx);
        assert_eq!(view.read(cx).previous.read(cx).value(), "previous");
        assert_eq!(state.value(cx), "aZc");
    })
    .unwrap();
}

#[gpui_kit::test]
fn textarea_body_click_focuses_editor_and_supports_editing(cx: &mut TestAppContext) {
    exercise_multiline_body_focus(cx, false);
}

#[gpui_kit::test]
fn editor_body_click_focuses_editor_and_supports_editing(cx: &mut TestAppContext) {
    exercise_multiline_body_focus(cx, true);
}
