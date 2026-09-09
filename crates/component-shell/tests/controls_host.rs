#[path = "../src/shell/support.rs"]
mod support;

#[path = "../src/shell/controls/mod.rs"]
mod controls;

use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, point, px};
use std::{
    fs,
    ops::Deref,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_APP: AtomicU64 = AtomicU64::new(0);

struct TempApp(PathBuf);

impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "controls-{}-{}",
            std::process::id(),
            NEXT_APP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("main.js"), source).unwrap();
        Self(path)
    }
}

impl Drop for TempApp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn mount(
    cx: &mut TestAppContext,
    source: &str,
) -> (VisualTestContext, Entity<gpui_shell::ScriptView>, TempApp) {
    cx.update(gpui_component_shell::init);
    let app = TempApp::new(source);
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    controls::register(&mut registry).unwrap();
    let runtime =
        gpui_shell::ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let loaded = runtime.load_application(&app.0, "main.js").unwrap();
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let slot = mounted.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime.mount_application(&loaded, window, cx).unwrap();
        *slot.borrow_mut() = Some(view.clone());
        gpui_component::Root::new(view, window, cx)
    });
    let context = VisualTestContext::from_window(*window.deref(), cx);
    let view = mounted.borrow().clone().unwrap();
    (context, view, app)
}

fn draw(context: &mut VisualTestContext) {
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));
}

/// A two-state control that cannot report its new state is set-only: the script
/// owns `checked`, so a click changes nothing and the control looks broken
/// while behaving exactly as registered. `on_change` is what closes the loop.
#[gpui::test]
fn clicking_a_two_state_control_reports_its_new_state(cx: &mut TestAppContext) {
    let source = r#"
import { View, div } from "gpui-kit";
import { Checkbox, Switch, Toggle } from "gpui-component";
export default class App extends View {
  init(_props, _cx) { this.checkbox = false; this.switch = false; this.toggle = false; }
  render() {
    return div().size_full()
      .child(new Checkbox("cb").label("Checkbox").checked(this.checkbox).absolute().left(0).top(0).w(200).h(30)
        .on_change((checked, cx) => { this.checkbox = checked; cx.notify(); }))
      .child(new Switch("sw").label("Switch").checked(this.switch).absolute().left(0).top(40).w(200).h(30)
        .on_change((checked, cx) => { this.switch = checked; cx.notify(); }))
      .child(new Toggle("tg").label("Toggle").checked(this.toggle).absolute().left(0).top(80).w(200).h(30)
        .on_change((checked, cx) => { this.toggle = checked; cx.notify(); }))
      .child(`state: ${this.checkbox}|${this.switch}|${this.toggle}`);
  }
}
"#;
    let (mut context, view, _app) = mount(cx, source);
    draw(&mut context);
    let initial = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(initial.contains("state: false|false|false"), "{initial}");

    for (label, position) in [
        ("checkbox", point(px(10.), px(14.))),
        ("switch", point(px(10.), px(54.))),
        ("toggle", point(px(10.), px(94.))),
    ] {
        context.simulate_click(position, Modifiers::default());
        draw(&mut context);
        let _ = label;
    }

    let clicked = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(
        clicked.contains("state: true|true|true"),
        "every control must report its click: {clicked}"
    );
}

/// A `Button` whose materializer honours `MaterializeRequest::on_click` still
/// cannot be clicked unless its descriptor declares the method: the engine
/// refuses a common behavior a registered component has not declared, so the
/// wiring on both ends is live and the call never arrives.
#[gpui::test]
fn clicking_a_button_reaches_the_script(cx: &mut TestAppContext) {
    let source = r#"
import { View, div } from "gpui-kit";
import { Button } from "gpui-component";
export default class App extends View {
  init(_props, _cx) { this.hits = 0; }
  render() {
    return div().size_full()
      .child(new Button("press").primary().label("Press me").absolute().left(0).top(0).w(200).h(32)
        .on_click((_event, cx) => { this.hits++; cx.notify(); }))
      .child(`hits: ${this.hits}`);
  }
}
"#;
    let (mut context, view, _app) = mount(cx, source);
    draw(&mut context);
    let before = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(before.contains("hits: 0"), "{before}");

    context.simulate_click(point(px(10.), px(16.)), Modifiers::default());
    draw(&mut context);

    let after = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(
        after.contains("hits: 1"),
        "the click must reach the script: {after}"
    );
}
