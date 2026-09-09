#[path = "../src/shell/scroll/mod.rs"]
mod scroll;

use gpui::{Entity, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px};
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
            "scroll-{}-{}",
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
    scroll::register(&mut registry).unwrap();
    let runtime =
        gpui_shell::ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let loaded = runtime.load_application(&app.0, "main.js").unwrap();
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let mounted_for_window = mounted.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime.mount_application(&loaded, window, cx).unwrap();
        *mounted_for_window.borrow_mut() = Some(view.clone());
        ScriptRoot(view)
    });
    let context = VisualTestContext::from_window(*window.deref(), cx);
    let view = mounted.borrow().clone().unwrap();
    (context, view, app)
}

fn draw(context: &mut VisualTestContext) {
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));
}

#[test]
fn catalog_exposes_state_and_two_closed_surfaces_only() {
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    scroll::register(&mut registry).unwrap();
    let frozen = registry.freeze().unwrap();
    assert_eq!(
        frozen
            .descriptors()
            .map(|item| item.name())
            .collect::<Vec<_>>(),
        ["Scroll", "Scrollbar"]
    );
    assert_eq!(frozen.states().count(), 1);
    assert_eq!(
        frozen
            .descriptors()
            .find(|item| item.name() == "Scroll")
            .unwrap()
            .constructors()[0]
            .arguments()[0]
            .schema(),
        &gpui_shell::ArgumentSchema::Entity("ScrollbarHandle")
    );
}

#[gpui::test]
fn shared_native_handle_scrolls_and_preserves_offset_across_refresh(cx: &mut TestAppContext) {
    let source = r#"
import { View, div } from "gpui-kit";
import { ScrollbarHandle, Scroll, Scrollbar } from "gpui-component";
export default class App extends View {
  init() { this.scroll = ScrollbarHandle(); }
  render() { return div().relative().w(160).h(100)
    .child(new Scroll(this.scroll).scroll_axis("vertical").size_full()
      .child(div().h(400).flex_shrink(0).child("Tall shared content")))
    .child(new Scrollbar("main-scrollbar", this.scroll).scroll_axis("vertical").mode("always").viewport_from_layout(true)); }
}
"#;
    let (mut context, view, _app) = mount(cx, source);
    draw(&mut context);
    context.update(|_, cx| assert_eq!(view.read(cx).build_error(), None));
    let handle = scroll::test_probe::latest_handle();
    assert_eq!(handle.offset().y, px(0.));

    context.simulate_event(ScrollWheelEvent {
        position: point(px(40.), px(40.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(-60.))),
        ..Default::default()
    });
    draw(&mut context);
    let after_wheel = handle.offset().y;
    assert_ne!(
        after_wheel,
        px(0.),
        "wheel must mutate the shared native handle"
    );

    context.update(|_, cx| view.update(cx, |view, cx| view.refresh(cx)));
    draw(&mut context);
    let after_refresh = scroll::test_probe::latest_handle();
    assert_eq!(
        after_refresh.offset().y,
        after_wheel,
        "retained state must survive JS refresh"
    );
}

fn invalid_errors(cx: &mut TestAppContext, expression: &str) -> Vec<String> {
    let source = format!(
        r#"import {{ View, div }} from "gpui-kit";
import {{ ScrollbarHandle, Scrollbar }} from "gpui-component";
export default class App extends View {{ init() {{ this.h = ScrollbarHandle(); }} render() {{ return {expression}; }} }}"#
    );
    let (mut context, _view, _app) = mount(cx, &source);
    scroll::test_probe::take_errors();
    draw(&mut context);
    scroll::test_probe::take_errors()
}

#[gpui::test]
fn native_scrollbar_rejects_children_and_shell_style_at_materializer_boundary(
    cx: &mut TestAppContext,
) {
    let children = invalid_errors(cx, "new Scrollbar(\"child-error\", this.h).child(div())");
    assert!(
        children
            .iter()
            .any(|error| error.contains("does not accept children")),
        "{children:?}"
    );
    let style = invalid_errors(cx, "new Scrollbar(\"style-error\", this.h).p(2)");
    assert!(
        style
            .iter()
            .any(|error| error.contains("does not support shell style")),
        "{style:?}"
    );
}
