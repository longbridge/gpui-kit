#[path = "../src/shell/window_effects/mod.rs"]
mod window_effects;

use gpui::{
    AppContext as _, Modifiers, ParentElement as _, Styled as _, TestAppContext, VisualTestContext,
    point, px,
};
use gpui_component::{Root, WindowExt as _};
use std::{
    cell::RefCell,
    fs,
    ops::Deref as _,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT_APP: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static BUILDS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn clear_builds() {
    BUILDS.with(|builds| builds.borrow_mut().clear());
}

fn push_build(label: String) {
    BUILDS.with(|builds| builds.borrow_mut().push(label));
}

fn builds() -> Vec<String> {
    BUILDS.with(|builds| builds.borrow().clone())
}

struct Host(gpui::Entity<gpui_shell::ScriptView>);
impl gpui::Render for Host {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
            .size_full()
            .child(self.0.clone())
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

struct TempApp(PathBuf);
impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "window-effects-{}-{}",
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
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Marker;
impl gpui_shell::ComponentMaterializer for Marker {
    fn materialize(
        &self,
        request: gpui_shell::MaterializeRequest<'_>,
    ) -> gpui_shell::anyhow::Result<gpui::AnyElement> {
        let label = request.payload().downcast_ref::<String>().unwrap().clone();
        if label == "fail" {
            gpui_shell::anyhow::bail!("forced lazy factory failure");
        }
        push_build(label.clone());
        request.finish(gpui::div().child(label))
    }
}

fn register_marker(registry: &mut gpui_shell::ComponentRegistry) {
    registry
        .register(
            gpui_shell::ComponentDescriptor::new("EffectMarker", Arc::new(Marker))
                .with_constructors(vec![gpui_shell::ConstructorDescriptor::new(
                    "EffectMarker",
                    vec![gpui_shell::ArgumentDescriptor::new(
                        "label",
                        gpui_shell::ArgumentSchema::String,
                    )],
                    |args| match args {
                        [gpui_shell::ComponentArgument::String(value)] => {
                            Ok(gpui_shell::ComponentPayload::new(value.clone()))
                        }
                        _ => Err("EffectMarker expects a label".into()),
                    },
                )])
                .with_methods(vec![])
                .with_documentation("Test-only lazy marker."),
        )
        .unwrap();
}

fn mount_isolated(
    cx: &mut TestAppContext,
    source: &str,
) -> (
    VisualTestContext,
    gpui::Entity<gpui_shell::ScriptView>,
    TempApp,
) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let app = TempApp::new(source);
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    window_effects::register(&mut registry).unwrap();
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
    let context = VisualTestContext::from_window(*window.deref(), cx);
    let view = mounted.borrow().clone().unwrap();
    (context, view, app)
}

#[test]
fn catalog_exposes_only_closed_command_triggers() {
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    window_effects::register(&mut registry).unwrap();
    let frozen = registry.freeze().unwrap();
    assert_eq!(
        frozen
            .descriptors()
            .map(|item| item.name())
            .collect::<Vec<_>>(),
        ["Dialog", "AlertDialog", "Sheet", "Notification"]
    );
    for descriptor in frozen.descriptors() {
        assert_eq!(
            descriptor.constructors()[0].arguments()[2].schema(),
            &gpui_shell::ArgumentSchema::Callback("(message: string, cx: Context) => void")
        );
    }
}

#[gpui::test]
fn real_click_events_open_native_surfaces_and_build_lazy_content(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    clear_builds();
    let app = TempApp::new(
        r#"
import { View, div } from "gpui-kit";
import { Dialog, AlertDialog, Sheet, Notification, EffectMarker } from "gpui-component";
export default class App extends View {
 init() { this.errors = 0; this.closed = 0; }
 render() { const report = (_message, cx) => { this.errors += 1; cx.notify(); };
  return div().v_flex().gap(2)
   .child(new Dialog("dialog", "Open dialog", report).w(180).title("Native dialog").on_cancel(cx => { this.closed += 1; cx.notify(); }).on_close(cx => { this.closed += 1; cx.notify(); }).content(new EffectMarker("dialog-lazy")))
   .child(new Sheet("sheet", "Open sheet", report).w(180).title("Native sheet").placement("left").on_close(cx => { this.closed += 1; cx.notify(); }).content(new EffectMarker("sheet-lazy")))
   .child(new AlertDialog("alert", "Open alert", report).w(180).title("Native alert").description("Closed contract").show_cancel(true).on_cancel(cx => { this.closed += 1; cx.notify(); }).on_close(cx => { this.closed += 1; cx.notify(); }))
   .child(new Notification("note", "Notify", report).w(180).title("Saved").message("Native notification").type("success").autohide(false))
   .child(new Dialog("fail-dialog", "Open failing dialog", report).w(180).content(new EffectMarker("fail")))
   .child(`Errors:${this.errors}`).child(`Closed:${this.closed}`);
 }
}
"#,
    );
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    window_effects::register(&mut registry).unwrap();
    register_marker(&mut registry);
    let runtime =
        gpui_shell::ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let loaded = runtime.load_application(&app.0, "main.js").unwrap();
    let mounted = Rc::new(RefCell::new(None));
    let mounted_for_window = mounted.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime.mount_application(&loaded, window, cx).unwrap();
        *mounted_for_window.borrow_mut() = Some(view.clone());
        let host = cx.new(|_| Host(view));
        Root::new(host, window, cx)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = mounted.borrow().clone().unwrap();
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|window, cx| window.draw(cx).clear(cx));
    assert!(builds().is_empty());

    context.simulate_click(point(px(80.), px(16.)), Modifiers::default());
    context.update(|window, cx| assert!(window.has_active_dialog(cx)));
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|window, cx| window.draw(cx).clear(cx));
    assert!(builds().iter().any(|item| item == "dialog-lazy"));
    context
        .update(|window, cx| window.dispatch_action(Box::new(gpui_component::dialog::Cancel), cx));

    assert!(!builds().iter().any(|item| item == "sheet-lazy"));
    context.simulate_click(point(px(80.), px(50.)), Modifiers::default());
    context.update(|window, cx| assert!(window.has_active_sheet(cx)));
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|window, cx| window.draw(cx).clear(cx));
    assert!(builds().iter().any(|item| item == "sheet-lazy"));
    context.simulate_click(point(px(320.), px(48.)), Modifiers::default());
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|window, cx| assert!(!window.has_active_sheet(cx)));

    context.simulate_click(point(px(80.), px(84.)), Modifiers::default());
    context.update(|window, cx| assert!(window.has_active_dialog(cx)));
    context
        .update(|window, cx| window.dispatch_action(Box::new(gpui_component::dialog::Cancel), cx));

    context.simulate_click(point(px(80.), px(118.)), Modifiers::default());
    context.update(|window, cx| assert_eq!(window.notifications(cx).len(), 1));

    context.simulate_click(point(px(80.), px(152.)), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|window, cx| window.draw(cx).clear(cx));
    let tree = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(tree.contains("Errors:1"), "{tree}");
    assert!(tree.contains("Closed:4"), "{tree}");

    context.update(|window, cx| window.close_dialog(cx));
    context.simulate_click(point(px(80.), px(152.)), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|window, cx| window.draw(cx).clear(cx));
    let reopened = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(reopened.contains("Errors:2"), "{reopened}");
}

