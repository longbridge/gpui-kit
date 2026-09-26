//! Application-facing constraints exercised through the rendered input.
//! GPUI's `Window::handle_a11y_action` is private: native SetValue dispatch
//! cannot currently be covered through the public test API.

use crate::common;
use gpui_kit::{
    App, AppContext, ClipboardItem, Context, Entity, FocusHandle, Focusable, Subscription,
    TestAppContext, Window, WindowHandle,
    base::Root,
    component::input::{Input, InputContentType, InputEvent, InputState},
    div, point,
    prelude::*,
    test::TestWindowExt,
};

use std::{cell::RefCell, rc::Rc};

struct Constraints {
    input: Entity<InputState>,
    previous_focus: Option<FocusHandle>,
    readonly: bool,
    disabled: bool,
    cleanable: bool,
    mask_toggle: bool,
    content_type: Option<InputContentType>,
    changes: usize,
    consume_paste: Option<bool>,
    paste_payloads: Rc<RefCell<Vec<String>>>,
    _subscription: Subscription,
}

impl Render for Constraints {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .when_some(self.previous_focus.as_ref(), |this, focus| {
                this.track_focus(focus)
            })
            .child(
                Input::new(&self.input)
                    .id("constrained")
                    .w_64()
                    .readonly(self.readonly)
                    .disabled(self.disabled)
                    .cleanable(self.cleanable)
                    .when_some(self.consume_paste, |input, consume| {
                        let payloads = self.paste_payloads.clone();
                        input.on_paste(move |item, _, _| {
                            payloads.borrow_mut().push(item.text().unwrap_or_default());
                            consume
                        })
                    })
                    .when(self.mask_toggle, |input| input.mask_toggle())
                    .when_some(self.content_type, |input, kind| input.content_type(kind)),
            )
    }
}

fn fixture(
    cx: &mut TestAppContext,
    configure: impl FnOnce(InputState) -> InputState,
) -> (WindowHandle<Root>, Entity<Constraints>) {
    cx.update(gpui_kit::init);
    common::open_window(cx, None, |window, cx| {
        cx.new(|cx| {
            let input = cx.new(|cx| configure(InputState::new(window, cx)));
            let subscription = cx.subscribe(&input, |this: &mut Constraints, _, event, _| {
                if matches!(event, InputEvent::Change) {
                    this.changes += 1;
                }
            });
            Constraints {
                input,
                previous_focus: None,
                readonly: false,
                disabled: false,
                cleanable: false,
                mask_toggle: false,
                content_type: None,
                changes: 0,
                consume_paste: None,
                paste_payloads: Rc::default(),
                _subscription: subscription,
            }
        })
    })
}

fn ui(
    handle: WindowHandle<Root>,
    cx: &mut TestAppContext,
    action: impl FnOnce(&mut Window, &mut App),
) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        action(window, cx);
    })
    .unwrap();
}

fn shortcut(window: &mut Window, key: &str, cx: &mut App) {
    let modifier = if cfg!(target_os = "macos") {
        "cmd"
    } else {
        "ctrl"
    };
    window.press(&format!("{modifier}-{key}"), cx);
}

fn clipboard(cx: &mut App, value: &str) {
    cx.write_to_clipboard(ClipboardItem::new_string(value.to_owned()));
}

fn assert_clipboard(cx: &mut App, expected: &str) {
    assert_eq!(
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .as_deref(),
        Some(expected)
    );
}

fn assert_owner(
    handle: WindowHandle<Root>,
    view: &Entity<Constraints>,
    cx: &mut TestAppContext,
    expected: &str,
    changes: usize,
) {
    // Leave the input's update before inspecting subscription effects.
    common::update_content(handle, view, cx, |view, _, cx| {
        assert_eq!(view.input.read(cx).value(), expected);
        assert_eq!(view.changes, changes);
    })
    .unwrap();
}

