#[path = "../src/shell/delegate_combobox/mod.rs"]
mod delegate_combobox;

use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, point, px};
use std::{fs, ops::Deref};

#[test]
fn combobox_catalog_exposes_native_single_select_contract() {
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    delegate_combobox::register(&mut registry).unwrap();
    assert_eq!(
        registry
            .freeze()
            .unwrap()
            .descriptors()
            .map(|item| item.name())
            .collect::<Vec<_>>(),
        ["Combobox"]
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
fn combobox_native_click_emits_change_and_confirm_for_stable_value(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let root = std::env::temp_dir().join(format!("delegate-combobox-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.js"),
        r#"import { View, div } from "gpui-kit";
import { Combobox } from "gpui-component";
export default class App extends View { render() {
  return div().size_full().child(new Combobox("people", () => [
    {id:"alpha",label:"Alpha"}, {id:"beta",label:"Beta"}
  ], value => { globalThis.__change = value; }, value => { globalThis.__confirm = value; })
    .searchable(false).placeholder("Choose"));
} }"#,
    )
    .unwrap();
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    delegate_combobox::register(&mut registry).unwrap();
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
    delegate_combobox::test_probe::take_changes();
    delegate_combobox::test_probe::take_confirms();
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.simulate_click(point(px(20.), px(20.)), Modifiers::default());
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.simulate_click(point(px(20.), px(55.)), Modifiers::default());
    context.run_until_parked();
    assert_eq!(
        delegate_combobox::test_probe::take_changes(),
        [vec!["alpha".to_owned()]]
    );
    assert_eq!(
        delegate_combobox::test_probe::take_confirms(),
        [vec!["alpha".to_owned()]]
    );
    context.update(|_, cx| assert_eq!(view.read(cx).build_error(), None));
    fs::remove_dir_all(root).unwrap();
}
