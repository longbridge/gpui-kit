mod common;
use gpui_kit::component::{
    button::Button,
    input::{Copy, Input, InputState, SelectAll, Textarea, TextareaState},
    menu::{ContextMenuExt, DropdownMenu, PopupMenuItem},
};
use gpui_kit::test::{TestAppContextExt, TestSupportExt, TestWindowExt};
use gpui_kit::{
    AnyWindowHandle, App, AppContext, ClipboardItem, Context, Entity, FocusHandle, Focusable as _,
    InputEvent, KeyDownEvent, KeyUpEvent, Keystroke, MouseButton, MouseDownEvent, MouseUpEvent,
    TestAppContext, Window, actions, div, point, prelude::*, px, size,
};
use std::{cell::Cell, rc::Rc, time::Duration};

actions!(menu_test, [Save, Unavailable]);
struct Commands {
    saved: bool,
    focus: gpui_kit::FocusHandle,
}
impl Render for Commands {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("workspace")
            .test_support()
            .track_focus(&self.focus)
            .size_full()
            .p_4()
            .on_action(cx.listener(|this, _: &Save, _, cx| {
                this.saved = true;
                cx.notify();
            }))
            .on_action(|_: &Unavailable, _, _| panic!("disabled command dispatched"))
            .child(
                Button::new("commands")
                    .label("Commands")
                    .dropdown_menu(|menu, window, cx| {
                        menu.menu_with_disabled("Unavailable", Box::new(Unavailable), true)
                            .menu("Save", Box::new(Save))
                            .submenu("More", window, cx, |menu, _, _| {
                                menu.menu("Save copy", Box::new(Save))
                            })
                    }),
            )
            .child(div().id("result").test_support().child(if self.saved {
                div().id("saved").test_support().child("Saved")
            } else {
                div().id("unsaved").test_support().child("Unsaved")
            }))
    }
}
#[gpui_kit::test]
async fn menu_skips_disabled_commands_confirms_and_restores_focus(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
        let view = cx.new(|cx| Commands {
            saved: false,
            focus: cx.focus_handle(),
        });
        view.read(cx).focus.clone().focus(window, cx);
        view
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("commands", cx);
        assert_eq!(window.find("popup-menu").focused(), Some(true));
        window.within("popup-menu").click(0usize, cx);
        assert!(window.find("unsaved").visible());
        window.within("popup-menu").press("down", cx);
        assert_eq!(
            window.within("popup-menu").find(1usize).selected(),
            Some(true)
        );
        window.within("popup-menu").press("enter", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("popup-menu").is_none() && window.try_find("saved").is_some()
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(window.find("workspace").focused(), Some(true));
        window.click("commands", cx);
        window.press("escape", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("popup-menu").is_none()
    })
    .await;
}

#[gpui_kit::test]
async fn hovering_submenu_opens_and_clicking_item_dismisses_the_chain(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
        let view = cx.new(|cx| Commands {
            saved: false,
            focus: cx.focus_handle(),
        });
        view.read(cx).focus.clone().focus(window, cx);
        view
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("commands", cx);
        let mut menu = window.within("popup-menu");
        menu.hover(2usize, cx);
        assert_eq!(menu.find(2usize).selected(), Some(true));
    })
    .unwrap();
    // Submenus already own a native "submenu" identity scope.
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("submenu").is_some()
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        assert_eq!(
            window.within("submenu").find(0usize).label(),
            Some("Save copy")
        );
        window.within("submenu").click(0usize, cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("saved").is_some()
    })
    .await;
    cx.update_window(handle.into(), |_, window, _| {
        assert!(window.try_find("submenu").is_none());
        assert!(window.try_find("popup-menu").is_none());
    })
    .unwrap();
}

