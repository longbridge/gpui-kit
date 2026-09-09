#[path = "../src/shell/support.rs"]
mod support;

#[path = "../src/shell/typed_child.rs"]
mod typed_child;

#[path = "../src/shell/layout/mod.rs"]
mod layout;

use gpui::{Entity, IntoElement as _, TestAppContext, VisualTestContext};
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
            "layout-{}-{}",
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
) -> (VisualTestContext, Entity<gpui_shell::ScriptView>, TempApp) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let app = TempApp::new(source);
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    layout::register(&mut registry).unwrap();
    let runtime =
        gpui_shell::ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let loaded = runtime.load_application(&app.0, "main.js").unwrap();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.mount_application(&loaded, window, cx))
        .unwrap();
    (context, view, app)
}
fn draw(context: &mut VisualTestContext, view: Entity<gpui_shell::ScriptView>) {
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(800.), gpui::px(600.)),
        move |_, _| view.into_any_element(),
    );
}

#[test]
fn layout_catalog_has_closed_real_state_and_typed_layout_contracts() {
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    layout::register(&mut registry).unwrap();
    let frozen = registry.freeze().unwrap();

    let textarea = frozen
        .descriptors()
        .find(|item| item.name() == "Textarea")
        .unwrap();
    assert_eq!(
        textarea.constructors()[0].arguments()[0].schema(),
        &gpui_shell::ArgumentSchema::Entity("TextareaState")
    );
    assert!(
        textarea
            .methods()
            .iter()
            .any(|method| method.name() == "readonly")
    );

    let resizable = frozen
        .descriptors()
        .find(|item| item.name() == "Resizable")
        .unwrap();
    assert_eq!(
        resizable.methods()[0].arguments()[0].schema(),
        &gpui_shell::ArgumentSchema::Enum(&["horizontal", "vertical"])
    );
    assert!(
        !frozen
            .descriptors()
            .any(|item| matches!(item.name(), "Scrollbar" | "Scroll"))
    );
}

#[gpui::test]
fn textarea_state_survives_two_native_draws_with_methods_and_style(cx: &mut TestAppContext) {
    let source = r#"
import { View } from "gpui-kit";
import { Textarea, TextareaState } from "gpui-component";
export default class App extends View {
  init() { this.editor = TextareaState(); }
  render() { return new Textarea(this.editor).appearance(true).bordered(false).readonly(true).aria_label("Notes").disabled(false).p(2).h(120); }
}
"#;
    let (mut context, view, _app) = mount(cx, source);
    draw(&mut context, view.clone());
    let first = context.update(|_, cx| {
        assert_eq!(view.read(cx).build_error(), None);
        view.read(cx).snapshot().unwrap().debug_tree()
    });
    context.update(|_, cx| view.update(cx, |view, cx| view.refresh(cx)));
    draw(&mut context, view.clone());
    let second = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert_eq!(first, second);
    for expected in [
        "Textarea",
        ":readonly(registered)",
        ":bordered(registered)",
        ".p[Number(2.0)]",
    ] {
        assert!(second.contains(expected), "missing `{expected}`:\n{second}");
    }
}

#[gpui::test]
fn resizable_consumes_two_real_typed_panels_with_methods_style_and_children(
    cx: &mut TestAppContext,
) {
    let source = r#"
import { View, div } from "gpui-kit";
import { Resizable, ResizablePanel } from "gpui-component";
export default class App extends View { render() { return new Resizable("workspace").axis("horizontal").cross_size(240)
  .child(new ResizablePanel().size(180).size_range(100,260).p(2).child(div().child("Navigation")))
  .child(new ResizablePanel().visible(true).child(div().child("Content"))); } }
"#;
    let (mut context, view, _app) = mount(cx, source);
    draw(&mut context, view.clone());
    let tree = context.update(|_, cx| {
        assert_eq!(view.read(cx).build_error(), None);
        view.read(cx).snapshot().unwrap().debug_tree()
    });
    for expected in [
        "Resizable",
        ":axis(registered)",
        "ResizablePanel",
        ":size_range(registered)",
        ".p[Number(2.0)]",
        "Navigation",
        "Content",
    ] {
        assert!(tree.contains(expected), "missing `{expected}`:\n{tree}");
    }
    assert_eq!(tree.matches("ResizablePanel").count(), 2, "{tree}");
}
