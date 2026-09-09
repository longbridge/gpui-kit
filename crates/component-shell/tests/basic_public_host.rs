use std::{
    fs,
    ops::Deref as _,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, point, px};

static NEXT_APP: AtomicU64 = AtomicU64::new(0);

struct TempApp(PathBuf);

impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "gpui-component-shell-basic-host-{}-{}",
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

struct ScriptRoot(Entity<gpui_shell::ScriptView>);

impl gpui::Render for ScriptRoot {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        self.0.clone()
    }
}

fn mount(
    cx: &mut TestAppContext,
    source: &str,
) -> (VisualTestContext, Entity<gpui_shell::ScriptView>) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let app = TempApp::new(source);
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
    let context = VisualTestContext::from_window(*window.deref(), cx);
    let view = mounted.borrow().clone().expect("mounted view");
    (context, view)
}

#[gpui::test]
fn basic_text_and_dropdown_materialize_through_public_host(cx: &mut TestAppContext) {
    let (mut context, view) = mount(
        cx,
        r#"
import { div, View } from "gpui-kit";
import { DropdownButton, Text } from "gpui-component";
export default class BasicRemaining extends View {
  init() { this.action_hits = 0; this.menu_hits = 0; }
  render() {
    return div()
      .child(new Text("Plain component text").p(2))
      .child(new DropdownButton("actions", "Actions")
        .absolute().left(0).top(60).w(180).h(40)
        .outline().disabled(false).selected(true)
        .size("small").variant("primary").menu_anchor("bottom_right")
        .on_click((_event, cx) => { this.action_hits += 1; cx.notify(); })
        .menu_item("Open", (cx) => { this.menu_hits += 1; cx.notify(); }))
      .child(`Counts: ${this.action_hits}|${this.menu_hits}`);
  }
}
"#,
    );
    context.update(|window, cx| window.draw(cx).clear(cx));
    let tree = context.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.build_error(), None);
        view.snapshot().expect("snapshot").debug_tree()
    });
    for expected in [
        "Text",
        ".p[Number(2.0)]",
        "DropdownButton",
        ":outline(registered)",
        ":size(registered)",
        ":variant(registered)",
        ":menu_anchor(registered)",
        ":menu_item(registered)",
    ] {
        assert!(tree.contains(expected), "missing `{expected}`:\n{tree}");
    }

    context.simulate_click(point(px(20.), px(72.)), Modifiers::default());
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));
    let after_action = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(after_action.contains("Counts: 1|0"), "{after_action}");

    let counts = |context: &mut VisualTestContext| {
        let tree = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
        let value = tree
            .split_once("Counts: ")
            .and_then(|(_, tail)| tail.split_once('"'))
            .map(|(value, _)| value)
            .expect("counts text");
        let (action, menu) = value.split_once('|').expect("two counters");
        (
            action.parse::<usize>().expect("action count"),
            menu.parse::<usize>().expect("menu count"),
            tree,
        )
    };
    context.simulate_click(point(px(90.), px(72.)), Modifiers::default());
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.simulate_keystrokes("down enter");
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));
    let (action_hits, menu_hits, after_menu) = counts(&mut context);
    assert!(action_hits >= 1, "{after_menu}");
    assert!(menu_hits >= 1, "{after_menu}");
}
