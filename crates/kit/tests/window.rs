use gpui::{AppContext, Context, TestAppContext, Window, div, prelude::*, px, size};
use gpui_kit::test::{TestSupportExt, TestWindowExt};

struct Example {
    open: bool,
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(
                div()
                    .id("trigger")
                    .test_support()
                    .aria_label("Open")
                    .w(px(120.))
                    .h(px(32.))
                    .child("Open")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.open = !this.open;
                        cx.notify();
                    })),
            )
            .when(self.open, |this| {
                this.child(
                    div()
                        .id("popup")
                        .test_support()
                        .aria_label("Hello")
                        .w(px(200.))
                        .h(px(80.))
                        .child("Hello"),
                )
            })
    }
}

#[gpui::test]
fn finds_completed_layout_and_dispatches_click(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, _| Example { open: false });
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        let trigger = window.find("trigger");
        assert_eq!(trigger.bounds().size, size(px(120.), px(32.)));
        assert_eq!(trigger.label(), Some("Open"));
        assert!(trigger.visible());
        assert!(window.try_find("popup").is_none());
        window.click("trigger", cx);
        assert_eq!(window.find("popup").label(), Some("Hello"));
        window.click("trigger", cx);
        assert!(window.try_find("popup").is_none());
    })
    .unwrap();
}

struct Geometry;
impl Render for Geometry {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(div().id("fill").test_support().w_full().h(px(20.)))
            .child(div().id("zero").test_support().size(px(0.)))
            .child(
                div()
                    .id("hidden")
                    .test_support()
                    .size(px(30.))
                    .invisible()
                    .child("Hidden"),
            )
            .child(
                div()
                    .id("transparent")
                    .test_support()
                    .size(px(30.))
                    .opacity(0.),
            )
            .child(
                div().w(px(20.)).h(px(20.)).overflow_hidden().child(
                    div()
                        .id("clipped")
                        .test_support()
                        .absolute()
                        .left(px(40.))
                        .size(px(10.)),
                ),
            )
            .child(
                div()
                    .id("offscreen")
                    .test_support()
                    .absolute()
                    .left(px(2000.))
                    .size(px(10.)),
            )
            .when(window.viewport_size().width > px(400.), |this| {
                this.child(div().id("sidebar").test_support().size(px(50.)))
            })
    }
}

#[gpui::test]
fn visibility_and_resize_use_resolved_geometry(cx: &mut TestAppContext) {
    let handle = cx.open_window(size(px(600.), px(500.)), |_, _| Geometry);
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(window.find("fill").bounds().size.width, px(600.));
        for id in ["zero", "hidden", "transparent", "clipped", "offscreen"] {
            assert!(!window.find(id).visible(), "{id}");
        }
        assert!(window.find("sidebar").visible());
    })
    .unwrap();
    cx.simulate_window_resize(handle.into(), size(px(300.), px(400.)));
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(window.find("fill").bounds().size.width, px(300.));
        assert!(window.try_find("sidebar").is_none());
    })
    .unwrap();
}

#[gpui::test]
fn windows_and_owned_snapshots_are_independent(cx: &mut TestAppContext) {
    let first = cx.add_window(|_, _| Example { open: false });
    let second = cx.add_window(|_, _| Example { open: true });
    cx.update_window(first.into(), |_, window, cx| {
        window.click("trigger", cx);
        let old = window.find("popup");
        window.click("trigger", cx);
        assert!(window.try_find("popup").is_none());
        assert_eq!(old.label(), Some("Hello"));
    })
    .unwrap();
    cx.update_window(second.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert!(window.try_find("popup").is_some());
    })
    .unwrap();
}

struct Duplicate;
impl Render for Duplicate {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(
                div()
                    .id("one")
                    .test_support()
                    .child(div().id("duplicate").test_support().size(px(10.))),
            )
            .child(
                div()
                    .id("two")
                    .test_support()
                    .child(div().id("duplicate").test_support().size(px(10.))),
            )
    }
}

#[gpui::test]
#[should_panic(expected = "ambiguous ElementId")]
fn duplicate_local_ids_fail_clearly(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, _| Duplicate);
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        window.try_find("duplicate");
    })
    .unwrap();
}

