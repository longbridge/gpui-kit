use std::{
    fs,
    ops::Deref as _,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use gpui::{Entity, IntoElement as _, TestAppContext, VisualTestContext};

static NEXT_APP: AtomicU64 = AtomicU64::new(0);

struct TempApp(PathBuf);

impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "gpui-component-shell-overlay-host-{}-{}",
            std::process::id(),
            NEXT_APP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create temporary application directory");
        fs::write(path.join("main.js"), source).expect("write temporary application entry");
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

impl gpui::Render for Empty {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
    }
}

fn mount(
    cx: &mut TestAppContext,
    source: &str,
) -> (VisualTestContext, Entity<gpui_shell::ScriptView>) {
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
    (context, view)
}

fn draw(context: &mut VisualTestContext, view: &Entity<gpui_shell::ScriptView>) {
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        {
            let view = view.clone();
            move |_, _| view.into_any_element()
        },
    );
}

#[gpui::test]
fn hover_card_materializes_real_trigger_content_style_and_closed_methods(cx: &mut TestAppContext) {
    let (mut context, view) = mount(
        cx,
        r#"
import { div, View } from "gpui-kit";
import { HoverCard } from "gpui-component";
export default class OverlayHost extends View {
  render() {
    return new HoverCard("profile")
      .trigger_element(div().child("Profile"))
      .content(div().child("Ada Lovelace"))
      .p(2)
      .card_anchor("bottom_center")
      .open_delay(125)
      .close_delay(250)
      .appearance(true)
      .on_open_change(open => { this.last_open = open; });
  }
}
"#,
    );
    draw(&mut context, &view);

    let tree = context.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.build_error(), None);
        view.snapshot().expect("hover-card snapshot").debug_tree()
    });
    for contract in [
        "HoverCard",
        ":trigger_element(registered)",
        "Ada Lovelace",
        ".p[Number(2.0)]",
        ":card_anchor(registered)",
        ":open_delay(registered)",
        ":close_delay(registered)",
        ":appearance(registered)",
        ":on_open_change(registered)",
    ] {
        assert!(tree.contains(contract), "missing {contract:?} in {tree}");
    }
}

#[gpui::test]
fn hover_card_rejects_whitespace_identity(cx: &mut TestAppContext) {
    let (mut context, view) = mount(
        cx,
        r#"
import { View } from "gpui-kit";
import { HoverCard } from "gpui-component";
export default class InvalidIdentity extends View {
  render() { return new HoverCard("  \t "); }
}
"#,
    );
    draw(&mut context, &view);
    let error = context.update(|_, cx| {
        view.read(cx)
            .build_error()
            .expect("whitespace identity must fail")
            .to_owned()
    });
    assert!(error.contains("HoverCard id must not be empty"), "{error}");
}