fn protected_after_focus(cx: &mut TestAppContext, disabled: bool) {
    let (handle, view) = fixture(cx, |input| input);
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        clipboard(cx, "edit");
        shortcut(window, "v", cx);
        shortcut(window, "a", cx);
        assert_eq!(window.find("constrained").focused(), Some(true));
    });
    assert_owner(handle, &view, cx, "edit", 1);
    common::update_content(handle, &view, cx, |view, _, cx| {
        assert_eq!(view.input.read(cx).selected_range(), 0..4);
        view.disabled = disabled;
        view.readonly = !disabled;
        cx.notify();
    })
    .unwrap();

    for key in ["v", "x", "backspace", "delete", "z"] {
        ui(handle, cx, |window, cx| {
            clipboard(cx, "sentinel");
            if key.len() == 1 {
                shortcut(window, key, cx);
            } else {
                window.press(key, cx);
            }
            assert_eq!(window.find("constrained").value(), Some("edit"));
            assert_clipboard(cx, "sentinel");
        });
        assert_owner(handle, &view, cx, "edit", 1);
    }
    ui(handle, cx, |window, cx| window.input("ignored", cx));
    assert_owner(handle, &view, cx, "edit", 1);
    common::update_content(handle, &view, cx, |view, _, cx| {
        assert_eq!(view.input.read(cx).selected_range(), 0..4);
        view.disabled = false;
        view.readonly = false;
        cx.notify();
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        shortcut(window, "z", cx);
        assert_eq!(window.find("constrained").value(), Some(""));
        window.input("R", cx);
        assert_eq!(window.find("constrained").value(), Some("R"));
    });
    assert_owner(handle, &view, cx, "R", 3);
}

#[gpui_kit::test]
fn readonly_after_focus_rejects_edits_without_consuming_history(cx: &mut TestAppContext) {
    protected_after_focus(cx, false);
}

#[gpui_kit::test]
fn disabled_after_focus_rejects_edits_without_consuming_history(cx: &mut TestAppContext) {
    protected_after_focus(cx, true);
}

#[gpui_kit::test]
fn readonly_allows_selection_copy_and_silent_owner_replacement(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| input.default_value("initial"));
    ui(handle, cx, |window, cx| window.click("constrained", cx));
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.readonly = true;
        cx.notify();
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        shortcut(window, "a", cx);
        shortcut(window, "c", cx);
        assert_clipboard(cx, "initial");
        assert_eq!(window.find("constrained").focused(), Some(true));
    });
    for (disabled, expected) in [(false, "owner"), (true, "disabled owner")] {
        common::update_content(handle, &view, cx, |view, _, cx| {
            view.disabled = disabled;
            view.readonly = !disabled;
            cx.notify();
        })
        .unwrap();
        // Apply the owner flags to the rendered component before replacing text.
        ui(handle, cx, |_, _| {});
        common::update_content(handle, &view, cx, |view, window, cx| {
            view.input
                .update(cx, |input, cx| input.set_value(expected, window, cx));
        })
        .unwrap();
        ui(handle, cx, |window, cx| {
            assert_eq!(window.find("constrained").value(), Some(expected));
            window.input("ignored", cx);
        });
        assert_owner(handle, &view, cx, expected, 0);
    }
}

#[gpui_kit::test]
fn masking_blocks_clipboard_until_revealed_and_preserves_the_value(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| input.masked(true));
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.mask_toggle = true;
        cx.notify();
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        clipboard(cx, "sëcret🦀");
        shortcut(window, "v", cx);
        shortcut(window, "a", cx);
        clipboard(cx, "sentinel");
        for key in ["c", "x"] {
            shortcut(window, key, cx);
            assert_clipboard(cx, "sentinel");
            assert_eq!(window.find("constrained").value(), None);
        }
        window.within("constrained").click("toggle-mask", cx);
        assert_eq!(window.find("constrained").value(), Some("sëcret🦀"));
        window.click("constrained", cx);
        shortcut(window, "a", cx);
        shortcut(window, "c", cx);
        assert_clipboard(cx, "sëcret🦀");
        window.within("constrained").click("toggle-mask", cx);
        assert_eq!(window.find("constrained").value(), None);
    });
    assert_owner(handle, &view, cx, "sëcret🦀", 1);
}