struct Cached {
    child: gpui::Entity<Example>,
}
impl Render for Cached {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(self.child.clone().cached(gpui::StyleRefinement::default()))
    }
}

#[gpui::test]
fn cached_paint_keeps_identity_and_accessibility_label(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, cx| Cached {
        child: cx.new(|_| Example { open: true }),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        for _ in 0..3 {
            window.draw(cx).clear(cx);
            assert_eq!(window.find("popup").label(), Some("Hello"));
            assert_eq!(window.find("trigger").bounds().size.height, px(32.));
        }
    })
    .unwrap();
}

struct Covered {
    clicks: std::rc::Rc<std::cell::Cell<usize>>,
}
impl Render for Covered {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        div()
            .size_full()
            .child(
                div()
                    .id("covered")
                    .test_support()
                    .size(px(100.))
                    .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
            )
            .child(
                div()
                    .id("cover")
                    .test_support()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size(px(100.))
                    .occlude()
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation()),
            )
    }
}

#[gpui::test]
fn click_obeys_occlusion_instead_of_calling_callback(cx: &mut TestAppContext) {
    let clicks = std::rc::Rc::new(std::cell::Cell::new(0));
    let handle = cx.add_window(|_, _| Covered {
        clicks: clicks.clone(),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("covered", cx);
        // Geometric visibility deliberately does not claim that pixels are unoccluded.
        assert!(window.find("covered").visible());
    })
    .unwrap();
    assert_eq!(clicks.get(), 0);
}

#[gpui::test]
#[should_panic(expected = "missing ElementId")]
fn clicking_missing_target_reports_identity(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, _| Example { open: false });
    cx.update_window(handle.into(), |_, window, cx| window.click("absent", cx))
        .unwrap();
}

#[gpui::test]
#[should_panic(expected = "is not visible")]
fn clicking_hidden_target_is_rejected(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, _| Geometry);
    cx.update_window(handle.into(), |_, window, cx| window.click("hidden", cx))
        .unwrap();
}

struct FocusedView {
    focus: gpui::FocusHandle,
}
impl Render for FocusedView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("focus-target")
            .test_support()
            .size(px(100.))
            .track_focus(&self.focus)
    }
}

#[gpui::test]
fn focus_is_from_the_completed_frame(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, cx| FocusedView {
        focus: cx.focus_handle(),
    });
    let focus = handle.update(cx, |view, _, _| view.focus.clone()).unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(window.find("focus-target").focused(), Some(false));
        window.focus(&focus, cx);
        assert_eq!(window.find("focus-target").focused(), Some(false));
        window.draw(cx).clear(cx);
        assert_eq!(window.find("focus-target").focused(), Some(true));
    })
    .unwrap();
}

struct KeyCapture {
    focus: gpui::FocusHandle,
    keys: std::rc::Rc<std::cell::RefCell<Vec<gpui::Keystroke>>>,
}
impl Render for KeyCapture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let keys = self.keys.clone();
        div()
            .id("keys")
            .test_support()
            .size(px(100.))
            .track_focus(&self.focus)
            .on_key_down(move |event, _, _| keys.borrow_mut().push(event.keystroke.clone()))
    }
}

#[gpui::test]
fn typed_characters_follow_gpui_keystroke_semantics(cx: &mut TestAppContext) {
    let keys = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let handle = cx.add_window(|_, cx| KeyCapture {
        focus: cx.focus_handle(),
        keys: keys.clone(),
    });
    let focus = handle.update(cx, |view, _, _| view.focus.clone()).unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.focus(&focus, cx);
        window.input("Aa- 中🦀", cx);
    })
    .unwrap();
    let keys = keys.borrow();
    assert_eq!(keys.len(), 6);
    assert_eq!(keys[0].key, "a");
    assert!(keys[0].modifiers.shift);
    assert!(!keys[1].modifiers.shift);
    assert_eq!(keys[2].key_char.as_deref(), Some("-"));
    assert_eq!(keys[3].key_char.as_deref(), Some(" "));
    assert_eq!(keys[4].key_char.as_deref(), Some("中"));
    assert_eq!(keys[5].key_char.as_deref(), Some("🦀"));
}

