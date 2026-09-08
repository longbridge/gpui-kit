use gpui::{
    AppContext, Context, Entity, SharedString, TestAppContext, Window, div, prelude::*, px,
};
use gpui_kit::test::{TestSupportExt, TestWindowExt};

struct Child {
    label: SharedString,
}
impl Render for Child {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("child")
            .test_support()
            .aria_label(self.label.clone())
            .size(px(40.))
            .child(self.label.clone())
    }
}
struct Host {
    child: Entity<Child>,
    hidden: bool,
    mounted: bool,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .when(self.hidden, |this| this.invisible())
            .when(self.mounted, |this| {
                this.child(self.child.clone().cached(Default::default()))
            })
    }
}

#[gpui::test]
fn cached_child_hides_and_reappears_after_refresh(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, cx| Host {
        child: cx.new(|_| Child {
            label: "original".into(),
        }),
        hidden: false,
        mounted: true,
    });
    for hidden in [false, true, false] {
        handle
            .update(cx, |host, _, cx| {
                host.hidden = hidden;
                cx.notify();
            })
            .unwrap();
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            assert_eq!(window.find("child").visible(), !hidden);
        })
        .unwrap();
    }
}

#[gpui::test]
fn unmount_and_remount_do_not_retain_old_metadata(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, cx| Host {
        child: cx.new(|_| Child {
            label: "original".into(),
        }),
        hidden: false,
        mounted: true,
    });
    let old = cx
        .update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            window.find("child")
        })
        .unwrap();
    handle
        .update(cx, |host, _, cx| {
            host.mounted = false;
            cx.notify();
        })
        .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        assert!(window.try_find("child").is_none());
    })
    .unwrap();
    handle
        .update(cx, |host, _, cx| {
            host.child.update(cx, |child, cx| {
                child.label = "replacement".into();
                cx.notify();
            });
            host.mounted = true;
            cx.notify();
        })
        .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        assert_eq!(window.find("child").label(), Some("replacement"));
        assert_eq!(old.label(), Some("original"));
    })
    .unwrap();
}

struct Rows {
    reversed: bool,
}
impl Render for Rows {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let indices = if self.reversed {
            [2usize, 1, 0]
        } else {
            [0, 1, 2]
        };
        div().flex().flex_col().children(indices.map(|index| {
            let label = SharedString::from(format!("row {index}"));
            div()
                .id(("row", index))
                .test_support()
                .aria_label(label.clone())
                .h(px(30.))
                .w(px(100.))
                .child(label)
        }))
    }
}

#[gpui::test]
fn composite_ids_follow_reordered_rows(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, _| Rows { reversed: false });
    for reversed in [false, true, false] {
        handle
            .update(cx, |rows, _, cx| {
                rows.reversed = reversed;
                cx.notify();
            })
            .unwrap();
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            let first = window.find(("row", 0usize));
            let last = window.find(("row", 2usize));
            assert_eq!(first.label(), Some("row 0"));
            assert_eq!(last.label(), Some("row 2"));
            assert_eq!(first.bounds().top() > last.bounds().top(), reversed);
        })
        .unwrap();
    }
}

#[gpui::test]
fn closing_one_window_preserves_other_window_and_owned_snapshot(cx: &mut TestAppContext) {
    let first = cx.add_window(|_, _| Child {
        label: "first".into(),
    });
    let second = cx.add_window(|_, _| Child {
        label: "second".into(),
    });
    let snapshot = cx
        .update_window(first.into(), |_, window, cx| {
            window.draw(cx).clear(cx);
            window.find("child")
        })
        .unwrap();
    cx.update_window(first.into(), |_, window, _| window.remove_window())
        .unwrap();
    assert!(cx.update_window(first.into(), |_, _, _| ()).is_err());
    cx.update_window(second.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        assert_eq!(window.find("child").label(), Some("second"));
    })
    .unwrap();
    assert_eq!(snapshot.label(), Some("first"));
}

#[test]
fn observation_preserves_accessibility_role_and_properties() {
    use gpui::{Element, Role};
    let native = div()
        .id("control")
        .role(Role::Button)
        .aria_label("Save")
        .aria_description("Save document");
    let observed = div()
        .id("control")
        .role(Role::Button)
        .aria_label("Save")
        .aria_description("Save document")
        .test_support();
    assert_eq!(Element::id(&native), Element::id(&observed));
    assert_eq!(native.a11y_role(), observed.a11y_role());
    let mut expected = gpui::accesskit::Node::new(Role::Button);
    let mut actual = gpui::accesskit::Node::new(Role::Button);
    native.write_a11y_info(&mut expected);
    observed.write_a11y_info(&mut actual);
    assert_eq!(actual.label(), expected.label());
    assert_eq!(actual.description(), expected.description());
}

#[test]
#[should_panic(expected = "test_support requires an existing ElementId")]
fn observation_rejects_anonymous_elements() {
    let _ = div().test_support();
}

struct Clipped {
    clicks: std::rc::Rc<std::cell::Cell<usize>>,
}
impl Render for Clipped {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        div().w(px(20.)).h(px(40.)).overflow_hidden().child(
            div()
                .id("partly-visible")
                .test_support()
                .absolute()
                .w(px(100.))
                .h(px(40.))
                .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
        )
    }
}

#[gpui::test]
fn clipped_center_does_not_bypass_native_hit_testing(cx: &mut TestAppContext) {
    let clicks = std::rc::Rc::new(std::cell::Cell::new(0));
    let handle = cx.add_window(|_, _| Clipped {
        clicks: clicks.clone(),
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        let target = window.find("partly-visible");
        assert!(target.visible());
        assert_eq!(target.bounds().size.width, px(100.));
        window.click("partly-visible", cx);
        assert_eq!(clicks.get(), 0);
        window.click_at("partly-visible", gpui::point(px(10.), px(20.)), cx);
    })
    .unwrap();
    assert_eq!(clicks.get(), 1);
}

struct ManyRows {
    count: usize,
}
impl Render for ManyRows {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .children((0..self.count).map(|index| {
                div()
                    .id(("record", index))
                    .test_support()
                    .w(px(100.))
                    .h(px(1.))
                    .flex_shrink_0()
            }))
    }
}

#[gpui::test]
fn large_frame_removes_stale_records_when_list_shrinks(cx: &mut TestAppContext) {
    let handle = cx.add_window(|_, _| ManyRows { count: 1000 });
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        for index in 0..1000usize {
            assert_eq!(
                window.find(("record", index)).bounds().top(),
                px(index as f32)
            );
        }
    })
    .unwrap();
    handle
        .update(cx, |rows, _, cx| {
            rows.count = 10;
            cx.notify();
        })
        .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        for index in 0..1000usize {
            assert_eq!(window.try_find(("record", index)).is_some(), index < 10);
        }
    })
    .unwrap();
}