#[gpui_kit::test]
fn password_content_types_hide_accessibility_values_even_when_revealed(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| input.default_value("secret").masked(true));
    for kind in [InputContentType::Password, InputContentType::NewPassword] {
        common::update_content(handle, &view, cx, |view, _, cx| {
            view.content_type = Some(kind);
            view.mask_toggle = true;
            cx.notify();
        })
        .unwrap();
        ui(handle, cx, |window, cx| {
            assert_eq!(
                window.find("constrained").role(),
                Some(gpui_kit::Role::PasswordInput)
            );
            assert_eq!(window.find("constrained").value(), None);
            window.within("constrained").click("toggle-mask", cx);
            assert_eq!(window.find("constrained").value(), None);
            window.click("constrained", cx);
            shortcut(window, "a", cx);
            shortcut(window, "c", cx);
            // Clipboard protection follows masking, independently of content type.
            assert_clipboard(cx, "secret");
            window.within("constrained").click("toggle-mask", cx);
            clipboard(cx, "sentinel");
            window.click("constrained", cx);
            shortcut(window, "a", cx);
            shortcut(window, "c", cx);
            assert_clipboard(cx, "sentinel");
            assert_eq!(window.find("constrained").value(), None);
        });
        assert_owner(handle, &view, cx, "secret", 0);
    }
}

#[gpui_kit::test]
fn validation_rejects_typing_without_moving_the_selection_or_notifying_owner(
    cx: &mut TestAppContext,
) {
    let (handle, view) = fixture(cx, |input| {
        input
            .default_value("12")
            .validate(|value, _| value.bytes().all(|c| c.is_ascii_digit()))
    });
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        shortcut(window, "a", cx);
        window.input("x", cx);
        assert_eq!(window.find("constrained").value(), Some("12"));
    });
    assert_owner(handle, &view, cx, "12", 0);
    common::update_content(handle, &view, cx, |view, _, cx| {
        assert_eq!(view.input.read(cx).selected_range(), 0..2);
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        window.input("7", cx);
        assert_eq!(window.find("constrained").value(), Some("7"));
    });
    assert_owner(handle, &view, cx, "7", 1);
}

#[gpui_kit::test]
fn invalid_paste_is_atomic_and_does_not_add_an_undo_entry(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| {
        input.validate(|value, _| value.len() <= 3 && value.bytes().all(|c| c.is_ascii_digit()))
    });
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        clipboard(cx, "12");
        shortcut(window, "v", cx);
        shortcut(window, "a", cx);
    });
    assert_owner(handle, &view, cx, "12", 1);
    for rejected in ["3x", "1234"] {
        ui(handle, cx, |window, cx| {
            clipboard(cx, rejected);
            shortcut(window, "v", cx);
            assert_eq!(window.find("constrained").value(), Some("12"));
            assert_clipboard(cx, rejected);
        });
        assert_owner(handle, &view, cx, "12", 1);
    }
    ui(handle, cx, |window, cx| {
        shortcut(window, "z", cx);
        assert_eq!(window.find("constrained").value(), Some(""));
    });
    assert_owner(handle, &view, cx, "", 2);
}

#[gpui_kit::test]
fn mask_formats_paste_rejects_overflow_and_round_trips_history(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| input.mask_pattern("99-99"));
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        clipboard(cx, "1234");
        shortcut(window, "v", cx);
        assert_eq!(window.find("constrained").value(), Some("12-34"));
    });
    assert_owner(handle, &view, cx, "12-34", 1);
    common::update_content(handle, &view, cx, |view, _, cx| {
        assert_eq!(view.input.read(cx).unmask_value(), "1234");
        assert_eq!(view.input.read(cx).cursor(), 5);
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        window.input("5x", cx);
        assert_eq!(window.find("constrained").value(), Some("12-34"));
    });
    assert_owner(handle, &view, cx, "12-34", 1);
    ui(handle, cx, |window, cx| {
        shortcut(window, "z", cx);
        assert_eq!(window.find("constrained").value(), Some(""));
    });
    assert_owner(handle, &view, cx, "", 2);
    ui(handle, cx, |window, cx| {
        shortcut(
            window,
            if cfg!(target_os = "macos") {
                "shift-z"
            } else {
                "y"
            },
            cx,
        );
        assert_eq!(window.find("constrained").value(), Some("12-34"));
    });
    assert_owner(handle, &view, cx, "12-34", 3);
}

