#[path = "../src/shell/support.rs"]
mod support;

#[path = "../src/shell/typed_child.rs"]
mod typed_child;

#[path = "../src/shell/lifecycle/mod.rs"]
mod lifecycle;

use gpui::{
    AppContext as _, Modifiers, ParentElement as _, Styled as _, TestAppContext, VisualTestContext,
    point, px,
};
use gpui_component::Root;
use std::{cell::RefCell, fs, ops::Deref as _, path::PathBuf, rc::Rc};

struct TempApp(PathBuf);

impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "lifecycle-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("main.js"), source).unwrap();
        Self(path)
    }
}

impl Drop for TempApp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Host(gpui::Entity<gpui_shell::ScriptView>);

impl gpui::Render for Host {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div().size_full().child(self.0.clone())
    }
}

fn tree(context: &mut VisualTestContext, view: &gpui::Entity<gpui_shell::ScriptView>) -> String {
    context.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.build_error(), None);
        view.snapshot().unwrap().debug_tree()
    })
}

fn draw(context: &mut VisualTestContext) {
    context.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test]
fn tooltip_uses_the_native_managed_overlay_on_hover(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let app = TempApp::new(
        r#"
import { View } from "gpui-kit";
import { Tooltip } from "gpui-component";
export default class Example extends View {
  render() { return new Tooltip("help", "Help", "Open documentation"); }
}
"#,
    );
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    lifecycle::register(&mut registry).unwrap();
    let runtime =
        gpui_shell::ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let loaded = runtime.load_application(&app.0, "main.js").unwrap();
    let mounted = Rc::new(RefCell::new(None));
    let slot = mounted.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime.mount_application(&loaded, window, cx).unwrap();
        *slot.borrow_mut() = Some(view.clone());
        let host = cx.new(|_| Host(view));
        Root::new(host, window, cx)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = mounted.borrow().clone().unwrap();

    draw(&mut context);
    assert!(!tree(&mut context, &view).contains("Open documentation"));
    let payloads = lifecycle::test_probe::take();
    assert!(!payloads.is_empty());
    assert!(
        payloads
            .iter()
            .all(|payload| payload == &("help".into(), "Help".into(), "Open documentation".into()))
    );
    context.simulate_mouse_move(point(px(20.), px(20.)), None, Modifiers::default());
    context
        .executor()
        .advance_clock(std::time::Duration::from_millis(700));
    context.run_until_parked();
    draw(&mut context);
    draw(&mut context);
    assert!(tree(&mut context, &view).contains("Tooltip"));
}

#[gpui::test]
fn menu_bar_installs_native_and_component_menu_models_after_render(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let app = TempApp::new(
        r#"
import { View } from "gpui-kit";
import { MenuBar, Menu, MenuItem, MenuSeparator } from "gpui-component";
export default class Example extends View {
  render() {
    return new MenuBar("main-menu").child(
      new Menu("File")
        .child(new MenuItem("Open", "file.open"))
        .child(new MenuSeparator())
        .child(new MenuItem("Quit", "app.quit").disabled(true))
    );
  }
}
"#,
    );
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    lifecycle::register(&mut registry).unwrap();
    let runtime =
        gpui_shell::ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let loaded = runtime.load_application(&app.0, "main.js").unwrap();
    let mounted = Rc::new(RefCell::new(None));
    let slot = mounted.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime.mount_application(&loaded, window, cx).unwrap();
        *slot.borrow_mut() = Some(view.clone());
        let host = cx.new(|_| Host(view));
        Root::new(host, window, cx)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    draw(&mut context);
    context.run_until_parked();
    draw(&mut context);
    let view = mounted.borrow().clone().unwrap();
    let rendered = tree(&mut context, &view);
    assert!(rendered.contains("MenuBar"), "{rendered}");
    context.update(|_, _| {});
    context.run_until_parked();

    context.update(|_, cx| {
        let component_menus = gpui_component::GlobalState::global(cx).app_menus();
        assert_eq!(component_menus.len(), 1);
        assert_eq!(component_menus[0].name.as_ref(), "File");
        assert_eq!(component_menus[0].items.len(), 3);
    });
}
