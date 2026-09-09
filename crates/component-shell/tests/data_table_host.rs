#[path = "../src/shell/support.rs"]
mod support;

#[path = "../src/shell/data_table/mod.rs"]
mod data_table;

use gpui::{Entity, TestAppContext, VisualTestContext};
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
            "gpui-component-shell-data-table-{}-{}",
            std::process::id(),
            NEXT_APP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
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
) -> (VisualTestContext, Entity<gpui_shell::ScriptView>) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    data_table::register(&mut registry).unwrap();
    let runtime =
        gpui_shell::ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let app = TempApp::new(source);
    let loaded = runtime.load_application(&app.0, "main.js").unwrap();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.mount_application(&loaded, window, cx))
        .unwrap();
    (context, view)
}

#[gpui::test]
fn retained_data_table_renders_lazy_cells_from_plain_rows(cx: &mut TestAppContext) {
    data_table::test_probe::reset();
    let source = r#"
import { View, div } from "gpui-kit";
import { DataTableState, DataTable } from "gpui-component";
export default class App extends View { render() { return new DataTable(
  DataTableState(["name", "status"]),
  () => [{name: "Ada", status: "Ready"}, {name: "Lin", status: "Busy"}],
  (row, column) => div().child(row[column])
).stripe(true).bordered(false).row_selectable(true).cell_selectable(true); } }
"#;
    let (mut context, view) = mount(cx, source);
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(640.), gpui::px(360.)),
        {
            let view = view.clone();
            move |_, _| gpui::IntoElement::into_any_element(view)
        },
    );
    context.update(|_, cx| assert_eq!(view.read(cx).build_error(), None));
    assert!(data_table::test_probe::cell_builds() >= 4);
}

#[gpui::test]
fn data_table_rejects_non_array_snapshot_without_panicking(cx: &mut TestAppContext) {
    data_table::test_probe::reset();
    let (mut context, view) = mount(
        cx,
        r#"
import { View, div } from "gpui-kit"; import { DataTableState, DataTable } from "gpui-component";
export default class App extends View { render() { return new DataTable(DataTableState(["name"]), () => ({name:"Ada"}), () => div()); } }
"#,
    );
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        {
            let view = view.clone();
            move |_, _| gpui::IntoElement::into_any_element(view)
        },
    );
    context.update(|_, cx| assert_eq!(view.read(cx).build_error(), None));
    assert_eq!(data_table::test_probe::cell_builds(), 0);
    assert_eq!(data_table::test_probe::errors(), 1);
}
