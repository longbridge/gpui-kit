#[path = "../src/shell/delegate_collections/mod.rs"]
mod delegate_collections;

use gpui::{Entity, TestAppContext, VisualTestContext};
use std::{fs, ops::Deref, path::PathBuf};

#[test]
fn delegate_collection_catalog_exposes_retained_list_contract() {
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    delegate_collections::register(&mut registry).unwrap();
    let registry = registry.freeze().unwrap();

    assert_eq!(
        registry
            .descriptors()
            .map(|item| item.name())
            .collect::<Vec<_>>(),
        ["List"]
    );
    assert_eq!(registry.states().count(), 0);
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

#[gpui::test]
fn list_uses_a_fresh_immutable_snapshot_and_lazy_row_renderer(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let root = std::env::temp_dir().join(format!("delegate-list-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.js"),
        r#"import { View, div } from "gpui-kit";
import { List } from "gpui-component";
export default class App extends View {
  init() { this.updated = false; }
  render() {
    const rows = this.updated ? [{id: "beta", label: "Beta"}] : [{id: "alpha", label: "Alpha"}];
    this.updated = true;
    return new List("people", () => rows, row => div().child(row.label));
  }
}"#,
    )
    .unwrap();
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    delegate_collections::register(&mut registry).unwrap();
    let runtime =
        gpui_shell::ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let loaded = runtime.load_application(&root, "main.js").unwrap();
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = mounted.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime.mount_application(&loaded, window, cx).unwrap();
        *capture.borrow_mut() = Some(view.clone());
        ScriptRoot(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = mounted.borrow().clone().unwrap();

    delegate_collections::test_probe::take_rows();
    context.update(|window, cx| window.draw(cx).clear(cx));
    let initial = delegate_collections::test_probe::take_rows();
    assert!(!initial.is_empty());
    assert!(initial.iter().all(|id| id == "alpha"), "{initial:?}");
    context.update(|_, cx| view.update(cx, |view, cx| view.refresh(cx)));
    context.run_until_parked();
    delegate_collections::test_probe::take_rows();
    context.update(|window, cx| window.draw(cx).clear(cx));
    let refreshed = delegate_collections::test_probe::take_rows();
    assert!(!refreshed.is_empty());
    assert!(refreshed.iter().all(|id| id == "beta"), "{refreshed:?}");
    context.update(|_, cx| assert_eq!(view.read(cx).build_error(), None));

    fs::remove_dir_all(PathBuf::from(root)).unwrap();
}