#[gpui_kit::test]
fn invalid_initial_value_can_be_repaired_then_validation_applies(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| {
        input
            .default_value("bad")
            .validate(|value, _| value.bytes().all(|c| c.is_ascii_digit()))
    });
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        shortcut(window, "a", cx);
        window.input("7", cx);
        assert_eq!(window.find("constrained").value(), Some("7"));
    });
    assert_owner(handle, &view, cx, "7", 1);
    ui(handle, cx, |window, cx| {
        window.input("x", cx);
        assert_eq!(window.find("constrained").value(), Some("7"));
        shortcut(window, "a", cx);
        window.press("backspace", cx);
        assert_eq!(window.find("constrained").value(), Some(""));
    });
    assert_owner(handle, &view, cx, "", 2);
}

#[gpui_kit::test]
fn clear_affordance_tracks_editability_and_emits_one_change(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| input.default_value("clear me"));
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.cleanable = true;
        cx.notify();
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        assert!(window.within("constrained").find("clean").visible());
    });
    for (readonly, disabled) in [(true, false), (false, true)] {
        common::update_content(handle, &view, cx, |view, _, cx| {
            view.readonly = readonly;
            view.disabled = disabled;
            cx.notify();
        })
        .unwrap();
        ui(handle, cx, |window, _| {
            assert!(window.within("constrained").try_find("clean").is_none());
            assert_eq!(window.find("constrained").value(), Some("clear me"));
        });
        assert_owner(handle, &view, cx, "clear me", 0);
    }
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.disabled = false;
        view.readonly = false;
        cx.notify();
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        window.within("constrained").click("clean", cx);
        assert_eq!(window.find("constrained").value(), Some(""));
        assert_eq!(window.find("constrained").focused(), Some(true));
        assert!(window.within("constrained").try_find("clean").is_none());
    });
    assert_owner(handle, &view, cx, "", 1);
    ui(handle, cx, |window, cx| {
        window.input("A", cx);
        assert_eq!(window.find("constrained").value(), Some("A"));
        assert!(window.within("constrained").find("clean").visible());
    });
    assert_owner(handle, &view, cx, "A", 2);
}

#[gpui_kit::test]
fn readonly_mouse_selection_copies_and_becomes_editable_again(cx: &mut TestAppContext) {
    // Keep the center of the field over a word regardless of font metrics.
    let word = "word".repeat(24);
    let (handle, view) = fixture(cx, |input| input.default_value(word.clone()));
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.readonly = true;
        cx.notify();
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        window.double_click("constrained", cx);
        assert_eq!(window.find("constrained").focused(), Some(true));
        shortcut(window, "c", cx);
        assert_clipboard(cx, &word);
        window.input("ignored", cx);
    });
    assert_owner(handle, &view, cx, &word, 0);
    common::update_content(handle, &view, cx, |view, _, cx| {
        assert_eq!(view.input.read(cx).selected_range(), 0..word.len());
        view.readonly = false;
        cx.notify();
    })
    .unwrap();
    ui(handle, cx, |window, cx| window.input("replacement", cx));
    assert_owner(handle, &view, cx, "replacement", 11);
}

