//! `Root` owns the window's overlay layers: a view that never mentions them
//! still gets its dialogs, including when its content is cached.

mod common;
use gpui_kit::component::{Root, WindowExt as _, notification::Notification};
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{
    App, AppContext as _, Context, IntoElement, ParentElement as _, Render, Styled as _,
    TestAppContext, Window, WindowOptions, div, px, size,
};

/// A view that never mentions the layers.
struct PlainView;

impl Render for PlainView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("plain")
    }
}

fn open_and_notify(window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, |dialog, _, _| dialog.title("Hello"));
    window.push_notification(Notification::new().message("Saved").autohide(false), cx);
}

#[gpui_kit::test]
fn a_root_renders_the_layers_a_plain_view_leaves_out(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, Some(size(px(800.), px(600.))), |_window, cx| {
        let view = cx.new(|_| PlainView);
        view
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("dialog").is_none());
        open_and_notify(window, cx);
    })
    .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(
            window.find("dialog").visible(),
            "the dialog must reach the screen without the view rendering the layer"
        );
        assert!(window.find("notification").visible());
    })
    .unwrap();
}

#[gpui_kit::test]
fn open_window_wraps_the_view_in_a_root_and_returns_the_view(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, view) = cx
        .update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| PlainView))
        })
        .expect("open_window");
    let root = handle.downcast::<Root>().expect("the root view is a Root");
    let root_view = root.read_with(cx, |root, _| root.view().clone()).unwrap();
    assert_eq!(root_view.entity_id(), view.entity_id());
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        open_and_notify(window, cx);
    })
    .unwrap();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("dialog").visible());
        assert!(window.find("notification").visible());
    })
    .unwrap();
}

#[gpui_kit::test]
fn automatic_notifications_are_positioned_inside_the_window(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_reduce_motion(true);
    });
    let (handle, _) = common::open_window(cx, Some(size(px(800.), px(600.))), |_window, cx| {
        let view = cx.new(|_| PlainView);
        view
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.push_notification(Notification::new().message("Saved").autohide(false), cx);
        window.render_frame(cx);
        let bounds = window.find("notification").bounds();
        let viewport = window.viewport_size();
        assert!(bounds.origin.x >= px(0.) && bounds.origin.y >= px(0.));
        assert!(bounds.origin.x + bounds.size.width <= viewport.width);
        assert!(bounds.origin.y + bounds.size.height <= viewport.height);
    })
    .unwrap();
}

#[gpui_kit::test]
fn initialization_leaves_quit_and_close_shortcuts_to_the_application(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        for shortcut in ["cmd-q", "cmd-w", "alt-f4"] {
            let keystroke = gpui_kit::Keystroke::parse(shortcut).unwrap();
            assert!(
                cx.all_bindings_for_input(&[keystroke]).is_empty(),
                "{shortcut} must remain an application-owned binding"
            );
        }
    });
}

struct CachedContentHost {
    content: gpui_kit::Entity<CachedContent>,
}

struct CachedContent {
    renders: std::rc::Rc<std::cell::Cell<usize>>,
}

impl Render for CachedContent {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        div().size_full().child("cached content")
    }
}

impl Render for CachedContentHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            self.content
                .clone()
                .cached(gpui_kit::StyleRefinement::default().size_full()),
        )
    }
}

#[gpui_kit::test]
fn cached_content_does_not_duplicate_automatic_layers(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_reduce_motion(true);
    });
    let renders = std::rc::Rc::new(std::cell::Cell::new(0));
    let (handle, _) = common::open_window(cx, Some(size(px(800.), px(600.))), |_window, cx| {
        let content = cx.new(|_| CachedContent {
            renders: renders.clone(),
        });
        let view = cx.new(|_| CachedContentHost { content });
        view
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.open_sheet(cx, |sheet, _, _| sheet.title("Sheet"));
        open_and_notify(window, cx);
        window.render_frame(cx);
        assert!(window.find("dialog").visible());
        assert!(window.find("sheet-content").visible());
        assert!(window.find("notification").visible());
    })
    .unwrap();
    let first_renders = renders.get();
    assert!(first_renders > 0);
    cx.update_window(handle.into(), |_, window, cx| {
        // Unlike render_frame, draw does not force a refresh of cached views.
        window.draw(cx).clear(cx);
        assert!(window.find("dialog").visible());
        assert!(window.find("sheet-content").visible());
        assert!(window.find("notification").visible());
    })
    .unwrap();
    assert_eq!(
        renders.get(),
        first_renders,
        "the second frame must reuse the cached host"
    );
}

#[gpui_kit::test]
fn base_startup_keeps_the_same_root_even_when_component_is_compiled(cx: &mut TestAppContext) {
    cx.update(gpui_kit::base::init);
    let (window, content) = cx
        .update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| PlainView))
        })
        .unwrap();
    window
        .downcast::<gpui_kit::base::Root>()
        .unwrap()
        .read_with(cx, |root, _| {
            assert_eq!(root.view().entity_id(), content.entity_id());
        })
        .unwrap();
}
