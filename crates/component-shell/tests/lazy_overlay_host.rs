use std::{
    fs,
    ops::Deref as _,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, point, px};

struct TempApp(PathBuf);

static NEXT_APP: AtomicU64 = AtomicU64::new(0);

impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "gpui-component-shell-lazy-overlay-{}-{}",
            std::process::id(),
            NEXT_APP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("create application directory");
        fs::write(path.join("main.js"), source).expect("write application source");
        Self(path)
    }
}

impl Drop for TempApp {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            assert_eq!(error.kind(), std::io::ErrorKind::NotFound, "{error}");
        }
    }
}

struct Host(Entity<gpui_shell::ScriptView>);

impl gpui::Render for Host {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        self.0.clone()
    }
}

fn tree(context: &mut VisualTestContext, view: &Entity<gpui_shell::ScriptView>) -> String {
    context.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.build_error(), None);
        view.snapshot().expect("render snapshot").debug_tree()
    })
}

fn draw(context: &mut VisualTestContext) {
    context.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test]
fn popover_content_is_lazy_and_open_changes_cross_the_registered_boundary(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let app = TempApp::new(
        r#"
import { div, View } from "gpui-kit";
import { Popover } from "gpui-component";
export default class LazyPopover extends View {
  init() { this.open = false; this.hits = 0; }
  render() {
    return div().pt(100).child(
      new Popover("actions", "Open actions")
        .open(this.open)
        .on_open_change((open, cx) => { this.open = open; cx.notify(); })
        .content(div().w(200).h(120)
          .on_click((_event, cx) => { this.hits += 1; cx.notify(); })
          .child(`Lazy content ${this.hits}`))
    ).child(`Open:${this.open}`);
  }
}

"#,
    );
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let loaded = runtime
        .load_application(&app.0, "main.js")
        .expect("load application");
    let runtime_for_window = runtime.clone();
    let window = cx.add_window(move |window, cx| {
        Host(
            runtime_for_window
                .mount_application(&loaded, window, cx)
                .expect("mount application"),
        )
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let host = window.root(&mut context).expect("host view");
    let view = context.update(|_, cx| host.read(cx).0.clone());

    draw(&mut context);
    draw(&mut context);
    let closed = tree(&mut context, &view);
    assert!(closed.contains("Popover"), "{closed}");
    assert!(!closed.contains("@trigger"), "{closed}");
    assert!(!closed.contains("text \"Lazy content 1\""), "{closed}");
    context.simulate_click(point(px(100.), px(200.)), Modifiers::default());
    draw(&mut context);
    let still_closed = tree(&mut context, &view);
    assert!(
        !still_closed.contains("text \"Lazy content 1\""),
        "{still_closed}"
    );

    context.simulate_click(point(px(100.), px(120.)), Modifiers::default());
    draw(&mut context);
    let opened = tree(&mut context, &view);
    assert!(opened.contains("text \"Open:true\""), "{opened}");

    context.simulate_click(point(px(100.), px(200.)), Modifiers::default());
    draw(&mut context);
    let clicked = tree(&mut context, &view);
    assert!(clicked.contains("text \"Lazy content 1\""), "{clicked}");

    context.simulate_click(point(px(350.), px(380.)), Modifiers::default());
    draw(&mut context);
    context.simulate_click(point(px(100.), px(120.)), Modifiers::default());
    draw(&mut context);
    draw(&mut context);
    context.simulate_click(point(px(100.), px(200.)), Modifiers::default());
    draw(&mut context);
    let reopened = tree(&mut context, &view);
    assert!(reopened.contains("text \"Lazy content 2\""), "{reopened}");
}

#[gpui::test]
fn hover_card_builds_lazy_content_only_after_hover_and_reports_lifecycle(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let app = TempApp::new(
        r#"
import { div, View } from "gpui-kit";
import { HoverCard } from "gpui-component";
export default class LazyHoverCard extends View {
  init() { this.open = false; this.hits = 0; }
  render() {
    return div().v_flex().w(400).h(400)
      .child(div().w(400).h(100))
      .child(new HoverCard("profile")
        .trigger_element(div().w(300).h(40).child("Profile"))
        .content(div().w(200).h(120)
          .on_click((_event, cx) => { this.hits += 1; cx.notify(); })
          .child(`Lazy profile ${this.hits}`))
        .card_anchor("top_left")
        .open_delay(0).close_delay(0)
        .on_open_change(open => { this.open = open; }))
      .child(`Hover:${this.open}`);
  }
}

"#,
    );
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let loaded = runtime
        .load_application(&app.0, "main.js")
        .expect("load application");
    let runtime_for_window = runtime.clone();
    let window = cx.add_window(move |window, cx| {
        Host(
            runtime_for_window
                .mount_application(&loaded, window, cx)
                .expect("mount application"),
        )
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let host = window.root(&mut context).expect("host view");
    let view = context.update(|_, cx| host.read(cx).0.clone());
    draw(&mut context);
    draw(&mut context);

    context.simulate_click(point(px(100.), px(200.)), Modifiers::default());
    draw(&mut context);
    let before_hover = tree(&mut context, &view);
    assert!(
        !before_hover.contains("text \"Lazy profile 1\""),
        "{before_hover}"
    );

    context.simulate_mouse_move(point(px(100.), px(120.)), None, Modifiers::default());
    context
        .executor()
        .advance_clock(std::time::Duration::from_millis(60));
    context.run_until_parked();
    draw(&mut context);
    draw(&mut context);
    context.simulate_click(point(px(100.), px(200.)), Modifiers::default());
    draw(&mut context);
    let opened = tree(&mut context, &view);
    assert!(opened.contains("text \"Lazy profile 1\""), "{opened}");
    assert!(opened.contains("text \"Hover:true\""), "{opened}");
}

#[gpui::test]
fn dropdown_menu_opens_real_items_and_dispatches_the_selected_callback(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let app = TempApp::new(
        r#"
import { div, View } from "gpui-kit";
import { DropdownMenu } from "gpui-component";
export default class LazyMenu extends View {
  init() { this.choice = "none"; }
  render() {
    return div().pt(100)
      .child(new DropdownMenu("actions", "Actions").w(160)
        .item("Rename", cx => { this.choice = "rename"; cx.notify(); })
        .item("Archive", cx => { this.choice = "archive"; cx.notify(); }))
      .child(`Choice:${this.choice}`);
  }
}
"#,
    );
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let loaded = runtime.load_application(&app.0, "main.js").expect("load");
    let runtime_for_window = runtime.clone();
    let window = cx.add_window(move |window, cx| {
        Host(
            runtime_for_window
                .mount_application(&loaded, window, cx)
                .expect("mount"),
        )
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let host = window.root(&mut context).expect("host");
    let view = context.update(|_, cx| host.read(cx).0.clone());
    draw(&mut context);
    draw(&mut context);
    context.simulate_click(point(px(80.), px(120.)), Modifiers::default());
    draw(&mut context);
    draw(&mut context);
    context.simulate_click(point(px(80.), px(165.)), Modifiers::default());
    draw(&mut context);
    let selected = tree(&mut context, &view);
    assert!(selected.contains("text \"Choice:rename\""), "{selected}");
}
