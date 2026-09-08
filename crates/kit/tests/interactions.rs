use gpui_kit::test::{TestAppContextExt, TestSupportExt, TestWindowExt};
use gpui_kit::{
    AppContext, Context, MouseButton, ScrollDelta, ScrollHandle, TestAppContext, Window, div,
    point, prelude::*, px, size,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

struct Scopes {
    clicks: Rc<RefCell<Vec<&'static str>>>,
}
impl Render for Scopes {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().flex().children(["toolbar", "dialog"].map(|scope| {
            let clicks = self.clicks.clone();
            div().id(scope).size(px(80.)).child(
                div().id("footer").child(
                    div()
                        .id("save")
                        .test_support()
                        .aria_label(scope)
                        .size(px(40.))
                        .on_click(move |_, _, _| clicks.borrow_mut().push(scope)),
                ),
            )
        }))
    }
}
#[gpui_kit::test]
fn scoped_queries_follow_existing_gpui_paths_without_observed_containers(cx: &mut TestAppContext) {
    let clicks = Rc::new(RefCell::new(vec![]));
    let handle = cx.add_window(|_, _| Scopes {
        clicks: clicks.clone(),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(
            window.within("toolbar").find("save").label(),
            Some("toolbar")
        );
        window.within("dialog").within("footer").click("save", cx);
        assert!(window.within("toolbar").try_find("missing").is_none());
    })
    .unwrap();
    assert_eq!(&*clicks.borrow(), &["dialog"]);
}

#[gpui_kit::test]
#[should_panic(expected = "Registered paths:")]
fn missing_targets_explain_the_registered_frame(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, _| Scopes {
        clicks: Default::default(),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.find("misspelled");
    })
    .unwrap();
}

struct Pointer {
    events: Rc<RefCell<Vec<String>>>,
}
impl Render for Pointer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let hover = self.events.clone();
        let left = self.events.clone();
        let right = self.events.clone();
        div()
            .id("surface")
            .test_support()
            .size(px(80.))
            .on_hover(move |entered, _, _| {
                if *entered {
                    hover.borrow_mut().push("hover".into());
                }
            })
            .on_mouse_up(MouseButton::Left, move |event, _, _| {
                left.borrow_mut()
                    .push(format!("left:{}", event.click_count))
            })
            .on_mouse_up(MouseButton::Right, move |_, _, _| {
                right.borrow_mut().push("right".into())
            })
    }
}
#[gpui_kit::test]
fn hover_right_click_and_double_click_dispatch_native_pointer_events(cx: &mut TestAppContext) {
    let events = Rc::new(RefCell::new(vec![]));
    let handle = cx.add_window(|_, _| Pointer {
        events: events.clone(),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.hover("surface", cx);
        window.right_click("surface", cx);
        window.double_click("surface", cx);
    })
    .unwrap();
    let events = events.borrow();
    assert!(events.iter().any(|event| event == "hover"));
    assert!(events.iter().any(|event| event == "right"));
    assert!(events.windows(2).any(|pair| pair == ["left:1", "left:2"]));
}

struct Scrolling {
    scroll: ScrollHandle,
}
impl Render for Scrolling {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("list")
            .test_support()
            .w(px(100.))
            .h(px(60.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .flex()
            .flex_col()
            .children((0..30usize).map(|index| {
                div()
                    .id(("row", index))
                    .test_support()
                    .h(px(20.))
                    .flex_shrink_0()
                    .child(format!("Row {index}"))
            }))
    }
}
#[gpui_kit::test]
fn scrolling_changes_clipping_and_resolved_row_positions(cx: &mut TestAppContext) {
    let scroll = ScrollHandle::new();
    let handle = cx.open_window(size(px(200.), px(200.)), |_, _| Scrolling {
        scroll: scroll.clone(),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(!window.find(("row", 10usize)).visible());
        window.scroll("list", ScrollDelta::Pixels(point(px(0.), px(-200.))), cx);
        assert!(window.find(("row", 10usize)).visible());
        assert!(!window.find(("row", 0usize)).visible());
    })
    .unwrap();
    assert!(scroll.offset().y < px(0.));
}

#[derive(Clone)]
struct Payload;
impl Render for Payload {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size(px(8.))
    }
}
struct Dropping {
    drops: Rc<RefCell<usize>>,
}
impl Render for Dropping {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let drops = self.drops.clone();
        div()
            .flex()
            .gap_8()
            .child(
                div()
                    .id("source")
                    .test_support()
                    .size(px(40.))
                    .on_drag(Payload, |payload, _, _, cx| cx.new(|_| payload.clone())),
            )
            .child(
                div()
                    .id("target")
                    .test_support()
                    .size(px(40.))
                    .on_drop(move |_: &Payload, _, _| *drops.borrow_mut() += 1),
            )
    }
}
#[gpui_kit::test]
fn dragging_runs_gpui_drag_creation_and_drop_hit_testing(cx: &mut TestAppContext) {
    let drops = Rc::new(RefCell::new(0));
    let handle = cx.add_window(|_, _| Dropping {
        drops: drops.clone(),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let from = window.find("source").bounds().center();
        let to = window.find("target").bounds().center();
        window.drag(from, to, cx);
    })
    .unwrap();
    assert_eq!(*drops.borrow(), 1);
}

