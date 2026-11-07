#![cfg(feature = "components")]
#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;

use std::{
    fs,
    ops::Deref as _,
    path::PathBuf,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use gpui::{Entity, TestAppContext, VisualTestContext};

/// How many probe states have been dropped; only the test below makes them.
static DROPPED: AtomicUsize = AtomicUsize::new(0);

struct Probe;

impl Drop for Probe {
    fn drop(&mut self) {
        DROPPED.fetch_add(1, Ordering::SeqCst);
    }
}

struct TempApp(PathBuf);

impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "gpui-shell-state-release-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temporary application directory");
        fs::write(path.join("main.js"), source).expect("write temporary application entry");
        Self(path)
    }
}

impl Drop for TempApp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
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

fn components() -> gpui_shell::FrozenComponentRegistry {
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register_state(
            gpui_shell::StateDescriptor::new("ProbeState", "ProbeState", vec![], |_, _, _| {
                Ok(Box::new(Rc::new(Probe)))
            })
            .with_methods(vec![
                gpui_shell::StateMethodDescriptor::new::<Rc<Probe>>(
                    "label",
                    "(): string",
                    |_, _, _, _| Ok(gpui_shell::ComponentDataValue::String("live".into())),
                )
                .with_readonly(true),
            ]),
        )
        .unwrap();
    registry.freeze().unwrap()
}

#[gpui::test]
fn a_script_releases_component_state_before_its_application(cx: &mut TestAppContext) {
    cx.update(gpui_shell::init);
    let runtime = gpui_shell::ShellRuntime::new_isolated_with_components(components()).unwrap();
    let app = TempApp::new(
        r#"
import { div, View } from "gpui-kit";
import { ProbeState } from "gpui-component";

const outcome = (call) => {
  try { return String(call()); } catch (error) {
    if (error.message.includes("during render")) return "refused";
    if (error.message.includes("has been released")) return "released";
    return "threw";
  }
};

export default class Releases extends View {
  init() { this.kept = ProbeState(); this.dropped = ProbeState(); this.results = "none"; }
  run() {
    const first = this.dropped.release();
    const second = this.dropped.release();
    this.results = [first, second, outcome(() => this.dropped.label()), this.kept.label()].join("-");
  }
  render() {
    const during_render = outcome(() => this.kept.release());
    return div().size_full()
      .child(div().id(`render-${during_render}`))
      .child(div().id(`results-${this.results}`))
      .child(div().id("run").w(200).h(40).on_click((_event, cx) => { this.run(); cx.notify(); }));
  }
}
"#,
    );
    let loaded = runtime
        .load_application(&app.0, "main.js")
        .expect("load application");
    let mounted = Rc::new(std::cell::RefCell::new(None));
    let mounted_for_window = mounted.clone();
    let runtime_for_window = runtime.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_window
            .mount_application(&loaded, window, cx)
            .expect("mount application");
        *mounted_for_window.borrow_mut() = Some(view.clone());
        ScriptRoot(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let tree = |context: &mut VisualTestContext| {
        context.update(|_, cx| {
            let view = mounted.borrow().clone().expect("mounted view");
            assert_eq!(view.read(cx).build_error(), None);
            view.read(cx).snapshot().expect("snapshot").debug_tree()
        })
    };

    let before = tree(&mut context);
    assert!(before.contains("render-refused"), "{before}");
    assert_eq!(DROPPED.load(Ordering::SeqCst), 0);

    context.simulate_click(
        gpui::point(gpui::px(100.), gpui::px(20.)),
        gpui::Modifiers::none(),
    );
    context.update(|window, cx| window.draw(cx).clear(cx));

    let after = tree(&mut context);
    assert!(
        after.contains("results-true-false-released-live"),
        "{after}"
    );
    assert!(after.contains("render-refused"), "{after}");
    assert_eq!(DROPPED.load(Ordering::SeqCst), 1);
}
