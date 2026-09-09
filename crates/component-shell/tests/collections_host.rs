#[path = "../src/shell/support.rs"]
mod support;

#[path = "../src/shell/typed_child.rs"]
mod typed_child;

#[path = "../src/shell/collections/mod.rs"]
mod collections;

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
            "collections-{}-{}",
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
    collections::register(&mut registry).unwrap();
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

fn draw(context: &mut VisualTestContext, view: Entity<gpui_shell::ScriptView>) {
    drop(view);
    context.update(|window, cx| window.draw(cx).clear(cx));
}

fn distinct_rows(rows: Vec<(String, String, bool)>) -> Vec<(String, String, bool)> {
    let mut distinct: Vec<(String, String, bool)> = Vec::new();
    for row in rows {
        if let Some(existing) = distinct.iter_mut().find(|existing| existing.0 == row.0) {
            *existing = row;
        } else {
            distinct.push(row);
        }
    }
    distinct
}
#[test]
fn collection_catalog_stays_bounded() {
    let mut r = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    collections::register(&mut r).unwrap();
    let f = r.freeze().unwrap();
    assert_eq!(
        f.descriptors().map(|d| d.name()).collect::<Vec<_>>(),
        ["TreeItem", "Tree"]
    );
    for deferred in [
        "List",
        "DataTable",
        "Select",
        "Combobox",
        "SearchableList",
        "VirtualList",
    ] {
        assert!(f.descriptors().all(|d| d.name() != deferred));
    }
}

#[gpui::test]
fn tree_native_interaction_and_data_sync_survive_public_js_refresh(cx: &mut TestAppContext) {
    let source = r#"
import { View } from "gpui-kit";
import { Tree, TreeItem } from "gpui-component";
export default class App extends View {
  init() { this.renders = 0; }
  render() { const updated = this.renders++ > 0;
    const folder = new TreeItem("src", updated ? "Sources renamed" : "Source").expanded(true)
      .child(new TreeItem("main", "main.rs"));
    if (updated) folder.child(new TreeItem("lib", "lib.rs"));
    return new Tree("files").p(2).child(folder)
    .child(new TreeItem("readme", "README").disabled(true)); }
}
"#;
    let (mut context, view, _app) = mount(cx, source);
    collections::test_probe::take_rows();
    draw(&mut context, view.clone());
    context.update(|_, cx| {
        assert_eq!(view.read(cx).build_error(), None);
    });
    let initial = distinct_rows(collections::test_probe::take_rows());
    assert_eq!(
        initial,
        [
            ("src".into(), "Source".into(), false),
            ("main".into(), "main.rs".into(), false),
            ("readme".into(), "README".into(), false),
        ]
    );

    context.simulate_click(point(px(20.), px(20.)), Modifiers::default());
    context.run_until_parked();
    collections::test_probe::take_rows();
    draw(&mut context, view.clone());
    let collapsed = distinct_rows(collections::test_probe::take_rows());
    assert_eq!(
        collapsed,
        [
            ("src".into(), "Source".into(), true),
            ("readme".into(), "README".into(), false),
        ],
        "native click must select and collapse the folder"
    );

    context.update(|_, cx| view.update(cx, |view, cx| view.refresh(cx)));
    context.run_until_parked();
    collections::test_probe::take_rows();
    draw(&mut context, view.clone());
    context.update(|_, cx| {
        assert_eq!(view.read(cx).build_error(), None);
    });
    let synced = distinct_rows(collections::test_probe::take_rows());
    assert_eq!(
        synced,
        [
            ("src".into(), "Sources renamed".into(), true),
            ("readme".into(), "README".into(), false),
        ],
        "data must sync while native selection and collapse persist"
    );

    context.simulate_click(point(px(20.), px(20.)), Modifiers::default());
    context.run_until_parked();
    collections::test_probe::take_rows();
    draw(&mut context, view);
    let expanded = distinct_rows(collections::test_probe::take_rows());
    assert_eq!(
        expanded,
        [
            ("src".into(), "Sources renamed".into(), true),
            ("main".into(), "main.rs".into(), false),
            ("lib".into(), "lib.rs".into(), false),
            ("readme".into(), "README".into(), false),
        ],
        "expanding after refresh must reveal the synchronized structure"
    );
}

fn draw_invalid(cx: &mut TestAppContext, expression: &str) -> Vec<String> {
    let source = format!(
        r#"import {{ View, div }} from "gpui-kit";
import {{ Tree, TreeItem }} from "gpui-component";
export default class App extends View {{ render() {{ return {expression}; }} }}"#
    );
    let (mut context, view, _app) = mount(cx, &source);
    collections::test_probe::take_errors();
    draw(&mut context, view);
    collections::test_probe::take_errors()
}

#[gpui::test]
fn typed_materializer_boundary_rejects_ordinary_and_registered_wrong_children(
    cx: &mut TestAppContext,
) {
    let ordinary = draw_invalid(cx, "new Tree(\"ordinary\").child(div())");
    assert!(
        ordinary
            .iter()
            .any(|error| error.contains("ordinary element")),
        "{ordinary:?}"
    );
    let registered = draw_invalid(cx, "new Tree(\"outer\").child(new Tree(\"wrong\"))");
    assert!(
        registered
            .iter()
            .any(|error| error.contains("received Tree")),
        "{registered:?}"
    );
}