/// A scrollable menu whose only submenu item is the last of 31 rows, so it
/// starts scrolled out of the 160px viewport.
struct ScrollableCommands {
    focus: gpui_kit::FocusHandle,
}
impl Render for ScrollableCommands {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("workspace")
            .test_support()
            .track_focus(&self.focus)
            .size_full()
            .p_4()
            .child(
                Button::new("commands")
                    .label("Commands")
                    .dropdown_menu(|menu, window, cx| {
                        (0..30)
                            .fold(menu.scrollable(true).max_h(px(160.)), |menu, ix| {
                                menu.menu(format!("Item {ix}"), Box::new(Save))
                            })
                            .submenu("More", window, cx, |menu, _, _| {
                                menu.menu("Save copy", Box::new(Save))
                            })
                    }),
            )
    }
}

/// The submenu is painted as a deferred draw outside the items container, so
/// the container's `overflow_y_scroll` clip must not hide it and its items
/// must still be hit-testable.
#[gpui_kit::test]
async fn submenu_opens_unclipped_from_a_scrollable_menu(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
        let view = cx.new(|cx| ScrollableCommands {
            focus: cx.focus_handle(),
        });
        view.read(cx).focus.clone().focus(window, cx);
        view
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("commands", cx);
        let mut menu = window.within("popup-menu");
        // `up` wraps to the last item and scrolls it into view, where the
        // pointer can reach it.
        menu.press("up", cx);
        assert!(menu.find(30usize).visible(), "{:?}", menu.find(30usize));
        menu.hover(30usize, cx);
        assert_eq!(menu.find(30usize).selected(), Some(true));
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("submenu").is_some()
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        let submenu = window.find("submenu");
        let viewport = gpui_kit::Bounds::new(Default::default(), window.viewport_size());
        assert!(submenu.visible(), "submenu is clipped: {submenu:?}");
        assert!(
            viewport.contains(&submenu.bounds().origin)
                && viewport.contains(&submenu.bounds().bottom_right()),
            "submenu {:?} is outside the window {viewport:?}",
            submenu.bounds()
        );
        let mut submenu = window.within("submenu");
        assert_eq!(submenu.find(0usize).label(), Some("Save copy"));
        submenu.click(0usize, cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("popup-menu").is_none()
    })
    .await;
}

struct ContextMenuInputs {
    source: Entity<TextareaState>,
    other: Entity<InputState>,
    action_target: Option<FocusHandle>,
}

impl Render for ContextMenuInputs {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let action_target = self.action_target.clone();
        div()
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .id("source-menu")
                    .w(px(320.))
                    .child(Textarea::new(&self.source).h(px(120.)))
                    .context_menu(move |menu, window, cx| {
                        menu.when_some(action_target.clone(), |menu, target| {
                            menu.action_context(target)
                        })
                        .menu("Copy", Box::new(Copy))
                        .submenu("More", window, cx, |menu, _, _| {
                            menu.menu("Copy", Box::new(Copy))
                                .menu("Select all", Box::new(SelectAll))
                        })
                    }),
            )
            .child(Input::new(&self.other).id("other-input").w(px(320.)))
            .child(
                div()
                    .id("unrelated-menu")
                    .test_support()
                    .w(px(120.))
                    .h(px(40.))
                    .child("Other commands")
                    .context_menu(|menu, _, _| {
                        menu.item(PopupMenuItem::new("Other").on_click(|_, _, _| {}))
                    }),
            )
    }
}

fn assert_selection_menu(
    source: &Entity<TextareaState>,
    other: &Entity<InputState>,
    window: &mut Window,
    cx: &mut App,
) {
    // The first opening frame can still have laid out the input before the
    // menu takes focus. Always check a subsequent frame as well.
    window.render_frame(cx);
    assert_eq!(window.find("popup-menu").focused(), Some(true));
    assert!(!source.focus_handle(cx).is_focused(window));
    assert!(source.read(cx).has_selection_focus(window, cx));
    assert_eq!(source.read(cx).selected_range(), 6..10);
    assert!(!other.read(cx).has_selection_focus(window, cx));
    assert_eq!(other.read(cx).selected_range(), 0..5);
}

