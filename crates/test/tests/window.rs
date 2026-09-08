use gpui::{AppContext, Context, TestAppContext, Window, div, prelude::*, px, size};
use gpui_test::{ObserveElement, TestWindowExt};

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
                    .observe()
                    .text("Open")
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
                        .observe()
                        .text("Hello")
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
        let trigger = window.find("trigger").unwrap();
        assert_eq!(trigger.bounds().size, size(px(120.), px(32.)));
        assert_eq!(trigger.text(), Some("Open"));
        assert!(trigger.visible());
        assert!(window.find("popup").is_none());
        window.click("trigger", cx);
        assert_eq!(window.find("popup").unwrap().text(), Some("Hello"));
        window.click("trigger", cx);
        assert!(window.find("popup").is_none());
    })
    .unwrap();
}

struct Geometry;
impl Render for Geometry {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(div().id("fill").observe().w_full().h(px(20.)))
            .child(div().id("zero").observe().size(px(0.)))
            .child(
                div()
                    .id("hidden")
                    .observe()
                    .size(px(30.))
                    .invisible()
                    .child("Hidden"),
            )
            .child(div().id("transparent").observe().size(px(30.)).opacity(0.))
            .child(
                div().w(px(20.)).h(px(20.)).overflow_hidden().child(
                    div()
                        .id("clipped")
                        .observe()
                        .absolute()
                        .left(px(40.))
                        .size(px(10.)),
                ),
            )
            .child(
                div()
                    .id("offscreen")
                    .observe()
                    .absolute()
                    .left(px(2000.))
                    .size(px(10.)),
            )
            .when(window.viewport_size().width > px(400.), |this| {
                this.child(div().id("sidebar").observe().size(px(50.)))
            })
    }
}

#[gpui::test]
fn visibility_and_resize_use_resolved_geometry(cx: &mut TestAppContext) {
    let handle = cx.open_window(size(px(600.), px(500.)), |_, _| Geometry);
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(window.find("fill").unwrap().bounds().size.width, px(600.));
        for id in ["zero", "hidden", "transparent", "clipped", "offscreen"] {
            assert!(!window.find(id).unwrap().visible(), "{id}");
        }
        assert!(window.find("sidebar").unwrap().visible());
    })
    .unwrap();
    cx.simulate_window_resize(handle.into(), size(px(300.), px(400.)));
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(window.find("fill").unwrap().bounds().size.width, px(300.));
        assert!(window.find("sidebar").is_none());
    })
    .unwrap();
}

#[gpui::test]
fn windows_and_owned_snapshots_are_independent(cx: &mut TestAppContext) {
    let first = cx.add_window(|_, _| Example { open: false });
    let second = cx.add_window(|_, _| Example { open: true });
    cx.update_window(first.into(), |_, window, cx| {
        window.click("trigger", cx);
        let old = window.find("popup").unwrap();
        window.click("trigger", cx);
        assert!(window.find("popup").is_none());
        assert_eq!(old.text(), Some("Hello"));
    })
    .unwrap();
    cx.update_window(second.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        assert!(window.find("popup").is_some());
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
                    .observe()
                    .child(div().id("duplicate").observe().size(px(10.))),
            )
            .child(
                div()
                    .id("two")
                    .observe()
                    .child(div().id("duplicate").observe().size(px(10.))),
            )
    }
}

#[gpui::test]
#[should_panic(expected = "ambiguous ElementId")]
fn duplicate_local_ids_fail_clearly(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, _| Duplicate);
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        window.find("duplicate");
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
fn cached_paint_keeps_identity_and_text(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, cx| Cached {
        child: cx.new(|_| Example { open: true }),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        for _ in 0..3 {
            window.draw(cx).clear(cx);
            assert_eq!(window.find("popup").unwrap().text(), Some("Hello"));
            assert_eq!(
                window.find("trigger").unwrap().bounds().size.height,
                px(32.)
            );
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
                    .observe()
                    .size(px(100.))
                    .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
            )
            .child(
                div()
                    .id("cover")
                    .observe()
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
        assert!(window.find("covered").unwrap().visible());
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
            .observe()
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
        assert!(!window.find("focus-target").unwrap().focused());
        window.focus(&focus, cx);
        assert!(!window.find("focus-target").unwrap().focused());
        window.draw(cx).clear(cx);
        assert!(window.find("focus-target").unwrap().focused());
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
            .observe()
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
        window.type_text("Aa- 中🦀", cx);
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
                    .observe()
                    .w(px(200.))
                    .h(px(100.))
                    .flex()
                    .gap(px(10.))
                    .child(div().id("left").observe().size(px(40.)))
                    .child(div().id("right").observe().size(px(40.))),
            )
    }
}

#[gpui::test]
fn resolved_bounds_support_centering_containment_and_overlap_assertions(cx: &mut TestAppContext) {
    let handle = cx.open_window(size(px(600.), px(400.)), |_, _| CenteredLayout);
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
        let dialog = window.find("dialog").unwrap().bounds();
        let left = window.find("left").unwrap().bounds();
        let right = window.find("right").unwrap().bounds();
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
        assert!(window.find("popup").is_none());
    })
    .unwrap();
    other
        .update_window(second.into(), |_, window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(window.find("popup").unwrap().text(), Some("Hello"));
        })
        .unwrap();
    cx.update_window(first.into(), |_, window, cx| {
        window.click("trigger", cx);
        assert!(window.find("popup").is_some());
    })
    .unwrap();
    other
        .update_window(second.into(), |_, window, cx| {
            window.click("trigger", cx);
            assert!(window.find("popup").is_none());
        })
        .unwrap();
    cx.update_window(first.into(), |_, window, _| {
        assert!(window.find("popup").is_some())
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
        assert!(window.find("native").is_none());
    })
    .unwrap();
}