struct CenteredLayout;
impl Render for CenteredLayout {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id("dialog")
                    .test_support()
                    .w(px(200.))
                    .h(px(100.))
                    .flex()
                    .gap(px(10.))
                    .child(div().id("left").test_support().size(px(40.)))
                    .child(div().id("right").test_support().size(px(40.))),
            )
    }
}

#[gpui::test]
fn resolved_bounds_support_centering_containment_and_overlap_assertions(cx: &mut TestAppContext) {
    let handle = cx.open_window(size(px(600.), px(400.)), |_, _| CenteredLayout);
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        let dialog = window.find("dialog").bounds();
        let left = window.find("left").bounds();
        let right = window.find("right").bounds();
        assert_eq!(dialog.center(), gpui::point(px(300.), px(200.)));
        assert!(left.left() >= dialog.left() && left.right() <= dialog.right());
        assert!(left.top() >= dialog.top() && left.bottom() <= dialog.bottom());
        assert!(!left.intersects(&right));
        assert_eq!(right.left() - left.right(), px(10.));
    })
    .unwrap();
}

#[gpui::test]
fn observations_do_not_cross_app_contexts(cx: &mut TestAppContext) {
    let mut other = cx.new_app();
    let first = cx.add_window(|_, _| Example { open: false });
    let second = other.add_window(|_, _| Example { open: true });
    cx.update_window(first.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert!(window.try_find("popup").is_none());
    })
    .unwrap();
    other
        .update_window(second.into(), |_, window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(window.find("popup").label(), Some("Hello"));
        })
        .unwrap();
    cx.update_window(first.into(), |_, window, cx| {
        window.click("trigger", cx);
        assert!(window.try_find("popup").is_some());
    })
    .unwrap();
    other
        .update_window(second.into(), |_, window, cx| {
            window.click("trigger", cx);
            assert!(window.try_find("popup").is_none());
        })
        .unwrap();
    cx.update_window(first.into(), |_, window, _| {
        assert!(window.try_find("popup").is_some())
    })
    .unwrap();
    other.quit();
}

struct NativeElement;
impl Render for NativeElement {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().id("native").size(px(100.))
    }
}

#[gpui::test]
fn native_elements_require_explicit_observation(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, _| NativeElement);
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert!(window.try_find("native").is_none());
    })
    .unwrap();
}

struct RepeatedObservation;
impl Render for RepeatedObservation {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("refined")
            .test_support()
            .aria_label("Original")
            .aria_toggled(gpui::accesskit::Toggled::False)
            .aria_selected(true)
            .test_support()
            .size(px(40.))
    }
}

#[gpui_kit::test]
fn repeated_observation_preserves_native_properties_and_identity(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, _| RepeatedObservation);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let snapshot = window.find("refined");
        assert_eq!(snapshot.label(), Some("Original"));
        assert_eq!(snapshot.checked(), Some(false));
        assert_eq!(snapshot.selected(), Some(true));
        assert_eq!(snapshot.bounds().size, size(px(40.), px(40.)));
    })
    .unwrap();
}

struct NativeProperties {
    checked: bool,
}
impl Render for NativeProperties {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("native-properties")
            .role(gpui::Role::CheckBox)
            .aria_toggled(if self.checked {
                gpui::accesskit::Toggled::True
            } else {
                gpui::accesskit::Toggled::False
            })
            .aria_selected(self.checked)
            .aria_expanded(!self.checked)
            .aria_label("Accessible name")
            .aria_value("Accessible value")
            .test_support()
            .size(px(40.))
            .on_click(cx.listener(|this, _, _, cx| {
                this.checked = !this.checked;
                cx.notify();
            }))
    }
}

