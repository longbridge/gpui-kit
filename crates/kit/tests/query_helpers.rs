mod common;
use gpui_kit::test::{TestQueryExt, TestSupportExt, TestWindowExt};
use gpui_kit::{AppContext, Context, Role, TestAppContext, Window, div, prelude::*, px, size};

struct AccessibleScopes;
impl Render for AccessibleScopes {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().flex().children(["toolbar", "dialog"].map(|scope| {
            div()
                .id(scope)
                .role(Role::Group)
                .aria_label("Container")
                .test_support()
                .flex()
                .flex_col()
                .w(px(100.))
                .child(
                    div()
                        .id("save")
                        .role(Role::Button)
                        .aria_label("保存")
                        .test_support()
                        .size(px(40.))
                        .child("Rendered text"),
                )
                .child(
                    div()
                        .id("status")
                        .role(Role::Status)
                        .aria_label(scope)
                        .test_support()
                        .size(px(40.)),
                )
        }))
    }
}

#[gpui_kit::test]
fn accessible_queries_are_exact_scoped_and_ordered(cx: &mut TestAppContext) {
    let (handle, _) = common::open_window(cx, Some(size(px(300.), px(200.))), |_, cx| {
        cx.new(|_| AccessibleScopes)
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.elements().len(), 6);
        assert_eq!(window.find_all_by_role(Role::Button).len(), 2);
        assert_eq!(window.find_all_by_label("保存").len(), 2);
        assert!(window.try_find_by_label("Rendered text").is_none());
        assert!(window.try_find_by_label("保").is_none());
        assert_eq!(window.find_by_label("toolbar").role(), Some(Role::Status));
        let scoped = window.within("dialog");
        assert_eq!(scoped.elements().len(), 2);
        assert!(scoped.find_all_by_role(Role::Group).is_empty());
        assert!(scoped.try_find_by_label("Container").is_none());
        assert!(scoped.find_all_by_label("toolbar").is_empty());
        assert_eq!(scoped.find_all_by_role(Role::Button).len(), 1);
        assert_eq!(scoped.find_all_by_label("保存").len(), 1);
        assert_eq!(
            scoped.find_by_label("保存").path().last(),
            Some(&"save".into())
        );
        let ordered = scoped.elements();
        assert_eq!(ordered[0].role(), Some(Role::Button));
        assert_eq!(ordered[1].role(), Some(Role::Status));
        assert!(ordered[0].bounds().origin.y < ordered[1].bounds().origin.y);
    })
    .unwrap();
}

#[gpui_kit::test]
#[should_panic(expected = "Matching paths:")]
fn duplicate_accessible_labels_require_a_scope(cx: &mut TestAppContext) {
    let (handle, _) = common::open_window(cx, None, |_, cx| cx.new(|_| AccessibleScopes));
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.try_find_by_label("保存");
    })
    .unwrap();
}

#[gpui_kit::test]
#[should_panic(expected = "Registered paths:")]
fn missing_accessible_labels_explain_the_registered_frame(cx: &mut TestAppContext) {
    let (handle, _) = common::open_window(cx, None, |_, cx| cx.new(|_| AccessibleScopes));
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.within("dialog").find_by_label("Missing");
    })
    .unwrap();
}

struct ChangingLabel {
    saved: bool,
}
impl Render for ChangingLabel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("status")
            .role(Role::Status)
            .aria_label(if self.saved { "Saved" } else { "Pending" })
            .test_support()
            .size(px(40.))
    }
}

#[gpui_kit::test]
fn accessible_queries_refresh_while_owned_snapshots_remain_unchanged(cx: &mut TestAppContext) {
    let (handle, content) =
        common::open_window(cx, None, |_, cx| cx.new(|_| ChangingLabel { saved: false }));
    let before = cx
        .update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            window.find_by_label("Pending")
        })
        .unwrap();
    common::update_content(handle, &content, cx, |view, _, cx| {
        view.saved = true;
        cx.notify();
    })
    .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find_by_label("Pending").is_none());
        assert_eq!(window.find_by_label("Saved").path(), before.path());
        assert_eq!(before.label(), Some("Pending"));
    })
    .unwrap();
}

struct HiddenTargets;
impl Render for HiddenTargets {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().children(["second", "first"].map(|id| {
            div()
                .id(id)
                .role(Role::Status)
                .aria_label("Hidden status")
                .test_support()
                .absolute()
                .top_0()
                .left_0()
                .size(px(40.))
                .invisible()
        }))
    }
}

#[gpui_kit::test]
fn invisible_queries_include_registrations_and_order_equal_origins_by_path(
    cx: &mut TestAppContext,
) {
    let (handle, _) = common::open_window(cx, None, |_, cx| cx.new(|_| HiddenTargets));
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let matches = window.find_all_by_label("Hidden status");
        assert_eq!(matches.len(), 2);
        assert!(matches.iter().all(|element| !element.visible()));
        assert_eq!(matches[0].bounds().origin, matches[1].bounds().origin);
        assert_eq!(matches[0].path().last(), Some(&"first".into()));
        assert_eq!(matches[1].path().last(), Some(&"second".into()));
        assert_eq!(window.find_all_by_role(Role::Status).len(), 2);
    })
    .unwrap();
}
