//! Shared contracts at the styled Input, Textarea and Editor boundaries.
use gpui_kit::{
    App, AppContext, ClipboardItem, Context, ElementId, ElementInputHandler, Entity, InputHandler,
    Render, Subscription, TestAppContext, Window, WindowHandle,
    base::Root,
    component::input::{
        Editor, EditorState, Input, InputEvent, InputState, Textarea, TextareaState,
    },
    div,
    prelude::*,
    px, size,
    test::TestWindowExt,
};

pub(super) struct Fields {
    input: Entity<InputState>,
    textarea: Entity<TextareaState>,
    editor: Entity<EditorState>,
    mounted: bool,
    readonly: bool,
    disabled: bool,
    revision: usize,
    focus_events: [[usize; 2]; 3],
    _subscriptions: Vec<Subscription>,
}

impl Fields {
    // GPUI does not expose the installed platform handler. This public bridge
    // exercises the same protocol against the rendered control's retained state.
    pub(super) fn handler(&self, ix: usize, window: &Window) -> Box<dyn InputHandler> {
        let bounds = window.find(self.ids()[ix].clone()).bounds();
        match ix {
            0 => Box::new(ElementInputHandler::new(bounds, self.input.clone())),
            1 => Box::new(ElementInputHandler::new(bounds, self.textarea.clone())),
            2 => Box::new(ElementInputHandler::new(bounds, self.editor.clone())),
            _ => unreachable!(),
        }
    }

    fn record_focus(&mut self, ix: usize, event: &InputEvent) {
        match event {
            InputEvent::Focus => self.focus_events[ix][0] += 1,
            InputEvent::Blur => self.focus_events[ix][1] += 1,
            _ => {}
        }
    }

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
                this.child(
                    Input::new(&self.input)
                        .readonly(self.readonly)
                        .disabled(self.disabled),
                )
                .child(
                    Textarea::new(&self.textarea)
                        .h_24()
                        .readonly(self.readonly)
                        .disabled(self.disabled),
                )
                .child(
                    Editor::new(&self.editor)
                        .h_24()
                        .readonly(self.readonly)
                        .disabled(self.disabled),
                )
            })
    }
}

pub(super) fn mount(
    cx: &mut TestAppContext,
) -> (WindowHandle<Root>, Entity<Fields>, [ElementId; 3]) {
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
                disabled: false,
                revision: 0,
                focus_events: [[0; 2]; 3],
                _subscriptions: Vec::new(),
            })
        });
    fields.update(cx, |fields, cx| {
        fields._subscriptions = vec![
            cx.subscribe(&fields.input, |fields, _, event, _| {
                fields.record_focus(0, event)
            }),
            cx.subscribe(&fields.textarea, |fields, _, event, _| {
                fields.record_focus(1, event)
            }),
            cx.subscribe(&fields.editor, |fields, _, event, _| {
                fields.record_focus(2, event)
            }),
        ];
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
    for (ix, id) in ids.into_iter().enumerate() {
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
            cx.write_to_clipboard(ClipboardItem::new_string(format!("uncopied-{ix}")));
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
fn focus_and_blur_events_fire_once_per_transition_across_all_controls(cx: &mut TestAppContext) {
    let (handle, fields, ids) = mount(cx);
    // GPUI delivers focus/blur subscriptions only for an active platform window.
    cx.update_window(handle.into(), |_, window, _| window.activate_window())
        .unwrap();
    cx.run_until_parked();
    let mut expected = [[0; 2]; 3];
    for ix in [0, 1, 2, 0] {
        cx.update_window(handle.into(), |_, window, cx| {
            window.click(ids[ix].clone(), cx);
        })
        .unwrap();
        cx.run_until_parked();
        expected[ix][0] += 1;
        assert_eq!(
            fields.read_with(cx, |fields, _| fields.focus_events),
            expected
        );

        cx.update_window(handle.into(), |_, window, cx| {
            window.click(ids[ix].clone(), cx);
            fields.update(cx, |fields, cx| {
                fields.revision += 1;
                cx.notify();
            });
            window.render_frame(cx);
            assert_eq!(window.find(ids[ix].clone()).focused(), Some(true));
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(
            fields.read_with(cx, |fields, _| fields.focus_events),
            expected
        );

        cx.update_window(handle.into(), |_, window, cx| window.blur(cx))
            .unwrap();
        cx.run_until_parked();
        expected[ix][1] += 1;
        assert_eq!(
            fields.read_with(cx, |fields, _| fields.focus_events),
            expected
        );
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            for id in &ids {
                assert_eq!(window.find(id.clone()).focused(), Some(false));
            }
            let retained = fields.read(cx).values(cx);
            window.input("unfocused", cx);
            assert_eq!(fields.read(cx).values(cx), retained);
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn disabling_focused_controls_blocks_edits_and_click_focus_until_reenabled(
    cx: &mut TestAppContext,
) {
    let (handle, fields, ids) = mount(cx);
    for id in ids {
        cx.update_window(handle.into(), |_, window, cx| {
            window.click(id.clone(), cx);
            window.input("saved🦀", cx);
            select_all(window, cx);
            fields.update(cx, |fields, cx| {
                fields.disabled = true;
                cx.notify();
            });
            window.render_frame(cx);
            window.input("blocked", cx);
            window.press("backspace", cx);
            cx.write_to_clipboard(ClipboardItem::new_string("blocked paste".into()));
            window.press(
                if cfg!(target_os = "macos") {
                    "cmd-v"
                } else {
                    "ctrl-v"
                },
                cx,
            );
            assert_eq!(window.find(id.clone()).value(), Some("saved🦀"));

            window.blur(cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, window, cx| {
            window.click(id.clone(), cx);
            assert_eq!(window.find(id.clone()).focused(), Some(false));
            window.input("still blocked", cx);
            assert_eq!(window.find(id.clone()).value(), Some("saved🦀"));
            fields.update(cx, |fields, cx| {
                fields.disabled = false;
                cx.notify();
            });
            window.render_frame(cx);
            window.click(id.clone(), cx);
            select_all(window, cx);
            window.input("enabled", cx);
            assert_eq!(window.find(id.clone()).focused(), Some(true));
            assert_eq!(window.find(id).value(), Some("enabled"));
        })
        .unwrap();
    }
    assert_eq!(
        fields.read_with(cx, |fields, cx| fields.values(cx)),
        ["enabled"; 3]
    );
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
            let retained = fields.read(cx).values(cx);
            window.input("orphan", cx);
            assert_eq!(fields.read(cx).values(cx), retained);
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

#[gpui_kit::test]
fn closing_window_releases_input_textarea_and_editor_states(cx: &mut TestAppContext) {
    let (handle, fields, ids) = mount(cx);
    let (input, textarea, editor) = fields.read_with(cx, |fields, _| {
        (
            fields.input.downgrade(),
            fields.textarea.downgrade(),
            fields.editor.downgrade(),
        )
    });
    cx.update_window(handle.into(), |_, window, cx| {
        for id in ids {
            window.click(id, cx);
            window.input("retained until close", cx);
        }
    })
    .unwrap();
    cx.run_until_parked();
    // Release the fixture's strong owner before closing the window. GPUI
    // disposes dropped entities while flushing an app update; draining the
    // executor alone does not dispose a parent dropped after that update.
    drop(fields);
    cx.update_window(handle.into(), |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
    assert!(
        input.upgrade().is_none(),
        "closed Input state must be released"
    );
    assert!(
        textarea.upgrade().is_none(),
        "closed Textarea state must be released"
    );
    assert!(
        editor.upgrade().is_none(),
        "closed Editor state must be released"
    );
}
