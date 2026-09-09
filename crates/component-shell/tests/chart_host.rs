#[path = "../src/shell/support.rs"]
mod support;

#[path = "../src/shell/chart/mod.rs"]
mod chart;

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
            "gpui-component-shell-chart-{}-{}",
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
    chart::register(&mut registry).unwrap();
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
fn concrete_charts_consume_plain_immutable_rows(cx: &mut TestAppContext) {
    let source = r#"
import { View, div } from "gpui-kit";
import { BarChart, LineChart, AreaChart, PieChart, RadarChart } from "gpui-component";
globalThis.calls = 0;
const rows = () => { globalThis.calls++; return [{label: "Jan", value: 2}, {label: "Feb", value: 5}]; };
export default class App extends View {
  render() { return div()
    .child(new BarChart(rows).grid(false).value_axis(true))
    .child(new LineChart(rows).linear().dot().grid(false))
    .child(new AreaChart(rows).step_after().grid(false))
    .child(new PieChart(rows).inner_radius(8).pad_angle(0.05).labels(true))
    .child(new RadarChart(rows).dot().grid_levels(3)); }
}
"#;
    let (mut context, view) = mount(cx, source);
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(800.), gpui::px(600.)),
        {
            let view = view.clone();
            move |_, _| gpui::IntoElement::into_any_element(view)
        },
    );
    context.update(|_, cx| {
        assert_eq!(view.read(cx).build_error(), None);
        let tree = view.read(cx).snapshot().unwrap().debug_tree();
        for name in [
            "BarChart",
            "LineChart",
            "AreaChart",
            "PieChart",
            "RadarChart",
        ] {
            assert!(tree.contains(name), "missing {name}:\n{tree}");
        }
    });
}

#[gpui::test]
fn chart_rows_reject_missing_fields_without_panicking(cx: &mut TestAppContext) {
    let (mut context, view) = mount(
        cx,
        r#"
import { View } from "gpui-kit";
import { BarChart } from "gpui-component";
export default class App extends View { render() { return new BarChart(() => [{label: "Jan"}]); } }
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
    assert!(
        chart::test_probe::take_error()
            .unwrap_or_default()
            .contains("finite number field `value`")
    );
}