#[gpui_kit::test]
fn disabled_single_and_double_click_do_not_focus_the_editor(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| input.default_value("fixed"));
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        shortcut(window, "a", cx);
        window.press("left", cx);
        window.press("right", cx);
        window.press("shift-right", cx);
        window.press("shift-right", cx);
        assert_eq!(view.read(cx).input.read(cx).selected_range(), 1..3);
    });
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.disabled = true;
        view.previous_focus = Some(cx.focus_handle());
        cx.notify();
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        let input = view.read(cx).input.clone();
        let focus = input.read(cx).focus_handle(cx);
        let previous_focus = view.read(cx).previous_focus.clone().unwrap();
        for preserve_focus in [false, true] {
            for target in ["body", "double-click", "padding"] {
                if preserve_focus {
                    previous_focus.focus(window, cx);
                } else {
                    window.blur(cx);
                }
                window.render_frame(cx);
                let before = window.focused(cx);
                assert_eq!(previous_focus.is_focused(window), preserve_focus);
                assert!(!focus.is_focused(window));
                match target {
                    "body" => window.click("constrained", cx),
                    "double-click" => window.double_click("constrained", cx),
                    "padding" => {
                        let frame = window.find("constrained").bounds();
                        let body = input.read(cx).text_bounds().expect("rendered text bounds");
                        let position = point((frame.left() + body.left()) / 2., body.center().y);
                        assert!(frame.contains(&position) && !body.contains(&position));
                        window.click_at("constrained", position - frame.origin, cx);
                    }
                    _ => unreachable!(),
                }
                assert_eq!(window.focused(cx), before, "focus after {target}");
                assert!(!focus.is_focused(window), "editor focus after {target}");
                assert_ne!(window.find("constrained").focused(), Some(true));
                assert_eq!(
                    input.read(cx).selected_range(),
                    1..3,
                    "selection after {target}"
                );
            }
        }
        assert_eq!(input.read(cx).value(), "fixed");
    });
    assert_owner(handle, &view, cx, "fixed", 0);
}

#[gpui_kit::test]
fn enabling_a_disabled_input_allows_mouse_focus_and_replacement(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| input.default_value("fixed"));
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.disabled = true;
        cx.notify();
    })
    .unwrap();
    // Render the disabled state before enabling the same retained input.
    ui(handle, cx, |window, cx| window.blur(cx));
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.disabled = false;
        cx.notify();
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        assert!(
            view.read(cx)
                .input
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
        shortcut(window, "a", cx);
        window.input("enabled", cx);
    });
    assert_owner(handle, &view, cx, "enabled", 7);
}

#[gpui_kit::test]
fn masked_mouse_selection_replaces_whole_secret_and_reveal_keeps_history(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| {
        input.default_value("first second third").masked(true)
    });
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.mask_toggle = true;
        cx.notify();
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        window.double_click("constrained", cx);
        clipboard(cx, "sentinel");
        shortcut(window, "c", cx);
        assert_clipboard(cx, "sentinel");
        clipboard(cx, "new secret");
        shortcut(window, "v", cx);
        assert_eq!(window.find("constrained").value(), None);
        window.within("constrained").click("toggle-mask", cx);
        assert_eq!(window.find("constrained").value(), Some("new secret"));
        window.click("constrained", cx);
        shortcut(window, "z", cx);
        assert_eq!(
            window.find("constrained").value(),
            Some("first second third")
        );
        window.within("constrained").click("toggle-mask", cx);
        assert_eq!(window.find("constrained").value(), None);
    });
    assert_owner(handle, &view, cx, "first second third", 2);
}

#[gpui_kit::test]
fn validation_rejects_partial_deletion_but_allows_clear_and_reentry(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| {
        input
            .default_value("12")
            .validate(|value, _| value.len() >= 2)
    });
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.cleanable = true;
        cx.notify();
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        shortcut(window, "a", cx);
        window.press("right", cx);
        window.press("backspace", cx);
        assert_eq!(window.find("constrained").value(), Some("12"));
        shortcut(window, "a", cx);
        window.press("left", cx);
        window.press("delete", cx);
        assert_eq!(window.find("constrained").value(), Some("12"));
    });
    assert_owner(handle, &view, cx, "12", 0);
    ui(handle, cx, |window, cx| {
        // Empty is deliberately accepted independently of the validator.
        window.within("constrained").click("clean", cx);
        assert_eq!(window.find("constrained").value(), Some(""));
        assert_eq!(window.find("constrained").focused(), Some(true));
        assert!(window.within("constrained").try_find("clean").is_none());
    });
    assert_owner(handle, &view, cx, "", 1);
    ui(handle, cx, |window, cx| {
        window.input("1", cx);
        assert_eq!(window.find("constrained").value(), Some(""));
        clipboard(cx, "34");
        shortcut(window, "v", cx);
    });
    assert_owner(handle, &view, cx, "34", 2);
}