struct Loading {
    ready: bool,
}
impl Render for Loading {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().when(self.ready, |this| {
            this.child(div().id("loaded").test_support().size(px(20.)))
        })
    }
}
#[gpui_kit::test]
async fn wait_for_drives_test_time_and_refreshes_async_changes(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, _| Loading { ready: false });
    let executor = cx.executor();
    cx.spawn(move |mut cx| async move {
        executor.timer(Duration::from_millis(25)).await;
        handle
            .update(&mut cx, |view, _, cx| {
                view.ready = true;
                cx.notify();
            })
            .unwrap();
    })
    .detach();
    cx.wait_for(handle.into(), Duration::from_millis(100), |window, _| {
        window.try_find("loaded").is_some()
    })
    .await;
}
#[gpui_kit::test]
#[should_panic(expected = "UI condition timed out")]
async fn wait_for_has_a_bounded_failure(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, _| Loading { ready: false });
    cx.wait_for(handle.into(), Duration::from_millis(20), |window, _| {
        window.try_find("loaded").is_some()
    })
    .await;
}

struct ChoiceStates {
    checkbox: gpui_kit::base::CheckboxState,
    radio: bool,
    pressed: bool,
}
impl Render for ChoiceStates {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let checkbox = cx.entity();
        let radio = cx.entity();
        let toggle = cx.entity();
        div()
            .flex()
            .gap_4()
            .child(
                gpui_kit::base::Checkbox::new("mixed")
                    .state(self.checkbox)
                    .size(px(40.))
                    .on_change(move |state, _, _, cx| {
                        checkbox.update(cx, |this, cx| {
                            this.checkbox = state;
                            cx.notify();
                        })
                    }),
            )
            .child(
                gpui_kit::base::Radio::new("radio")
                    .checked(self.radio)
                    .size(px(40.))
                    .on_change(move |checked, _, _, cx| {
                        radio.update(cx, |this, cx| {
                            this.radio = checked;
                            cx.notify();
                        })
                    }),
            )
            .child(
                gpui_kit::base::Toggle::new("toggle")
                    .pressed(self.pressed)
                    .size(px(40.))
                    .on_change(move |pressed, _, _, cx| {
                        toggle.update(cx, |this, cx| {
                            this.pressed = pressed;
                            cx.notify();
                        })
                    }),
            )
    }
}
#[gpui_kit::test]
fn mixed_checkbox_radio_and_toggle_report_distinct_states(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let handle = cx.add_window(|_, _| ChoiceStates {
        checkbox: gpui_kit::base::CheckboxState::Indeterminate,
        radio: false,
        pressed: false,
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("mixed").checked(), Some(false));
        assert_eq!(window.find("mixed").indeterminate(), Some(true));
        window.click("mixed", cx);
        assert_eq!(window.find("mixed").checked(), Some(true));
        assert_eq!(window.find("mixed").indeterminate(), Some(false));
        window.click("radio", cx);
        assert_eq!(window.find("radio").checked(), Some(true));
        assert_eq!(window.find("radio").selected(), Some(true));
        window.click("toggle", cx);
        assert_eq!(window.find("toggle").checked(), Some(true));
        assert_eq!(
            window.find("toggle").expanded(),
            None,
            "unsupported properties remain unknown"
        );
    })
    .unwrap();
}

