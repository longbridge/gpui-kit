use std::{
    fs,
    ops::Deref,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use gpui::{Entity, IntoElement as _, Modifiers, TestAppContext, VisualTestContext, point, px};

static NEXT_APP: AtomicU64 = AtomicU64::new(0);

struct TempApp(PathBuf);

impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "gpui-component-shell-host-{}-{}",
            std::process::id(),
            NEXT_APP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("icons")).expect("create temporary asset directory");
        fs::write(path.join("main.js"), source).expect("write temporary application entry");
        fs::write(
            path.join("icons/check.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="m5 12 4 4L19 6"/></svg>"#,
        )
        .expect("write temporary SVG asset");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempApp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove temporary application directory");
    }
}

struct Empty;

impl gpui_shell::gpui::Render for Empty {
    fn render(
        &mut self,
        _: &mut gpui_shell::gpui::Window,
        _: &mut gpui_shell::gpui::Context<Self>,
    ) -> impl gpui_shell::gpui::IntoElement {
        gpui_shell::gpui::div()
    }
}

struct ScriptRoot(Entity<gpui_shell::ScriptView>);

impl gpui_shell::gpui::Render for ScriptRoot {
    fn render(
        &mut self,
        _: &mut gpui_shell::gpui::Window,
        _: &mut gpui_shell::gpui::Context<Self>,
    ) -> impl gpui_shell::gpui::IntoElement {
        self.0.clone()
    }
}

fn mount(
    cx: &mut TestAppContext,
    source: &str,
) -> (VisualTestContext, Entity<gpui_shell::ScriptView>, TempApp) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let app = TempApp::new(source);
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let loaded = runtime
        .load_application(app.path(), "main.js")
        .expect("load application");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.mount_application(&loaded, window, cx))
        .expect("mount application");
    (context, view, app)
}

fn draw(context: &mut VisualTestContext, view: Entity<gpui_shell::ScriptView>) {
    context.draw(
        gpui_shell::gpui::Point::default(),
        gpui_shell::gpui::size(gpui_shell::gpui::px(800.), gpui_shell::gpui::px(600.)),
        move |_, _| view.into_any_element(),
    );
}

#[gpui_shell::gpui::test]
fn icon_accepts_an_application_relative_asset_and_rejects_traversal(cx: &mut TestAppContext) {
    let valid = r#"
import { View } from "gpui-kit";
import { Icon } from "gpui-component";
export default class App extends View {
  render() { return new Icon("icons/check.svg").size("small"); }
}
"#;
    let (mut context, view, _app) = mount(cx, valid);
    draw(&mut context, view.clone());
    context.update(|_, cx| {
        assert_eq!(view.read(cx).build_error(), None);
        let tree = view.read(cx).snapshot().unwrap().debug_tree();
        assert!(tree.contains("Icon"), "{tree}");
        assert!(tree.contains(":size(registered)"), "{tree}");
    });

    let invalid = r#"
import { View } from "gpui-kit";
import { Icon } from "gpui-component";
export default class App extends View {
  render() { return new Icon("../outside.svg"); }
}
"#;
    let (mut context, view, _app) = mount(cx, invalid);
    draw(&mut context, view.clone());
    context.update(|_, cx| {
        let error = view.read(cx).build_error().expect("traversal must fail");
        assert!(error.contains("application asset root"), "{error}");
    });
}

#[gpui_shell::gpui::test]
fn sidebar_materializes_typed_items_menu_header_and_wrapper_style_in_order(
    cx: &mut TestAppContext,
) {
    let source = r#"
import { View, div } from "gpui-kit";
import {
  Sidebar, SidebarFooter, SidebarHeader, SidebarMenu, SidebarMenuItem, SidebarToggleButton,
} from "gpui-component";
export default class App extends View {
  render() {
    return div()
      .child(new SidebarToggleButton().side("left").collapsed(false).p(2))
      .child(
        new Sidebar("nav")
          .side("left")
          .collapsible("icon")
          .header(new SidebarHeader().selected(true).child("Workspace"))
          .footer(new SidebarFooter().child("Account"))
          .child(new SidebarMenu()
            .child(new SidebarMenuItem("First").selected(true).disabled(true))
            .child(new SidebarMenuItem("Second")))
      );
  }
}
"#;
    let (mut context, view, _app) = mount(cx, source);
    draw(&mut context, view.clone());
    let tree = context.update(|_, cx| {
        assert_eq!(view.read(cx).build_error(), None);
        view.read(cx).snapshot().unwrap().debug_tree()
    });
    for expected in [
        "SidebarToggleButton",
        ".p[Number(2.0)]",
        "Sidebar",
        "SidebarHeader",
        "Workspace",
        "SidebarFooter",
        "Account",
        "SidebarMenu",
        ":disabled[Bool(true)]",
        ":selected[Bool(true)]",
    ] {
        assert!(tree.contains(expected), "missing `{expected}`:\n{tree}");
    }
    assert_eq!(tree.matches("SidebarMenuItem").count(), 2, "{tree}");
    let first = tree.find("SidebarMenuItem").unwrap();
    let second = tree[first + 1..].find("SidebarMenuItem").unwrap() + first + 1;
    let disabled = tree.find(":disabled[Bool(true)]").unwrap();
    assert!(first < disabled && disabled < second, "{tree}");
}

#[gpui_shell::gpui::test]
fn sidebar_toggle_invokes_the_registered_common_click_callback(cx: &mut TestAppContext) {
    let source = r#"
import { View, div } from "gpui-kit";
import { Sidebar, SidebarMenu, SidebarMenuItem, SidebarToggleButton } from "gpui-component";
export default class App extends View {
  init() { this.hits = 0; this.nav_disabled = false; }
  render() {
    return div().w(300).h(100)
      .child(new SidebarToggleButton().w(120).h(40).on_click((_event, cx) => {
        this.hits += 1;
        cx.notify();
      }))
      .child(new Sidebar("disabled-nav").w(240).child(
        new SidebarMenu().child(
          new SidebarMenuItem("Disabled destination")
            .selected(true)
            .disabled(this.nav_disabled)
            .on_click((_event, cx) => {
              this.hits += 100;
              this.nav_disabled = true;
              cx.notify();
            })
        )
      ))
      .child(`Hits: ${this.hits}`);
  }
}
"#;
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let app = TempApp::new(source);
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let loaded = runtime
        .load_application(app.path(), "main.js")
        .expect("load application");
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let mounted_for_window = mounted.clone();
    let runtime_for_window = runtime.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_window
            .mount_application(&loaded, window, cx)
            .expect("mount application");
        *mounted_for_window.borrow_mut() = Some(view.clone());
        ScriptRoot(view)
    });
    let view = mounted.borrow().clone().expect("mounted view");
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.simulate_click(point(px(8.), px(8.)), Modifiers::default());
    context.run_until_parked();
    for x in [130., 170., 210.] {
        for y in [8., 20., 32., 44., 56., 68.] {
            context.simulate_click(point(px(x), px(y)), Modifiers::default());
            context.run_until_parked();
            context.update(|window, cx| window.draw(cx).clear(cx));
        }
    }
    context.update(|window, cx| window.draw(cx).clear(cx));
    let tree = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(tree.contains("Hits: 101"), "{tree}");
}