#[gpui_kit::test]
fn validation_policy_changes_apply_to_the_existing_selection(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| {
        input
            .default_value("12")
            .validate(|value, _| value.bytes().all(|c| c.is_ascii_digit()))
    });
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        shortcut(window, "a", cx);
        window.input("letters", cx);
    });
    assert_owner(handle, &view, cx, "12", 0);
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.input
            .update(cx, |input, cx| input.set_validator(|_, _| true, cx));
    })
    .unwrap();
    ui(handle, cx, |window, cx| window.input("letters", cx));
    assert_owner(handle, &view, cx, "letters", 7);
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.input.update(cx, |input, cx| {
            input.set_validator(|value, _| value.bytes().all(|c| c.is_ascii_digit()), cx);
        });
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        shortcut(window, "a", cx);
        clipboard(cx, "bad");
        shortcut(window, "v", cx);
        // Existing invalid text stays editable so the user can repair it.
        assert_eq!(window.find("constrained").value(), Some("bad"));
    });
    assert_owner(handle, &view, cx, "bad", 8);
    ui(handle, cx, |window, cx| {
        shortcut(window, "a", cx);
        clipboard(cx, "34");
        shortcut(window, "v", cx);
    });
    assert_owner(handle, &view, cx, "34", 9);
    ui(handle, cx, |window, cx| {
        shortcut(window, "a", cx);
        clipboard(cx, "bad");
        shortcut(window, "v", cx);
        assert_eq!(window.find("constrained").value(), Some("34"));
    });
    common::update_content(handle, &view, cx, |view, _, cx| {
        assert_eq!(view.input.read(cx).selected_range(), 0..2);
    })
    .unwrap();
    assert_owner(handle, &view, cx, "34", 9);
}

#[gpui_kit::test]
fn paste_hook_consumption_preserves_selection_and_redo_then_fallback_normalizes(
    cx: &mut TestAppContext,
) {
    let (handle, view) = fixture(cx, |input| input.default_value("old"));
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        shortcut(window, "a", cx);
        clipboard(cx, "new");
        shortcut(window, "v", cx);
        shortcut(window, "z", cx);
    });
    assert_owner(handle, &view, cx, "old", 2);
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.consume_paste = Some(true);
        cx.notify();
    })
    .unwrap();
    let payload = "中\r\n\t文";
    ui(handle, cx, |window, cx| {
        clipboard(cx, payload);
        shortcut(window, "v", cx);
        assert_eq!(window.find("constrained").value(), Some("old"));
        assert_clipboard(cx, payload);
    });
    assert_owner(handle, &view, cx, "old", 2);
    common::update_content(handle, &view, cx, |view, _, cx| {
        assert_eq!(view.input.read(cx).selected_range(), 0..3);
        assert_eq!(&*view.paste_payloads.borrow(), &[payload]);
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        shortcut(
            window,
            if cfg!(target_os = "macos") {
                "shift-z"
            } else {
                "y"
            },
            cx,
        );
        assert_eq!(window.find("constrained").value(), Some("new"));
        shortcut(window, "z", cx);
    });
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.consume_paste = Some(false);
        cx.notify();
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        shortcut(window, "v", cx);
        assert_eq!(window.find("constrained").value(), Some("中\t文"));
        shortcut(window, "z", cx);
        assert_eq!(window.find("constrained").value(), Some("old"));
    });
    assert_owner(handle, &view, cx, "old", 6);
    common::update_content(handle, &view, cx, |view, _, cx| {
        assert_eq!(&*view.paste_payloads.borrow(), &[payload, payload]);
        assert_eq!(view.input.read(cx).selected_range(), 0..3);
    })
    .unwrap();
}