struct VirtualRows {
    scroll: gpui_kit::base::VirtualListScrollHandle,
}
impl Render for VirtualRows {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("viewport")
            .test_support()
            .w(px(100.))
            .h(px(60.))
            .child(
                gpui_kit::base::v_virtual_list(
                    cx.entity(),
                    "rows",
                    Rc::new(vec![size(px(100.), px(20.)); 1000]),
                    |_, range, _, _| {
                        range
                            .map(|index| {
                                div()
                                    .id(("virtual-row", index))
                                    .test_support()
                                    .h(px(20.))
                                    .child(format!("Row {index}"))
                            })
                            .collect()
                    },
                )
                .track_scroll(&self.scroll),
            )
    }
}
#[gpui_kit::test]
fn scrolling_a_real_virtual_list_registers_new_rows_and_releases_old_ones(cx: &mut TestAppContext) {
    let handle = cx.open_window(size(px(200.), px(200.)), |_, _| VirtualRows {
        scroll: gpui_kit::base::VirtualListScrollHandle::new(),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find(("virtual-row", 0usize)).visible());
        assert!(window.try_find(("virtual-row", 50usize)).is_none());
        window.scroll(
            "viewport",
            ScrollDelta::Pixels(point(px(0.), px(-1000.))),
            cx,
        );
        assert!(window.find(("virtual-row", 50usize)).visible());
        assert!(window.try_find(("virtual-row", 0usize)).is_none());
    })
    .unwrap();
}

struct Pair<T: Render> {
    left: gpui_kit::Entity<T>,
    right: gpui_kit::Entity<T>,
}
impl<T: Render> Render for Pair<T> {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .gap_4()
            .p_4()
            .child(div().id("left").w(px(180.)).child(self.left.clone()))
            .child(div().id("right").w(px(180.)).child(self.right.clone()))
    }
}

#[gpui_kit::test]
fn scoped_pointer_events_do_not_reach_duplicate_ids_in_another_scope(cx: &mut TestAppContext) {
    let left = Rc::new(RefCell::new(vec![]));
    let right = Rc::new(RefCell::new(vec![]));
    let handle = cx.add_window(|_, cx| Pair {
        left: cx.new(|_| Pointer {
            events: left.clone(),
        }),
        right: cx.new(|_| Pointer {
            events: right.clone(),
        }),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let mut dialog = window.within("right");
        dialog.hover("surface", cx);
        dialog.right_click("surface", cx);
        dialog.double_click("surface", cx);
    })
    .unwrap();
    assert!(left.borrow().is_empty());
    let events = right.borrow();
    assert!(events.iter().any(|event| event == "hover"));
    assert!(events.iter().any(|event| event == "right"));
    assert!(events.windows(2).any(|pair| pair == ["left:1", "left:2"]));
}

#[gpui_kit::test]
fn scoped_scroll_moves_only_the_selected_list(cx: &mut TestAppContext) {
    let left = ScrollHandle::new();
    let right = ScrollHandle::new();
    let handle = cx.add_window(|_, cx| Pair {
        left: cx.new(|_| Scrolling {
            scroll: left.clone(),
        }),
        right: cx.new(|_| Scrolling {
            scroll: right.clone(),
        }),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window
            .within("right")
            .scroll("list", ScrollDelta::Pixels(point(px(0.), px(-200.))), cx);
        assert!(window.within("right").find(("row", 10usize)).visible());
        assert!(!window.within("left").find(("row", 10usize)).visible());
    })
    .unwrap();
    assert_eq!(left.offset().y, px(0.));
    assert!(right.offset().y < px(0.));
}

#[gpui_kit::test]
fn scoped_drag_to_resolves_both_ids_inside_the_scope(cx: &mut TestAppContext) {
    let left = Rc::new(RefCell::new(0));
    let right = Rc::new(RefCell::new(0));
    let handle = cx.add_window(|_, cx| Pair {
        left: cx.new(|_| Dropping {
            drops: left.clone(),
        }),
        right: cx.new(|_| Dropping {
            drops: right.clone(),
        }),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.within("right").drag_to("source", "target", cx);
    })
    .unwrap();
    assert_eq!(*left.borrow(), 0);
    assert_eq!(*right.borrow(), 1);
}