fn press_context_menu_key(handle: AnyWindowHandle, key: &str, cx: &mut TestAppContext) {
    let keystroke = Keystroke::parse(key).unwrap();
    cx.update_window(handle, |_, window, cx| {
        window.dispatch_event(
            KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            }
            .to_platform_input(),
            cx,
        );
    })
    .unwrap();
    // Native key-down and key-up arrive in separate app updates. Process the
    // dismissal subscription before key-up can redraw and refocus the menu.
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.dispatch_event(KeyUpEvent { keystroke }.to_platform_input(), cx);
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn context_menu_preserves_only_its_inputs_selection_and_keyboard_actions(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, inputs) = common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
        cx.new(|cx| ContextMenuInputs {
            source: cx.new(|cx| TextareaState::new(window, cx).context_menu(false)),
            other: cx.new(|cx| InputState::new(window, cx)),
            action_target: None,
        })
    });
    let (source, other) = inputs.read_with(cx, |inputs, _| {
        (inputs.source.clone(), inputs.other.clone())
    });
    let source_id = ("input", source.entity_id());
    cx.update_window(handle.into(), |_, window, cx| {
        window.activate_window();
        window.click("other-input", cx);
        window.input("other", cx);
        window.press("secondary-a", cx);
        window.click(source_id, cx);
        window.input("alpha\ncopy", cx);
        for _ in 0..4 {
            window.press("shift-left", cx);
        }
    })
    .unwrap();
    cx.run_until_parked();

    // First use automatic ownership, then an explicit action target for Copy.
    for explicit_target in [false, true] {
        inputs.update(cx, |inputs, cx| {
            inputs.action_target = explicit_target.then(|| source.focus_handle(cx));
            cx.notify();
        });
        cx.update_window(handle.into(), |_, window, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string("stale".into()));
            window.right_click(source_id, cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, window, cx| {
            assert_selection_menu(&source, &other, window, cx);
            if !explicit_target {
                // Replace the menu at a point inside the same input, outside
                // the old menu's bounds.
                let position = source.read(cx).input_bounds().origin + point(px(4.), px(4.));
                window.dispatch_event(
                    MouseDownEvent {
                        button: MouseButton::Right,
                        position,
                        click_count: 1,
                        first_mouse: false,
                        modifiers: Default::default(),
                    }
                    .to_platform_input(),
                    cx,
                );
                window.dispatch_event(
                    MouseUpEvent {
                        button: MouseButton::Right,
                        position,
                        click_count: 1,
                        modifiers: Default::default(),
                    }
                    .to_platform_input(),
                    cx,
                );
            }
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, window, cx| {
            assert_selection_menu(&source, &other, window, cx);
            window.press("down", cx);
            assert_eq!(
                window.within("popup-menu").find(0usize).selected(),
                Some(true)
            );
            if !explicit_target {
                window.press("down", cx);
                window.press("right", cx);
                assert_eq!(
                    window.within("submenu").find("popup-menu").focused(),
                    Some(true)
                );
                assert!(source.read(cx).has_selection_focus(window, cx));
            }
        })
        .unwrap();
        press_context_menu_key(
            handle.into(),
            if explicit_target { "enter" } else { "escape" },
            cx,
        );
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("popup-menu").is_none());
            assert!(
                source.focus_handle(cx).is_focused(window),
                "focus must return to the source after explicit_target={explicit_target}: focused={:?}, source={:?}",
                window.focused(cx),
                source.focus_handle(cx),
            );
            assert_eq!(source.read(cx).selected_range(), 6..10);
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some(if explicit_target { "copy" } else { "stale" })
            );
        })
        .unwrap();
    }

    cx.update_window(handle.into(), |_, window, cx| {
        window.right_click("unrelated-menu", cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("popup-menu").focused(), Some(true));
        assert!(!source.read(cx).has_selection_focus(window, cx));
        assert!(!other.read(cx).has_selection_focus(window, cx));
    })
    .unwrap();
    press_context_menu_key(handle.into(), "escape", cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.right_click(source_id, cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert_selection_menu(&source, &other, window, cx);
        window.hover("other-input", cx);
        let position = window.find("other-input").bounds().center();
        window.dispatch_event(
            MouseDownEvent {
                button: MouseButton::Left,
                position,
                click_count: 1,
                first_mouse: false,
                modifiers: Default::default(),
            }
            .to_platform_input(),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        let position = window.find("other-input").bounds().center();
        window.dispatch_event(
            MouseUpEvent {
                button: MouseButton::Left,
                position,
                click_count: 1,
                modifiers: Default::default(),
            }
            .to_platform_input(),
            cx,
        );
        window.press("secondary-a", cx);
        window.input("destination", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("popup-menu").is_none());
        assert_eq!(window.find("other-input").focused(), Some(true));
        assert_eq!(other.read(cx).value().as_ref(), "destination");
        assert_eq!(source.read(cx).value().as_ref(), "alpha\ncopy");
        assert!(!source.read(cx).has_selection_focus(window, cx));
    })
    .unwrap();
}