#[gpui_kit::test]
fn protected_input_does_not_invoke_paste_hook_and_reenable_restores_it(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| input.default_value("keep"));
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.consume_paste = Some(false);
        cx.notify();
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        shortcut(window, "a", cx);
        clipboard(cx, "replacement");
    });
    for disabled in [false, true] {
        common::update_content(handle, &view, cx, |view, _, cx| {
            view.readonly = !disabled;
            view.disabled = disabled;
            cx.notify();
        })
        .unwrap();
        ui(handle, cx, |window, cx| {
            shortcut(window, "v", cx);
            assert_eq!(window.find("constrained").value(), Some("keep"));
        });
        assert_owner(handle, &view, cx, "keep", 0);
        common::update_content(handle, &view, cx, |view, _, cx| {
            assert!(view.paste_payloads.borrow().is_empty());
            view.readonly = false;
            view.disabled = false;
            cx.notify();
        })
        .unwrap();
    }
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        shortcut(window, "a", cx);
        shortcut(window, "v", cx);
        assert_eq!(window.find("constrained").value(), Some("replacement"));
    });
    assert_owner(handle, &view, cx, "replacement", 1);
    common::update_content(handle, &view, cx, |view, _, _| {
        assert_eq!(&*view.paste_payloads.borrow(), &["replacement"]);
    })
    .unwrap();
}

#[gpui_kit::test]
fn masked_word_delete_takes_the_secret_and_undo_preserves_privacy(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| input.default_value("first second").masked(true));
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        window.press("end", cx);
        let modifier = if cfg!(target_os = "macos") {
            "alt"
        } else {
            "ctrl"
        };
        window.press(&format!("{modifier}-backspace"), cx);
        assert_eq!(window.find("constrained").value(), None);
    });
    assert_owner(handle, &view, cx, "", 1);
    ui(handle, cx, |window, cx| {
        shortcut(window, "z", cx);
        window.press("home", cx);
        let modifier = if cfg!(target_os = "macos") {
            "alt"
        } else {
            "ctrl"
        };
        window.press(&format!("{modifier}-delete"), cx);
        assert_eq!(window.find("constrained").value(), None);
    });
    assert_owner(handle, &view, cx, "", 3);
    ui(handle, cx, |window, cx| {
        shortcut(window, "z", cx);
        clipboard(cx, "public");
        shortcut(window, "a", cx);
        shortcut(window, "c", cx);
        assert_clipboard(cx, "public");
        assert_eq!(window.find("constrained").value(), None);
    });
    assert_owner(handle, &view, cx, "first second", 4);
}

#[gpui_kit::test]
fn validation_rejection_preserves_redo_and_replacement_selection(cx: &mut TestAppContext) {
    let (handle, view) = fixture(cx, |input| {
        input
            .default_value("12")
            .validate(|value, _| value.len() <= 3 && value.bytes().all(|c| c.is_ascii_digit()))
    });
    ui(handle, cx, |window, cx| {
        window.click("constrained", cx);
        window.press("end", cx);
        clipboard(cx, "3");
        shortcut(window, "v", cx);
        shortcut(window, "z", cx);
        window.press("shift-home", cx);
        window.input("x", cx);
        clipboard(cx, "4567");
        shortcut(window, "v", cx);
        assert_eq!(window.find("constrained").value(), Some("12"));
    });
    assert_owner(handle, &view, cx, "12", 2);
    common::update_content(handle, &view, cx, |view, _, cx| {
        assert_eq!(view.input.read(cx).selected_range(), 0..2);
        assert_eq!(view.input.read(cx).cursor(), 0);
    })
    .unwrap();
    ui(handle, cx, |window, cx| {
        shortcut(
            window,
            if cfg!(target_os = "macos") {
                "shift-z"
            } else {
                "y"
            },
            cx,
        );
        assert_eq!(window.find("constrained").value(), Some("123"));
        shortcut(window, "z", cx);
        shortcut(window, "a", cx);
        clipboard(cx, "45");
        shortcut(window, "v", cx);
        shortcut(
            window,
            if cfg!(target_os = "macos") {
                "shift-z"
            } else {
                "y"
            },
            cx,
        );
        assert_eq!(window.find("constrained").value(), Some("45"));
    });
    assert_owner(handle, &view, cx, "45", 5);
}
