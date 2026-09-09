#[path = "../src/shell/delegate_select/mod.rs"]
mod delegate_select;

use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, point, px};
use std::{fs, ops::Deref};

#[test]
fn select_catalog_exposes_native_retained_contract() {
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    delegate_select::register(&mut registry).unwrap();
    let registry = registry.freeze().unwrap();
    assert_eq!(
        registry
            .descriptors()
            .map(|item| item.name())
            .collect::<Vec<_>>(),
        ["Select"]
    );
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
fn select_native_click_emits_selected_stable_value(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let root = std::env::temp_dir().join(format!("delegate-select-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("main.js"), r#"import { View, div } from "gpui-kit";
import { Select } from "gpui-component";
export default class App extends View {
  render() {
    return div().size_full().child(new Select("people", () => [
      {id: "alpha", label: "Alpha"}, {id: "beta", label: "Beta"}
    ], row => div().child(row.label), value => { globalThis.__selected = value; }).placeholder("Choose"));
  }
}"#).unwrap();
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    delegate_select::register(&mut registry).unwrap();
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
    delegate_select::test_probe::take_selected();
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.simulate_click(point(px(20.), px(20.)), Modifiers::default());
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.simulate_click(point(px(20.), px(55.)), Modifiers::default());
    context.run_until_parked();
    assert_eq!(delegate_select::test_probe::take_selected(), ["alpha"]);
    context.update(|_, cx| assert_eq!(view.read(cx).build_error(), None));
    fs::remove_dir_all(root).unwrap();
}