#[gpui_kit::test]
fn right_click_on_unfocused_input_keeps_selection(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, inputs) = common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
        cx.new(|cx| ContextMenuInputs {
            source: cx.new(|cx| TextareaState::new(window, cx).context_menu(false)),
            other: cx.new(|cx| InputState::new(window, cx)),
            action_target: None,
        })
    });
    let source = inputs.read_with(cx, |inputs, _| inputs.source.clone());
    let source_id = ("input", source.entity_id());
    cx.update_window(handle.into(), |_, window, cx| {
        source.update(cx, |state, cx| {
            state.set_value("alpha\ncopy", window, cx);
            state.set_selected_range(6..10, cx);
        });
        window.click("other-input", cx);
        window.render_frame(cx);
        assert!(!source.focus_handle(cx).is_focused(window));
        window.right_click(source_id, cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("popup-menu").focused(), Some(true));
        assert_eq!(source.read(cx).selected_range(), 6..10);
        assert!(
            source.read(cx).has_selection_focus(window, cx),
            "the newly focused input was missed by ContextMenu ownership lookup"
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn submenu_select_all_uses_the_parent_input_action_target(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, inputs) = common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
        cx.new(|cx| ContextMenuInputs {
            source: cx.new(|cx| TextareaState::new(window, cx).context_menu(false)),
            other: cx.new(|cx| InputState::new(window, cx)),
            action_target: None,
        })
    });
    let (source, other) = inputs.read_with(cx, |inputs, _| {
        (inputs.source.clone(), inputs.other.clone())
    });
    let source_id = ("input", source.entity_id());
    inputs.update(cx, |inputs, cx| {
        inputs.action_target = Some(source.focus_handle(cx));
        cx.notify();
    });
    cx.update_window(handle.into(), |_, window, cx| {
        source.update(cx, |state, cx| {
            state.set_value("alpha\ncopy", window, cx);
            state.set_selected_range(6..10, cx);
        });
        window.click("other-input", cx);
        window.input("other", cx);
        window.right_click(source_id, cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.press("down", cx);
        window.press("down", cx);
        window.press("right", cx);
        window
            .within("submenu")
            .within("popup-menu")
            .hover(1usize, cx);
    })
    .unwrap();
    cx.run_until_parked();
    // Native pointer events run in separate app updates. Let dismissal
    // subscriptions run before another frame can refocus the open menu.
    cx.update_window(handle.into(), |_, window, cx| {
        let position = window
            .within("submenu")
            .within("popup-menu")
            .find(1usize)
            .bounds()
            .center();
        window.dispatch_event(
            MouseDownEvent {
                button: MouseButton::Left,
                position,
                click_count: 1,
                first_mouse: false,
                modifiers: Default::default(),
            }
            .to_platform_input(),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        let position = window
            .within("submenu")
            .within("popup-menu")
            .find(1usize)
            .bounds()
            .center();
        window.dispatch_event(
            MouseUpEvent {
                button: MouseButton::Left,
                position,
                click_count: 1,
                modifiers: Default::default(),
            }
            .to_platform_input(),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("popup-menu").is_none());
        assert_eq!(source.read(cx).selected_range(), 0..10);
        assert_eq!(source.read(cx).value().as_ref(), "alpha\ncopy");
        assert_eq!(other.read(cx).value().as_ref(), "other");
        assert!(source.focus_handle(cx).is_focused(window));
    })
    .unwrap();
}

struct NestedContextMenus {
    inner_chosen: Rc<Cell<usize>>,
    outer_chosen: Rc<Cell<usize>>,
}

impl Render for NestedContextMenus {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let inner_chosen = self.inner_chosen.clone();
        let outer_chosen = self.outer_chosen.clone();
        div().size_full().p_4().child(
            div()
                .id("card")
                .flex()
                .flex_col()
                .gap_4()
                .w(px(320.))
                .child(
                    div()
                        .id("card-title")
                        .test_support()
                        .h(px(40.))
                        .child("Card"),
                )
                .child(
                    Button::new("share")
                        .label("Share")
                        .context_menu(move |menu, _, _| {
                            let chosen = inner_chosen.clone();
                            menu.item(PopupMenuItem::new("Copy link").on_click(move |_, _, _| {
                                chosen.set(chosen.get() + 1);
                            }))
                        }),
                )
                .context_menu(move |menu, _, _| {
                    let chosen = outer_chosen.clone();
                    menu.item(PopupMenuItem::new("Remove card").on_click(move |_, _, _| {
                        chosen.set(chosen.get() + 1);
                    }))
                }),
        )
    }
}

#[gpui_kit::test]
fn nested_context_menu_uses_the_innermost_right_click_target(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let inner_chosen = Rc::new(Cell::new(0));
    let outer_chosen = Rc::new(Cell::new(0));
    let (handle, _) = common::open_window(cx, Some(size(px(640.), px(480.))), |_, cx| {
        cx.new(|_| NestedContextMenus {
            inner_chosen: inner_chosen.clone(),
            outer_chosen: outer_chosen.clone(),
        })
    });

    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.right_click("share", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let mut menu = window.within("popup-menu");
        assert!(menu.find_by_label("Copy link").visible());
        assert!(menu.try_find_by_label("Remove card").is_none());
        menu.press("down", cx);
        assert_eq!(menu.find_by_label("Copy link").selected(), Some(true));
    })
    .unwrap();
    press_context_menu_key(handle.into(), "enter", cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("popup-menu").is_none());
        assert_eq!(inner_chosen.get(), 1);
        assert_eq!(outer_chosen.get(), 0);
        window.right_click("card-title", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let menu = window.within("popup-menu");
        assert!(menu.find_by_label("Remove card").visible());
        assert!(menu.try_find_by_label("Copy link").is_none());
    })
    .unwrap();
    press_context_menu_key(handle.into(), "escape", cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("popup-menu").is_none());
        assert_eq!(inner_chosen.get(), 1);
        assert_eq!(outer_chosen.get(), 0);
        window.right_click("card-title", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let mut menu = window.within("popup-menu");
        menu.press("down", cx);
        assert_eq!(menu.find_by_label("Remove card").selected(), Some(true));
    })
    .unwrap();
    press_context_menu_key(handle.into(), "enter", cx);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("popup-menu").is_none());
        assert_eq!(inner_chosen.get(), 1);
        assert_eq!(outer_chosen.get(), 1);
    })
    .unwrap();
}
