#[path = "../src/shell/support.rs"]
mod support;

#[path = "../src/shell/media/mod.rs"]
mod media;

use gpui::{Entity, IntoElement as _, TestAppContext, VisualTestContext};
use std::{
    borrow::Cow,
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
            "media-{}-{}",
            std::process::id(),
            NEXT_APP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("assets")).unwrap();
        fs::write(path.join("main.js"), source).unwrap();
        fs::write(
            path.join("assets/pixel.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2"><rect width="2" height="2" fill="red"/></svg>"#,
        )
        .unwrap();
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

#[derive(Clone)]
struct EditorProbePayload(gpui_shell::ComponentArgument);

struct EditorProbe;
impl gpui_shell::ComponentMaterializer for EditorProbe {
    fn materialize(
        &self,
        mut request: gpui_shell::MaterializeRequest<'_>,
    ) -> gpui_shell::anyhow::Result<gpui::AnyElement> {
        let argument = &request
            .payload()
            .downcast_ref::<EditorProbePayload>()
            .ok_or_else(|| gpui_shell::anyhow::anyhow!("probe payload"))?
            .0;
        let state = request
            .with_state::<Entity<gpui_component::input::EditorState>, _>(argument, Clone::clone)?;
        let observation = request.with_window_app(|_, cx| {
            Ok(format!(
                "{:?}:{}",
                state.entity_id(),
                state.read(cx).value()
            ))
        })?;
        EDITOR_OBSERVATIONS.lock().unwrap().push(observation);
        Ok(gpui::div().into_any_element())
    }
}

static EDITOR_OBSERVATIONS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn register_probe(registry: &mut gpui_shell::ComponentRegistry) {
    registry
        .register(
            gpui_shell::ComponentDescriptor::new("EditorProbe", Arc::new(EditorProbe))
                .with_constructors(vec![gpui_shell::ConstructorDescriptor::new(
                    "EditorProbe",
                    vec![gpui_shell::ArgumentDescriptor::new(
                        "state",
                        gpui_shell::ArgumentSchema::Entity("EditorState"),
                    )],
                    |args| match args {
                        [argument @ gpui_shell::ComponentArgument::Entity { .. }] => Ok(
                            gpui_shell::ComponentPayload::new(EditorProbePayload(argument.clone())),
                        ),
                        _ => Err("EditorProbe expects EditorState".into()),
                    },
                )])
                .with_methods(vec![])
                .with_documentation("Test-only retained state probe."),
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
    media::register(&mut registry).unwrap();
    register_probe(&mut registry);
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
        gpui::size(gpui::px(640.), gpui::px(360.)),
        move |_, _| view.into_any_element(),
    );
}

#[gpui::test]
fn local_image_and_retained_editor_cross_the_public_host_and_draw(cx: &mut TestAppContext) {
    let source = r#"
import { div, View } from "gpui-kit";
import { Editor, EditorProbe, EditorState, Image } from "gpui-component";
export default class Media extends View {
  init() { this.editor = EditorState("fn main() {}"); }
  render() { return div()
    .child(new Image("assets/pixel.svg").w(24).h(24))
    .child(new Editor(this.editor).appearance(true).bordered(false).readonly(true).aria_label("Source").disabled(false).p(2).h(180))
    .child(new EditorProbe(this.editor)); }
}
"#;
    EDITOR_OBSERVATIONS.lock().unwrap().clear();
    let (mut context, view, _app) = mount(cx, source);
    for _ in 0..2 {
        draw(&mut context, view.clone());
        let tree = context.update(|_, cx| {
            assert_eq!(view.read(cx).build_error(), None);
            view.read(cx).snapshot().unwrap().debug_tree()
        });
        for expected in [
            "Image",
            "Editor",
            ":readonly(registered)",
            ".p[Number(2.0)]",
        ] {
            assert!(tree.contains(expected), "missing `{expected}`:\n{tree}");
        }
        context.update(|_, cx| view.update(cx, |view, cx| view.refresh(cx)));
    }
    let observations = EDITOR_OBSERVATIONS.lock().unwrap();
    assert!(observations.len() >= 2, "{observations:?}");
    assert!(
        observations
            .iter()
            .all(|value| value.ends_with(":fn main() {}")),
        "{observations:?}"
    );
    assert!(
        observations.windows(2).all(|pair| pair[0] == pair[1]),
        "{observations:?}"
    );
}

#[derive(Clone)]
struct RecordingAssets {
    inner: gpui_shell::AppAssets,
    loaded: Arc<Mutex<Vec<String>>>,
}

impl gpui::AssetSource for RecordingAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        self.loaded.lock().unwrap().push(path.to_owned());
        self.inner.load(path)
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        self.inner.list(path)
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

#[test]
fn native_image_draw_loads_through_the_installed_application_assets() {
    let app_dir = TempApp::new(
        r#"
import { View } from "gpui-kit";
import { Image } from "gpui-component";
export default class MediaImage extends View { render() { return new Image("assets/pixel.svg").size(32); } }
"#,
    );
    let loaded_paths = Arc::new(Mutex::new(Vec::new()));
    let assets = RecordingAssets {
        inner: gpui_shell::AppAssets::new(app_dir.0.clone()),
        loaded: loaded_paths.clone(),
    };
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    media::register(&mut registry).unwrap();
    let runtime =
        gpui_shell::ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let loaded = runtime.load_application(&app_dir.0, "main.js").unwrap();
    let mut app = gpui::TestApp::with_text_system_and_assets(
        Arc::new(gpui::NoopTextSystem::new()),
        Arc::new(assets),
    );
    app.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let mut window = app.open_window(move |window, cx| {
        ScriptRoot(runtime.mount_application(&loaded, window, cx).unwrap())
    });
    window.draw();
    window.draw();
    let paths = loaded_paths.lock().unwrap();
    assert!(
        paths.iter().any(|path| path == "assets/pixel.svg"),
        "{paths:?}"
    );
}

#[test]
fn catalog_exposes_only_renderable_media_surfaces() {
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    media::register(&mut registry).unwrap();
    let frozen = registry.freeze().unwrap();
    assert_eq!(
        frozen
            .descriptors()
            .map(|descriptor| descriptor.name())
            .collect::<Vec<_>>(),
        ["Image", "Editor"]
    );
    assert_eq!(
        frozen
            .states()
            .map(|state| state.export())
            .collect::<Vec<_>>(),
        ["EditorState"]
    );
}