#[gpui_kit::test]
fn native_properties_follow_rendered_changes(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, _| NativeProperties { checked: false });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let before = window.find("native-properties");
        assert_eq!(before.role(), Some(gpui::Role::CheckBox));
        assert_eq!(before.checked(), Some(false));
        assert_eq!(before.indeterminate(), Some(false));
        assert_eq!(before.selected(), Some(false));
        assert_eq!(before.expanded(), Some(true));
        // Accessible labels/values are not necessarily logical text/values.
        assert_eq!(before.label(), Some("Accessible name"));
        assert_eq!(before.value(), Some("Accessible value"));
        window.click("native-properties", cx);
        let after = window.find("native-properties");
        assert_eq!(after.checked(), Some(true));
        assert_eq!(after.indeterminate(), Some(false));
        assert_eq!(after.selected(), Some(true));
        assert_eq!(after.expanded(), Some(false));
        assert_eq!(before.checked(), Some(false));
    })
    .unwrap();
}

struct UnknownProperties;
impl Render for UnknownProperties {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("unknown")
            .test_support()
            .size(px(40.))
            .child("Drawn text")
    }
}

#[gpui_kit::test]
fn missing_native_properties_stay_unknown(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, _| UnknownProperties);
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let target = window.find("unknown");
        assert_eq!(
            target.label(),
            None,
            "child text is not an accessibility label"
        );
        assert_eq!(target.value(), None);
        assert_eq!(target.focused(), None);
        assert!(format!("{target:?}").contains("focused: None"));
        assert_eq!(target.checked(), None);
        assert_eq!(target.indeterminate(), None);
        assert_eq!(target.selected(), None);
        assert_eq!(target.expanded(), None);
        assert_eq!(
            target.disabled(),
            None,
            "no disabled flag does not mean enabled"
        );
    })
    .unwrap();
}

struct LateObservation {
    focus: gpui::FocusHandle,
}
impl Render for LateObservation {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("late-focus")
            .track_focus(&self.focus)
            .test_support()
            .size(px(40.))
    }
}

#[gpui_kit::test]
#[should_panic(expected = "focus binding was not observed")]
fn focus_query_diagnoses_observation_after_track_focus(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, cx| LateObservation {
        focus: cx.focus_handle(),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let snapshot = window.find("late-focus");
        assert!(format!("{snapshot:?}").contains("focused: <binding missed>"));
        snapshot.focused();
    })
    .unwrap();
}

struct FocusParts {
    handles: Vec<gpui::FocusHandle>,
}
impl Render for FocusParts {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .child(
                gpui_kit::base::AccordionTrigger::new("disclosure")
                    .track_focus(&self.handles[0])
                    .size_8(),
            )
            .child(
                gpui_kit::base::TableRow::new("row", 1)
                    .track_focus(&self.handles[1])
                    .size_8(),
            )
            .child(
                gpui_kit::base::TableCell::new("cell", 1)
                    .track_focus(&self.handles[2])
                    .size_8(),
            )
            .child(
                gpui_kit::base::TableHeader::new("header")
                    .track_focus(&self.handles[3])
                    .size_8(),
            )
    }
}
#[gpui_kit::test]
fn native_parts_forward_their_public_focus_binding(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, cx| FocusParts {
        handles: (0..4).map(|_| cx.focus_handle()).collect(),
    });
    let handles = handle
        .update(cx, |view, _, _| view.handles.clone())
        .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        for (id, focus) in ["disclosure", "row", "cell", "header"]
            .into_iter()
            .zip(&handles)
        {
            assert_eq!(window.find(id).focused(), Some(false));
            window.focus(focus, cx);
            window.render_frame(cx);
            assert_eq!(window.find(id).focused(), Some(true));
        }
    })
    .unwrap();
}

struct RenamedFocus {
    focus: gpui::FocusHandle,
}
impl Render for RenamedFocus {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("original")
            .test_support()
            .id("renamed")
            .track_focus(&self.focus)
            .id("final")
            .size(px(100.))
    }
}
#[gpui_kit::test]
fn renaming_observed_elements_preserves_focus_binding(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, cx| RenamedFocus {
        focus: cx.focus_handle(),
    });
    let focus = handle.update(cx, |view, _, _| view.focus.clone()).unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("final").focused(), Some(false));
        window.focus(&focus, cx);
        window.render_frame(cx);
        assert_eq!(window.find("final").focused(), Some(true));
        assert!(window.try_find("original").is_none());
        assert!(window.try_find("renamed").is_none());
        window.blur(cx);
        window.render_frame(cx);
        assert_eq!(window.find("final").focused(), Some(false));
    })
    .unwrap();
}