#[gpui::test]
fn closed_alert_and_notification_reject_common_named_slots(cx: &mut TestAppContext) {
    for expression in [
        r#"new AlertDialog("alert", "Alert", (_message, _cx) => {}).content(div())"#,
        r#"new AlertDialog("alert", "Alert", (_message, _cx) => {}).trigger(div())"#,
        r#"new AlertDialog("alert", "Alert", (_message, _cx) => {}).header(div())"#,
        r#"new AlertDialog("alert", "Alert", (_message, _cx) => {}).footer(div())"#,
        r#"new Notification("note", "Notify", (_message, _cx) => {}).content(div())"#,
        r#"new Notification("note", "Notify", (_message, _cx) => {}).trigger(div())"#,
        r#"new Notification("note", "Notify", (_message, _cx) => {}).header(div())"#,
        r#"new Notification("note", "Notify", (_message, _cx) => {}).footer(div())"#,
    ] {
        let source = format!(
            r#"import {{ View, div }} from "gpui-kit";
import {{ AlertDialog, Notification }} from "gpui-component";
export default class App extends View {{ render() {{ return {}; }} }}"#,
            expression
        );
        let (mut context, view, _app) = mount_isolated(cx, &source);
        window_effects::test_probe::take_slot_rejections();
        context.update(|window, cx| window.draw(cx).clear(cx));
        context.update(|window, cx| window.draw(cx).clear(cx));
        context.update(|_, cx| assert_eq!(view.read(cx).build_error(), None));
        let errors = window_effects::test_probe::take_slot_rejections();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("do not accept named slots")),
            "{expression}: {errors:?}"
        );
    }
}

