//! Shared contracts at the styled Input, Textarea and Editor boundaries.
use gpui_kit::{
    App, AppContext, Context, ElementId, Entity, Render, TestAppContext, Window, WindowHandle,
    base::Root,
    component::input::{Editor, EditorState, Input, InputState, Textarea, TextareaState},
    div,
    prelude::*,
    px, size,
    test::TestWindowExt,
};

struct Fields {
    input: Entity<InputState>,
    textarea: Entity<TextareaState>,
    editor: Entity<EditorState>,
    mounted: bool,
    readonly: bool,
    revision: usize,
}

impl Fields {
    fn ids(&self) -> [ElementId; 3] {
        [
            ("input", self.input.entity_id()).into(),
            ("input", self.textarea.entity_id()).into(),
            ("input", self.editor.entity_id()).into(),
        ]
    }

    fn values(&self, cx: &App) -> [String; 3] {
        [
            self.input.read(cx).value().to_string(),
            self.textarea.read(cx).value().to_string(),
            self.editor.read(cx).value().to_string(),
        ]
    }
}

impl Render for Fields {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_2()
            .child(format!("Revision {}", self.revision))
            .when(self.mounted, |this| {
                this.child(Input::new(&self.input).readonly(self.readonly))
                    .child(Textarea::new(&self.textarea).h_24().readonly(self.readonly))
                    .child(Editor::new(&self.editor).h_24().readonly(self.readonly))
            })
    }
}

fn mount(cx: &mut TestAppContext) -> (WindowHandle<Root>, Entity<Fields>, [ElementId; 3]) {
    cx.update(gpui_kit::init);
    // Fixed window bounds are the test's viewport, not production control styling.
    let (window, fields) =
        crate::common::open_window(cx, Some(size(px(480.), px(480.))), |window, cx| {
            cx.new(|cx| Fields {
                input: cx.new(|cx| InputState::new(window, cx)),
                textarea: cx.new(|cx| TextareaState::new(window, cx)),
                editor: cx.new(|cx| EditorState::new(window, cx)),
                mounted: true,
                readonly: false,
                revision: 0,
            })
        });
    let ids = fields.read_with(cx, |fields, _| fields.ids());
    (window, fields, ids)
}

fn select_all(window: &mut Window, cx: &mut App) {
    window.press(
        if cfg!(target_os = "macos") {
            "cmd-a"
        } else {
            "ctrl-a"
        },
        cx,
    );
}

#[gpui_kit::test]
fn switching_between_input_textarea_and_editor_routes_text_to_current_focus(
    cx: &mut TestAppContext,
) {
    let (handle, fields, ids) = mount(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        for (ix, id) in ids.iter().enumerate() {
            window.click(id.clone(), cx);
            window.input(["name", "正文", "code"][ix], cx);
            for (other, other_id) in ids.iter().enumerate() {
                assert_eq!(window.find(other_id.clone()).focused(), Some(other == ix));
            }
        }
        assert_eq!(fields.read(cx).values(cx), ["name", "正文", "code"]);
        for (id, expected) in ids.iter().zip(["name", "正文", "code"]) {
            assert_eq!(window.find(id.clone()).value(), Some(expected));
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn parent_rerender_preserves_each_controls_focus_and_unicode_selection(cx: &mut TestAppContext) {
    let (handle, fields, ids) = mount(cx);
    for id in ids {
        cx.update_window(handle.into(), |_, window, cx| {
            window.click(id.clone(), cx);
            window.input("A🦀", cx);
            window.press("shift-left", cx);
            fields.update(cx, |fields, cx| {
                fields.revision += 1;
                cx.notify();
            });
            window.render_frame(cx);
            assert_eq!(window.find(id.clone()).focused(), Some(true));
            window.input("z", cx);
            assert_eq!(window.find(id).value(), Some("Az"));
        })
        .unwrap();
    }
    fields.read_with(cx, |fields, cx| {
        assert_eq!(fields.values(cx), ["Az", "Az", "Az"])
    });
}

#[gpui_kit::test]
fn readonly_transition_keeps_selection_copyable_and_reenable_restores_editing(
    cx: &mut TestAppContext,
) {
    let (handle, fields, ids) = mount(cx);
    for id in ids {
        cx.update_window(handle.into(), |_, window, cx| {
            window.click(id.clone(), cx);
            window.input("retained", cx);
            select_all(window, cx);
            fields.update(cx, |fields, cx| {
                fields.readonly = true;
                cx.notify();
            });
            window.render_frame(cx);
            window.input("rejected", cx);
            window.press("backspace", cx);
            window.press(
                if cfg!(target_os = "macos") {
                    "cmd-c"
                } else {
                    "ctrl-c"
                },
                cx,
            );
            assert_eq!(
                cx.read_from_clipboard().and_then(|item| item.text()),
                Some("retained".into())
            );
            assert_eq!(window.find(id.clone()).value(), Some("retained"));
            assert_eq!(window.find(id.clone()).focused(), Some(true));
            fields.update(cx, |fields, cx| {
                fields.readonly = false;
                cx.notify();
            });
            window.render_frame(cx);
            window.input("editable", cx);
            assert_eq!(window.find(id).value(), Some("editable"));
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn unmounting_focused_controls_removes_targets_and_remount_keeps_retained_values(
    cx: &mut TestAppContext,
) {
    let (handle, fields, ids) = mount(cx);
    for id in ids.iter() {
        cx.update_window(handle.into(), |_, window, cx| {
            window.click(id.clone(), cx);
            select_all(window, cx);
            window.input("saved", cx);
            fields.update(cx, |fields, cx| {
                fields.mounted = false;
                cx.notify();
            });
            window.render_frame(cx);
            for id in &ids {
                assert!(window.try_find(id.clone()).is_none());
            }
            window.input("orphan", cx);
            fields.update(cx, |fields, cx| {
                fields.mounted = true;
                cx.notify();
            });
            window.render_frame(cx);
            assert_eq!(window.find(id.clone()).value(), Some("saved"));
            window.click(id.clone(), cx);
            select_all(window, cx);
            window.input("restored", cx);
            assert_eq!(window.find(id.clone()).value(), Some("restored"));
        })
        .unwrap();
    }
    fields.read_with(cx, |fields, cx| {
        assert_eq!(fields.values(cx), ["restored", "restored", "restored"])
    });
}
