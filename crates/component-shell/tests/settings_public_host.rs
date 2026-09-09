#[path = "../src/shell/support.rs"]
mod support;

#[path = "../src/shell/typed_child.rs"]
mod typed_child;

#[path = "../src/shell/settings/mod.rs"]
mod settings;

use gpui::{Entity, IntoElement as _, ParentElement as _, TestAppContext, VisualTestContext};
use std::{
    fs,
    ops::Deref,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT_APP: AtomicU64 = AtomicU64::new(0);
struct TempApp(PathBuf);
impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "settings-{}-{}",
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

static LAZY_BUILDS: Mutex<Vec<String>> = Mutex::new(Vec::new());
struct LazyMarker;
impl gpui_shell::ComponentMaterializer for LazyMarker {
    fn materialize(
        &self,
        request: gpui_shell::MaterializeRequest<'_>,
    ) -> gpui_shell::anyhow::Result<gpui::AnyElement> {
        let label = request
            .payload()
            .downcast_ref::<String>()
            .ok_or_else(|| gpui_shell::anyhow::anyhow!("LazyMarker received incompatible payload"))?
            .clone();
        LAZY_BUILDS.lock().unwrap().push(label.clone());
        request.finish(gpui::div().child(label))
    }
}
fn register_marker(registry: &mut gpui_shell::ComponentRegistry) {
    registry
        .register(
            gpui_shell::ComponentDescriptor::new("LazyMarker", Arc::new(LazyMarker))
                .with_constructors(vec![gpui_shell::ConstructorDescriptor::new(
                    "LazyMarker",
                    vec![gpui_shell::ArgumentDescriptor::new(
                        "label",
                        gpui_shell::ArgumentSchema::String,
                    )],
                    |args| match args {
                        [gpui_shell::ComponentArgument::String(value)] => {
                            Ok(gpui_shell::ComponentPayload::new(value.clone()))
                        }
                        _ => Err("LazyMarker expects text".into()),
                    },
                )])
                .with_methods(vec![])
                .with_documentation("Test-only lazy materialization marker."),
        )
        .unwrap();
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
    settings::register(&mut registry).unwrap();
    register_marker(&mut registry);
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

#[gpui::test]
fn full_settings_hierarchy_rebuilds_lazy_native_slots_across_draws(cx: &mut TestAppContext) {
    let source = r#"
import { View, div } from "gpui-kit";
import { LazyMarker, Settings, SettingPage, SettingGroup, SettingItem } from "gpui-component";
export default class App extends View { render(){ return new Settings("prefs").size("small").sidebar_width(220).sidebar_size_range(160,320)
 .child(new SettingPage("General").description("Application preferences").default_open(true).content(new LazyMarker("suffix-built"))
  .child(new SettingGroup().title("Appearance").description("Visual choices").p(2)
   .child(new SettingItem("Theme").description("Choose appearance").layout("vertical").keywords(["color","theme"]).disabled(false).content(new LazyMarker("field-built"))))); } }
"#;
    LAZY_BUILDS.lock().unwrap().clear();
    let (mut context, view, _app) = mount(cx, source);
    let counts = || {
        let builds = LAZY_BUILDS.lock().unwrap();
        ["suffix-built", "field-built"].map(|label| {
            builds
                .iter()
                .filter(|built| built.as_str() == label)
                .count()
        })
    };
    assert_eq!(counts(), [0, 0], "lazy slots must not build during mount");

    let draw_and_assert_tree =
        |context: &mut VisualTestContext, view: &Entity<gpui_shell::ScriptView>| {
            context.draw(
                gpui::Point::default(),
                gpui::size(gpui::px(800.), gpui::px(600.)),
                {
                    let view = view.clone();
                    move |_, _| view.into_any_element()
                },
            );
            let tree = context.update(|_, cx| {
                assert_eq!(view.read(cx).build_error(), None);
                view.read(cx).snapshot().unwrap().debug_tree()
            });
            for expected in [
                "Settings",
                "SettingPage",
                "SettingGroup",
                "SettingItem",
                ":sidebar_size_range(registered)",
                ".p[Number(2.0)]",
            ] {
                assert!(tree.contains(expected), "missing `{expected}`:\n{tree}");
            }
        };

    draw_and_assert_tree(&mut context, &view);
    assert_eq!(
        counts(),
        [1, 1],
        "first draw must build each lazy slot once"
    );
    context.update(|_, cx| view.update(cx, |view, cx| view.refresh(cx)));
    assert_eq!(counts(), [1, 1], "refresh alone must not build lazy slots");
    draw_and_assert_tree(&mut context, &view);
    assert_eq!(
        counts(),
        [2, 2],
        "second draw must rebuild each lazy slot once"
    );
}

#[test]
fn catalog_names_the_real_native_hierarchy() {
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    settings::register(&mut registry).unwrap();
    let frozen = registry.freeze().unwrap();
    assert_eq!(
        frozen.descriptors().map(|d| d.name()).collect::<Vec<_>>(),
        ["SettingItem", "SettingGroup", "SettingPage", "Settings"]
    );
    assert!(frozen.states().next().is_none());
}