#[gpui::test]
fn dialog_and_sheet_duplicate_content_is_last_call_wins(cx: &mut TestAppContext) {
    for (expression, second) in [
        (
            r#"new Dialog("dialog", "Dialog", (_message, _cx) => {}).content(new EffectMarker("dialog-first")).content(new EffectMarker("dialog-second"))"#,
            "dialog-second",
        ),
        (
            r#"new Sheet("sheet", "Sheet", (_message, _cx) => {}).content(new EffectMarker("sheet-first")).content(new EffectMarker("sheet-second"))"#,
            "sheet-second",
        ),
    ] {
        let source = format!(
            r#"import {{ View }} from "gpui-kit";
import {{ Dialog, Sheet, EffectMarker }} from "gpui-component";
export default class App extends View {{ render() {{ return {}; }} }}"#,
            expression
        );
        cx.update(|cx| {
            gpui_component_shell::init(cx);
        });
        clear_builds();
        let app = TempApp::new(&source);
        let mut registry = gpui_shell::ComponentRegistry::new(
            gpui_shell::COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        window_effects::register(&mut registry).unwrap();
        register_marker(&mut registry);
        let runtime =
            gpui_shell::ShellRuntime::new_isolated_with_components(registry.freeze().unwrap())
                .unwrap();
        let loaded = runtime.load_application(&app.0, "main.js").unwrap();
        let window = cx.add_window(move |window, cx| {
            let view = runtime.mount_application(&loaded, window, cx).unwrap();
            let host = cx.new(|_| Host(view));
            Root::new(host, window, cx)
        });
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        context.update(|window, cx| window.draw(cx).clear(cx));
        context.update(|window, cx| window.draw(cx).clear(cx));
        assert!(builds().is_empty(), "{expression}");
        context.simulate_click(point(px(40.), px(16.)), Modifiers::default());
        context.update(|window, cx| window.draw(cx).clear(cx));
        context.update(|window, cx| window.draw(cx).clear(cx));
        let builds = builds();
        assert!(builds.iter().all(|built| built == second), "{builds:?}");
        assert!(!builds.is_empty(), "{expression}");
    }
}

#[gpui::test]
fn failed_factory_and_failed_reporter_are_both_diagnosed(cx: &mut TestAppContext) {
    let source = r#"import { View } from "gpui-kit";
import { Dialog, EffectMarker } from "gpui-component";
export default class App extends View { render() { return new Dialog("fail", "Fail", () => { throw new Error("reporter exploded"); }).content(new EffectMarker("fail")); } }"#;
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let app = TempApp::new(source);
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    window_effects::register(&mut registry).unwrap();
    register_marker(&mut registry);
    let runtime =
        gpui_shell::ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let loaded = runtime.load_application(&app.0, "main.js").unwrap();
    let window = cx.add_window(move |window, cx| {
        let view = runtime.mount_application(&loaded, window, cx).unwrap();
        let host = cx.new(|_| Host(view));
        Root::new(host, window, cx)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    window_effects::test_probe::take_reporter_failures();
    context.simulate_click(point(px(40.), px(16.)), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    let errors = window_effects::test_probe::take_reporter_failures();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("forced lazy factory failure"),
        "{errors:?}"
    );
    assert!(errors[0].contains("reporter exploded"), "{errors:?}");
}