#[gpui_kit::test]
fn drag_to_uses_native_drop_dispatch(cx: &mut TestAppContext) {
    let drops = Rc::new(RefCell::new(0));
    let handle = cx.add_window(|_, _| Dropping {
        drops: drops.clone(),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.drag_to("source", "target", cx);
    })
    .unwrap();
    assert_eq!(*drops.borrow(), 1);
}

struct KeyboardJump {
    left: gpui_kit::FocusHandle,
    right: gpui_kit::FocusHandle,
    keys: Rc<RefCell<Vec<String>>>,
}
impl Render for KeyboardJump {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let keys_left = self.keys.clone();
        let keys_right = self.keys.clone();
        let next = self.left.clone();
        div()
            .flex()
            .child(
                div().id("left").child(
                    div()
                        .id("field")
                        .test_support()
                        .track_focus(&self.left)
                        .size(px(40.))
                        .on_key_down(move |event, _, _| {
                            keys_left.borrow_mut().push(event.keystroke.key.clone())
                        }),
                ),
            )
            .child(
                div().id("right").child(
                    div()
                        .id("field")
                        .test_support()
                        .track_focus(&self.right)
                        .size(px(40.))
                        .on_key_down(move |event, window, cx| {
                            keys_right.borrow_mut().push(event.keystroke.key.clone());
                            next.focus(window, cx);
                        }),
                ),
            )
    }
}

#[gpui_kit::test]
fn scoped_input_stops_when_a_handler_moves_focus_to_another_scope(cx: &mut TestAppContext) {
    let keys = Rc::new(RefCell::new(vec![]));
    let handle = cx.add_window(|_, cx| KeyboardJump {
        left: cx.focus_handle(),
        right: cx.focus_handle(),
        keys: keys.clone(),
    });
    handle
        .update(cx, |view, window, cx| view.right.focus(window, cx))
        .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            window.within("right").input("ab", cx);
        }))
        .expect_err("the second character must not reach the other scope");
        assert!(
            error
                .downcast_ref::<String>()
                .unwrap()
                .contains("no observed keyboard focus inside scope")
        );
        assert_eq!(window.within("left").find("field").focused(), Some(true));
    })
    .unwrap();
    assert_eq!(&*keys.borrow(), &["a"]);
}

struct KeyboardFrames {
    focus: gpui_kit::FocusHandle,
    renders: Rc<std::cell::Cell<usize>>,
    keys: Rc<RefCell<Vec<String>>>,
}
impl Render for KeyboardFrames {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        let keys = self.keys.clone();
        div().id("editor-scope").child(
            div()
                .id("editor")
                .test_support()
                .track_focus(&self.focus)
                .size(px(40.))
                .on_key_down(move |event, _, _| {
                    keys.borrow_mut().push(event.keystroke.key.clone());
                }),
        )
    }
}

#[gpui_kit::test]
fn scoped_keyboard_refreshes_without_extra_renders(cx: &mut TestAppContext) {
    let renders = Rc::new(std::cell::Cell::new(0));
    let keys = Rc::new(RefCell::new(vec![]));
    let handle = cx.add_window(|_, cx| KeyboardFrames {
        focus: cx.focus_handle(),
        renders: renders.clone(),
        keys: keys.clone(),
    });
    let focus = handle.update(cx, |view, _, _| view.focus.clone()).unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        // External focus changes must be refreshed before the scoped guard runs.
        window.focus(&focus, cx);
        renders.set(0);
        window.within("editor-scope").input("abcdef", cx);
        assert_eq!(keys.borrow().len(), 6);
        // Compare steady-state rendering after the focus transition has settled.
        renders.set(0);
        window.within("editor-scope").input("abcdef", cx);
        let scoped_renders = renders.get();
        renders.set(0);
        window.input("abcdef", cx);
        assert_eq!(renders.get(), scoped_renders);
        assert_eq!(keys.borrow().len(), 18);
        renders.set(0);
        window.within("editor-scope").press("backspace", cx);
        let scoped_renders = renders.get();
        renders.set(0);
        window.press("backspace", cx);
        assert_eq!(renders.get(), scoped_renders);
        assert_eq!(keys.borrow().len(), 20);
    })
    .unwrap();
}
