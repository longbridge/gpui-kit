//! End-to-end tests for the script render protocol.
//!
//! These exercise the whole path — VM, method dispatch, spec arena, event
//! callbacks — without painting a frame, because the element description is
//! plain data. They run against whichever engine is enabled, which is what
//! keeps the fallback engine honest.

use crate::{
    HostModule, HostValue, ScriptView, ShellRuntime, capability::Capabilities, policy::Policy,
};
use gpui::{
    AppContext as _, IntoElement as _, Modifiers, ParentElement as _, TestAppContext,
    VisualTestContext, point, px,
};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
};

const COUNTER: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex, Button } from "gpui-base";

export default class Counter extends View {
  init() {
    this.count = 0;
  }

  render(cx) {
    return v_flex()
      .size_full()
      .items_center()
      .gap_2()
      .p(16)
      .bg(`#ffffff`)
      .child(div().text_color(`#111111`).child(`Count: ${this.count}`))
      .child(
        Button.new("increment")
          .px(12)
          .py(6)
          .rounded(6)
          .bg(`#2563eb`)
          .on_click((event, cx) => {
            this.count += 1;
            cx.notify();
          })
          .child(div().text_color(`#ffffff`).child("Increment")),
      );
  }
}
"#;

/// The entry name only affects diagnostics, but each engine has its own
/// convention and the tests should read the way real code does.
const ENTRY: &str = "counter.js";

#[gpui::test]
fn component_is_imported_by_name_and_receives_an_explicit_id(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let received = Rc::new(RefCell::new(None));
    let received_by_builder = received.clone();
    let module = HostModule::new("mail").component("Body", move |mut args, _, _| {
        *received_by_builder.borrow_mut() = Some((
            args.id().to_owned(),
            args.props().clone(),
            args.children().len(),
        ));
        gpui::div()
            .children(args.take_children())
            .into_any_element()
    });
    crate::policy::set_default(
        Policy::new()
            .with_host_module(module)
            .expect("component module"),
    );

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { Body } from "mail";

export default class HostBody extends View {
  render() {
    return Body.new("message-body", { html: "<strong>Hello</strong>", zoom: 1.25 })
      .child(div().child("fallback"));
  }
}

"#;
    let view_type = runtime.load_source("host-body.js", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("render");

    assert!(
        tree.contains("module_component mail.Body \"message-body\""),
        "{tree}"
    );
    assert!(tree.contains("html"), "props missing from dump: {tree}");
    assert!(tree.contains("text \"fallback\""), "child missing: {tree}");
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
    assert_eq!(
        received.borrow().as_ref().map(|(_, _, children)| *children),
        Some(1),
        "the host owns the materialized children",
    );
    assert_eq!(
        received.borrow().as_ref().map(|(id, _, _)| id.as_str()),
        Some("message-body"),
    );
    assert_eq!(
        received
            .borrow()
            .as_ref()
            .and_then(|(_, props, _)| props.get("zoom"))
            .and_then(HostValue::as_number),
        Some(1.25),
    );
    crate::policy::set_default(Policy::new());
}

#[gpui::test]
fn component_requires_an_explicit_string_id(cx: &mut TestAppContext) {
    crate::policy::set_default(
        Policy::new()
            .with_host_module(
                HostModule::new("mail").component("Body", |_, _, _| gpui::div().into_any_element()),
            )
            .expect("component module"),
    );

    for (name, expression) in [
        ("missing-component-id.js", "Body.new()"),
        ("numeric-component-id.js", "Body.new(42, {})"),
    ] {
        let source = format!(
            r#"
import {{ View }} from "gpui-kit";
import {{ Body }} from "mail";
export default class InvalidBody extends View {{
  render() {{ return {expression}; }}
}}
"#,
        );
        let message = render_error(cx, name, &source);
        assert!(
            message.contains("non-empty string id"),
            "unexpected error for {expression}: {message}"
        );
    }

    crate::policy::set_default(Policy::new());
}

#[gpui::test]
fn revoking_a_component_module_releases_its_builder_while_the_view_lives(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let retained = Rc::new(());
    let captured = retained.clone();
    crate::policy::set_default(
        Policy::new()
            .with_host_module(
                HostModule::new("extension").component("Leaf", move |_, _, _| {
                    let _keep_alive = &captured;
                    gpui::div().into_any_element()
                }),
            )
            .expect("component module"),
    );

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source(
            "extension.js",
            r#"
import { View } from "gpui-kit";
import { Leaf } from "extension";
export default class Extension extends View {
  render() { return Leaf.new("leaf", {}); }
}
"#,
        )
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    draw(&mut context, &view);
    assert_eq!(Rc::strong_count(&retained), 2);

    crate::clear_exported_modules();
    assert_eq!(
        Rc::strong_count(&retained),
        1,
        "a live snapshot must not retain a revoked component factory"
    );
    drop(view);
    crate::policy::set_default(Policy::new());
}

#[gpui::test]
fn text_view_records_format_content_and_behavior(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View } from "gpui-kit";
import { TextView } from "gpui-base";
export default class RichText extends View {
  render() {
    return TextView.html("message", "<p>Hello <strong>world</strong></p>")
      .selectable(true)
      .scrollable(false)
      .on_link_click((_url, _cx) => {});
  }
}
"#;
    let view_type = runtime.load_source("rich-text.js", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("render");
    assert!(tree.contains("TextView html \"message\""), "{tree}");
    assert!(tree.contains("<strong>world</strong>"), "{tree}");
    assert!(tree.contains(":selectable"), "{tree}");
    assert!(tree.contains(":on_link_click(fn)"), "{tree}");
}

#[gpui::test]
fn a_script_view_produces_an_element_description(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let view_type = runtime.load_source(ENTRY, COUNTER).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    assert!(tree.starts_with("v_flex"), "unexpected root: {tree}");
    assert!(tree.contains("text \"Count: 0\""), "missing label: {tree}");
    assert!(
        tree.contains("Button \"increment\""),
        "missing button: {tree}"
    );
    assert!(tree.contains(":on_click(fn)"), "missing handler: {tree}");
}

#[gpui::test]
fn a_registered_component_payload_reaches_its_materializer(cx: &mut TestAppContext) {
    use gpui::{AnyElement, IntoElement as _, ParentElement as _, div};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use crate::{
        COMPONENT_REGISTRY_API_VERSION, ComponentDescriptor, ComponentMaterializer,
        ComponentPayload, ComponentRegistry, ConstructorDescriptor, MaterializeRequest,
        snapshot::RenderSnapshot,
        spec::{Component, RegisteredComponentSpec, SpecArena},
    };

    struct TestBoxMaterializer(Arc<AtomicUsize>);

    impl ComponentMaterializer for TestBoxMaterializer {
        fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            assert_eq!(request.payload().downcast_ref::<String>().unwrap(), "alpha");
            let error = request
                .with_window_app(|_, _| Err::<(), _>(anyhow::anyhow!("synthetic overlay failed")))
                .expect_err("a scoped overlay error must cross the service seam");
            assert_eq!(error.to_string(), "synthetic overlay failed");
            let has_window = request.with_window_app(|window, cx| {
                Ok(cx.has_global::<gpui_base::Theme>() && window.bounds().size.width > px(0.))
            })?;
            assert!(
                has_window,
                "registered overlays receive the active render window"
            );
            self.0.fetch_add(1, Ordering::SeqCst);
            let children = request.take_children()?;
            Ok(div()
                .child("synthetic overlay")
                .children(children)
                .into_any_element())
        }
    }

    cx.update(|cx| crate::init(cx));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    let id = registry
        .register(
            ComponentDescriptor::new("TestBox", Arc::new(TestBoxMaterializer(calls.clone())))
                .with_constructors(vec![ConstructorDescriptor::new(
                    "TestBox",
                    Vec::new(),
                    |_| Ok(ComponentPayload::new(())),
                )])
                .with_methods(Vec::new()),
        )
        .unwrap();
    let runtime = ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();

    let mut arena = SpecArena::new();
    let child = arena.push(Component::Registered(RegisteredComponentSpec::new(
        id,
        "TestBox",
        ComponentPayload::new(String::from("alpha")),
    )));
    let root = arena.push(Component::Registered(RegisteredComponentSpec::new(
        id,
        "TestBox",
        ComponentPayload::new(String::from("alpha")),
    )));
    arena.attach(root, child).unwrap();
    let snapshot = RenderSnapshot::new(&runtime, 0, root, arena, None, None);
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    context.update(|window, cx| {
        drop(crate::materialize::materialize(
            &runtime, &snapshot, window, cx,
        ));
    });

    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(snapshot.debug_tree(), "TestBox\n  TestBox\n");
}

#[gpui::test]
fn a_registered_slot_factory_rebuilds_after_its_request_and_snapshot_owner_return(
    cx: &mut TestAppContext,
) {
    use std::cell::RefCell;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use gpui::{AnyElement, IntoElement as _, div};

    use crate::{
        COMPONENT_REGISTRY_API_VERSION, ComponentDescriptor, ComponentElementFactory,
        ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
        MaterializeRequest,
        snapshot::RenderSnapshot,
        spec::{Component, RegisteredComponentSpec, SpecArena, SpecOp},
    };

    thread_local! {
        static DELAYED_FACTORY: RefCell<Option<ComponentElementFactory>> = const { RefCell::new(None) };
        static FAILING_FACTORY: RefCell<Option<ComponentElementFactory>> = const { RefCell::new(None) };
        static NESTED_FACTORY: RefCell<Option<ComponentElementFactory>> = const { RefCell::new(None) };
        static SELF_FACTORY: RefCell<Option<ComponentElementFactory>> = const { RefCell::new(None) };
    }

    struct DelayedOverlay;
    impl ComponentMaterializer for DelayedOverlay {
        fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            let factory = request
                .take_slot_factory("content")
                .ok_or_else(|| anyhow::anyhow!("missing content factory"))?;
            assert!(request.take_slot("content").unwrap().is_none());
            DELAYED_FACTORY.with(|held| *held.borrow_mut() = Some(factory));
            let failing = request
                .take_slot_factory("failing")
                .ok_or_else(|| anyhow::anyhow!("missing failing factory"))?;
            FAILING_FACTORY.with(|held| *held.borrow_mut() = Some(failing));
            let nested = request
                .take_slot_factory("nested")
                .ok_or_else(|| anyhow::anyhow!("missing nested factory"))?;
            NESTED_FACTORY.with(|held| *held.borrow_mut() = Some(nested));
            request.finish(div())
        }
    }

    struct FailingSlot(Arc<AtomicUsize>);
    impl ComponentMaterializer for FailingSlot {
        fn materialize(&self, _request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            self.0.fetch_add(1, Ordering::SeqCst);
            anyhow::bail!("nested slot failed")
        }
    }

    struct NestedFactorySlot;
    impl ComponentMaterializer for NestedFactorySlot {
        fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            request.with_window_app(|window, cx| {
                FAILING_FACTORY.with(|held| {
                    held.borrow()
                        .as_ref()
                        .expect("inner factory")
                        .build(window, cx)
                        .map(drop)
                })
            })?;
            request.finish(div())
        }
    }

    cx.update(|cx| crate::init(cx));
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    let id = registry
        .register(
            ComponentDescriptor::new("DelayedOverlay", Arc::new(DelayedOverlay))
                .with_constructors(vec![ConstructorDescriptor::new(
                    "DelayedOverlay",
                    Vec::new(),
                    |_| Ok(ComponentPayload::new(())),
                )])
                .with_methods(Vec::new()),
        )
        .unwrap();
    let failing_calls = Arc::new(AtomicUsize::new(0));
    let failing_id = registry
        .register(
            ComponentDescriptor::new("FailingSlot", Arc::new(FailingSlot(failing_calls.clone())))
                .with_constructors(vec![ConstructorDescriptor::new(
                    "FailingSlot",
                    Vec::new(),
                    |_| Ok(ComponentPayload::new(())),
                )])
                .with_methods(Vec::new()),
        )
        .unwrap();
    let nested_id = registry
        .register(
            ComponentDescriptor::new("NestedFactorySlot", Arc::new(NestedFactorySlot))
                .with_constructors(vec![ConstructorDescriptor::new(
                    "NestedFactorySlot",
                    Vec::new(),
                    |_| Ok(ComponentPayload::new(())),
                )])
                .with_methods(Vec::new()),
        )
        .unwrap();
    let runtime = ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let mut arena = SpecArena::new();
    let content = arena.push(Component::Div);
    let failing = arena.push(Component::Registered(RegisteredComponentSpec::new(
        failing_id,
        "FailingSlot",
        ComponentPayload::new(()),
    )));
    let nested = arena.push(Component::Registered(RegisteredComponentSpec::new(
        nested_id,
        "NestedFactorySlot",
        ComponentPayload::new(()),
    )));
    let root = arena.push(Component::Registered(RegisteredComponentSpec::new(
        id,
        "DelayedOverlay",
        ComponentPayload::new(()),
    )));
    arena
        .push_op(root, SpecOp::Slot("content", content))
        .unwrap();
    arena
        .push_op(root, SpecOp::Slot("failing", failing))
        .unwrap();
    arena.push_op(root, SpecOp::Slot("nested", nested)).unwrap();
    let snapshot = RenderSnapshot::new(&runtime, 7, root, arena, None, None);
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    context.update(|window, cx| {
        drop(crate::materialize::materialize(
            &runtime, &snapshot, window, cx,
        ))
    });
    assert_eq!(
        failing_calls.load(Ordering::SeqCst),
        0,
        "slot factory must be lazy"
    );
    drop(snapshot);
    context.update(|window, cx| {
        DELAYED_FACTORY.with(|held| {
            let factory = held.borrow().clone().expect("factory escaped request");
            drop(factory.build(window, cx).expect("first delayed build"));
            drop(factory.build(window, cx).expect("second delayed build"));
        });
        FAILING_FACTORY.with(|held| {
            let factory = held
                .borrow()
                .clone()
                .expect("failing factory escaped request");
            for _ in 0..2 {
                let error = match factory.build(window, cx) {
                    Ok(_) => panic!("nested materializer failure must propagate"),
                    Err(error) => error,
                };
                assert!(
                    format!("{error:#}").contains("nested slot failed"),
                    "{error:#}"
                );
            }
        });
        NESTED_FACTORY.with(|held| {
            let error = match held.borrow().as_ref().unwrap().build(window, cx) {
                Ok(_) => panic!("nested factory failure must cross both frames"),
                Err(error) => error,
            };
            let chain = format!("{error:#}");
            assert!(chain.contains("NestedFactorySlot"), "{chain}");
            assert!(chain.contains("nested slot failed"), "{chain}");
        });
        let self_factory = ComponentElementFactory::new(|window, cx| {
            SELF_FACTORY.with(|held| held.borrow().as_ref().unwrap().build(window, cx))
        });
        SELF_FACTORY.with(|held| *held.borrow_mut() = Some(self_factory.clone()));
        for _ in 0..2 {
            let error = match self_factory.build(window, cx) {
                Ok(_) => panic!("self-recursive factory must fail"),
                Err(error) => error,
            };
            assert_eq!(
                error.to_string(),
                "component element factory is already building"
            );
        }
        SELF_FACTORY.with(|held| held.borrow_mut().take());

        let panics = Rc::new(Cell::new(true));
        let panic_flag = panics.clone();
        let panic_factory = ComponentElementFactory::new(move |_, _| {
            if panic_flag.replace(false) {
                panic!("synthetic factory panic");
            }
            Ok(div().into_any_element())
        });
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = panic_factory.build(window, cx);
            }))
            .is_err()
        );
        drop(
            panic_factory
                .build(window, cx)
                .expect("guard resets after panic"),
        );
    });
    assert_eq!(failing_calls.load(Ordering::SeqCst), 3);
    drop(runtime);
    context.update(|window, cx| {
        DELAYED_FACTORY.with(|held| {
            let error = match held.borrow().as_ref().unwrap().build(window, cx) {
                Ok(_) => panic!("released runtime must invalidate the factory"),
                Err(error) => error,
            };
            assert_eq!(
                error.to_string(),
                "component element factory runtime has been released"
            );
            held.borrow_mut().take();
        });
        FAILING_FACTORY.with(|held| {
            held.borrow_mut().take();
        });
        NESTED_FACTORY.with(|held| {
            held.borrow_mut().take();
        });
    });
}

#[gpui::test]
fn descriptor_drives_runtime_and_typescript(cx: &mut TestAppContext) {
    use std::sync::{Arc, Mutex};

    use gpui::{AnyElement, IntoElement as _, div};

    use crate::{
        ArgumentDescriptor, ArgumentSchema, COMPONENT_REGISTRY_API_VERSION, ComponentArgument,
        ComponentDescriptor, ComponentMaterializer, ComponentPayload, ComponentRegistry,
        ConstructorDescriptor, MaterializeRequest, MethodDescriptor,
    };

    struct TestBoxMaterializer(Arc<Mutex<Vec<String>>>);

    impl ComponentMaterializer for TestBoxMaterializer {
        fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            let id = request.payload().downcast_ref::<String>().unwrap().clone();
            let tone = request
                .methods()
                .find(|method| method.name() == "tone")
                .unwrap()
                .payload()
                .downcast_ref::<String>()
                .unwrap()
                .clone();
            self.0.lock().unwrap().extend([id, tone]);
            Ok(div().into_any_element())
        }
    }

    cx.update(|cx| crate::init(cx));
    let materialized = Arc::new(Mutex::new(Vec::new()));
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register(
            ComponentDescriptor::new(
                "TestBox",
                Arc::new(TestBoxMaterializer(materialized.clone())),
            )
            .with_constructors(vec![ConstructorDescriptor::new(
                "TestBox",
                vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(id)] => Ok(ComponentPayload::new(id.clone())),
                    _ => Err("TestBox.new(id) expects a string".into()),
                },
            )])
            .with_methods(vec![
                MethodDescriptor::new(
                    "tone",
                    vec![ArgumentDescriptor::new("value", ArgumentSchema::String)],
                    |arguments| match arguments {
                        [ComponentArgument::String(value)] => {
                            Ok(ComponentPayload::new(value.clone()))
                        }
                        _ => Err("tone(value) expects a string".into()),
                    },
                )
                .with_documentation("A test-only method.")
                .with_documentation("Sets the visual tone."),
            ])
            .with_documentation("A test component."),
        )
        .unwrap();
    let components = registry.freeze().unwrap();
    let declarations = crate::typings::declarations_with_components(&components);
    assert!(declarations.contains("declare module \"gpui-component\""));
    // `disabled`, `selected` and `on_click` join the omission because this
    // descriptor declares none of them, and the runtime refuses them for a
    // registered component that does not.
    assert!(declarations.contains(
        "export type TestBoxElement = Omit<Element, \"tone\" | \"disabled\" | \"selected\" | \"on_click\"> & {"
    ));
    assert!(declarations.contains("import { ClickEvent, Context, Element } from \"gpui-kit\";"));
    assert!(declarations.contains("export const TestBox: { new(id: string): TestBoxElement }"));
    assert!(declarations.contains("tone(value: string): TestBoxElement;"));
    assert!(declarations.contains("A test component."));
    assert!(declarations.contains("Sets the visual tone."));

    let runtime = ShellRuntime::new_isolated_with_components(components).unwrap();
    let view_type = runtime
        .load_source(
            "registered.js",
            r#"
import { View } from "gpui-kit";
import { TestBox } from "gpui-component";
export default class Registered extends View {
  render() { return new TestBox("alpha").tone("quiet"); }
}
"#,
        )
        .unwrap();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .unwrap();
    let snapshot = context
        .update(|window, cx| {
            runtime.build_snapshot(&object, None, crate::policy::default(), window, cx)
        })
        .unwrap();
    let tree = snapshot.debug_tree();
    context.update(|window, cx| {
        drop(crate::materialize::materialize(
            &runtime, &snapshot, window, cx,
        ));
    });

    assert!(tree.starts_with("TestBox"), "unexpected tree: {tree}");
    assert!(tree.contains(":tone(registered)"), "missing method: {tree}");
    assert_eq!(*materialized.lock().unwrap(), ["alpha", "quiet"]);
}

#[gpui::test]
fn registered_method_schema_overrides_legacy_dispatch(cx: &mut TestAppContext) {
    use crate::{
        ArgumentDescriptor, ArgumentSchema, COMPONENT_REGISTRY_API_VERSION, ComponentDescriptor,
        ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
        MaterializeRequest, MethodDescriptor,
    };
    use gpui::{AnyElement, IntoElement as _, div};
    use std::sync::Arc;

    struct EmptyMaterializer;
    impl ComponentMaterializer for EmptyMaterializer {
        fn materialize(&self, _: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            Ok(div().into_any_element())
        }
    }

    cx.update(crate::init);
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register(
            ComponentDescriptor::new("StrictBox", Arc::new(EmptyMaterializer))
                .with_constructors(vec![ConstructorDescriptor::new(
                    "StrictBox",
                    Vec::new(),
                    |_| Ok(ComponentPayload::new(())),
                )])
                .with_methods(vec![
                    MethodDescriptor::new(
                        "disabled",
                        vec![ArgumentDescriptor::new("value", ArgumentSchema::Boolean)],
                        |_| Ok(ComponentPayload::new(())),
                    )
                    .with_documentation("A test-only method."),
                ]),
        )
        .unwrap();
    let runtime = ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let view_type = runtime
        .load_source(
            "strict.js",
            r#"
import { View } from "gpui-kit";
import { StrictBox } from "gpui-component";
export default class Strict extends View { render() { return new StrictBox().disabled("yes"); } }
"#,
        )
        .unwrap();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .unwrap();
    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("disabled(value) expects a boolean"),
        "{error:#}"
    );
}

#[gpui::test]
fn registered_argument_conversion_has_recursive_depth_and_aggregate_budgets(
    cx: &mut TestAppContext,
) {
    use crate::{
        ArgumentDescriptor, ArgumentSchema, COMPONENT_REGISTRY_API_VERSION, ComponentDescriptor,
        ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
        MaterializeRequest,
    };
    use gpui::{AnyElement, IntoElement as _, div};
    use std::sync::Arc;

    struct EmptyMaterializer;
    impl ComponentMaterializer for EmptyMaterializer {
        fn materialize(&self, _: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            Ok(div().into_any_element())
        }
    }

    cx.update(crate::init);
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register(
            ComponentDescriptor::new("BudgetBox", Arc::new(EmptyMaterializer))
                .with_constructors(vec![ConstructorDescriptor::new(
                    "BudgetBox",
                    vec![ArgumentDescriptor::new("value", ArgumentSchema::String)],
                    |_| Ok(ComponentPayload::new(())),
                )])
                .with_methods(Vec::new()),
        )
        .unwrap();
    let runtime = ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    for (name, body, expected) in [
        (
            "deep.js",
            "let value = 'x'; for (let i = 0; i < 40; i++) value = [value]; return new BudgetBox(value);",
            "nested too deeply",
        ),
        (
            "wide.js",
            "let value = []; for (let i = 0; i < 101; i++) { let row = []; for (let j = 0; j < 100; j++) row.push('x'); value.push(row); } return new BudgetBox(value);",
            "too many nested values",
        ),
    ] {
        let source = format!(
            "import {{ View }} from 'gpui-kit'; import {{ BudgetBox }} from 'gpui-component'; export default class Budgeted extends View {{ render() {{ {body} }} }}"
        );
        let view_type = runtime.load_source(name, &source).unwrap();
        let object = context
            .update(|window, cx| runtime.instantiate(&view_type, window, cx))
            .unwrap();
        let error = context
            .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
            .unwrap_err();
        assert!(error.to_string().contains(expected), "{error:#}");
    }
}

#[gpui::test]
fn failed_registered_factory_does_not_claim_elements_or_persist_callbacks(cx: &mut TestAppContext) {
    use crate::{
        ArgumentDescriptor, ArgumentSchema, COMPONENT_REGISTRY_API_VERSION, ComponentDescriptor,
        ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
        MaterializeRequest, MethodDescriptor,
    };
    use gpui::{AnyElement, IntoElement as _, div};
    use std::sync::Arc;

    struct EmptyMaterializer;
    impl ComponentMaterializer for EmptyMaterializer {
        fn materialize(&self, _: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            Ok(div().into_any_element())
        }
    }

    cx.update(crate::init);
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register(
            ComponentDescriptor::new("RejectBox", Arc::new(EmptyMaterializer))
                .with_constructors(vec![
                    ConstructorDescriptor::new(
                        "RejectBox",
                        vec![
                            ArgumentDescriptor::new("child", ArgumentSchema::Element),
                            ArgumentDescriptor::new(
                                "handler",
                                ArgumentSchema::Callback("() => void"),
                            ),
                        ],
                        |_| Err("RejectBox deliberately rejects its payload".into()),
                    ),
                    ConstructorDescriptor::new("MethodBox", Vec::new(), |_| {
                        Ok(ComponentPayload::new(()))
                    }),
                    ConstructorDescriptor::new(
                        "ValidateBox",
                        vec![
                            ArgumentDescriptor::new(
                                "handler",
                                ArgumentSchema::Callback("() => void"),
                            ),
                            ArgumentDescriptor::new("enabled", ArgumentSchema::Boolean),
                        ],
                        |_| Ok(ComponentPayload::new(())),
                    ),
                ])
                .with_methods(vec![
                    MethodDescriptor::new(
                        "reject",
                        vec![
                            ArgumentDescriptor::new("child", ArgumentSchema::Element),
                            ArgumentDescriptor::new(
                                "handler",
                                ArgumentSchema::Callback("() => void"),
                            ),
                        ],
                        |_| Err("reject deliberately rejects its payload".into()),
                    )
                    .with_documentation("A test-only method."),
                ]),
        )
        .unwrap();
    let runtime = ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let view_type = runtime
        .load_source(
            "transactional-component.js",
            r#"
import { View, div } from "gpui-kit";
import { MethodBox, RejectBox, ValidateBox } from "gpui-component";
export default class Transactional extends View {
  render() {
    const child = div();
    try { new RejectBox(child, () => {}); } catch (_) {}
    const methodChild = div();
    try { new MethodBox().reject(methodChild, () => {}); } catch (_) {}
    try { new ValidateBox(() => {}, "not boolean"); } catch (_) {}
    return div().child(child).child(methodChild);
  }
}
"#,
        )
        .unwrap();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .unwrap();
    let snapshot = context
        .update(|window, cx| {
            runtime.build_snapshot(&object, None, crate::policy::default(), window, cx)
        })
        .unwrap();

    assert_eq!(snapshot.debug_tree(), "div\n  div\n  div\n");
    assert_eq!(runtime.live_callbacks(), 0);
}

#[gpui::test]
fn registered_entity_arguments_validate_the_actual_entity_kind(cx: &mut TestAppContext) {
    use crate::{
        ArgumentDescriptor, ArgumentSchema, COMPONENT_REGISTRY_API_VERSION, ComponentDescriptor,
        ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
        MaterializeRequest, MethodDescriptor,
    };
    use gpui::{AnyElement, IntoElement as _, div};
    use std::sync::Arc;

    struct EmptyMaterializer;
    impl ComponentMaterializer for EmptyMaterializer {
        fn materialize(&self, _: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            Ok(div().into_any_element())
        }
    }

    cx.update(crate::init);
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register(
            ComponentDescriptor::new("FocusBox", Arc::new(EmptyMaterializer))
                .with_constructors(vec![ConstructorDescriptor::new(
                    "FocusBox",
                    vec![ArgumentDescriptor::new(
                        "focus",
                        ArgumentSchema::Entity("FocusHandle"),
                    )],
                    |_| Ok(ComponentPayload::new(())),
                )])
                .with_methods(vec![
                    MethodDescriptor::new(
                        "disabled",
                        vec![ArgumentDescriptor::new("disabled", ArgumentSchema::Boolean)],
                        |_| Ok(ComponentPayload::new(())),
                    )
                    .with_documentation("A test-only method."),
                    MethodDescriptor::new(
                        "selected",
                        vec![ArgumentDescriptor::new("selected", ArgumentSchema::Boolean)],
                        |_| Ok(ComponentPayload::new(())),
                    )
                    .with_documentation("A test-only method."),
                    MethodDescriptor::new(
                        "on_click",
                        vec![ArgumentDescriptor::new(
                            "handler",
                            ArgumentSchema::Callback("(event, cx) => void"),
                        )],
                        |_| Ok(ComponentPayload::new(())),
                    )
                    .with_documentation("A test-only method."),
                ]),
        )
        .unwrap();
    let runtime = ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let view_type = runtime
        .load_source(
            "wrong-entity-kind.js",
            r#"
import { View } from "gpui-kit";
import { InputState } from "gpui-base";
import { FocusBox } from "gpui-component";
export default class WrongKind extends View {
  init() { this.input = InputState.new(); }
  render() { return new FocusBox(this.input); }
}
"#,
        )
        .unwrap();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .unwrap();
    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("FocusBox(focus) expects a FocusHandle entity"),
        "{error:#}"
    );
}

#[gpui::test]
fn materialize_request_resolves_opaque_component_arguments(cx: &mut TestAppContext) {
    use crate::{
        ArgumentDescriptor, ArgumentSchema, COMPONENT_REGISTRY_API_VERSION, ComponentArgument,
        ComponentCallback, ComponentDescriptor, ComponentMaterializer, ComponentPayload,
        ComponentRegistry, ConstructorDescriptor, MaterializeRequest,
    };
    use gpui::{AnyElement, IntoElement as _, ParentElement as _, div};
    use std::{cell::RefCell, sync::Arc};

    thread_local! {
        static CALLBACK: RefCell<Option<ComponentCallback>> = const { RefCell::new(None) };
    }

    struct ResolvingMaterializer;
    impl ComponentMaterializer for ResolvingMaterializer {
        fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            let arguments = request
                .payload()
                .downcast_ref::<Vec<ComponentArgument>>()
                .unwrap()
                .clone();
            let child = request.resolve_element(&arguments[0])?;
            let entity = request.resolve_entity(&arguments[1])?;
            anyhow::ensure!(entity.kind() == "FocusHandle");
            let callback = request.resolve_callback(&arguments[2])?;
            CALLBACK.with(|stored| *stored.borrow_mut() = Some(callback));
            Ok(div().child(child).into_any_element())
        }
    }

    cx.update(crate::init);
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register(
            ComponentDescriptor::new("ResolvedBox", Arc::new(ResolvingMaterializer))
                .with_constructors(vec![ConstructorDescriptor::new(
                    "ResolvedBox",
                    vec![
                        ArgumentDescriptor::new("child", ArgumentSchema::Element),
                        ArgumentDescriptor::new("focus", ArgumentSchema::Entity("FocusHandle")),
                        ArgumentDescriptor::new("handler", ArgumentSchema::Callback("() => void")),
                    ],
                    |arguments| Ok(ComponentPayload::new(arguments.to_vec())),
                )])
                .with_methods(Vec::new()),
        )
        .unwrap();
    let runtime = ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let view_type = runtime
        .load_source(
            "resolved-component.js",
            r#"
import { View, div } from "gpui-kit";
import { ResolvedBox } from "gpui-component";
export default class Resolved extends View {
  init(_props, cx) { this.focus = cx.focus_handle(); }
  render() {
    return new ResolvedBox(div(), this.focus, (text, number, flag, cx) => {
      throw new Error(`adapter callback ran: ${text}|${number}|${flag}|${typeof cx.notify}`);
    });
  }
}
"#,
        )
        .unwrap();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .unwrap();
    let snapshot = context
        .update(|window, cx| {
            runtime.build_snapshot(&object, None, crate::policy::default(), window, cx)
        })
        .unwrap();
    context.update(|window, cx| {
        drop(crate::materialize::materialize(
            &runtime, &snapshot, window, cx,
        ));
    });
    let callback = CALLBACK.with(|stored| stored.borrow_mut().take().unwrap());
    let error = context
        .update(|window, cx| {
            callback.invoke_with(
                &[
                    crate::ComponentCallbackArgument::String("alpha".into()),
                    crate::ComponentCallbackArgument::Number(2.5),
                    crate::ComponentCallbackArgument::Boolean(true),
                ],
                window,
                cx,
            )
        })
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("adapter callback ran: alpha|2.5|true|function"),
        "{error:#}"
    );
}

#[gpui::test]
fn registered_component_state_is_created_once_and_updated_during_materialization(
    cx: &mut TestAppContext,
) {
    use crate::{
        ArgumentDescriptor, ArgumentSchema, COMPONENT_REGISTRY_API_VERSION, ComponentArgument,
        ComponentDescriptor, ComponentMaterializer, ComponentPayload, ComponentRegistry,
        ConstructorDescriptor, MaterializeRequest, StateDescriptor,
    };
    use gpui::{AnyElement, IntoElement as _, div};
    use std::cell::RefCell;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    thread_local! {
        static STATE_HANDLE: RefCell<Option<crate::ComponentState<usize>>> = const { RefCell::new(None) };
    }

    struct Adapter(Arc<AtomicUsize>);
    impl ComponentMaterializer for Adapter {
        fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            let argument = request
                .payload()
                .downcast_ref::<ComponentArgument>()
                .unwrap()
                .clone();
            let value = request.with_state::<usize, _>(&argument, |value| *value)?;
            let handle = request.state_handle(&argument)?;
            STATE_HANDLE.with(|stored| *stored.borrow_mut() = Some(handle));
            self.0.store(value, Ordering::SeqCst);
            Ok(div().into_any_element())
        }
    }

    cx.update(crate::init);
    let created = Arc::new(AtomicUsize::new(0));
    let observed = Arc::new(AtomicUsize::new(0));
    let factory_created = created.clone();
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register_state(StateDescriptor::new(
            "CounterState",
            "CounterState",
            vec![],
            move |_, _, _| {
                factory_created.fetch_add(1, Ordering::SeqCst);
                Ok(Box::new(40usize))
            },
        ))
        .unwrap();
    registry
        .register_state(StateDescriptor::new(
            "OtherState",
            "OtherState",
            vec![],
            |_, _, _| Ok(Box::new(())),
        ))
        .unwrap();
    registry
        .register(
            ComponentDescriptor::new("StatefulBox", Arc::new(Adapter(observed.clone())))
                .with_constructors(vec![ConstructorDescriptor::new(
                    "StatefulBox",
                    vec![ArgumentDescriptor::new(
                        "state",
                        ArgumentSchema::Entity("CounterState"),
                    )],
                    |arguments| Ok(ComponentPayload::new(arguments[0].clone())),
                )])
                .with_methods(vec![]),
        )
        .unwrap();
    let runtime = ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let class = runtime
        .load_source(
            "stateful.js",
            r#"
import { View } from "gpui-kit";
import { CounterState, StatefulBox } from "gpui-component";
export default class Stateful extends View {
  init() { this.state = CounterState(); }
  render() { return new StatefulBox(this.state); }
}"#,
        )
        .unwrap();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&class, window, cx))
        .unwrap();
    for expected in [40, 41] {
        let snapshot = context
            .update(|window, cx| {
                runtime.build_snapshot(&object, None, crate::policy::default(), window, cx)
            })
            .unwrap();
        context.update(|window, cx| {
            drop(crate::materialize::materialize(
                &runtime, &snapshot, window, cx,
            ))
        });
        assert_eq!(observed.load(Ordering::SeqCst), expected);
        if expected == 40 {
            context
                .update(|window, cx| {
                    STATE_HANDLE.with(|stored| {
                        stored
                            .borrow()
                            .as_ref()
                            .unwrap()
                            .update(window, cx, |value, _, _| *value += 1)
                    })
                })
                .unwrap();
        }
    }
    assert_eq!(created.load(Ordering::SeqCst), 1);

    let forged = runtime
        .load_source(
            "forged-state.js",
            r#"
import { View } from "gpui-kit";
import { StatefulBox } from "gpui-component";
export default class Forged extends View {
  render() {
    return new StatefulBox({ __componentStateHandle: 0, __componentStateProof: "guessed" });
  }
}"#,
        )
        .unwrap();
    let forged = context
        .update(|window, cx| runtime.instantiate(&forged, window, cx))
        .unwrap();
    let error = context
        .update(|window, cx| {
            runtime.build_snapshot(&forged, None, crate::policy::default(), window, cx)
        })
        .err()
        .expect("forged retained state must be rejected");
    assert!(
        error.to_string().contains("expects a CounterState entity"),
        "{error:#}"
    );

    let wrong_kind = runtime
        .load_source(
            "wrong-state-kind.js",
            r#"
import { View } from "gpui-kit";
import { OtherState, StatefulBox } from "gpui-component";
export default class WrongKind extends View {
  init() { this.state = OtherState(); }
  render() { return new StatefulBox(this.state); }
}"#,
        )
        .unwrap();
    let wrong_kind = context
        .update(|window, cx| runtime.instantiate(&wrong_kind, window, cx))
        .unwrap();
    let error = context
        .update(|window, cx| {
            runtime.build_snapshot(&wrong_kind, None, crate::policy::default(), window, cx)
        })
        .err()
        .expect("wrong retained state kind must be rejected");
    assert!(
        error.to_string().contains("expects a CounterState entity"),
        "{error:#}"
    );
}

#[gpui::test]
fn failed_registered_state_factory_rolls_back_without_retaining_a_slot(cx: &mut TestAppContext) {
    use crate::{
        ArgumentDescriptor, ArgumentSchema, COMPONENT_REGISTRY_API_VERSION, ComponentRegistry,
        StateDescriptor,
    };

    cx.update(crate::init);
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register_state(StateDescriptor::new(
            "BrokenState",
            "BrokenState",
            vec![ArgumentDescriptor::new(
                "callback",
                ArgumentSchema::Callback("() => void"),
            )],
            |_, _, _| Err("state factory refused creation".into()),
        ))
        .unwrap();
    let runtime = ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let class = runtime
        .load_source(
            "broken-state.js",
            r#"
import { View } from "gpui-kit";
import { BrokenState } from "gpui-component";
export default class Broken extends View { init() { BrokenState(() => {}); } render() { return "never"; } }
"#,
        )
        .unwrap();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let error = context
        .update(|window, cx| runtime.instantiate(&class, window, cx))
        .unwrap_err();
    assert!(
        error.to_string().contains("state factory refused creation"),
        "{error:#}"
    );
    assert_eq!(runtime.retained_component_state_count(), 0);
    assert_eq!(runtime.live_callbacks(), 0);
}

#[gpui::test]
fn failed_application_load_releases_states_created_during_module_evaluation(
    cx: &mut TestAppContext,
) {
    use crate::{COMPONENT_REGISTRY_API_VERSION, ComponentRegistry, StateDescriptor};

    cx.update(crate::init);
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register_state(StateDescriptor::new(
            "LoadState",
            "LoadState",
            vec![],
            |_, _, _| Ok(Box::new(())),
        ))
        .unwrap();
    let runtime = ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let directory = std::env::temp_dir().join(format!(
        "gpui-shell-failed-state-load-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("main.js"),
        r#"
import { LoadState } from "gpui-component";
LoadState();
throw new Error("module load failed after state creation");
export default class Never {}
"#,
    )
    .unwrap();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let error = context
        .update(|window, cx| {
            let (_scope, _) = crate::scope::enter_runtime(
                &runtime,
                window,
                cx,
                crate::scope::ScopePhase::Event,
                None,
            );
            runtime.load_app(&directory, "main.js")
        })
        .err()
        .expect("application load must fail");
    assert!(
        error
            .to_string()
            .contains("module load failed after state creation"),
        "{error:#}"
    );
    assert_eq!(runtime.retained_component_state_count(), 0);
    let _ = std::fs::remove_dir_all(&directory);
}

#[gpui::test]
fn successful_application_reload_purges_old_state_and_keeps_new_generation(
    cx: &mut TestAppContext,
) {
    use crate::{COMPONENT_REGISTRY_API_VERSION, ComponentRegistry, StateDescriptor};

    cx.update(crate::init);
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register_state(StateDescriptor::new(
            "ReloadState",
            "ReloadState",
            vec![],
            |_, _, _| Ok(Box::new(())),
        ))
        .unwrap();
    let runtime = ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let directory = std::env::temp_dir().join(format!(
        "gpui-shell-successful-state-reload-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let source = |version| {
        format!(
            r#"
import {{ ReloadState }} from "gpui-component";
ReloadState();
export default class Reloaded {{ render() {{ return "version {version}"; }} }}
"#
        )
    };
    std::fs::write(directory.join("main.js"), source(1)).unwrap();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let first = context
        .update(|window, cx| {
            let (_scope, _) = crate::scope::enter_runtime(
                &runtime,
                window,
                cx,
                crate::scope::ScopePhase::Event,
                None,
            );
            runtime.load_app(&directory, "main.js")
        })
        .unwrap();
    std::fs::write(directory.join("main.js"), source(2)).unwrap();
    let second = context
        .update(|window, cx| {
            let (_scope, _) = crate::scope::enter_runtime(
                &runtime,
                window,
                cx,
                crate::scope::ScopePhase::Event,
                None,
            );
            runtime.load_app(&directory, "main.js")
        })
        .unwrap();
    let first = context
        .update(|window, cx| runtime.instantiate(&first, window, cx))
        .unwrap();
    let second = context
        .update(|window, cx| runtime.instantiate(&second, window, cx))
        .unwrap();
    assert_eq!(runtime.retained_component_state_count(), 2);
    let old = first.application_generation().unwrap();
    let new = second.application_generation().unwrap();

    context.update(|_, cx| runtime.release_application_generation(&old, cx));

    assert_eq!(runtime.retained_component_state_count(), 1);
    assert!(!old.is_active());
    assert!(new.is_active());
    context.update(|_, cx| runtime.release_application_generation(&new, cx));
    assert_eq!(runtime.retained_component_state_count(), 0);
    let _ = std::fs::remove_dir_all(&directory);
}

#[gpui::test]
fn registered_parent_can_inspect_and_materialize_a_registered_typed_child(cx: &mut TestAppContext) {
    use crate::{
        COMPONENT_REGISTRY_API_VERSION, ComponentDescriptor, ComponentMaterializer,
        ComponentPayload, ComponentRegistry, ConstructorDescriptor, MaterializeRequest,
    };
    use gpui::{AnyElement, ParentElement as _, div};
    use std::{cell::RefCell, sync::Arc};

    thread_local! {
        static CHILD_NAME: RefCell<Option<&'static str>> = const { RefCell::new(None) };
    }

    struct LeafMaterializer;
    impl ComponentMaterializer for LeafMaterializer {
        fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            request.finish(div().child("leaf"))
        }
    }

    struct ParentMaterializer;
    impl ComponentMaterializer for ParentMaterializer {
        fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            let mut children = request.take_typed_children().unwrap();
            anyhow::ensure!(children.len() == 1);
            let mut child = children.pop().unwrap();
            CHILD_NAME.with(|name| *name.borrow_mut() = child.component_name());
            let child = request.materialize_child(&mut child)?;
            request.finish(div().child(child))
        }
    }

    fn descriptor(
        name: &'static str,
        materializer: Arc<dyn ComponentMaterializer>,
    ) -> ComponentDescriptor {
        ComponentDescriptor::new(name, materializer)
            .with_constructors(vec![ConstructorDescriptor::new(name, Vec::new(), |_| {
                Ok(ComponentPayload::new(()))
            })])
            .with_methods(Vec::new())
    }

    cx.update(crate::init);
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register(descriptor("TypedLeaf", Arc::new(LeafMaterializer)))
        .unwrap();
    registry
        .register(descriptor("TypedParent", Arc::new(ParentMaterializer)))
        .unwrap();
    let runtime = ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let view_type = runtime
        .load_source(
            "typed-child.js",
            r#"
import { View } from "gpui-kit";
import { TypedLeaf, TypedParent } from "gpui-component";
export default class TypedChildView extends View {
  render() { return new TypedParent().child(new TypedLeaf()); }
}
"#,
        )
        .unwrap();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .unwrap();
    let snapshot = context
        .update(|window, cx| {
            runtime.build_snapshot(&object, None, crate::policy::default(), window, cx)
        })
        .unwrap();
    context.update(|window, cx| {
        drop(crate::materialize::materialize(
            &runtime, &snapshot, window, cx,
        ));
    });

    assert_eq!(CHILD_NAME.with(|name| *name.borrow()), Some("TypedLeaf"));
}

#[gpui::test]
fn registered_button_receives_common_parts_and_dispatches_click(cx: &mut TestAppContext) {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use gpui::AnyElement;
    use gpui_base::Button;

    use crate::{
        ArgumentDescriptor, ArgumentSchema, COMPONENT_REGISTRY_API_VERSION, ComponentDescriptor,
        ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
        MaterializeRequest, MethodDescriptor,
    };

    #[derive(Default)]
    struct Observed {
        states: Mutex<Vec<(String, bool, bool, usize)>>,
        click_recordings: AtomicUsize,
    }

    struct AdapterButton(Arc<Observed>);

    impl ComponentMaterializer for AdapterButton {
        fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            let id = request.payload().downcast_ref::<String>().unwrap().clone();
            self.0.states.lock().unwrap().push((
                id.clone(),
                request.disabled(),
                request.selected(),
                request.children_len(),
            ));
            let mut button = Button::new(id).disabled(request.disabled());
            if let Some(on_click) = request.on_click() {
                button = button.on_click(move |event, window, cx| {
                    on_click.invoke(event, window, cx);
                });
            }
            request.finish(button)
        }
    }

    cx.update(crate::init);
    let observed = Arc::new(Observed::default());
    let observed_clicks = Arc::clone(&observed);
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register(
            ComponentDescriptor::new(
                "AdapterButton",
                Arc::new(AdapterButton(Arc::clone(&observed))),
            )
            .with_constructors(vec![ConstructorDescriptor::new(
                "AdapterButton",
                vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                |arguments| match arguments {
                    [crate::ComponentArgument::String(id)] => Ok(ComponentPayload::new(id.clone())),
                    _ => unreachable!("argument schema validated before payload construction"),
                },
            )])
            .with_methods(vec![
                MethodDescriptor::new(
                    "disabled",
                    vec![ArgumentDescriptor::new("disabled", ArgumentSchema::Boolean)],
                    |_| Ok(ComponentPayload::new(())),
                )
                .with_documentation("A test-only method."),
                MethodDescriptor::new(
                    "selected",
                    vec![ArgumentDescriptor::new("selected", ArgumentSchema::Boolean)],
                    |_| Ok(ComponentPayload::new(())),
                )
                .with_documentation("A test-only method."),
                MethodDescriptor::new(
                    "on_click",
                    vec![ArgumentDescriptor::new(
                        "handler",
                        ArgumentSchema::Callback("(event, cx) => void"),
                    )],
                    move |_| {
                        observed_clicks
                            .click_recordings
                            .fetch_add(1, Ordering::SeqCst);
                        Ok(ComponentPayload::new(()))
                    },
                )
                .with_documentation("A test-only method."),
            ]),
        )
        .unwrap();
    registry
        .register(
            ComponentDescriptor::new(
                "RejectClick",
                Arc::new(AdapterButton(Arc::clone(&observed))),
            )
            .with_constructors(vec![ConstructorDescriptor::new(
                "RejectClick",
                Vec::new(),
                |_| Ok(ComponentPayload::new(String::from("reject"))),
            )])
            .with_methods(vec![
                MethodDescriptor::new(
                    "on_click",
                    vec![ArgumentDescriptor::new(
                        "handler",
                        ArgumentSchema::Callback("(event, cx) => void"),
                    )],
                    |_| Err("click recorder refused the handler".into()),
                )
                .with_documentation("A test-only method."),
            ]),
        )
        .unwrap();
    let runtime = ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { AdapterButton } from "gpui-component";
export default class AdapterButtons extends View {
  init() { this.clicks = 0; }
  render() {
    return div().w(300).h(100)
      .child(new AdapterButton("disabled").disabled(true).selected(true).w(120).h(40).child("Disabled"))
      .child(new AdapterButton("active").w(120).h(40).on_click((_event, cx) => {
        this.clicks += 1;
        cx.notify();
      }).child("Clicks: " + this.clicks));
  }
}
"#;
    let view_type = runtime.load_source("adapter-button.js", source).unwrap();
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let refusing = runtime
        .load_source(
            "refusing-click.js",
            r#"
import { View } from "gpui-kit";
import { RejectClick } from "gpui-component";
export default class RefusingClick extends View {
  render() { return new RejectClick().on_click(() => {}); }
}
"#,
        )
        .unwrap();
    let object = context
        .update(|window, cx| runtime.instantiate(&refusing, window, cx))
        .unwrap();
    let callbacks_before = runtime.live_callbacks();
    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("click recorder refused the handler"),
        "{error:#}"
    );
    assert_eq!(
        runtime.live_callbacks(),
        callbacks_before,
        "a recorder failure must roll back the callback it validated"
    );

    let undeclared = runtime
        .load_source(
            "undeclared-common.js",
            r#"
import { View } from "gpui-kit";
import { AdapterButton } from "gpui-component";
export default class UndeclaredCommon extends View {
  render() { return new AdapterButton("bad").checked(true); }
}
"#,
        )
        .unwrap();
    let object = context
        .update(|window, cx| runtime.instantiate(&undeclared, window, cx))
        .unwrap();
    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("unknown element method `checked`")
    );

    let view = window.root(&mut context).unwrap();
    context.update(|_, cx| view.update(cx, |_, cx| cx.notify()));
    context.run_until_parked();
    context.update(|window, _| window.refresh());
    context.update(|window, cx| window.draw(cx).clear(cx));

    let initial_tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        !initial_tree.is_empty(),
        "script view did not publish a snapshot"
    );
    let states = observed.states.lock().unwrap().clone();
    assert!(
        states.contains(&("disabled".into(), true, true, 1)),
        "{states:?}"
    );
    assert!(
        states.contains(&("active".into(), false, false, 1)),
        "{states:?}"
    );
    assert_eq!(observed.click_recordings.load(Ordering::SeqCst), 1);
    assert_eq!(
        runtime.live_callbacks(),
        1,
        "the descriptor transaction and common Behavior must share one callback"
    );

    context.simulate_click(point(px(20.), px(60.)), Modifiers::default());
    context.run_until_parked();
    let view = window.root(&mut context).unwrap();
    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(tree.contains("Clicks: 1"), "{tree}");
}

#[gpui::test]
fn deprecated_alias_constructs_the_same_component_and_warns_once(cx: &mut TestAppContext) {
    use crate::{
        COMPONENT_REGISTRY_API_VERSION, ComponentDescriptor, ComponentMaterializer,
        ComponentPayload, ComponentRegistry, ConstructorDescriptor, MaterializeRequest,
    };
    use gpui::{AnyElement, IntoElement as _, div};
    use std::sync::Arc;

    struct EmptyMaterializer;
    impl ComponentMaterializer for EmptyMaterializer {
        fn materialize(&self, _: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            Ok(div().into_any_element())
        }
    }

    cx.update(crate::init);
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register(
            ComponentDescriptor::new("TestBox", Arc::new(EmptyMaterializer))
                .with_constructors(vec![
                    ConstructorDescriptor::new("TestBox", Vec::new(), |_| {
                        Ok(ComponentPayload::new(()))
                    }),
                    ConstructorDescriptor::new("OldTestBox", Vec::new(), |_| {
                        Ok(ComponentPayload::new(()))
                    })
                    .with_deprecation("TestBox", "Use TestBox instead."),
                ])
                .with_methods(Vec::new())
                .with_documentation("A test box."),
        )
        .unwrap();
    let components = registry.freeze().unwrap();
    let declarations = crate::typings::declarations_with_components(&components);
    assert!(declarations.contains("@deprecated Use TestBox instead."));
    let runtime = ShellRuntime::new_isolated_with_components(components).unwrap();
    let view_type = runtime
        .load_source(
            "alias.js",
            r#"
import { View } from "gpui-kit";
import { OldTestBox } from "gpui-component";
export default class Alias extends View { render() { return new OldTestBox(); } }
"#,
        )
        .unwrap();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .unwrap();
    for _ in 0..2 {
        let tree = context
            .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
            .unwrap();
        assert_eq!(tree, "TestBox\n");
    }
    assert_eq!(runtime.deprecation_warning_count("OldTestBox"), 1);
}

#[test]
fn isolated_runtimes_write_their_own_component_declarations() {
    use crate::{
        COMPONENT_REGISTRY_API_VERSION, ComponentDescriptor, ComponentMaterializer,
        ComponentPayload, ComponentRegistry, ConstructorDescriptor, MaterializeRequest,
    };
    use gpui::{AnyElement, IntoElement as _, div};
    use std::sync::Arc;

    struct EmptyMaterializer;
    impl ComponentMaterializer for EmptyMaterializer {
        fn materialize(&self, _: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            Ok(div().into_any_element())
        }
    }

    let runtime = |export: &'static str| {
        let mut registry = ComponentRegistry::new(
            COMPONENT_REGISTRY_API_VERSION,
            crate::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        registry
            .register(
                ComponentDescriptor::new(export, Arc::new(EmptyMaterializer))
                    .with_constructors(vec![ConstructorDescriptor::new(export, Vec::new(), |_| {
                        Ok(ComponentPayload::new(()))
                    })])
                    .with_methods(Vec::new()),
            )
            .unwrap();
        ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap()
    };

    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("gpui-shell-component-types-{unique}"));
    let first = root.join("first");
    let second = root.join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(first.join("main.js"), "export default class First {}").unwrap();
    std::fs::write(second.join("main.js"), "export default class Second {}").unwrap();
    let first_runtime = runtime("FirstBox");
    let second_runtime = runtime("SecondBox");
    first_runtime.load_app(&first, "main.js").unwrap();
    second_runtime.load_app(&second, "main.js").unwrap();

    let first_types = std::fs::read_to_string(first.join("gpui-kit.d.ts")).unwrap();
    let second_types = std::fs::read_to_string(second.join("gpui-kit.d.ts")).unwrap();
    assert!(first_types.contains("FirstBox"));
    assert!(!first_types.contains("SecondBox"));
    assert!(second_types.contains("SecondBox"));
    assert!(!second_types.contains("FirstBox"));
    assert_eq!(first_types, first_runtime.type_declarations());
    assert_eq!(second_types, second_runtime.type_declarations());
    std::fs::remove_dir_all(root).unwrap();
}

#[gpui::test]
fn element_map_returns_the_transform_result(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";

export default class MappedElement extends View {
  render(cx) {
    return div().map((root) =>
      root.id("mapped-root").child(div().map(() => "mapped child"))
    );
  }
}
"#;
    let view_type = runtime
        .load_source("mapped-element.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("render");

    assert!(tree.contains("mapped-root"), "missing mapped root: {tree}");
    assert!(
        tree.contains("text \"mapped child\""),
        "missing mapped result: {tree}"
    );
}

#[gpui::test]
fn flex_elements_record_pointer_handlers(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r##"
import { View, div } from "gpui-kit";

export default class PointerHandlers extends View {
  render(cx) {
    return div()
      .id("plot")
      .on_mouse_move((_event, _cx) => {})
      .on_hover((_hovered, _cx) => {})
      .child("Plot");
  }
}
"##;
    let view_type = runtime
        .load_source("pointer-handlers.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    assert!(
        tree.contains(":on_mouse_move(fn)"),
        "missing move handler: {tree}"
    );
    assert!(
        tree.contains(":on_hover(fn)"),
        "missing hover handler: {tree}"
    );
}

#[gpui::test]
fn a_mouse_move_rebuild_keeps_later_callbacks_live(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex, Button } from "gpui-base";

export default class PointerRebuild extends View {
  init() {
    this.moves = 0;
    this.clicks = 0;
    this.hovered = false;
    this.hoverEvents = 0;
    this.moveFunction = null;
    this.clickFunction = null;
  }
  render(cx) {
    return v_flex()
      .w(300)
      .h(160)
      .child(
        div()
          .id("plot")
          .w(300)
          .h(80)
          .on_mouse_move((event, cx) => {
            this.moves += 1;
            this.moveFunction = event.modifiers.function;
            cx.notify();
          })
          .on_hover((hovered, cx) => {
            this.hoverEvents += 1;
            if (this.hovered === hovered) return;
            this.hovered = hovered;
            cx.notify();
          })
          .child(`Moves: ${this.moves}; Move function: ${this.moveFunction}; Hovered: ${this.hovered}; Hover events: ${this.hoverEvents}`),
      )
      .child(
        Button.new("after-hover")
          .w(300)
          .h(80)
          .on_click((event, cx) => {
            this.clicks += 1;
            this.clickFunction = event.modifiers.function;
            cx.notify();
          })
          .child(`Clicks: ${this.clicks}; Click function: ${this.clickFunction}`),
      );
  }
}
"#;
    let view_type = runtime
        .load_source("pointer-rebuild.js", source)
        .expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let tree = |context: &mut VisualTestContext| {
        let view = window.root(context).expect("view");
        context.update(|_, cx| {
            view.read(cx)
                .snapshot()
                .map(crate::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    };

    context.update(|window, cx| window.draw(cx).clear(cx));
    context.simulate_mouse_move(point(px(20.), px(20.)), None, Modifiers::default());
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));
    assert!(tree(&mut context).contains("Moves: 1"));
    assert!(tree(&mut context).contains("Move function: false"));
    assert!(tree(&mut context).contains("Hovered: true"));
    assert!(
        tree(&mut context).contains("Hover events: 1"),
        "replacing a snapshot under a stationary pointer must not dispatch a stale exit"
    );

    context.simulate_mouse_move(point(px(20.), px(120.)), None, Modifiers::default());
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));
    assert!(tree(&mut context).contains("Hovered: false"));
    assert!(
        tree(&mut context).contains("Hover events: 2"),
        "leaving the element must still dispatch a real exit"
    );

    context.simulate_click(point(px(20.), px(120.)), Modifiers::default());
    context.run_until_parked();
    assert!(tree(&mut context).contains("Clicks: 1"));
    assert!(tree(&mut context).contains("Click function: false"));
}

#[gpui::test]
fn flex_elements_dispatch_their_click_handlers(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { h_flex, v_flex } from "gpui-base";

export default class ClickableFlexes extends View {
  init(_props, cx) { this.clicks = [0, 0, 0]; }

  row(element, index, name) {
    return element
      .w_full()
      .h(40)
      .on_click((_event, cx) => {
        this.clicks[index] += 1;
        cx.notify();
      })
      .child(`${name}: ${this.clicks[index]}`);
  }

  render(cx) {
    return v_flex()
      .w(300)
      .h(120)
      .child(this.row(div(), 0, "div"))
      .child(this.row(h_flex(), 1, "h_flex"))
      .child(this.row(v_flex(), 2, "v_flex"));
  }
}
"#;
    let view_type = runtime
        .load_source("clickable-flexes.js", source)
        .expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    for y in [20., 60., 100.] {
        context.simulate_click(point(px(10.), px(y)), Modifiers::default());
    }
    context.update(|window, cx| window.draw(cx).clear(cx));

    let view = window.root(&mut context).expect("view");
    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    for label in ["div: 1", "h_flex: 1", "v_flex: 1"] {
        assert!(tree.contains(&format!("text {label:?}")), "{tree}");
    }
}

#[gpui::test]
fn a_full_color_image_survives_script_render_and_materialize(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View, image } from "gpui-kit";
export default class BrandImage extends View {
  render(cx) { return image("assets/brand.svg").size(28); }
}
"#;
    let view_type = runtime.load_source("brand-image.js", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("render");

    assert!(tree.contains("image \"assets/brand.svg\""), "{tree}");
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

#[gpui::test]
fn an_external_link_survives_the_script_render(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { Link } from "gpui-base";
export default class ExternalLink extends View {
  render() {
    return Link.new("authorize")
      .href("https://example.com/device")
      .child("Open authorization");
  }
}
"#;
    let view_type = runtime
        .load_source("external-link.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    assert!(tree.contains("Link \"authorize\""), "missing Link: {tree}");
    assert!(
        tree.contains(":href[Str(\"https://example.com/device\")]"),
        "missing external target: {tree}"
    );
}

#[gpui::test]
fn an_external_link_requires_a_parseable_http_origin(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { Link } from "gpui-base";
export default class InvalidExternalLink extends View {
  render() { return Link.new("broken").href("https://"); }
}
"#;
    let view_type = runtime
        .load_source("invalid-external-link.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a URL without an origin must be refused at the call site");
    assert!(
        error.to_string().contains("absolute HTTP(S) URL"),
        "unexpected error: {error}"
    );
}

#[gpui::test]
fn render_context_exposes_base_aligned_theme_tokens(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
export default class Themed extends View {
  render(cx) {
    return div()
      .text_color(cx.theme().foreground)
      .bg(cx.theme().surface)
      .p(cx.theme().spacing.md)
      .rounded(cx.theme().radius.md).child("semantic");
  }
}

"#;
    let view_type = runtime
        .load_source("context-theme.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render with cx.theme()")
    });
    assert!(
        tree.contains("text_color[Str(\"#"),
        "theme color was not resolved: {tree}"
    );
    assert!(
        tree.contains("p[Number("),
        "theme spacing was not resolved: {tree}"
    );
}

#[gpui::test]
fn repeated_theme_reads_cross_the_snapshot_boundary_once(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View } from "gpui-kit";

let snapshotReads = 0;
const readSnapshot = globalThis.__theme_snapshot;
globalThis.__theme_snapshot = () => {
  snapshotReads += 1;
  return readSnapshot();
};

export default class Themed extends View {
  render(cx) {
    cx.theme();
    cx.theme();
    cx.theme();
    return `${snapshotReads}:${cx.theme().appearance}`;
  }
}
"#;
    let view_type = runtime
        .load_source("cached-context-theme.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let first = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("one render must transfer one theme snapshot");
    assert!(
        first.contains("text \"1:light\""),
        "unexpected theme: {first}"
    );
    let unchanged = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("an unchanged later render must reuse the transferred theme snapshot");
    assert!(
        unchanged.contains("text \"1:light\""),
        "unchanged theme crossed the snapshot boundary again: {unchanged}"
    );

    context.update(|_, cx| {
        gpui_base::Theme::global_mut(cx).appearance = gpui_base::ThemeAppearance::Dark;
    });
    let changed = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("a changed appearance must refresh the transferred theme snapshot");
    assert!(
        changed.contains("text \"2:dark\""),
        "changed theme did not cross the snapshot boundary exactly once: {changed}"
    );
}

#[gpui::test]
fn render_context_theme_snapshot_is_deeply_read_only(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
export default class Themed extends View {
  render(cx) {
    const theme = cx.theme();
    if (!Object.isFrozen(theme)
        || !Object.isFrozen(theme.colors)
        || !Object.isFrozen(theme.spacing)
        || !Object.isFrozen(theme.radius)) {
      throw new Error("theme snapshot must be deeply frozen");
    }
    return "semantic";
  }
}
"#;
    let view_type = runtime
        .load_source("read-only-context-theme.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("all nested theme token groups must be read-only");
}

#[gpui::test]
fn render_context_theme_rejects_a_stale_context(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
export default class Themed extends View {
  render(cx) {
    if (this.savedTheme) this.savedTheme();
    else this.savedTheme = cx.theme;
    return "semantic";
  }
}
"#;
    let view_type = runtime
        .load_source("stale-context-theme.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("first render captures cx.theme");
    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a theme reader from an earlier render must be stale");
    assert!(
        error.to_string().contains("cx is no longer valid"),
        "{error}"
    );
}

#[test]
fn link_typings_expose_a_real_external_target() {
    let types =
        crate::typings::declarations_with_components(&crate::FrozenComponentRegistry::default());
    assert!(types.contains("export const Link: ComponentType;"));
    assert!(types.contains("href(url: string): Element;"));
}

#[gpui::test]
fn an_element_cannot_be_added_to_two_parents(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Broken extends View {
  render() {
    const shared = div().child("reused");
    return v_flex().child(shared).child(shared);
  }
}
"#;

    let view_type = runtime.load_source("broken", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("reusing an element must fail");

    assert!(
        error.to_string().contains("already added to a parent"),
        "unexpected error: {error}"
    );
}

#[gpui::test]
fn an_unknown_style_method_suggests_the_closest_name(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, div } from "gpui-kit";

export default class Typo extends View {
  render() {
    return div().items_centre();
  }
}
"#;

    let view_type = runtime.load_source("typo", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a typo must fail");

    assert!(
        error.to_string().contains("items_center"),
        "expected a suggestion, got: {error}"
    );
}

#[gpui::test]
fn a_view_renders_through_gpui(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let view_type = runtime.load_source(ENTRY, COUNTER).expect("load");

    // The view is constructed inside the window builder, because `init` may
    // create retained state and that needs a live `Window`.
    let runtime_for_view = runtime.clone();
    let window = cx.add_window(|window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view.clone(), object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    // A real paint must not panic: it exercises materialize, not just the
    // description.
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        |_, _| gpui::div(),
    );
    context.run_until_parked();
}

/// A nested view is a retained invalidation boundary, not an inline rendering
/// helper. Initial props, parent-driven updates and child callbacks all reach
/// the child instance, while the parent's published description stays put.
#[gpui::test]
fn nested_view_updates_and_callbacks_rebuild_only_the_child(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex, Checkbox, InputState } from "gpui-base";

class Child extends View {
  init(props) {
    this.label = "pending";
    this.clicks = 0;
    Promise.resolve().then(() => {
      this.label = props.label;
      this.input = InputState.new({ value: "owned by child" });
    });
  }

  update(props) {
    Promise.resolve().then(() => { this.label = props.label; });
  }

  render(cx) {
    return Checkbox.new("child")
      .w(300)
      .h(40)
      .on_change((_checked, cx) => {
        const expected = this.clicks === 0 ? "before" : "after";
        if (this.label !== expected) return;
        this.clicks += 1;
        cx.notify();
      })
      .child(`${this.label}:${this.clicks}`);
  }
}

export default class Parent extends View {
  init(_props, cx) {
    this.renders = 0;
    this.child = cx.new(Child, { label: "before" });
  }

  render(cx) {
    this.renders += 1;
    return v_flex()
      .w(300)
      .h(80)
      .child(
        Checkbox.new("update-child")
          .w(300)
          .h(40)
          .on_change(() => this.child.set_props({ label: "after" }))
          .child(`parent renders:${this.renders}`),
      )
      .child(
        Checkbox.new("refresh-parent")
          .on_change((_checked, cx) => cx.notify()),
      )
      .child(this.child);
  }
}
"#;
    let view_type = runtime
        .load_source("nested-view-isolation.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let parent = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate parent");

    draw(&mut context, &parent);
    assert_eq!(runtime.metrics().read().script_renders(), 2);
    let parent_tree = |context: &mut VisualTestContext| {
        context.update(|_, cx| {
            parent
                .read(cx)
                .snapshot()
                .map(crate::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    };
    assert!(parent_tree(&mut context).contains("parent renders:1"));
    let child = context.update(|_, cx| {
        let snapshot = parent.read(cx).snapshot().expect("parent snapshot");
        (0..snapshot.len() as u32)
            .filter_map(|id| snapshot.arena().node(id))
            .find_map(|node| match node.component() {
                Some(crate::spec::Component::ChildView(child)) => Some(child.view().clone()),
                _ => None,
            })
            .expect("retained child entity")
    });
    let change_callback =
        |context: &mut VisualTestContext, view: &gpui::Entity<ScriptView>, target: &str| {
            context.update(|_, cx| {
                let snapshot = view.read(cx).snapshot().expect("view snapshot");
                (0..snapshot.len() as u32)
                    .filter_map(|id| snapshot.arena().node(id))
                    .find(|node| {
                        matches!(
                            node.component(),
                            Some(crate::spec::Component::Checkbox(id)) if id == target
                        )
                    })
                    .and_then(|node| {
                        node.ops().iter().find_map(|op| match op {
                            crate::spec::SpecOp::Callback("on_change", id) => Some(*id),
                            _ => None,
                        })
                    })
                    .expect("on_change callback")
            })
        };

    // Initial props reached `init`: the child only notifies when its label is
    // the expected initial value.
    let callback = change_callback(&mut context, &child, "child");
    context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));
    draw(&mut context, &parent);
    assert_eq!(runtime.metrics().read().script_renders(), 3);
    assert!(parent_tree(&mut context).contains("parent renders:1"));

    // `set_props` refreshes the child once and does not rebuild the parent.
    let callback = change_callback(&mut context, &parent, "update-child");
    context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));
    draw(&mut context, &parent);
    assert_eq!(runtime.metrics().read().script_renders(), 4);
    assert!(parent_tree(&mut context).contains("parent renders:1"));

    // Replacing the parent snapshot does not retire the child's current
    // callback generation: the callback belongs to the child entity.
    let child_callback = change_callback(&mut context, &child, "child");
    let callback = change_callback(&mut context, &parent, "refresh-parent");
    context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));
    draw(&mut context, &parent);
    assert_eq!(runtime.metrics().read().script_renders(), 5);
    assert!(parent_tree(&mut context).contains("parent renders:2"));

    // The child callback only notifies after observing the updated props and
    // remains live across that parent snapshot replacement.
    context.update(|window, cx| runtime.dispatch_change(child_callback, true, window, cx));
    draw(&mut context, &parent);
    assert_eq!(runtime.metrics().read().script_renders(), 6);
    assert!(parent_tree(&mut context).contains("parent renders:2"));
}

#[gpui::test]
fn targeted_notify_accepts_a_child_created_in_the_same_init(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View } from "gpui-kit";

class Child extends View {
  init(props) { this.shared = props.shared; }
  render() { return `child:${this.shared.label}`; }
}

export default class Parent extends View {
  init(_props, cx) {
    this.shared = { label: "before" };
    this.child = cx.new(Child, { shared: this.shared });
    this.shared.label = "after";
    cx.notify(this.child);
  }
  render() { return this.child; }
}
"#;
    let view_type = runtime
        .load_source("targeted-notify-during-init.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let parent = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("a newly returned Entity is live for notification");

    draw(&mut context, &parent);
    let child_tree = context.update(|_, cx| {
        let snapshot = parent.read(cx).snapshot().expect("parent snapshot");
        let child = (0..snapshot.len() as u32)
            .filter_map(|id| snapshot.arena().node(id))
            .find_map(|node| match node.component() {
                Some(crate::spec::Component::ChildView(child)) => Some(child.view().clone()),
                _ => None,
            })
            .expect("retained child entity");
        child
            .read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(child_tree.contains("child:after"), "{child_tree}");
}

#[gpui::test]
fn targeted_notify_rebuilds_the_child_without_running_update(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View } from "gpui-kit";
import { v_flex, Checkbox } from "gpui-base";

class Child extends View {
  init(props) {
    this.shared = props.shared;
    this.renders = 0;
    this.updates = 0;
  }
  update() { this.updates += 1; }
  render() {
    this.renders += 1;
    return `child:${this.shared.label}:renders=${this.renders}:updates=${this.updates}`;
  }
}

export default class Parent extends View {
  init(_props, cx) {
    this.renders = 0;
    this.shared = { label: "before" };
    this.child = cx.new(Child, { shared: this.shared });
  }
  render() {
    this.renders += 1;
    return v_flex()
      .child(
        Checkbox.new("notify-child")
          .on_change((_checked, cx) => {
            this.shared.label = "after";
            cx.notify(this.child);
            cx.notify(this.child);
          })
          .child(`parent renders:${this.renders}`),
      )
      .child(this.child);
  }
}
"#;
    let view_type = runtime
        .load_source("targeted-notify.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let parent = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate parent");

    draw(&mut context, &parent);
    let child = context.update(|_, cx| {
        let snapshot = parent.read(cx).snapshot().expect("parent snapshot");
        (0..snapshot.len() as u32)
            .filter_map(|id| snapshot.arena().node(id))
            .find_map(|node| match node.component() {
                Some(crate::spec::Component::ChildView(child)) => Some(child.view().clone()),
                _ => None,
            })
            .expect("retained child entity")
    });
    let callback = context.update(|_, cx| {
        let snapshot = parent.read(cx).snapshot().expect("parent snapshot");
        (0..snapshot.len() as u32)
            .filter_map(|id| snapshot.arena().node(id))
            .find(|node| {
                matches!(
                    node.component(),
                    Some(crate::spec::Component::Checkbox(id)) if id == "notify-child"
                )
            })
            .and_then(|node| {
                node.ops().iter().find_map(|op| match op {
                    crate::spec::SpecOp::Callback("on_change", id) => Some(*id),
                    _ => None,
                })
            })
            .expect("notify callback")
    });
    let before = runtime.metrics().read();

    context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));
    draw(&mut context, &parent);

    let delta = runtime.metrics().read().since(&before);
    assert_eq!(delta.script_renders(), 1, "two notifications coalesce");
    let parent_tree = context.update(|_, cx| {
        parent
            .read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(parent_tree.contains("parent renders:1"), "{parent_tree}");
    let child_tree = context.update(|_, cx| {
        child
            .read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        child_tree.contains("child:after:renders=2:updates=0"),
        "{child_tree}"
    );
}

#[gpui::test]
fn targeted_notify_rejects_malformed_and_released_entities(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View } from "gpui-kit";
import { v_flex, Checkbox } from "gpui-base";

class Child extends View { render() { return "child"; } }

export default class Parent extends View {
  init(_props, cx) {
    this.child = cx.new(Child);
    this.showChild = true;
    this.error = "none";
  }
  render() {
    return v_flex()
      .child(Checkbox.new("malformed").on_change((_checked, cx) => {
        try { cx.notify({}); }
        catch (error) { this.error = error.message; cx.notify(); }
      }))
      .child(Checkbox.new("released").on_change((_checked, cx) => {
        this.showChild = false;
        this.child.release();
        try { cx.notify(this.child); }
        catch (error) { this.error = error.message; cx.notify(); }
      }))
      .child(this.error)
      .children(this.showChild ? [this.child] : []);
  }
}
"#;
    let view_type = runtime
        .load_source("targeted-notify-errors.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let parent = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate parent");
    draw(&mut context, &parent);

    let callback = |target: &str, context: &mut VisualTestContext| {
        context.update(|_, cx| {
            let snapshot = parent.read(cx).snapshot().expect("parent snapshot");
            (0..snapshot.len() as u32)
                .filter_map(|id| snapshot.arena().node(id))
                .find(|node| {
                    matches!(
                        node.component(),
                        Some(crate::spec::Component::Checkbox(id)) if id == target
                    )
                })
                .and_then(|node| {
                    node.ops().iter().find_map(|op| match op {
                        crate::spec::SpecOp::Callback("on_change", id) => Some(*id),
                        _ => None,
                    })
                })
                .expect("targeted notify error callback")
        })
    };
    let tree = |context: &mut VisualTestContext| {
        context.update(|_, cx| {
            parent
                .read(cx)
                .snapshot()
                .map(crate::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    };

    let malformed = callback("malformed", &mut context);
    context.update(|window, cx| runtime.dispatch_change(malformed, true, window, cx));
    draw(&mut context, &parent);
    assert!(tree(&mut context).contains("expects an Entity"));

    let released = callback("released", &mut context);
    context.update(|window, cx| runtime.dispatch_change(released, true, window, cx));
    draw(&mut context, &parent);
    let released_tree = tree(&mut context);
    assert!(
        released_tree.contains("expects a live Entity"),
        "{released_tree}"
    );
}

/// A generic `with_js` nested inside child construction must not recursively
/// consume the operations that follow that construction. The first update is
/// ordered after its create, and each grandchild remains owned by the child
/// whose init requested it.
#[gpui::test]
fn nested_view_operations_from_one_job_are_fifo_and_keep_descendant_ownership(
    cx: &mut TestAppContext,
) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex, Checkbox } from "gpui-base";

class Grandchild extends View {
  render() { return "grandchild"; }
}

class Child extends View {
  init(props, cx) {
    this.label = props.label;
    this.grandchild = cx.new(Grandchild);
  }
  update(props) { this.label = props.label; }
  render() {
    return v_flex().child(this.label).child(this.grandchild);
  }
}

export default class Parent extends View {
  init(_props, cx) {
    this.first = cx.new(Child, { label: "first" });
    this.first.set_props({ label: "updated" });
    this.second = cx.new(Child, { label: "second" });
  }
  render(cx) {
    return v_flex()
      .child(Checkbox.new("release-first").on_change(() => this.first.release()))
      .child(this.first)
      .child(this.second);
  }
}
"#;
    let view_type = runtime
        .load_source("nested-view-fifo.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let parent = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("the queued creates and update must apply in source order");

    draw(&mut context, &parent);
    let children = context.update(|_, cx| {
        let snapshot = parent.read(cx).snapshot().expect("parent snapshot");
        (0..snapshot.len() as u32)
            .filter_map(|id| snapshot.arena().node(id))
            .filter_map(|node| match node.component() {
                Some(crate::spec::Component::ChildView(child)) => Some(child.view().clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    });
    assert_eq!(children.len(), 2);
    let trees = context.update(|_, cx| {
        children
            .iter()
            .map(|child| {
                child
                    .read(cx)
                    .snapshot()
                    .map(crate::RenderSnapshot::debug_tree)
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
    });
    assert!(trees[0].contains("updated"), "first child: {}", trees[0]);
    assert!(trees[1].contains("second"), "second child: {}", trees[1]);
    assert_eq!(
        runtime.entities().len(),
        4,
        "two children and their two owned grandchildren must be retained"
    );

    let release = context.update(|_, cx| {
        let snapshot = parent.read(cx).snapshot().expect("parent snapshot");
        (0..snapshot.len() as u32)
            .filter_map(|id| snapshot.arena().node(id))
            .find(|node| {
                matches!(
                    node.component(),
                    Some(crate::spec::Component::Checkbox(id)) if id == "release-first"
                )
            })
            .and_then(|node| {
                node.ops().iter().find_map(|op| match op {
                    crate::spec::SpecOp::Callback("on_change", id) => Some(*id),
                    _ => None,
                })
            })
            .expect("release callback")
    });
    context.update(|window, cx| runtime.dispatch_change(release, true, window, cx));
    assert_eq!(
        runtime.entities().len(),
        2,
        "releasing the first child must release its grandchild, not its sibling subtree"
    );
    assert!(context.update(|_, cx| children[0].read(cx).snapshot().is_none()));
    assert!(context.update(|_, cx| children[1].read(cx).snapshot().is_some()));
}

/// A release requested by the caller's promise wave while its queued create is
/// becoming a real entity must be ordered after that create, not report a
/// spurious miss and leave the child live.
#[gpui::test]
fn a_release_during_nested_view_creation_retires_the_candidate(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
class Child extends View { render() { return "child"; } }
export default class Parent extends View {
  init(_props, cx) {
    this.released = false;
    this.child = cx.new(Child);
    Promise.resolve().then(() => { this.released = this.child.release(); });
  }
  render(cx) { return `released:${this.released}`; }
}
"#;
    let view_type = runtime
        .load_source("nested-view-in-flight-release.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let parent = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate parent");

    draw(&mut context, &parent);
    let tree = context.update(|_, cx| {
        parent
            .read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(tree.contains("released:true"), "{tree}");
    assert!(runtime.entities().is_empty(), "the candidate remained live");
}

/// A throwing update runs against an isolated instance and a retained-work
/// checkpoint. Neither object fields nor entities/tasks created by its causal
/// wave may become part of the live child.
#[gpui::test]
fn failed_nested_update_rolls_back_script_fields_entities_and_tasks(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex, Checkbox, InputState } from "gpui-base";

class Child extends View {
  init() {
    this.callback = () => {};
    this.callback.label = "callable-good";
    this.state = { label: "good", clicks: 0 };
  }
  update(props) {
    this.state.label = props.label;
    this.state.partial = "must disappear";
    this.callback.label = "callable-bad";
    this.input = InputState.new({ value: "must roll back" });
    this.tick = cx.timer.every(60_000, () => {});
    Promise.resolve().then(() => {
      this.later = InputState.new({ value: "causal rollback" });
    });
    throw new Error("reject update");
  }
  render(cx) {
    return Checkbox.new("child-after-failure")
      .on_change((_checked, cx) => { this.state.clicks += 1; cx.notify(); })
      .child(`${this.state.label}:${this.state.clicks}:${this.state.partial ?? "clean"}:${this.callback.label}`);
  }
}

export default class Parent extends View {
  init(_props, cx) { this.child = cx.new(Child); }
  render(cx) {
    return v_flex()
      .child(Checkbox.new("fail-update").on_change(() => {
        this.child.set_props({ label: "half committed" });
      }))
      .child(this.child);
  }
}
"#;
    let view_type = runtime
        .load_source("nested-update-rollback.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let parent = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate parent");
    draw(&mut context, &parent);
    let child = context.update(|_, cx| {
        let snapshot = parent.read(cx).snapshot().expect("parent snapshot");
        (0..snapshot.len() as u32)
            .filter_map(|id| snapshot.arena().node(id))
            .find_map(|node| match node.component() {
                Some(crate::spec::Component::ChildView(child)) => Some(child.view().clone()),
                _ => None,
            })
            .expect("child")
    });
    let change =
        |context: &mut VisualTestContext, view: &gpui::Entity<ScriptView>, target: &str| {
            context.update(|_, cx| {
                let snapshot = view.read(cx).snapshot().expect("snapshot");
                (0..snapshot.len() as u32)
                    .filter_map(|id| snapshot.arena().node(id))
                    .find(|node| {
                        matches!(
                            node.component(),
                            Some(crate::spec::Component::Checkbox(id)) if id == target
                        )
                    })
                    .and_then(|node| {
                        node.ops().iter().find_map(|op| match op {
                            crate::spec::SpecOp::Callback("on_change", id) => Some(*id),
                            _ => None,
                        })
                    })
                    .expect("change callback")
            })
        };
    let retained_before = runtime.entities().len();
    let aliases_before = runtime.nested_view_alias_count();
    let tasks_before = crate::engine::quickjs::task_count();
    let fail = change(&mut context, &parent, "fail-update");
    context.update(|window, cx| runtime.dispatch_change(fail, true, window, cx));
    assert_eq!(runtime.entities().len(), retained_before);
    assert_eq!(runtime.nested_view_alias_count(), aliases_before);
    assert_eq!(crate::engine::quickjs::task_count(), tasks_before);

    // Force a fresh child build through its pre-update callback. If `update`
    // touched the live object this would expose "half committed" here.
    let refresh = change(&mut context, &child, "child-after-failure");
    context.update(|window, cx| runtime.dispatch_change(refresh, true, window, cx));
    draw(&mut context, &parent);
    let tree = context.update(|_, cx| {
        child
            .read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("good:1:clean:callable-good"),
        "failed update leaked into child: {tree}"
    );
}

/// Native retained-view tokens remain authority-bearing even when two
/// independently authorized applications intentionally share one runtime.
#[gpui::test]
fn nested_view_tokens_reject_foreign_application_mount_update_and_release(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let root = std::env::temp_dir().join(format!(
        "gpui-shell-nested-token-provenance-{}",
        std::process::id()
    ));
    let victim_dir = root.join("victim");
    let attacker_dir = root.join("attacker");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&victim_dir).expect("victim directory");
    std::fs::create_dir_all(&attacker_dir).expect("attacker directory");
    std::fs::write(
        victim_dir.join("main.js"),
        r#"import { div, View } from "gpui-kit";
class Child extends View {
  init(props) { this.label = props.label; }
  update(props) { this.label = props.label; }
  render() { return this.label; }
}
export default class Victim extends View {
  init(_props, cx) { this.child = cx.new(Child, { label: "victim-intact" }); }
  render() { return this.child; }
}"#,
    )
    .expect("victim source");
    std::fs::write(
        attacker_dir.join("main.js"),
        r#"import { div, View } from "gpui-kit";
export default class Attacker extends View {
  init() {
    this.results = [];
    try { globalThis.__view_set_props(0, { label: "stolen" }); this.results.push("updated"); }
    catch (_) { this.results.push("update-refused"); }
    try { globalThis.__view_release(0); this.results.push("released"); }
    catch (_) { this.results.push("release-refused"); }
  }
  render(cx) {
    try { globalThis.__child_view(0); this.results.push("mounted"); }
    catch (_) { this.results.push("mount-refused"); }
    return this.results.join(",");
  }
}"#,
    )
    .expect("attacker source");

    let victim_type = runtime
        .load_app(&victim_dir, "main.js")
        .expect("load victim");
    let attacker_type = runtime
        .load_app(&attacker_dir, "main.js")
        .expect("load attacker");
    let victim_policy = Rc::new(Policy::default());
    let attacker_policy = Rc::new(Policy::default());
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let victim = context
        .update(|window, cx| {
            runtime.instantiate_view_with_policy(&victim_type, victim_policy.clone(), window, cx)
        })
        .expect("instantiate victim");
    draw(&mut context, &victim);
    let victim_child = context.update(|_, cx| {
        let snapshot = victim.read(cx).snapshot().expect("victim snapshot");
        (0..snapshot.len() as u32)
            .filter_map(|id| snapshot.arena().node(id))
            .find_map(|node| match node.component() {
                Some(crate::spec::Component::ChildView(child)) => Some(child.view().clone()),
                _ => None,
            })
            .expect("victim child")
    });
    let attacker = context
        .update(|window, cx| {
            runtime.instantiate_view_with_policy(
                &attacker_type,
                attacker_policy.clone(),
                window,
                cx,
            )
        })
        .expect("foreign operations must be synchronously catchable");
    draw(&mut context, &attacker);
    let tree = |view: &gpui::Entity<ScriptView>, context: &mut VisualTestContext| {
        context.update(|_, cx| {
            view.read(cx)
                .snapshot()
                .map(crate::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    };
    assert!(tree(&victim_child, &mut context).contains("victim-intact"));
    let attacker_tree = tree(&attacker, &mut context);
    assert!(attacker_tree.contains("update-refused"), "{attacker_tree}");
    assert!(attacker_tree.contains("release-refused"), "{attacker_tree}");
    assert!(attacker_tree.contains("mount-refused"), "{attacker_tree}");
    let _ = std::fs::remove_dir_all(root);
}

/// The public release method is the complete subtree teardown boundary: typed
/// records, opaque aliases, tasks, callbacks and frame-retained snapshots all
/// retire together while the parent stays mounted.
#[gpui::test]
fn public_nested_release_retires_descendants_callbacks_tasks_snapshots_and_aliases(
    cx: &mut TestAppContext,
) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex, Checkbox, InputState } from "gpui-base";
class Grandchild extends View {
  init(_props, cx) {
    this.input = InputState.new({ value: "grandchild" });
    this.tick = cx.timer.every(60_000, () => {});
  }
  render() { return Checkbox.new("grandchild-event").on_change(() => {}).child("grandchild"); }
}
class Child extends View {
  init(_props, cx) {
    this.input = InputState.new({ value: "child" });
    this.tick = cx.timer.every(60_000, () => {});
    this.grandchild = cx.new(Grandchild);
  }
  render() {
    return v_flex()
      .child(Checkbox.new("child-event").on_change(() => {}).child("child"))
      .child(this.grandchild);
  }
}
export default class Parent extends View {
  init(_props, cx) { this.child = cx.new(Child); }
  render(cx) {
    return v_flex()
      .child(Checkbox.new("release-subtree").on_change(() => this.child.release()))
      .child(this.child);
  }
}
"#;
    let baseline_tasks = crate::engine::quickjs::task_count();
    let view_type = runtime
        .load_source("nested-public-release-cleanup.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let parent = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate parent");
    draw(&mut context, &parent);
    let child = context.update(|_, cx| {
        let snapshot = parent.read(cx).snapshot().expect("parent snapshot");
        (0..snapshot.len() as u32)
            .filter_map(|id| snapshot.arena().node(id))
            .find_map(|node| match node.component() {
                Some(crate::spec::Component::ChildView(child)) => Some(child.view().clone()),
                _ => None,
            })
            .expect("child")
    });
    let grandchild = context.update(|_, cx| {
        let snapshot = child.read(cx).snapshot().expect("child snapshot");
        (0..snapshot.len() as u32)
            .filter_map(|id| snapshot.arena().node(id))
            .find_map(|node| match node.component() {
                Some(crate::spec::Component::ChildView(child)) => Some(child.view().clone()),
                _ => None,
            })
            .expect("grandchild")
    });
    assert_eq!(runtime.entities().len(), 4);
    assert_eq!(runtime.nested_view_alias_count(), 2);
    assert_eq!(crate::engine::quickjs::task_count(), baseline_tasks + 2);
    assert_eq!(runtime.live_callbacks(), 3);

    let release = context.update(|_, cx| {
        let snapshot = parent.read(cx).snapshot().expect("parent snapshot");
        (0..snapshot.len() as u32)
            .filter_map(|id| snapshot.arena().node(id))
            .find(|node| {
                matches!(
                    node.component(),
                    Some(crate::spec::Component::Checkbox(id)) if id == "release-subtree"
                )
            })
            .and_then(|node| {
                node.ops().iter().find_map(|op| match op {
                    crate::spec::SpecOp::Callback("on_change", id) => Some(*id),
                    _ => None,
                })
            })
            .expect("release callback")
    });
    context.update(|window, cx| runtime.dispatch_change(release, true, window, cx));

    assert!(runtime.entities().is_empty());
    assert_eq!(runtime.nested_view_alias_count(), 0);
    assert_eq!(crate::engine::quickjs::task_count(), baseline_tasks);
    assert_eq!(
        runtime.live_callbacks(),
        1,
        "only the still-mounted parent's release callback may remain live"
    );
    assert!(context.update(|_, cx| child.read(cx).snapshot().is_none()));
    assert!(context.update(|_, cx| grandchild.read(cx).snapshot().is_none()));
}

/// Child rendering uses the same transactional snapshot publication as a root:
/// a failed replacement build records the error but leaves the last good child
/// description mounted, without rebuilding the parent.
#[gpui::test]
fn child_render_failure_preserves_its_previous_good_snapshot(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex, Checkbox } from "gpui-base";
class Child extends View {
  init() { this.fail = false; }
  render(cx) {
    if (this.fail) throw new Error("child render rejected");
    return Checkbox.new("break-child")
      .on_change((_checked, cx) => { this.fail = true; cx.notify(); })
      .child("last good child");
  }
}
export default class Parent extends View {
  init(_props, cx) { this.renders = 0; this.child = cx.new(Child); }
  render(cx) {
    this.renders += 1;
    return v_flex().child(`parent:${this.renders}`).child(this.child);
  }
}
"#;
    let view_type = runtime
        .load_source("nested-child-render-failure.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let parent = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate parent");
    draw(&mut context, &parent);
    let child = context.update(|_, cx| {
        let snapshot = parent.read(cx).snapshot().expect("parent snapshot");
        (0..snapshot.len() as u32)
            .filter_map(|id| snapshot.arena().node(id))
            .find_map(|node| match node.component() {
                Some(crate::spec::Component::ChildView(child)) => Some(child.view().clone()),
                _ => None,
            })
            .expect("child")
    });
    let previous = context.update(|_, cx| {
        child
            .read(cx)
            .snapshot()
            .expect("good child snapshot")
            .debug_tree()
    });
    let callback = context.update(|_, cx| {
        let snapshot = child.read(cx).snapshot().expect("child snapshot");
        (0..snapshot.len() as u32)
            .filter_map(|id| snapshot.arena().node(id))
            .find_map(|node| {
                node.ops().iter().find_map(|op| match op {
                    crate::spec::SpecOp::Callback("on_change", id) => Some(*id),
                    _ => None,
                })
            })
            .expect("child callback")
    });
    context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));
    draw(&mut context, &parent);

    context.update(|_, cx| {
        let child = child.read(cx);
        assert_eq!(
            child.snapshot().map(crate::RenderSnapshot::debug_tree),
            Some(previous)
        );
        assert!(
            child
                .build_error()
                .is_some_and(|error| error.contains("child render rejected"))
        );
        let parent = parent.read(cx);
        let tree = parent
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default();
        assert!(tree.contains("parent:1"), "parent rebuilt: {tree}");
    });
}

#[gpui::test]
fn a_released_nested_view_cannot_be_mounted_again(cx: &mut TestAppContext) {
    let message = nested_view_build_error(
        cx,
        "released-nested-view.js",
        r#"
import { div, View } from "gpui-kit";
class Child extends View { render() { return "child"; } }
export default class Parent extends View {
  init(_props, cx) {
    this.child = cx.new(Child);
    if (!this.child.release()) throw new Error("the live child was not released");
  }
  render() { return this.child; }
}
"#,
    );
    assert!(message.contains("released"), "unexpected error: {message}");
}

#[gpui::test]
fn a_nested_view_handle_can_only_be_mounted_once_per_snapshot(cx: &mut TestAppContext) {
    let message = nested_view_build_error(
        cx,
        "duplicate-nested-view.js",
        r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
class Child extends View { render() { return "child"; } }
export default class Parent extends View {
  init(_props, cx) { this.child = cx.new(Child); }
  render(cx) {
    return v_flex().child(this.child).child(this.child);
  }
}
"#,
    );
    assert!(
        message.contains("once") && message.contains("snapshot"),
        "unexpected error: {message}"
    );
}

#[gpui::test]
fn nested_view_creation_rejects_constructible_non_view_functions(cx: &mut TestAppContext) {
    let message = nested_view_build_error(
        cx,
        "nested-view-class-contract.js",
        r#"
import { div, View } from "gpui-kit";
function ConstructibleButNotAView() {}
export default class Parent extends View {
  render(cx) {
    cx.new(ConstructibleButNotAView);
    return "parent";
  }
}
"#,
    );
    assert!(
        message.contains("expects a View subclass"),
        "unexpected class validation error: {message}"
    );
}

/// Constructor/init failures cannot be drained re-entrantly into the native
/// call, so the explicit synchronous API contract reports them from the
/// enclosing host entry. The failed public create still rolls back everything
/// it retained before that error reached Rust.
#[gpui::test]
fn public_nested_constructor_failure_reaches_the_host_boundary_and_rolls_back(
    cx: &mut TestAppContext,
) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { InputState } from "gpui-base";
class Child extends View {
  // `init` is where a view is handed a context, so it is where a view that
  // allocates can allocate. A `constructor` has none, which is the point of
  // this fixture: the rollback still has to reach what it made.
  init(_props, cx) {
    this.input = InputState.new({ value: "constructor allocation" });
    this.tick = cx.timer.every(60_000, () => {});
    throw new Error("child constructor rejected");
  }
  render() { return "unreachable"; }
}
export default class Parent extends View {
  init(_props, cx) { this.child = cx.new(Child); }
  render(cx) { return "parent"; }
}
"#;
    let tasks = crate::engine::quickjs::task_count();
    let view_type = runtime
        .load_source("nested-public-constructor-failure.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let error = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect_err("the enclosing host entry must report constructor failure");
    assert!(
        error.to_string().contains("child constructor rejected"),
        "{error}"
    );
    assert!(runtime.entities().is_empty());
    assert_eq!(runtime.nested_view_alias_count(), 0);
    assert_eq!(crate::engine::quickjs::task_count(), tasks);
}

#[gpui::test]
fn nested_view_creation_updates_and_release_are_rejected_during_render(cx: &mut TestAppContext) {
    let creation = nested_view_build_error(
        cx,
        "nested-view-created-in-render.js",
        r#"
import { div, View } from "gpui-kit";
class Child extends View { render(cx) { return "child"; } }
export default class Parent extends View {
  render(cx) {
    cx.new(Child);
    return "parent";
  }
}
"#,
    );
    assert!(
        creation.contains("cx.new") && creation.contains("during render"),
        "unexpected creation error: {creation}"
    );

    let update = nested_view_build_error(
        cx,
        "nested-view-updated-in-render.js",
        r#"
import { div, View } from "gpui-kit";
class Child extends View { render() { return "child"; } }
export default class Parent extends View {
  init(_props, cx) { this.child = cx.new(Child); }
  render() {
    this.child.set_props({ value: 2 });
    return "parent";
  }
}
"#,
    );
    assert!(
        update.contains("set_props") && update.contains("during render"),
        "unexpected update error: {update}"
    );

    let release = nested_view_build_error(
        cx,
        "nested-view-released-in-render.js",
        r#"
import { div, View } from "gpui-kit";
class Child extends View { render() { return "child"; } }
export default class Parent extends View {
  init(_props, cx) { this.child = cx.new(Child); }
  render(cx) {
    this.child.release();
    return "parent";
  }
}
"#,
    );
    assert!(
        release.contains("release") && release.contains("during render"),
        "unexpected release error: {release}"
    );
}

/// Layout callbacks can catch their own API errors. Publishing the messages in
/// the next parent snapshot proves the call failed at the JavaScript call site
/// and that the diagnostic names layout rather than the broader render path.
#[gpui::test]
fn nested_view_creation_updates_and_release_name_the_layout_phase(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex, v_virtual_list, Button } from "gpui-base";
class Child extends View { render() { return "child"; } }
export default class Parent extends View {
  init(_props, cx) {
    this.child = cx.new(Child);
    this.errors = [];
  }
  render(cx) {
    return v_flex()
      .w(300)
      .h(80)
      .child(Button.new("publish").h(40).on_click((_event, cx) => cx.notify()).child(this.errors.join(" | ")))
      .child(v_virtual_list("rows", 1, 40, (index) => String(index), (_range, cx) => {
        this.errors = [];
        // The renderer's own `cx`, not the render pass's: the outer one names a
        // call the nested layout scope has already replaced.
        try { cx.new(Child); } catch (error) { this.errors.push(String(error)); }
        try { this.child.set_props({ value: 2 }); } catch (error) { this.errors.push(String(error)); }
        try { this.child.release(); } catch (error) { this.errors.push(String(error)); }
        return ["row"];
      }));
  }
}
"#;
    let view_type = runtime
        .load_source("nested-view-layout-phase.js", source)
        .expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let parent = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate parent");
        RootedScriptView(parent)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let parent = window
        .root(&mut context)
        .expect("parent view")
        .read_with(&context, |root, _| root.0.clone());
    context.simulate_click(point(px(20.), px(20.)), Modifiers::default());
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = context.update(|_, cx| {
        parent
            .read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(tree.contains("cx.new"), "creation error missing: {tree}");
    assert!(tree.contains("set_props"), "update error missing: {tree}");
    assert!(tree.contains("release"), "release error missing: {tree}");
    assert!(tree.matches("during layout").count() >= 3, "{tree}");
}

fn nested_view_build_error(cx: &mut TestAppContext, name: &str, source: &str) -> String {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source(name, source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let parent = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate parent");
    draw(&mut context, &parent);
    context.update(|_, cx| {
        parent
            .read(cx)
            .build_error()
            .expect("the nested-view call must fail where it was written")
            .to_owned()
    })
}

struct Empty;

/// Test-only window root that lets an already-created script-view entity take
/// part in real window layout without wrapping its retained state.
struct RootedScriptView(gpui::Entity<ScriptView>);

impl gpui::Render for RootedScriptView {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        self.0.clone()
    }
}

impl gpui::Render for Empty {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
    }
}

use std::ops::Deref;

#[gpui::test]
fn the_bundled_example_application_loads_and_renders(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    // The example is the contract with users: if it stops rendering, the
    // quickstart in the README is wrong.
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/js_todolist")
        .canonicalize()
        .expect("example directory");

    let view_type = runtime
        .load_app(&directory, "main.js")
        .expect("load example");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    assert!(tree.contains("Button"), "example has no button: {tree}");
    assert!(tree.contains("text"), "example has no text: {tree}");
}

#[gpui::test]
fn state_styles_reuse_the_ordinary_style_methods(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, div } from "gpui-kit";
import { Button } from "gpui-base";

export default class Styled extends View {
  render(cx) {
    return div()
      .hover((el) => el.bg(`#eff6ff`))
      .child(
        Button.new("go")
          .bg(`#2563eb`)
          .hover((el) => el.opacity(0.9))
          .active((el) => el.opacity(0.8))
          .child("Go"),
      );
  }
}
"#;

    let view_type = runtime.load_source("styled", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    assert!(tree.contains(":hover(.bg"), "hover not recorded: {tree}");
    assert!(
        tree.contains(":active(.opacity"),
        "active not recorded: {tree}"
    );
}

#[gpui::test]
fn transition_declarations_survive_the_script_render(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, div } from "gpui-kit";

export default class Motion extends View {
  render() {
    return div()
      .id("sidebar")
      .w(320)
      .transition("width", { duration: 180, delay: 20, easing: "ease-out" });
  }
}
"#;

    let view_type = runtime.load_source("motion", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    assert!(
        tree.contains(":transition(width, 180ms, 20ms, ease-out)"),
        "the native motion target and policy were not retained in the snapshot: {tree}"
    );
}

#[gpui::test]
fn native_overflow_scroll_behaviors_survive_script_render_and_materialize(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class ScrollableQuotes extends View {
  render() {
    return v_flex()
      .child(v_flex().id("both").size(80).overflow_scroll().child("Both"))
      .child(v_flex().id("horizontal").size(80).overflow_x_scroll().child("Horizontal"))
      .child(v_flex().id("watchlist-quotes").h(120).overflow_y_scroll()
        .children(Array.from({ length: 30 }, (_, index) => `Quote ${index}`)))
      .child(v_flex().id("bar-both").size(80).overflow_scrollbar().child("Both bars"))
      .child(v_flex().id("bar-horizontal").size(80).overflow_x_scrollbar().child("Horizontal bar"))
      .child(v_flex().id("bar-vertical").h(120).overflow_y_scrollbar()
        .children(Array.from({ length: 30 }, (_, index) => `Bar quote ${index}`)));
  }
}
"#;

    let view_type = runtime.load_source("scroll-y", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("native overflow scroll methods must be supported behaviors");
    for behavior in [
        "overflow_scroll",
        "overflow_x_scroll",
        "overflow_y_scroll",
        "overflow_scrollbar",
        "overflow_x_scrollbar",
        "overflow_y_scrollbar",
    ] {
        assert!(
            tree.contains(&format!(":{behavior}")),
            "missing {behavior}: {tree}"
        );
    }
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// A `Scrollbar` is paired with its scroll area by name and by nothing else, so
/// what has to be tested is that the pair survives a real frame: the area has
/// to register a scroll position under its id, and the bar has to find it there
/// on the frame after.
#[gpui::test]
fn a_scrollbar_drives_the_scroll_area_that_shares_its_name(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { Scrollbar, v_flex } from "gpui-base";

export default class Watchlist extends View {
  render() {
    return v_flex()
      .relative()
      .h(120)
      .child(
        v_flex().id("watchlist").size_full().overflow_y_scroll()
          .children(Array.from({ length: 40 }, (_, index) => `Quote ${index}`)))
      .child(
        Scrollbar.vertical("watchlist")
          .mode("always")
          .viewport_from_layout()
          .absolute()
          .inset_0());
  }
}
"#;
    let view_type = runtime.load_source("scrollbar", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("Scrollbar must be a supported component");

    assert!(
        tree.contains("Scrollbar \"watchlist\""),
        "missing scrollbar: {tree}"
    );
    assert!(
        tree.contains(":axis"),
        "`Scrollbar.vertical` narrows the axis, so the description must carry it: {tree}"
    );
    assert!(
        tree.contains(":mode") && tree.contains(":viewport_from_layout"),
        "the show mode and the layout viewport must survive into the description: {tree}"
    );

    // Two frames, because the pairing only exists once something has been laid
    // out: the first registers the scroll position under `watchlist`, and the
    // second is the one on which the bar would report an area it never found.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
    draw(&mut context, &view);
}

/// A tab list holds no selection: each tab is told whether it is selected and
/// reports activation back, so the description has to carry both directions.
#[gpui::test]
fn a_tab_list_carries_selection_in_and_activation_out(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { Tabs, Tab } from "gpui-base";

export default class Settings extends View {
  init() { this.tab = 0; }
  render(cx) {
    const names = ["Account", "Network"];
    return Tabs.new("settings").children(
      names.map((name, index) =>
        Tab.new(`settings-${index}`)
          .selected(index === this.tab)
          .disabled(index === 1)
          .accessibility_label(name)
          .set_position(index + 1, names.length)
          .on_click((_event, cx) => { this.tab = index; cx.notify(); })
          .child(name)));
  }
}
"#;
    let view_type = runtime.load_source("tabs", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("Tabs and Tab must be supported components");

    assert!(
        tree.contains("Tabs \"settings\""),
        "missing tab list: {tree}"
    );
    assert!(
        tree.contains("Tab \"settings-0\""),
        "missing first tab: {tree}"
    );
    assert!(
        tree.contains(":set_position"),
        "the announced position must survive into the description: {tree}"
    );
    assert!(
        tree.contains(":selected"),
        "selection is controlled, so it must be described: {tree}"
    );
    assert!(
        tree.contains(":on_click"),
        "activation is reported back, so the handler must be described: {tree}"
    );

    // And the whole thing has to materialize: `Tab` is a `Stateful<Div>` under
    // the hood, so a state style that has nowhere to land would fail here
    // rather than in a story.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// A logarithmic slider whose range reaches zero is a script mistake that base
/// asserts on — which would take the whole application down rather than report
/// anything. It has to arrive as a `TypeError`.
#[gpui::test]
fn a_logarithmic_slider_that_reaches_zero_is_refused_rather_than_asserted(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { SliderState } from "gpui-base";

export default class Gain extends View {
  init() { this.gain = SliderState.new({ min: 0, max: 1000, scale: "logarithmic" }); }
  render(cx) { return "unreachable"; }
}
"#;
    let view_type = runtime.load_source("gain", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let Err(error) = context.update(|window, cx| runtime.instantiate(&view_type, window, cx))
    else {
        panic!("a logarithmic scale cannot start at zero");
    };
    assert!(
        error.to_string().contains("logarithmic"),
        "the error has to name what is wrong: {error}"
    );
}

/// JavaScript numbers are f64, while Base stores slider numbers as f32. A
/// finite f64 above f32::MAX must be rejected before narrowing; otherwise two
/// distinct logarithmic bounds both become infinity and Base asserts.
#[gpui::test]
fn a_slider_rejects_numbers_that_do_not_fit_its_native_representation(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { SliderState } from "gpui-base";

export default class Gain extends View {
  init() { this.gain = SliderState.new({ min: 1e100, max: 2e100, scale: "logarithmic" }); }
  render(cx) { return "unreachable"; }
}
"#;
    let view_type = runtime.load_source("huge-gain", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        context.update(|window, cx| runtime.instantiate(&view_type, window, cx))
    }));
    let result = result.expect("a script number must not reach Base as infinity and panic");
    let error = result.expect_err("a number outside f32 must be a script error");
    assert!(
        error.to_string().contains("native slider"),
        "the error must explain the representable boundary: {error}"
    );
}

/// A slider is four parts the script composes, and one thing the script does
/// not do: compute where the value is.
///
/// The description carries no position at all — that is the assertion below —
/// because a drag never re-enters the VM. The value moves in the state, the
/// frame after reads it back, and a position described here would be the one
/// the last script render saw.
#[gpui::test]
fn a_slider_is_composed_by_the_script_and_positioned_by_the_shell(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { SliderState, Slider, SliderTrack, SliderIndicator, SliderThumb, v_flex } from "gpui-base";

export default class Volume extends View {
  init(_props, cx) {
    this.volume = SliderState.new({ min: 0, max: 100, step: 5, value: 40 });
    this.volume.on("change", (value, cx) => { this.latest = value; cx.notify(); });
  }
  render(cx) {
    return v_flex()
      .child(`${this.volume.value()} of ${this.volume.max_value()}`)
      .child(
        Slider.new(this.volume).child(
          SliderTrack.new(this.volume).flex().items_center().h(24).w_full().child(
            SliderIndicator.new(this.volume)
              .relative()
              .w_full()
              .h(6)
              .bg(`#e5e7eb`)
              .range_style((fill) => fill.bg(`#2563eb`))
              .child(SliderThumb.new(this.volume).size(16).bg(`#2563eb`)),
          ),
        ),
      );
  }
}
"#;
    let view_type = runtime.load_source("slider", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("Slider and its three parts must be supported components");

    for part in [
        "Slider #",
        "SliderTrack #",
        "SliderIndicator #",
        "SliderThumb #",
    ] {
        assert!(tree.contains(part), "missing {part}: {tree}");
    }
    assert!(
        tree.contains(":range_style("),
        "the fill is declared as a style, not described as an element: {tree}"
    );
    // The value reached the script, which is what makes a readout beside the
    // slider possible at all.
    assert!(
        tree.contains("40 of 100"),
        "the state must be readable: {tree}"
    );
    // And none of the geometry did. `left`, `bottom` and `absolute` are what a
    // script would have had to compute; finding one here would mean the thumb
    // is pinned to the value this render saw.
    for frozen in [".left[", ".bottom[", ".absolute"] {
        assert!(
            !tree.contains(frozen),
            "the description must carry no slider geometry, found `{frozen}`: {tree}"
        );
    }

    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime.clone(), object)));
    draw(&mut context, &view);
    let renders = runtime.metrics().read().script_renders();

    // The claim, end to end: a value changed outside the script repaints the
    // slider — new geometry, new announcement — without entering the VM.
    let state = runtime
        .entities()
        .first_slider()
        .expect("the script's slider state");
    context.update(|window, cx| {
        state.update(cx, |state, cx| state.set_value(80.0f32, window, cx));
    });
    draw(&mut context, &view);

    assert_eq!(
        runtime.metrics().read().script_renders(),
        renders,
        "moving a slider must repaint without re-entering the script"
    );
}

/// A `NumberInput` holds no state of its own: the value lives in the same
/// `InputState` a text field uses, and the step, the bounds and the mask are
/// set on that state rather than on the element. What the element carries is
/// the three slots — and all three have to survive into the description,
/// because base's step buttons are unstyled and its frame has no editor.
#[gpui::test]
fn a_number_input_carries_three_slots_over_a_plain_input_state(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { NumberInput, InputState, h_flex } from "gpui-base";

export default class Quantity extends View {
  init() {
    this.state = InputState.new({ value: "1" });
    this.state.set_step(1);
    this.state.set_min(0);
    this.state.set_max(10);
    this.stepped = null;
  }
  render(cx) {
    return NumberInput.new(this.state)
      .decrement_button(h_flex().w(20).child("-"))
      .increment_button(h_flex().w(20).child("+"))
      .input(h_flex().flex_1())
      .controls_right()
      .on_step((action, cx) => { this.stepped = action; cx.notify(); });
  }
}
"#;
    let view_type = runtime.load_source("quantity", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("NumberInput must be a supported component");

    assert!(tree.contains("NumberInput #"), "missing the root: {tree}");
    for slot in ["@input", "@decrement_button", "@increment_button"] {
        assert!(
            tree.contains(slot),
            "a slot element is rendered by the component and never as a child, so {slot} must \
             survive into the description: {tree}"
        );
    }
    assert!(
        tree.contains(":controls_right"),
        "where the buttons go is the script's choice, so it must be described: {tree}"
    );
    assert!(
        tree.contains(":on_step(fn)"),
        "stepping is reported back, so the handler must be described: {tree}"
    );

    // And it has to materialize. The two button slots are replayed onto a
    // `Button` the base layer builds rather than materialized into elements of
    // their own, so a mistake in that replay fails here rather than in a story.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// A code of no cells accepts no keystroke and shows nothing, and a mistyped
/// length is a window laying out a hundred thousand boxes. Base refuses
/// neither, so the binding has to.
#[gpui::test]
fn an_otp_length_outside_the_usable_range_is_refused(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { OtpState } from "gpui-base";

export default class Code extends View {
  init() { this.code = OtpState.new(0); }
  render(cx) { return "unreachable"; }
}
"#;
    let view_type = runtime.load_source("code", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let Err(error) = context.update(|window, cx| runtime.instantiate(&view_type, window, cx))
    else {
        panic!("a code with no cells is not a code");
    };
    assert!(
        error.to_string().contains("between 1 and 64"),
        "the error has to name the range: {error}"
    );
}

/// The one component whose contents the script does not describe.
///
/// The description carries the templates and no digits — that is the first
/// assertion — because a digit described here would be the digit the last
/// script render saw. The second is the consequence: the code changes, the
/// frame after it draws the new cells, and the VM is never entered.
#[gpui::test]
fn an_otp_input_is_styled_by_the_script_and_filled_by_the_shell(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { OtpState, OtpInput, v_flex } from "gpui-base";

export default class Code extends View {
  init(_props, cx) {
    this.code = OtpState.new(6);
    this.code.on("change", (event, cx) => { this.done = true; cx.notify(); });
  }
  render(cx) {
    return v_flex()
      .child(`${this.code.len()} digits`)
      .child(
        OtpInput.new(this.code)
          .flex()
          .gap(8)
          .cell_style((cell) => cell.size(40).border_1().rounded(6))
          .cell_active_style((cell) => cell.border_color(`#2563eb`))
          .caret_style((caret) => caret.w(2).h(18).bg(`#111111`)),
      );
  }
}
"#;
    let view_type = runtime.load_source("otp", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("OtpInput must be a supported component");

    assert!(tree.contains("OtpInput #"), "missing OtpInput: {tree}");
    for template in [":cell_style(", ":cell_active_style(", ":caret_style("] {
        assert!(
            tree.contains(template),
            "the cells are declared as styles, not described as elements: \
             missing {template}: {tree}"
        );
    }
    // The length reached the script, which is what makes a label beside the
    // code possible at all.
    assert!(
        tree.contains("6 digits"),
        "the state must be readable: {tree}"
    );

    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime.clone(), object)));
    draw(&mut context, &view);
    let renders = runtime.metrics().read().script_renders();

    // The claim, end to end. `OtpState` emits `Change` only when the code is
    // complete, so a partial code reaching the screen is exactly the case a
    // script-described cell could not serve.
    let state = runtime
        .entities()
        .first_otp()
        .expect("the script's OTP state");
    context.update(|window, cx| {
        state.update(cx, |state, cx| state.set_value("12", window, cx));
    });
    draw(&mut context, &view);

    assert_eq!(
        runtime.metrics().read().script_renders(),
        renders,
        "a digit landing must repaint without re-entering the script"
    );
}

/// The same claim from the other end: a real keystroke.
///
/// Base owns the key handling and the shell owns the cells, so this is the one
/// test that exercises both halves at once — and it is the case the binding
/// exists for. `OtpState` emits `change` only when the code is complete, so a
/// script asked to describe the cells would have nothing to redraw the first
/// two digits from.
#[gpui::test]
fn typing_into_an_otp_input_reaches_the_state_without_the_script(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { OtpState, OtpInput } from "gpui-base";

export default class Code extends View {
  init() { this.code = OtpState.new(4); }
  render(cx) {
    return OtpInput.new(this.code)
      .flex()
      .gap(8)
      .cell_style((cell) => cell.size(40).border_1());
  }
}
"#;
    let view_type = runtime.load_source("otp-keys", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let (_root, context) = cx.add_window_view(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        let view = cx.new(|_| ScriptView::new(runtime_for_view, object));
        crate::root::ShellRoot::new(view.into(), window, cx)
    });
    context.update(|window, cx| window.draw(cx).clear(cx));

    let state = runtime
        .entities()
        .first_otp()
        .expect("the script's OTP state");
    context.update(|window, cx| {
        state.update(cx, |state, cx| state.focus(window, cx));
    });
    context.update(|window, cx| window.draw(cx).clear(cx));

    let renders = runtime.metrics().read().script_renders();
    context.simulate_keystrokes("1 2");
    context.update(|window, cx| window.draw(cx).clear(cx));

    assert_eq!(
        context.update(|_, cx| state.read(cx).value().to_string()),
        "12",
        "keys typed into the code have to reach the state base owns"
    );
    assert_eq!(
        runtime.metrics().read().script_renders(),
        renders,
        "and drawing them must not need the script"
    );
}

/// `change` reports every edit; `complete` reports the transition to a full
/// code. They are separate because validation and submit affordances need the
/// former while verification requests need the latter.
#[gpui::test]
fn otp_change_and_complete_are_distinct_events(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { OtpState, OtpInput, v_flex } from "gpui-base";

export default class Code extends View {
  init(_props, cx) {
    this.code = OtpState.new(2);
    this.changes = 0;
    this.completes = 0;
    this.code.on("change", (_event, cx) => { this.changes += 1; cx.notify(); });
    this.code.on("complete", (_event, cx) => { this.completes += 1; cx.notify(); });
  }
  render(cx) {
    return v_flex()
      .child(OtpInput.new(this.code).cell_style((cell) => cell.size(40)))
      .child(`changes ${this.changes} completes ${this.completes}`);
  }
}
"#;
    let view_type = runtime.load_source("otp-events.js", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("view")
        .read_with(&context, |root, _| root.0.clone());
    let state = runtime.entities().first_otp().expect("OTP state");
    context.update(|window, cx| state.update(cx, |state, cx| state.focus(window, cx)));
    context.update(|window, cx| window.draw(cx).clear(cx));

    context.simulate_keystrokes("1");
    context.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        context.update(|_, cx| state.read(cx).value().to_string()),
        "1",
        "the real key event must reach the focused OTP state"
    );
    let partial = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        partial.contains("changes 1 completes 0"),
        "a partial edit is a change, not a completion: {partial}"
    );

    context.simulate_keystrokes("2");
    context.update(|window, cx| window.draw(cx).clear(cx));
    let complete = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        complete.contains("changes 2 completes 1"),
        "the final edit reports both semantic events once: {complete}"
    );
}

/// Retained-state setters are controlled shell mutations: native cells repaint
/// and script-derived presentation is invalidated without an extra
/// `cx.notify()`. Registering the same event again replaces the old listener,
/// bounding subscription lifetime even when initialization code is retried.
#[gpui::test]
fn otp_setters_refresh_script_ui_and_same_event_subscription_is_replaced(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { Button, OtpState, OtpInput, v_flex } from "gpui-base";

export default class Code extends View {
  init(_props, cx) {
    this.code = OtpState.new(2);
    this.old_calls = 0;
    this.new_calls = 0;
    this.code.on("change", (_event, cx) => { this.old_calls += 1; cx.notify(); });
    this.code.on("change", (_event, cx) => { this.new_calls += 1; cx.notify(); });
  }
  render(cx) {
    return v_flex()
      .child(Button.new("set-code").w(100).h(40).on_click(() => {
        this.code.set_value("42");
        this.code.set_masked(true);
      }))
      .child(OtpInput.new(this.code).cell_style((cell) => cell.size(40)))
      .child(`value ${this.code.value()} masked ${this.code.is_masked()} old ${this.old_calls} new ${this.new_calls}`);
  }
}
"#;
    let view_type = runtime
        .load_source("otp-controlled.js", source)
        .expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("view")
        .read_with(&context, |root, _| root.0.clone());
    let state = runtime.entities().first_otp().expect("OTP state");
    context.update(|window, cx| state.update(cx, |state, cx| state.focus(window, cx)));
    context.update(|window, cx| window.draw(cx).clear(cx));

    context.simulate_keystrokes("1");
    context.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        context.update(|_, cx| state.read(cx).value().to_string()),
        "1",
        "the real key event must reach the focused OTP state"
    );
    let after_change = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        after_change.contains("value 1 masked false old 0 new 1"),
        "registering the same event replaces the old listener: {after_change}"
    );

    context.simulate_click(point(px(50.), px(20.)), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    let after_set = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        after_set.contains("value 42 masked true old 0 new 1"),
        "rendered-event setters invalidate script-derived UI themselves: {after_set}"
    );
}

/// A progress bar is three parts, and the script draws all of it: the root
/// announces the number and paints nothing, the track and the indicator are the
/// only things a user sees. So the description has to carry both the announced
/// value and the geometry the script computed from it.
#[gpui::test]
fn a_progress_bar_announces_a_value_the_script_draws_itself(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { Progress, ProgressTrack, ProgressIndicator } from "gpui-base";

export default class Download extends View {
  init() { this.percent = 40; }
  render(cx) {
    return Progress.new("download")
      .value(this.percent)
      .accessibility_label("Downloading")
      .child(
        ProgressTrack.new()
          .w(200)
          .h(6)
          .bg(`#e5e7eb`)
          .child(
            ProgressIndicator.new()
              .w(`${this.percent}%`)
              .h(6)
              .bg(`#2563eb`)));
  }
}

"#;
    let view_type = runtime.load_source("progress", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("Progress and its two parts must be supported components");

    assert!(
        tree.contains("Progress \"download\""),
        "missing progress root: {tree}"
    );
    assert!(
        tree.contains(":value[Number(40.0)]"),
        "the announced percentage is controlled, so it must be described: {tree}"
    );
    assert!(
        tree.contains(":accessibility_label[Str(\"Downloading\")]"),
        "the progress name must survive into the description: {tree}"
    );
    assert!(
        tree.contains("ProgressTrack") && tree.contains("ProgressIndicator"),
        "the visible bar is built from the two parts: {tree}"
    );
    // The bar is drawn entirely by the script, so the width it computed is the
    // only thing that can be asserted about the picture.
    assert!(
        tree.contains(".w[Str(\"40%\")]"),
        "the indicator's own width has to survive into the description: {tree}"
    );

    // And the whole thing has to materialize: the two parts are not
    // interactive, so a `finish` that assumed otherwise would fail here rather
    // than in a story.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

#[gpui::test]
fn fps_monitor_is_available_as_a_native_overlay(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { fps_monitor } from "gpui-fps";

export default class Monitor extends View {
  render(cx) {
    return div().relative().size_full().child(
      fps_monitor()
        .anchor("bottom_left")
        .frame_budget(8.33)
    );
  }
}
"#;
    let view_type = runtime.load_source("fps-monitor.js", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let spec = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("render spec");
    assert!(
        spec.contains("FpsMonitor :anchor") && spec.contains(":frame_budget[Number(8.33)]"),
        "the snapshot must retain the native monitor and its performance options: {spec}"
    );

    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// `Radio` and `Toggle` are controlled in the same one-directional way, and
/// asymmetrically: the radio reports only *becoming* chosen, while the toggle
/// reports the value the script would otherwise flip itself. Both directions
/// have to survive into the description.
#[gpui::test]
fn a_radio_group_and_a_toggle_carry_their_controlled_state_both_ways(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { Radio, Toggle, v_flex } from "gpui-base";

export default class Preferences extends View {
  init() { this.appearance = 0; this.bold = false; }
  render(cx) {
    const names = ["Light", "Dark"];
    return v_flex()
      .children(names.map((name, index) =>
        Radio.new(`appearance-${index}`)
          .checked(index === this.appearance)
          .disabled(index === 1)
          .accessibility_label(name)
          .set_position(index + 1, names.length)
          .on_change((_checked, cx) => { this.appearance = index; cx.notify(); })
          .child(name)))
      .child(
        Toggle.new("bold")
          .pressed(this.bold)
          .accessibility_label("Bold")
          .on_change((pressed, cx) => { this.bold = pressed; cx.notify(); })
          .child("B"));
  }
}
"#;
    let view_type = runtime.load_source("radio_toggle", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("Radio and Toggle must be supported components");

    assert!(
        tree.contains("Radio \"appearance-0\""),
        "missing first radio: {tree}"
    );
    assert!(tree.contains("Toggle \"bold\""), "missing toggle: {tree}");
    assert!(
        tree.contains(":checked") && tree.contains(":pressed"),
        "both controlled states must be described: {tree}"
    );
    assert!(
        tree.contains(":set_position"),
        "the announced position must survive into the description: {tree}"
    );
    assert!(
        tree.contains(":on_change"),
        "the reported change must be described: {tree}"
    );

    // And the whole thing has to materialize: both are `Stateful<Div>` under
    // the hood, so a state style with nowhere to land would fail here rather
    // than in a story.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// A table is composed, not driven: the script nests the groups, rows and
/// cells itself, and the one-based indices ride in the constructors because a
/// cell that does not know its column announces itself in the wrong place.
#[gpui::test]
fn a_table_describes_its_shape_and_its_accessibility_indices(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell, TableCaption } from "gpui-base";

export default class Positions extends View {
  init() { this.picked = -1; }
  render(cx) {
    const columns = ["Symbol", "Last"];
    const rows = [["AAPL", "228.52"], ["MSFT", "417.14"]];
    return Table.new("positions")
      .accessibility_label("Open positions")
      .row_count(200)
      .column_count(columns.length)
      .child(TableCaption.new("positions-caption").child("Open positions"))
      .child(
        TableHeader.new("positions-header").child(
          TableRow.new("positions-head-row", 1).children(
            columns.map((name, index) =>
              TableHead.new(`positions-head-${index}`, index + 1).child(name)))))
      .child(
        TableBody.new("positions-body").children(
          rows.map((cells, row) =>
            TableRow.new(`positions-row-${row}`, row + 2)
              .hover((el) => el.bg(`#ffffff`))
              .on_click((_event, cx) => { this.picked = row; cx.notify(); })
              .children(
                cells.map((value, column) =>
                  TableCell.new(`positions-cell-${row}-${column}`, column + 1)
                    .child(value))))));
  }
}
"#;
    let view_type = runtime.load_source("table", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("the Table family must be supported");

    assert!(
        tree.contains("Table \"positions\""),
        "missing table root: {tree}"
    );
    assert!(
        tree.contains(":row_count") && tree.contains(":column_count"),
        "the whole table's size must survive into the description: {tree}"
    );
    assert!(
        tree.contains(":accessibility_label[Str(\"Open positions\")]"),
        "the table name must survive into the description: {tree}"
    );
    // The body's first row is the second row of the table, because the header
    // row is the first — which is exactly the arithmetic an index exists to
    // record, and exactly what a plain nest of divs cannot say.
    assert!(
        tree.contains("TableRow \"positions-row-0\" #2"),
        "a row must carry its one-based index: {tree}"
    );
    assert!(
        tree.contains("TableCell \"positions-cell-0-1\" #2"),
        "a cell must carry its one-based column index: {tree}"
    );
    assert!(
        tree.contains("TableCaption \"positions-caption\""),
        "the caption slot must be described: {tree}"
    );
    assert!(
        tree.contains(":on_click"),
        "a row is where a table's click lands, so the handler must be described: {tree}"
    );

    // And the whole thing has to materialize: every part is a `Stateful<Div>`,
    // so a state style that had nowhere to land would fail here rather than in
    // a story.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// A one-based index is not advisory. Zero is not "close enough" to the first
/// column; it is every cell announced one place to the left, so it is refused
/// where the script wrote it rather than cast into something plausible.
#[gpui::test]
fn a_table_index_below_one_is_refused_at_the_call_site(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { TableCell } from "gpui-base";

export default class BadTable extends View {
  render(cx) { return TableCell.new("cell", 0).child("AAPL"); }
}

"#;
    let view_type = runtime.load_source("bad-table", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a zero column index must fail at the script call site");

    assert!(
        error.to_string().contains("whole number of at least 1"),
        "the error must say what a valid index is: {error}"
    );
}

#[gpui::test]
fn accessibility_counts_and_positions_reject_invalid_numbers(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    for (name, expression, expected) in [
        (
            "position-zero",
            "Tab.new('tab').set_position(0, 2)",
            "set_position",
        ),
        (
            "position-after-size",
            "Tab.new('tab').set_position(3, 2)",
            "set_position",
        ),
        (
            "position-fraction",
            "Tab.new('tab').set_position(1.5, 2)",
            "set_position",
        ),
        (
            "row-negative",
            "Table.new('table').row_count(-1)",
            "row_count",
        ),
        (
            "column-fraction",
            "Table.new('table').column_count(2.5)",
            "column_count",
        ),
        (
            "progress-nan",
            "Progress.new('progress').value(NaN)",
            "value",
        ),
    ] {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let source = format!(
            "import {{ View }} from 'gpui-kit'; import {{ Tab, Table, Progress }} from 'gpui-base'; export default class Bad extends View {{ render() {{ return {expression}; }} }}"
        );
        let view_type = runtime.load_source(name, &source).expect("load");
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let object = context
            .update(|window, cx| runtime.instantiate(&view_type, window, cx))
            .expect("instantiate");

        let error = context
            .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
            .expect_err("an invalid accessibility number must fail at its call site");
        assert!(
            error.to_string().contains(expected),
            "{name} must identify {expected}: {error}"
        );
    }
}

#[gpui::test]
fn motion_rejects_properties_the_native_layer_cannot_interpolate(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";

export default class BadMotion extends View {
  render() {
    return div().id("panel").transition("padding", 120);
  }
}
"#;
    let view_type = runtime.load_source("bad-motion", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("unsupported motion properties must fail at the script call site");

    assert!(
        error
            .to_string()
            .contains("opacity, width, height, left or top"),
        "the error must name the supported native motion properties: {error}"
    );
}

#[gpui::test]
fn spring_declarations_survive_the_script_render(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";

export default class Motion extends View {
  render() {
    return div().id("indicator").left(48).spring("left", {
      response: 250,
      damping: 0.85,
      epsilon: 0.25,
    });
  }
}
"#;
    let view_type = runtime.load_source("spring-motion", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });
    assert!(
        tree.contains(":spring(left, 250ms, 0.85, 0.25)"),
        "the native spring target and policy were not retained in the snapshot: {tree}"
    );
}

#[gpui::test]
fn transition_rejects_an_unknown_native_easing(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
export default class BadMotion extends View {
  render() {
    return div().opacity(0.5).transition("opacity", { duration: 120, easing: "bounce" });
  }
}
"#;
    let view_type = runtime.load_source("bad-easing", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("an easing Rust cannot sample must fail at the call site");
    assert!(
        error
            .to_string()
            .contains("linear, ease-in, ease-out or ease-in-out"),
        "the error must name the snapshot-safe easing values: {error}"
    );
}

#[gpui::test]
fn motion_rejects_non_finite_or_physically_invalid_policies(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    for (name, declaration, expected) in [
        (
            "nan-duration",
            r#"div().opacity(0.5).transition("opacity", { duration: NaN })"#,
            "duration must be a finite non-negative number",
        ),
        (
            "negative-delay",
            r#"div().opacity(0.5).transition("opacity", { duration: 120, delay: -1 })"#,
            "delay must be a finite non-negative number",
        ),
        (
            "negative-damping",
            r#"div().left(20).spring("left", { damping: -0.1 })"#,
            "damping must be a finite non-negative number",
        ),
        (
            "zero-epsilon",
            r#"div().left(20).spring("left", { epsilon: 0 })"#,
            "epsilon must be a finite positive number",
        ),
    ] {
        let source = format!(
            r#"
import {{ View, div }} from "gpui-kit";
export default class BadMotion extends View {{
  render() {{ return {declaration}; }}
}}
"#
        );
        let view_type = runtime.load_source(name, &source).expect("load");
        let object = context
            .update(|window, cx| runtime.instantiate(&view_type, window, cx))
            .expect("instantiate");
        let error = context
            .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
            .expect_err("invalid motion policies must fail at the script call site");
        assert!(
            error.to_string().contains(expected),
            "`{name}` must explain its invalid field: {error}"
        );
    }
}

#[gpui::test]
fn theme_tokens_resolve_outside_a_call_scope(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx);
        gpui_base::Theme::global_mut(cx).tokens.colors.background = gpui::white();
        gpui_base::Theme::global_mut(cx).tokens.colors.primary = gpui::white();
        crate::theme_tokens::sync(cx);
    });

    // Materialization happens after the call scope closes, so a palette that
    // could only be read through the scope resolved every color to `None` and
    // painted an unstyled black window. This is that regression.
    assert!(
        crate::theme_tokens::token_color("background").is_some(),
        "semantic tokens must resolve without an open call scope"
    );
    assert!(crate::theme_tokens::token_color("primary").is_some());
    assert!(crate::theme_tokens::token_color("not_a_token").is_none());
}

#[gpui::test]
fn javascript_can_replace_the_active_gpui_base_theme(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r##"
import { View, div } from "gpui-kit";
import { set_theme } from "gpui-base";
export default class ThemeSwitch extends View {
  init() {
    const color = "#111111";
    set_theme({ appearance: "dark", tokens: {
      colors: {
        background: color, foreground: color, surface: color, surface_foreground: color,
        primary: color, primary_foreground: color, secondary: color, secondary_foreground: color,
        muted: color, muted_foreground: color, accent: color, accent_foreground: color,
        destructive: color, destructive_foreground: color, border: color, input: color, ring: color,
        selection: color,
      },
      spacing: { xxs: 2, xs: 4, sm: 8, md: 12, lg: 16, xl: 24, xxl: 32 },
      radius: { none: 0, sm: 3, md: 6, lg: 8, xl: 12, full: 9999 },
    }});
  }
  render(cx) { return div(); }
}
"##;
    let view_type = runtime.load_source("theme-switch", source).expect("load");
    let loaded = runtime.clone();
    let window = cx.add_window(move |window, cx| {
        let object = loaded.instantiate(&view_type, window, cx).unwrap();
        ScriptView::new(loaded, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));

    context.update(|_, cx| {
        assert_eq!(
            gpui_base::Theme::global(cx).appearance,
            gpui_base::ThemeAppearance::Dark
        );
    });
}

/// A script states its type scale, and the chrome the shell draws follows it.
///
/// The regression this guards is a dense application with oversized
/// notifications: everything `ShellRoot` draws for itself states no text size
/// and inherited GPUI's 16px default, while the components an application
/// builds set their own — so a window drawn at 12px got 16px toasts over it.
/// `md` is the base, and it is an override: a theme that says nothing about
/// type keeps the scale it was given.
#[gpui::test]
fn javascript_can_state_the_type_scale(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r##"
import { View, div } from "gpui-kit";
import { set_theme } from "gpui-base";
const color = "#111111";
const base = {
  colors: {
    background: color, foreground: color, surface: color, surface_foreground: color,
    primary: color, primary_foreground: color, secondary: color, secondary_foreground: color,
    muted: color, muted_foreground: color, accent: color, accent_foreground: color,
    destructive: color, destructive_foreground: color, border: color, input: color, ring: color,
    selection: color,
  },
  spacing: { xxs: 2, xs: 4, sm: 8, md: 12, lg: 16, xl: 24, xxl: 32 },
  radius: { none: 0, sm: 3, md: 6, lg: 8, xl: 12, full: 9999 },
};
export default class TypeScale extends View {
  init() {
    // Only what this application has an opinion about. The line height, the
    // weight and both font families are left as they are.
    set_theme({ appearance: "light", tokens: { ...base, typography: { md: { size: 12 } } } });
  }
  render(cx) { return div(); }
}
"##;
    let view_type = runtime.load_source("type-scale", source).expect("load");
    let loaded = runtime.clone();
    let window = cx.add_window(move |window, cx| {
        let object = loaded.instantiate(&view_type, window, cx).unwrap();
        ScriptView::new(loaded, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));

    context.update(|_, cx| {
        let typography = gpui_base::Theme::global(cx).tokens.typography.clone();
        assert_eq!(
            typography.md.size,
            gpui::px(12.),
            "the size the script stated is the one the theme carries"
        );
        assert_eq!(
            typography.md.line_height,
            gpui_base::TypographyTokens::default().md.line_height,
            "and an entry it left out keeps the value it had"
        );
        assert_eq!(
            typography.sans,
            gpui_base::TypographyTokens::default().sans,
            "including the font families, which it never mentioned"
        );
        assert_eq!(
            typography.sm.size,
            gpui_base::TypographyTokens::default().sm.size,
            "and the steps beside the one it named"
        );
    });
}

/// A size that cannot be drawn is refused rather than applied.
#[gpui::test]
fn a_type_scale_of_zero_is_refused(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r##"
import { View, div } from "gpui-kit";
import { set_theme } from "gpui-base";
const color = "#111111";
export default class ZeroScale extends View {
  init() {
    this.refused = "";
    try {
      set_theme({ appearance: "light", tokens: {
        colors: {
          background: color, foreground: color, surface: color, surface_foreground: color,
          primary: color, primary_foreground: color, secondary: color, secondary_foreground: color,
          muted: color, muted_foreground: color, accent: color, accent_foreground: color,
          destructive: color, destructive_foreground: color, border: color, input: color,
          ring: color, selection: color,
        },
        spacing: { xxs: 2, xs: 4, sm: 8, md: 12, lg: 16, xl: 24, xxl: 32 },
        radius: { none: 0, sm: 3, md: 6, lg: 8, xl: 12, full: 9999 },
        typography: { md: { size: 0 } },
      }});
    } catch (error) {
      this.refused = String(error && error.message ? error.message : error);
    }
  }
  render(cx) { return div().child(this.refused || "applied"); }
}
"##;
    let view_type = runtime.load_source("zero-scale", source).expect("load");
    let loaded = runtime.clone();
    let window = cx.add_window(move |window, cx| {
        let object = loaded.instantiate(&view_type, window, cx).unwrap();
        ScriptView::new(loaded, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));

    context.update(|_, cx| {
        assert_eq!(
            gpui_base::Theme::global(cx).tokens.typography.md.size,
            gpui_base::TypographyTokens::default().md.size,
            "a refused scale leaves the one that was working alone"
        );
    });
}

/// The todo list exists to exercise the whole runtime at once: retained input
/// state, controlled checkboxes, a dialog, a toast, capability-gated storage,
/// and a filter that must survive every mutation. If a subsystem regresses,
/// this is the test that notices.
#[gpui::test]
fn the_todolist_example_exercises_the_runtime(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/js_todolist")
        .canonicalize()
        .expect("example directory");

    let view_type = runtime
        .load_app(&directory, "main.js")
        .expect("load example");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    // `init` creates retained state, so instantiation needs a live host call.
    let object = context.update(|window, cx| runtime.instantiate(&view_type, window, cx));
    let object = object.expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    for expected in [
        "Todo",             // the object
        "Input",            // retained state reached the description
        "\"Add\"",          // the action that creates work
        "No items yet",     // the empty state explains the next step
        "Clear completed…", // an ellipsis, because it opens a dialog
    ] {
        assert!(
            tree.contains(expected),
            "todolist is missing `{expected}`:\n{tree}"
        );
    }
}

/// Multi-line state is a different Rust type from single-line state, so the
/// seams that could confuse the two — the store, the element, the subscription
/// — have to be exercised in the same view as an ordinary input. The row count
/// is part of that: the layout default is one row even for a textarea, so a
/// binding that dropped `rows` would produce something shaped like an input.
#[gpui::test]
fn a_textarea_holds_multi_line_state_beside_an_input(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { Input, InputState, Textarea, TextareaState } from "gpui-base";

export default class Note extends View {
  init() {
    this.title = InputState.new({ value: "Shopping" });
    this.body = TextareaState.new({ placeholder: "Notes", value: "milk", rows: 6 });
    this.body.on("change", () => {});
    this.body.set_soft_wrap(true);
    this.body.set_auto_grow(3, 12);
  }
  render(cx) {
    return div()
      .child(Input.new(this.title))
      .child(Textarea.new(this.body).h(160))
      .child(this.body.value());
  }
}
"#;
    let view_type = runtime.load_source("note", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("Textarea must be a supported component");

    assert!(tree.contains("Textarea #"), "missing the textarea: {tree}");
    assert!(
        tree.contains("Input #"),
        "the single-line input must still be its own component: {tree}"
    );
    assert!(
        tree.contains("\"milk\""),
        "the retained text must be readable from the script: {tree}"
    );

    // And both have to materialize. A textarea handle that resolved as an input
    // — or the other way round — would fail here rather than in a story.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

#[gpui::test]
fn an_unknown_input_event_names_the_valid_ones(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, div } from "gpui-kit";
import { InputState } from "gpui-base";

export default class Bad extends View {
  init() {
    this.field = InputState.new({});
    this.field.on("entered", () => {});
  }
  render(cx) {
    return div();
  }
}
"#;

    let view_type = runtime.load_source("bad", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let error = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect_err("an unknown event name must fail");

    assert!(
        error.to_string().contains("submit"),
        "the error should list the valid events, got: {error}"
    );
}

/// Hot reload has to pick up a change in an imported module, not only in the
/// entry point. QuickJS caches an evaluated module by name and an ES module
/// cannot be unloaded, so a naive reload re-evaluates `main.js` against the
/// first version of everything it imports — and looks like it worked.
#[gpui::test]
fn a_reload_picks_up_a_change_in_an_imported_module(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory = std::env::temp_dir().join(format!("gpui-shell-reload-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("temp directory");

    std::fs::write(
        directory.join("main.js"),
        r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { caption } from "./caption.js";

export default class Reloading extends View {
  render(cx) {
    return v_flex().child(caption());
  }
}
"#,
    )
    .expect("write main");
    std::fs::write(
        directory.join("caption.js"),
        "export const caption = () => \"before\";\n",
    )
    .expect("write caption");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let render = |context: &mut VisualTestContext| {
        let view_type = runtime.load_app(&directory, "main.js").expect("load");
        let object = context
            .update(|window, cx| runtime.instantiate(&view_type, window, cx))
            .expect("instantiate");
        context.update(|window, cx| {
            runtime
                .render_to_spec(&object, None, window, cx)
                .expect("render")
        })
    };

    assert!(render(&mut context).contains("before"));

    std::fs::write(
        directory.join("caption.js"),
        "export const caption = () => \"after\";\n",
    )
    .expect("rewrite caption");

    let reloaded = render(&mut context);
    assert!(
        reloaded.contains("after"),
        "the imported module was served from the cache:\n{reloaded}"
    );

    std::fs::remove_dir_all(&directory).ok();
}

#[gpui::test]
fn oversized_entry_and_imported_modules_are_refused(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    let directory =
        std::env::temp_dir().join(format!("gpui-shell-module-limit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("application directory");

    let entry = directory.join("main.js");
    let file = std::fs::File::create(&entry).expect("entry module");
    file.set_len(8 * 1024 * 1024 + 1).expect("sparse entry");
    let error = runtime
        .load_app(&directory, "main.js")
        .expect_err("oversized entry module must fail");
    assert!(error.to_string().contains("module") && error.to_string().contains("limit"));

    std::fs::write(&entry, "import './huge.js'; export default class Panel {};")
        .expect("entry module");
    let imported = std::fs::File::create(directory.join("huge.js")).expect("imported module");
    imported
        .set_len(8 * 1024 * 1024 + 1)
        .expect("sparse import");
    let error = runtime
        .load_app(&directory, "main.js")
        .expect_err("oversized imported module must fail");
    assert!(error.to_string().contains("module") && error.to_string().contains("limit"));
    let _ = std::fs::remove_dir_all(directory);
}

/// An embedded runtime reloads on a save, with no host doing anything but
/// asking for it once.
///
/// The binary has `--watch` because the person running it is the person
/// editing. A host that embeds the runtime has no flag to offer, so a debug
/// build simply *is* the development build — and this is the test that says so,
/// since the behaviour is otherwise invisible until someone saves a file.
#[gpui::test]
fn an_embedded_runtime_reloads_when_a_source_changes(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory = std::env::temp_dir().join(format!("gpui-shell-watch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a temporary application");
    let source = |caption: &str| {
        format!(
            "import {{ div, View }} from \"gpui-kit\";\n\
             import {{ v_flex }} from \"gpui-base\";\n\
             export default class Panel extends View {{\n\
               render() {{ return v_flex().child(\"{caption}\"); }}\n\
             }}\n"
        )
    };
    std::fs::write(directory.join("main.js"), source("before")).expect("writing main.js");

    let view_type = runtime.load_app(&directory, "main.js").expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let view = context.update(|window, cx| {
        let object = runtime
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        let view = cx.new(|_| ScriptView::new(runtime.clone(), object));
        // Exercise the watcher itself in both debug and release test builds.
        let watch =
            crate::watch::Watcher::start(&runtime, &view, directory.clone(), "main.js", window, cx)
                .expect("watch");
        watch.forget();
        view
    });

    let description = |context: &mut VisualTestContext| {
        context.update(|_, cx| {
            view.read(cx)
                .snapshot()
                .map(crate::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    };

    draw(&mut context, &view);
    assert!(description(&mut context).contains("before"));

    // The watcher compares modification stamps, so the file has to look older
    // than the write that follows it — a test that runs inside one filesystem
    // tick would otherwise see nothing change.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(directory.join("main.js"), source("after")).expect("rewriting main.js");

    // Two polls, with real time in between. The poll interval is on the
    // executor's clock, which `advance_clock` moves; the debounce is measured
    // against the wall, because it is absorbing a burst of saves from an editor
    // rather than counting frames. So the first poll notices the change and the
    // second one — after the tree has been still for the debounce window —
    // reports it.
    let settle = |context: &mut VisualTestContext| {
        context
            .executor()
            .advance_clock(crate::watch::POLL_INTERVAL * 2);
        context.run_until_parked();
    };
    settle(&mut context);
    std::thread::sleep(std::time::Duration::from_millis(250));
    settle(&mut context);

    draw(&mut context, &view);

    assert!(
        description(&mut context).contains("after"),
        "a saved change should have reached the view without anyone asking: {}",
        description(&mut context)
    );

    let _ = std::fs::remove_dir_all(&directory);
}

/// Replacement init runs before the root swaps its object, so nested creation
/// must take provenance from the replacement object rather than the old root.
/// Committing the reload retires the old child's snapshots and token alias.
#[gpui::test]
fn hot_reload_keeps_replacement_children_and_retires_old_snapshots_and_aliases(
    cx: &mut TestAppContext,
) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let directory =
        std::env::temp_dir().join(format!("gpui-shell-nested-reload-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("application directory");
    let source = |caption: &str, probe_stale_token: bool| {
        let probe = if probe_stale_token {
            "this.probe = []; try { globalThis.__view_set_props(0, {}); } catch (_) { this.probe.push('update-refused'); } try { globalThis.__view_release(0); } catch (_) { this.probe.push('release-refused'); }"
        } else {
            "this.probe = [];"
        };
        format!(
            "import {{ div, View }} from \"gpui-kit\";\n\
             import {{ v_flex }} from \"gpui-base\";\n\
             class Child extends View {{ render() {{ return \"{caption}\"; }} }}\n\
             export default class Parent extends View {{\n\
               init(_props, cx) {{ this.child = cx.new(Child); {probe} }}\n\
               render(cx) {{ if ({probe_stale_token}) {{ try {{ globalThis.__child_view(0); }} catch (_) {{ this.probe.push('mount-refused'); }} }} return v_flex().child(this.child).child(this.probe.join(',')); }}\n\
             }}\n"
        )
    };
    std::fs::write(directory.join("main.js"), source("old child", false)).expect("initial source");

    let view_type = runtime.load_app(&directory, "main.js").expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let parent = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate parent");
    draw(&mut context, &parent);
    let mounted_child = |context: &mut VisualTestContext| {
        context.update(|_, cx| {
            let snapshot = parent.read(cx).snapshot().expect("parent snapshot");
            (0..snapshot.len() as u32)
                .filter_map(|id| snapshot.arena().node(id))
                .find_map(|node| match node.component() {
                    Some(crate::spec::Component::ChildView(child)) => Some(child.view().clone()),
                    _ => None,
                })
                .expect("mounted child")
        })
    };
    let old_child = mounted_child(&mut context);
    assert_eq!(runtime.entities().len(), 1);
    assert_eq!(runtime.nested_view_alias_count(), 1);

    std::fs::write(directory.join("main.js"), source("replacement child", true))
        .expect("replacement source");
    context
        .update(|window, cx| {
            crate::watch::reload(&runtime, &parent, &directory, "main.js", window, cx)
        })
        .expect("reload");

    assert!(
        context.update(|_, cx| old_child.read(cx).snapshot().is_none()),
        "application retirement left the old mounted child snapshot live"
    );
    assert_eq!(
        runtime.entities().len(),
        1,
        "only the replacement child lives"
    );
    assert_eq!(
        runtime.nested_view_alias_count(),
        1,
        "the old opaque token alias survived application release"
    );

    draw(&mut context, &parent);
    let replacement = mounted_child(&mut context);
    let tree = context.update(|_, cx| {
        replacement
            .read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(tree.contains("replacement child"), "{tree}");
    let parent_tree = context.update(|_, cx| {
        parent
            .read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(parent_tree.contains("update-refused"), "{parent_tree}");
    assert!(parent_tree.contains("release-refused"), "{parent_tree}");
    assert!(parent_tree.contains("mount-refused"), "{parent_tree}");
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui::test]
fn reload_replaces_old_tasks_and_rolls_back_failed_new_tasks(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    let directory =
        std::env::temp_dir().join(format!("gpui-shell-reload-tasks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("application directory");

    let source = |caption: &str| {
        format!(
            "import {{ div, View }} from \"gpui-kit\";\n\
             export default class Panel extends View {{\n\
               init(_props, cx) {{ cx.timer.every(60_000, () => {{}}); }}\n\
               render(cx) {{ return \"{caption}\"; }}\n\
             }}\n"
        )
    };
    std::fs::write(directory.join("main.js"), source("first")).expect("initial source");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let baseline = crate::engine::quickjs::task_count();
    let view = context.update(|window, cx| {
        let policy = Rc::new(Policy::default());
        let (_scope, _) = crate::scope::enter_with_runtime(
            &runtime,
            window,
            cx,
            crate::scope::ScopePhase::Task,
            None,
            policy.clone(),
        );
        let view_type = runtime.load_app(&directory, "main.js").expect("load");
        runtime
            .instantiate_view_with_policy(&view_type, policy, window, cx)
            .expect("instantiate")
    });
    assert_eq!(crate::engine::quickjs::task_count(), baseline + 1);

    std::fs::write(directory.join("main.js"), source("second")).expect("replacement source");
    context
        .update(|window, cx| {
            crate::watch::reload(&runtime, &view, &directory, "main.js", window, cx)
        })
        .expect("successful reload");
    assert_eq!(
        crate::engine::quickjs::task_count(),
        baseline + 1,
        "the old instance's timer must be retired when the new one commits"
    );

    std::fs::write(
        directory.join("main.js"),
        "import { with_cx } from \"gpui-kit\";\n\
         cx.timer.every(60_000, () => {});\n\
         throw new Error(\"reload failed\");",
    )
    .expect("failing source");
    context
        .update(|window, cx| {
            crate::watch::reload(&runtime, &view, &directory, "main.js", window, cx)
        })
        .expect_err("the replacement must fail");
    assert_eq!(
        crate::engine::quickjs::task_count(),
        baseline + 1,
        "work created by a failed reload must be rolled back"
    );

    std::fs::write(
        directory.join("main.js"),
        "import { View } from \"gpui-kit\";\n\
         export default class Broken extends View {\n\
           init(_props, cx) { cx.timer.every(60_000, () => {}); throw new Error(\"init failed\"); }\n\
           render(cx) { return \"unreachable\"; }\n\
         }",
    )
    .expect("initialization-failing source");
    context
        .update(|window, cx| {
            crate::watch::reload(&runtime, &view, &directory, "main.js", window, cx)
        })
        .expect_err("initialization failure must roll back the candidate generation");
    assert_eq!(
        crate::engine::quickjs::task_count(),
        baseline + 1,
        "work created by a candidate init must be rolled back without touching the live app"
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[gpui::test]
fn reload_evaluates_modules_under_the_views_frozen_capabilities(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let observed = Rc::new(Cell::new(false));
    crate::export_module(HostModule::new("audit").function("observe", {
        let observed = observed.clone();
        move |_| {
            observed.set(crate::scope::policy().capabilities().has_read_access());
            Ok(HostValue::from(true))
        }
    }))
    .expect("`audit` is not a reserved name");

    let directory =
        std::env::temp_dir().join(format!("gpui-shell-reload-policy-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("application directory");
    let source = |caption: &str| {
        format!(
            "import {{ div, View }} from \"gpui-kit\";\n\
             import {{ observe }} from \"audit\";\n\
             observe();\n\
             export default class Panel extends View {{\n\
               render() {{ return \"{caption}\"; }}\n\
             }}"
        )
    };
    std::fs::write(directory.join("main.js"), source("first")).expect("initial source");

    crate::set_capabilities(Capabilities::new().read_roots([directory.clone()]));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context.update(|window, cx| {
        let view_type = runtime.load_app(&directory, "main.js").expect("load");
        runtime
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate")
    });

    crate::set_capabilities(Capabilities::new());
    observed.set(false);
    std::fs::write(directory.join("main.js"), source("second")).expect("replacement source");
    context
        .update(|window, cx| {
            crate::watch::reload(&runtime, &view, &directory, "main.js", window, cx)
        })
        .expect("reload");
    assert!(
        observed.get(),
        "module evaluation must keep the view's frozen capability grant"
    );

    crate::clear_exported_modules();
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui::test]
fn loading_a_second_application_keeps_the_first_dynamic_import_root(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    let base = std::env::temp_dir().join(format!("gpui-shell-multi-root-{}", std::process::id()));
    let first = base.join("first");
    let second = base.join("second");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&first).expect("first application");
    std::fs::create_dir_all(&second).expect("second application");
    std::fs::write(
        first.join("feature.js"),
        "export const label = 'first feature';",
    )
    .expect("first feature");
    std::fs::write(
        first.join("main.js"),
        "import { View } from \"gpui-kit\";\n\
         export default class First extends View {\n\
           init(_props, cx) {\n\
             this.label = 'waiting';\n\
             cx.spawn(async (cx) => {\n\
               await cx.sleep(1);\n\
               this.label = (await import('./feature.js')).label;\n\
               cx.notify();\n\
             });\n\
           }\n\
           render(cx) { return this.label; }\n\
         }",
    )
    .expect("first entry");
    std::fs::write(
        second.join("main.js"),
        "import { View } from \"gpui-kit\";\n\
         export default class Second extends View { render() { return 'second'; } }",
    )
    .expect("second entry");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let first_view = context.update(|window, cx| {
        let view_type = runtime.load_app(&first, "main.js").expect("load first");
        runtime
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate first")
    });
    context.update(|window, cx| {
        let view_type = runtime.load_app(&second, "main.js").expect("load second");
        runtime
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate second")
    });

    context
        .executor()
        .advance_clock(std::time::Duration::from_millis(2));
    context.run_until_parked();
    draw(&mut context, &first_view);
    let tree = context.update(|_, cx| {
        first_view
            .read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("first feature"),
        "the first application's lazy import used the wrong root: {tree}"
    );

    let _ = std::fs::remove_dir_all(base);
}

/// An unknown bare specifier says which built-ins this runtime has.
///
/// A script written against a different version of the runtime fails here, and
/// "cannot resolve module `gpui-base`" alone does not say whether the name is a
/// typo or the binary is older than the application it is loading.
#[gpui::test]
fn an_unknown_built_in_module_names_the_ones_that_exist(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory =
        std::env::temp_dir().join(format!("gpui-shell-unknown-module-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("application directory");
    std::fs::write(
        directory.join("main.js"),
        "import { View } from \"gpui-kit\";\n\
         import { Button } from \"gpui-future\";\n\
         export default class Panel extends View { render() { return Button.new(\"x\"); } }\n",
    )
    .expect("main.js");

    let error = runtime
        .load_app(&directory, "main.js")
        .expect_err("an unknown built-in must be refused");
    let message = error.to_string();
    for expected in [
        "`gpui-kit`",
        "`gpui-base`",
        "`gpui-fps`",
        "different versions",
    ] {
        assert!(
            message.contains(expected),
            "the refusal must name {expected}: {message}"
        );
    }

    let _ = std::fs::remove_dir_all(directory);
}

/// `.child()` names the mistake it was given.
///
/// Every retained handle in this API is a `{__handle}` wrapper, so a focus
/// handle reaching `.child()` used to arrive as an undefined element id and
/// fail as a numeric conversion error naming nothing.
#[gpui::test]
fn child_names_what_it_will_not_accept(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let view_type = runtime
        .load_source(
            "wrong-child.js",
            r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Wrong extends View {
  init(_props, cx) {
    this.focus = cx.focus_handle();
  }
  render(cx) {
    return v_flex().child(this.focus);
  }
}
"#,
        )
        .expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a focus handle is not a child");
    let message = error.to_string();
    assert!(
        message.contains("element, a string, or an entity"),
        "the refusal must say what it wanted: {message}"
    );
}

/// Work starts where the context is, which is `init`.
///
/// A module's top level has no `cx` and no way to get one, and that is the
/// point rather than a gap: GPUI has no module top level either, and work there
/// would belong to no view — nothing would own it, and nothing would cancel it.
#[gpui::test]
fn work_starts_from_init_where_the_context_is(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory =
        std::env::temp_dir().join(format!("gpui-shell-top-level-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("application directory");
    std::fs::write(
        directory.join("main.js"),
        r#"
import { View } from "gpui-kit";

export default class Panel extends View {
  init(_props, cx) {
    this.task = cx.spawn(async () => {});
  }
  render() {
    return "started";
  }
}
"#,
    )
    .expect("main.js");

    let before = crate::engine::quickjs::task_count();
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| {
        let policy = Rc::new(Policy::default());
        let (_scope, _) = crate::scope::enter_with_runtime(
            &runtime,
            window,
            cx,
            crate::scope::ScopePhase::Task,
            None,
            policy.clone(),
        );
        let view_type = runtime.load_app(&directory, "main.js").expect("load");
        runtime
            .instantiate_view_with_policy(&view_type, policy, window, cx)
            .expect("instantiate")
    });

    assert_eq!(
        crate::engine::quickjs::task_count(),
        before + 1,
        "work started from init must reach the scheduler"
    );

    let _ = std::fs::remove_dir_all(directory);
}

/// The claim the async `cx` exists to make: the context a task body was handed
/// still works after an `await`.
///
/// It used to name the call that *started* the task, and that call had returned
/// by the time the continuation ran — so every resumed line had to reach for
/// `with_cx` to get a usable one. The context now names no frame, so there is
/// nothing to go stale.
#[gpui::test]
fn a_task_context_still_works_after_an_await(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let view_type = runtime
        .load_source(
            "async-context.js",
            r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Panel extends View {
  init(_props, cx) {
    this.label = "waiting";
    cx.spawn(async (cx) => {
      await cx.sleep(1);
      // Both of these are the `cx` this body was handed, used well after the
      // call that handed it over returned.
      this.label = `resumed during ${cx.phase()}`;
      cx.notify();
    });
  }
  render(cx) {
    return v_flex().child(this.label);
  }
}
"#,
        )
        .expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    // `instantiate_view` rather than `instantiate`: the latter opens its scope
    // with no view attached, so a task started from `init` has no owner and its
    // `cx.notify()` reaches nothing.
    let view = context.update(|window, cx| {
        runtime
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate")
    });

    draw(&mut context, &view);
    context
        .executor()
        .advance_clock(std::time::Duration::from_millis(5));
    context.run_until_parked();
    draw(&mut context, &view);

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("resumed during task"),
        "a task context must survive its await and report the live phase: {tree}"
    );
}

fn draw(context: &mut VisualTestContext, view: &gpui::Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| gpui::IntoElement::into_any_element(view),
    );
}

/// A grouping container carries the group semantics and nothing else: the state
/// stays on the children, and `axis` is announced rather than laid out.
#[gpui::test]
fn a_group_announces_its_axis_without_laying_its_children_out(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { RadioGroup, ToggleGroup, Checkbox, Button } from "gpui-base";

export default class Preferences extends View {
  init() { this.density = 0; this.bold = false; }
  render(cx) {
    const densities = ["Compact", "Comfortable"];
    return RadioGroup.new("density")
      .axis("vertical")
      .flex()
      .flex_col()
      .children(
        densities.map((name, index) =>
          Checkbox.new(`density-${index}`)
            .checked(index === this.density)
            .accessibility_label(name)
            .on_change((_checked, cx) => { this.density = index; cx.notify(); })
            .child(name)))
      .child(
        ToggleGroup.new("formatting")
          .axis("horizontal")
          .flex()
          .child(
            Button.new("bold")
              .selected(this.bold)
              .on_click((_event, cx) => { this.bold = !this.bold; cx.notify(); })
              .child("Bold")));
  }
}
"#;
    let view_type = runtime.load_source("groups", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("RadioGroup and ToggleGroup must be supported components");

    assert!(
        tree.contains("RadioGroup \"density\""),
        "missing radio group: {tree}"
    );
    assert!(
        tree.contains("ToggleGroup \"formatting\""),
        "missing toggle group: {tree}"
    );
    assert!(
        tree.contains(":axis"),
        "the announced orientation must survive into the description: {tree}"
    );
    // The layout is the script's, not the axis's: a group that says
    // `axis("vertical")` still has to say `flex_col()` to stack.
    assert!(
        tree.contains(".flex_col"),
        "axis must not stand in for layout: {tree}"
    );

    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// The axis grammar mirrors `gpui::Axis`, so an unknown value is a script error
/// rather than a silent fallback to the container's default.
#[gpui::test]
fn an_unknown_axis_is_rejected_at_the_call_site(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { ToggleGroup } from "gpui-base";

export default class BadAxis extends View {
  render(cx) {
    return ToggleGroup.new("formatting").axis("inline");
  }
}
"#;
    let view_type = runtime.load_source("bad-axis", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("an unknown axis must fail at the script call site");

    assert!(
        error.to_string().contains("horizontal or vertical"),
        "the error must name the legal axes: {error}"
    );
}

/// The source both slot tests render, with the open state substituted in.
///
/// The content is a click target rather than a piece of text because the gate
/// is a *render* decision: the description carries the content either way, so
/// only something that has to exist on screen to work can tell the two apart.
const COLLAPSIBLE: &str = r#"
import { View, div } from "gpui-kit";
import { v_flex, Collapsible } from "gpui-base";

export default class Section extends View {
  init() { this.open = OPEN; this.hits = 0; }

  render(cx) {
    return v_flex()
      .w(300)
      .h(200)
      .child(
        Collapsible.new()
          .flex_col()
          .w(300)
          .open(this.open)
          .child(div().w_full().h(40).child("Header"))
          .content(
            div()
              .id("body")
              .w_full()
              .h(40)
              .on_click((_event, cx) => { this.hits += 1; cx.notify(); })
              .child(`Body: ${this.hits}`),
          ),
      );
  }
}
"#;

/// Renders the collapsible source with the given open state, clicks where the
/// content sits when it is drawn, and returns the description that came out.
fn collapsible_tree(cx: &mut TestAppContext, open: bool) -> String {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = COLLAPSIBLE.replace("OPEN", if open { "true" } else { "false" });
    let view_type = runtime.load_source("section.js", &source).expect("load");

    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    // The header occupies the first 40 pixels, so this lands on the content
    // when there is one and on nothing at all when there is not.
    context.simulate_click(point(px(10.), px(60.)), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));

    let view = window.root(&mut context).expect("view");
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    })
}

/// An open `Collapsible` draws the element in its `content` slot, and the slot
/// reaches the description as a slot rather than as a child.
#[gpui::test]
fn an_open_collapsible_renders_the_element_in_its_content_slot(cx: &mut TestAppContext) {
    let tree = collapsible_tree(cx, true);

    assert!(tree.contains("Collapsible"), "missing collapsible: {tree}");
    assert!(
        tree.contains(":open[Bool(true)]"),
        "the open state is controlled, so it must be described: {tree}"
    );
    // Filling a slot detaches the element from `children`, so a dump that
    // walked children alone would lose it — which is why the slot is printed
    // under the node that holds it.
    assert!(
        tree.contains("@content"),
        "the content must be described as a slot: {tree}"
    );
    assert!(
        tree.contains("text \"Body: 1\""),
        "an open collapsible must draw its content, so the click must land: {tree}"
    );
}

/// A closed one describes the same content and draws none of it.
#[gpui::test]
fn a_closed_collapsible_describes_its_content_without_drawing_it(cx: &mut TestAppContext) {
    let tree = collapsible_tree(cx, false);

    assert!(
        tree.contains(":open[Bool(false)]"),
        "the closed state must be described: {tree}"
    );
    // The description is open-agnostic: `open` gates what is rendered, not what
    // the script said. The header proves the collapsible itself is on screen.
    assert!(
        tree.contains("@content"),
        "a closed collapsible still describes its content: {tree}"
    );
    assert!(
        tree.contains("text \"Header\""),
        "ordinary children are drawn either way: {tree}"
    );
    assert!(
        !tree.contains("text \"Body: 1\""),
        "a closed collapsible draws no content, so there is nothing there to click: {tree}"
    );
}

/// Filling a slot consumes the element exactly as adding it to a parent does.
///
/// The error has to say so in words that fit a slot: the same check also guards
/// a state style's declarations, and a script that reused a collapsible's
/// content used to be told it was holding a state style.
#[gpui::test]
fn an_element_given_to_a_slot_cannot_also_be_a_child(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex, Collapsible } from "gpui-base";

export default class Reused extends View {
  render(cx) {
    const body = div().child("body");
    return v_flex().child(Collapsible.new().open(true).content(body)).child(body);
  }
}
"#;
    let view_type = runtime.load_source("reused-slot", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("an element given to a slot must not also be a child");

    let message = error.to_string();
    assert!(
        message.contains("named slot such as content"),
        "the error must name the reason the element is gone: {message}"
    );
    assert!(
        !message.contains("this element holds the declarations of a state style"),
        "a slot is not a state style, and the error must not say it is: {message}"
    );
}

/// A granted `process.exit` reaches the host, with the code the script asked
/// for.
///
/// The request used to be written into a cell no production code read: the
/// script got a success and the window stayed open. So the test is not "the
/// flag was set" but "the host was told", which is the only version of this
/// that can go wrong quietly.
#[gpui::test]
fn a_granted_exit_reaches_the_host(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    crate::set_capabilities(crate::Capabilities::new().exit(true));

    let asked: std::rc::Rc<std::cell::Cell<Option<i32>>> = Default::default();
    let recorded = asked.clone();
    crate::on_exit_request(move |request, _, _| recorded.set(Some(request.code())));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Quitter extends View {
  init() {
    process.exit(7);
  }

  render(cx) {
    return v_flex().child("still here");
  }
}
"#;
    let view_type = runtime.load_source("quitter.js", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    assert_eq!(
        asked.get(),
        Some(7),
        "the host was never told the script asked to exit"
    );

    crate::clear_exit_handler();
}

/// A watcher does not keep its view alive, and stops when the view goes.
///
/// The loop polls every quarter second for the life of the window. Holding the
/// view strongly would mean a panel removed from a dock is never dropped — the
/// runtime it points at is never dropped either — and the poller goes on stating
/// a directory for a panel nobody can see. Mount and unmount a few and they
/// accumulate.
#[gpui::test]
fn a_watcher_releases_its_view(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory = std::env::temp_dir().join(format!("gpui-shell-release-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a temporary application");
    std::fs::write(
        directory.join("main.js"),
        "import { View } from \"gpui-kit\";\n\
         import { v_flex } from \"gpui-base\";\n\
         export default class Panel extends View { render(cx) { return v_flex(); } }\n",
    )
    .expect("writing main.js");

    let view_type = runtime.load_app(&directory, "main.js").expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let weak = context.update(|window, cx| {
        let object = runtime
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        let view = cx.new(|_| ScriptView::new(runtime.clone(), object));
        let watch =
            crate::watch::Watcher::start(&runtime, &view, directory.clone(), "main.js", window, cx)
                .expect("watch");
        watch.forget();
        view.downgrade()
    });

    // Nothing else is holding it: the panel it stood for has been removed.
    context
        .executor()
        .advance_clock(crate::watch::POLL_INTERVAL * 2);
    context.run_until_parked();

    assert!(
        weak.upgrade().is_none(),
        "the watcher is still holding the view it was watching for"
    );

    let _ = std::fs::remove_dir_all(&directory);
}

/// Two runtimes coexist on one thread, each under its own authority.
///
/// This used to be refused: the grant and the store were thread state, so a
/// second runtime would silently run under the first one's permissions. They now
/// live on a `Policy` that travels on the call frame, so the two cannot collide
/// and the refusal has nothing left to protect.
#[gpui::test]
fn two_runtimes_share_a_thread_without_sharing_a_grant(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let _first = ShellRuntime::new_isolated().expect("the first runtime");
    let _second = ShellRuntime::new_isolated().expect("the second runtime");
}

/// A scope opened under a policy answers `fs` with *that* grant.
///
/// This is the seam every capability check goes through, and the half of the
/// P0 fix that the scheduler's capture relies on: a task that kept its policy
/// is only correct if restoring that policy actually changes what the engine
/// sees.
#[gpui::test]
fn a_scope_answers_with_the_policy_it_was_opened_under(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    crate::set_capabilities(Capabilities::new());
    let plugin = Rc::new(
        Policy::new().with_capabilities(Capabilities::new().read_roots([PathBuf::from("/tmp/p")])),
    );
    let runtime = ShellRuntime::new_isolated().expect("runtime");

    context.update(|window, cx| {
        assert!(
            !crate::scope::policy().capabilities().has_read_access(),
            "the default grants nothing"
        );

        // No view: the case a plugin's module top level runs in.
        let (guard, _) = crate::scope::enter_with_runtime(
            &runtime,
            window,
            cx,
            crate::scope::ScopePhase::Task,
            None,
            plugin.clone(),
        );
        assert!(
            crate::scope::policy().capabilities().has_read_access(),
            "inside the scope the plugin's grant is what fs sees"
        );
        drop(guard);

        assert!(
            !crate::scope::policy().capabilities().has_read_access(),
            "and it does not outlive the call"
        );
    });
}

/// Two policies hold two grants at the same time.
///
/// The point the single process-wide slot could not reach: authority belongs to
/// the code that is running, not to the moment it runs in.
#[gpui::test]
fn two_policies_hold_two_grants_at_once(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let reader = Rc::new(
        Policy::default()
            .with_capabilities(Capabilities::new().read_roots([PathBuf::from("/tmp/reader")])),
    );
    let writer = Rc::new(
        Policy::default()
            .with_capabilities(Capabilities::new().write_roots([PathBuf::from("/tmp/writer")])),
    );

    assert!(reader.capabilities().has_read_access());
    assert!(!reader.capabilities().has_write_access());
    assert!(writer.capabilities().has_write_access());
    assert!(!writer.capabilities().has_read_access());

    // Both are alive at the same instant, and neither is the other.
    assert!(!Rc::ptr_eq(&reader, &writer));
}

/// A focus handle created during `render` would be a new one every frame, so
/// the focus a script thought it was tracking would be dropped by the next
/// repaint. That is the same failure `InputState.new(...)` is refused for, and
/// it is refused the same way.
#[gpui::test]
fn a_focus_handle_cannot_be_created_during_render(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";

export default class Late extends View {
  render(cx) {
    return div().track_focus(cx.focus_handle());
  }
}
"#;
    let view_type = runtime.load_source("late-focus", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a focus handle created in render must be refused");

    let message = error.to_string();
    assert!(
        message.contains("cannot run during render"),
        "the error must name the phase: {message}"
    );
    assert!(
        message.contains("init()"),
        "and where the handle belongs instead: {message}"
    );
}

#[gpui::test]
fn an_existing_focus_handle_cannot_focus_during_render(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";

export default class LateFocus extends View {
  init(_props, cx) { this.focus = cx.focus_handle(); }
  render(cx) {
    this.focus.focus();
    return div();
  }
}
"#;
    let view_type = runtime
        .load_source("late-focus-mutation", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("focus mutation during render must be refused");

    assert!(
        error
            .to_string()
            .contains("cannot run during render or layout"),
        "the error must explain the phase boundary: {error}"
    );
}

/// A script hears the keys typed at an element it holds the focus of.
///
/// The whole point is the round trip through the focus path: a handler is
/// installed, the window's focus is moved onto the element tracking the
/// handle, real keystrokes are simulated, and what the script recorded is read
/// back out of the next frame. A test that only asserted the description
/// carried `on_key_down` would pass with nothing wired to GPUI at all.
///
/// The chord is asserted rather than the bare key, because that is the form a
/// script compares against and the form that would silently disagree if the
/// modifiers were dropped on the way across.
#[gpui::test]
fn a_script_hears_the_keys_typed_at_a_focused_element(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Keys extends View {
  init(_props, cx) {
    this.handle = cx.focus_handle();
    this.pressed = [];
    this.released = [];
  }

  render(_cx) {
    return v_flex()
      .w(200)
      .h(100)
      .child(
        div()
          .id("surface")
          .w(200)
          .h(60)
          .tab_index(1)
          .track_focus(this.handle)
          .on_key_down((event, cx) => {
            this.pressed.push(`${event.keystroke}/${event.key}/${event.is_held}`);
            cx.notify();
          })
          .on_key_up((event, cx) => {
            this.released.push(event.keystroke);
            cx.notify();
          }),
      )
      .child(div().child(`down=${this.pressed.join(" ")} up=${this.released.join(" ")}`));
  }
}
"#;
    let view_type = runtime.load_source("keys", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    // A real window root rather than a detached draw: a key event is routed
    // down the window's focus path, so an element painted outside one is never
    // on it and would hear nothing however well it was wired.
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("root view")
        .read_with(&context, |root, _| root.0.clone());

    let handles = runtime.entities().focus_handles();
    assert_eq!(handles.len(), 1, "the script created one focus handle");
    context.update(|window, cx| handles[0].focus(window, cx));
    context.update(|window, cx| window.draw(cx).clear(cx));

    context.simulate_keystrokes("cmd-s escape");
    // `simulate_keystrokes` sends only the press half — GPUI's
    // `Window::dispatch_keystroke` dispatches a `KeyDownEvent` and stops — so
    // the release is posted directly. Without it `on_key_up` would be asserted
    // by a test that never delivered one.
    context.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("escape").expect("keystroke"),
    });
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("down=cmd-s/s/false escape/escape/false"),
        "each press must arrive as the whole chord, the bare key and the held flag: {tree}"
    );
    assert!(
        tree.contains("up=escape"),
        "a release must arrive on the same focus path as a press: {tree}"
    );
}

/// A script receives modifier-only transitions on the focused element.
#[gpui::test]
fn a_script_hears_modifier_changes_at_a_focused_element(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Modifiers extends View {
  init(_props, cx) {
    this.handle = cx.focus_handle();
    this.changes = [];
  }

  render(_cx) {
    return v_flex()
      .w(200)
      .h(100)
      .child(
        div()
          .id("surface")
          .w(200)
          .h(60)
          .tab_index(1)
          .track_focus(this.handle)
          .on_modifiers_changed((event, cx) => {
            this.changes.push([
              event.modifiers.shift,
              event.modifiers.control,
              event.modifiers.alt,
              event.modifiers.platform,
              event.modifiers.function,
              event.capslock.on,
            ].join("/"));
            cx.notify();
          }),
      )
      .child(div().child(this.changes.join(" ")));
  }
}
"#;
    let view_type = runtime.load_source("modifiers", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("root view")
        .read_with(&context, |root, _| root.0.clone());

    let handles = runtime.entities().focus_handles();
    assert_eq!(handles.len(), 1, "the script created one focus handle");
    context.update(|window, cx| handles[0].focus(window, cx));
    context.update(|window, cx| window.draw(cx).clear(cx));

    context.simulate_event(gpui::ModifiersChangedEvent {
        modifiers: gpui::Modifiers {
            control: true,
            function: true,
            ..Default::default()
        },
        capslock: gpui::Capslock { on: true },
    });
    context.simulate_event(gpui::ModifiersChangedEvent::default());
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("false/true/false/false/true/true false/false/false/false/false/false"),
        "modifier press and release must preserve GPUI's complete event payload: {tree}"
    );
}

/// A script binds a chord to an action, and the action reaches its handler.
///
/// This is the whole loop, and every step of it is the real one: the script
/// installs a binding through GPUI's keymap, names a key context on an
/// element, registers a handler for an action id, and a simulated chord walks
/// the focus path to it. Nothing here is a shell shortcut — the only thing the
/// shell contributes is that every script action is one `ShellAction` type
/// with the id inside.
///
/// The unbound chord is asserted alongside, because a handler that fired for
/// every keystroke would pass the first assertion on its own.
#[gpui::test]
fn a_script_binds_a_chord_and_the_action_reaches_its_handler(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Pane extends View {
  init(_props, cx) {
    this.handle = cx.focus_handle();
    this.log = [];
    cx.bind_keys([
      { keystroke: "cmd-s", action: "save", context: "Pane" },
      { keystroke: "ctrl-shift-k", action: "close", context: "Pane" },
    ]);
  }

  render(_cx) {
    return v_flex()
      .w(200)
      .h(100)
      .child(
        div()
          .id("pane")
          .w(200)
          .h(60)
          .key_context("Pane")
          .tab_index(1)
          .track_focus(this.handle)
          .on_action("save", (event, cx) => {
            this.log.push(event.action);
            cx.notify();
          })
          .on_action("close", (event, cx) => {
            this.log.push(event.action);
            cx.notify();
          }),
      )
      .child(div().child(`log=${this.log.join(" ")}`));
  }
}
"#;
    let view_type = runtime.load_source("actions", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("root view")
        .read_with(&context, |root, _| root.0.clone());

    let handles = runtime.entities().focus_handles();
    context.update(|window, cx| handles[0].focus(window, cx));
    context.update(|window, cx| window.draw(cx).clear(cx));

    // `cmd-q` is bound to nothing, and must reach neither handler.
    // `cmd-q` is bound to nothing, and must reach neither handler.
    context.simulate_keystrokes("cmd-s cmd-q ctrl-shift-k");
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("log=save close"),
        "each bound chord must reach the handler registered for its action, \
         and an unbound one must reach none: {tree}"
    );
}

/// An accordion item passes its own `open` down to the trigger under it.
///
/// That pass-down is the whole of what base contributes here — none of the
/// five parts draws anything — and it is what stops a script from having to
/// set the state twice in agreement with itself. It is observable because
/// `AccordionTrigger` asks for the *opposite* of what it was told: pressing
/// the trigger of an open item reports `false`, and pressing a shut one
/// reports `true`. A trigger that never received the item's state would report
/// the same value for both.
#[gpui::test]
fn an_accordion_item_passes_its_open_state_down_to_its_trigger(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import {
  v_flex,
  Accordion,
  AccordionItem,
  AccordionHeader,
  AccordionPanel,
  AccordionTrigger,
} from "gpui-base";

export default class Faq extends View {
  init(_props, _cx) {
    this.open = "first";
    this.log = [];
  }

  item(key, title, body) {
    return AccordionItem.new()
      .open(this.open === key)
      .header(
        AccordionHeader.new(
          AccordionTrigger.new(`${key}-trigger`)
            .w(200)
            .h(40)
            .on_change((open, cx) => {
              this.log.push(`${key}:${open}`);
              cx.notify();
            })
            .child(title),
        ).aria_level(2),
      )
      .panel(AccordionPanel.new().keep_mounted(key === "second").child(body));
  }

  render(_cx) {
    return v_flex()
      .child(
        Accordion.new("faq")
          .child(this.item("first", "One", "answer-one"))
          .child(this.item("second", "Two", "answer-two")),
      )
      .child(div().child(`log=${this.log.join(" ")}`));
  }
}
"#;
    let view_type = runtime.load_source("accordion", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("root view")
        .read_with(&context, |root, _| root.0.clone());

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("Accordion \"faq\"") && tree.contains("AccordionTrigger \"first-trigger\""),
        "the root and each trigger must keep the ids they were built with: {tree}"
    );
    assert!(
        tree.contains("@header") && tree.contains("@panel"),
        "the header and panel must be recorded as slots, not as ordinary children: {tree}"
    );
    assert!(
        tree.contains(":aria_level[Number(2.0)]") && tree.contains(":keep_mounted"),
        "the announced heading level and the mounting policy must reach their parts: {tree}"
    );

    // The first item is open and the second is shut. Both triggers are 40 tall,
    // and the open item's panel sits between them, so the second trigger is
    // found by walking down rather than by assuming a fixed offset.
    context.simulate_click(point(px(20.), px(20.)), Modifiers::default());
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("log=first:false"),
        "pressing the trigger of an open item must ask for it to close, which it can only \
         know from the item's own state: {tree}"
    );
}

/// A single date is stored as one, not as a range with no end.
///
/// `set_value("2026-08-15")` and `set_value(["2026-08-01", null])` mean
/// different things to base — `Date::is_single`, `is_complete` and
/// `is_in_range` all answer differently — but they read back as the same
/// string, because a range whose end is unset renders as its start. So the
/// round trip a script can see agrees either way, and only the state behind it
/// tells them apart. That is what this asserts.
#[gpui::test]
fn a_single_calendar_date_is_not_stored_as_an_open_range(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View } from "gpui-kit";
import { v_flex, CalendarState } from "gpui-base";

export default class Picker extends View {
  init(_props, _cx) {
    this.calendar = CalendarState.new();
    this.calendar.set_value("2026-08-15");
  }
  render(_cx) {
    return v_flex().child(`value=${this.calendar.value()}`);
  }
}
"#;
    let view_type = runtime.load_source("single-date", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    draw(&mut context, &view);

    let state = runtime
        .entities()
        .first_calendar()
        .expect("the script created a calendar state");
    let date = context.update(|_, cx| state.read(cx).date());
    assert!(
        matches!(date, gpui_base::Date::Single(Some(_))),
        "a plain string must select one day, not open a range: got {date:?}"
    );
    assert!(
        date.is_single(),
        "base has to agree it is a single date, because its own logic branches on that"
    );
}

/// A range is stored as one, and reads back as a pair.
///
/// The other half of the same wire question. A pair going in has to arrive as
/// `Date::Range`, and a range coming out has to arrive as a pair rather than
/// collapsing to its start — which is what would happen if the two directions
/// disagreed about how many slots a range takes.
#[gpui::test]
fn a_calendar_range_survives_the_round_trip_as_a_range(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View } from "gpui-kit";
import { v_flex, CalendarState } from "gpui-base";

export default class Picker extends View {
  init(_props, _cx) {
    // One state, read twice. Two would make which one the Rust side inspects
    // depend on a hash order.
    this.calendar = CalendarState.new();
    this.calendar.set_value(["2026-08-03", "2026-08-09"]);
    this.read = this.calendar.value();
    this.calendar.set_value(["2026-08-03", null]);
    this.open = this.calendar.value();
  }
  render(_cx) {
    // Joined rather than JSON, because the debug tree escapes quotes and an
    // assertion full of backslashes says less than the value it checks.
    const show = (v) => (Array.isArray(v) ? `pair(${v.join("|")})` : `single(${v})`);
    return v_flex()
      .child(`value=${show(this.read)}`)
      .child(`open=${show(this.open)}`);
  }
}
"#;
    let view_type = runtime.load_source("range", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    draw(&mut context, &view);

    let state = runtime
        .entities()
        .first_calendar()
        .expect("the script created a calendar state");
    let date = context.update(|_, cx| state.read(cx).date());
    // The state was left holding the half-finished range, which is the case
    // worth pinning: it holds one date, exactly as a single selection does, and
    // only the variant tells them apart.
    assert!(
        matches!(date, gpui_base::Date::Range(Some(_), None)),
        "a pair with an unset end must stay a range, not become a single date: got {date:?}"
    );
    assert!(
        !date.is_single(),
        "base branches on this, and a range mistaken for a single date takes the \
         wrong branch in every one of them"
    );

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("value=pair(2026-08-03|2026-08-09)"),
        "a range must read back as a pair, not as its start: {tree}"
    );
    assert!(
        tree.contains("open=pair(2026-08-03|)"),
        "a range whose end is not chosen yet must stay a pair, which is the whole \
         difference between it and a single date: {tree}"
    );
}

/// Every name this change adds is reachable from a script, under its documented
/// name and in its documented place.
///
/// A breadth test rather than a behavior one: each API's behavior is pinned by
/// its own test above, and what this catches is the other failure — a member
/// bound in the host but never reachable from the prelude, or reachable under
/// a name the documentation does not use. That mistake compiles, passes every
/// behavioral test, and is only found by someone reading the docs and typing
/// what they say.
///
/// The reads are called; the mutations are only checked to *be* functions,
/// because calling `toggle_fullscreen()` in a test window is not a breadth
/// check, it is a side effect.
#[gpui::test]
fn every_added_script_api_is_reachable_under_its_documented_name(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import {
  v_flex,
  Avatar,
  AvatarImage,
  AvatarFallback,
  Accordion,
  AccordionItem,
  AccordionHeader,
  AccordionPanel,
  AccordionTrigger,
  Pagination,
  pagination_items,
  CalendarState,
} from "gpui-base";

export default class Surface extends View {
  init(_props, cx) {
    this.missing = [];
    this.calendar = CalendarState.new();

    // `cx` members, and the one call that installs a keymap.
    for (const name of ["stop_propagation", "propagate", "bind_keys"]) {
      if (typeof cx[name] !== "function") this.missing.push(`cx.${name}`);
    }
    cx.bind_keys([{ keystroke: "ctrl-alt-y", action: "smoke" }]);

    // `window` members. The reads are called; the mutations are only probed,
    // because zooming a test window is not a breadth check.
    for (const name of [
      "rem_size", "line_height", "viewport_size", "bounds", "mouse_position",
      "appearance", "is_window_active", "is_fullscreen", "is_maximized",
      "set_rem_size", "refresh", "focus_next", "focus_prev",
      "activate_window", "minimize_window", "zoom_window", "toggle_fullscreen",
      "dispatch_action",
    ]) {
      if (typeof window[name] !== "function") this.missing.push(`window.${name}`);
    }
    for (const read of [
      "rem_size", "line_height", "viewport_size", "bounds", "mouse_position",
      "appearance", "is_window_active", "is_fullscreen", "is_maximized",
    ]) {
      if (window[read]() === undefined) this.missing.push(`window.${read}() answered nothing`);
    }

    // The calendar handle's own surface.
    for (const name of [
      "month_days", "year", "month", "today", "value", "set_value",
      "next_month", "prev_month", "on", "release",
    ]) {
      if (typeof this.calendar[name] !== "function") this.missing.push(`CalendarState.${name}`);
    }
    this.calendar.on("change", () => {});
  }

  render(_cx) {
    // Every new element method, on one element, plus every new constructor.
    const probe = div()
      .on_key_down(() => {})
      .on_key_up(() => {})
      .on_mouse_down("left", () => {})
      .on_mouse_up("right", () => {})
      .on_mouse_down_out(() => {})
      .on_scroll_wheel(() => {})
      .on_action("smoke", () => {})
      .key_context("Smoke")
      .aria_level(2)
      .keep_mounted(true);

    const avatar = Avatar.new()
      .image(AvatarImage.new("a.png"))
      .fallback(AvatarFallback.new().child("AB"));

    const accordion = Accordion.new("acc").child(
      AccordionItem.new()
        .open(true)
        .header(AccordionHeader.new(AccordionTrigger.new("t").on_change(() => {})))
        .panel(AccordionPanel.new().child("body")),
    );

    const pager = Pagination.new("pager").child(
      div().child(`items=${pagination_items(2, 9, 5).length}`),
    );

    return v_flex()
      .child(probe)
      .child(avatar)
      .child(accordion)
      .child(pager)
      .child(div().child(`grid=${this.calendar.month_days()[0].length > 0}`))
      .child(div().child(`missing=[${this.missing.join(", ")}]`));
  }
}
"#;
    let view_type = runtime.load_source("smoke", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("root view")
        .read_with(&context, |root, _| root.0.clone());

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("missing=[]"),
        "every documented name must be reachable under that name: {tree}"
    );
    assert!(
        tree.contains("items=5") && tree.contains("grid=true"),
        "the two calculations must answer something usable, not an empty list: {tree}"
    );
}

/// A `CalendarState` answers the month grid, and moves it.
///
/// The grid is the reason the state is bound at all — which dates fall in
/// which week, where the neighbouring months' days go, and how many weeks the
/// month needs — so it is what the test pins down, on a month whose shape is
/// not in doubt. August 2026 begins on a Saturday, so the first week is six
/// days of July followed by the 1st, and the last week runs into September.
///
/// `prev_month` is asserted in the same test because a grid that never moved
/// would pass every assertion about a single month.
#[gpui::test]
fn a_calendar_state_answers_the_month_grid_and_moves_it(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex, CalendarState } from "gpui-base";

export default class Month extends View {
  init(_props, cx) {
    this.calendar = CalendarState.new();
    this.calendar.set_value("2026-08-15");
    // The state opens on today's month, so it is moved onto a known one: the
    // grid's shape is what is being asserted, and it has to be the same shape
    // whenever this test runs.
    while (this.calendar.year() > 2026 || (this.calendar.year() === 2026 && this.calendar.month() > 8)) {
      this.calendar.prev_month();
    }
    while (this.calendar.year() < 2026 || (this.calendar.year() === 2026 && this.calendar.month() < 8)) {
      this.calendar.next_month();
    }
    this.august = this.calendar.month_days()[0];
    this.calendar.prev_month();
    this.july = this.calendar.month_days()[0];
  }

  render(_cx) {
    return v_flex()
      .child(div().child(`weeks=${this.august.length}`))
      .child(div().child(`first=${this.august[0].join(",")}`))
      .child(div().child(`last=${this.august[this.august.length - 1].join(",")}`))
      .child(div().child(`moved=${this.july[1][0]}`))
      .child(div().child(`value=${this.calendar.value()}`));
  }
}
"#;
    let view_type = runtime.load_source("calendar", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    draw(&mut context, &view);

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("weeks=6"),
        "August 2026 starts on a Saturday and needs six weeks: {tree}"
    );
    assert!(
        tree.contains(
            "first=2026-07-26,2026-07-27,2026-07-28,2026-07-29,2026-07-30,2026-07-31,2026-08-01"
        ),
        "the first week must lead with the previous month's days: {tree}"
    );
    assert!(
        tree.contains(
            "last=2026-08-30,2026-08-31,2026-09-01,2026-09-02,2026-09-03,2026-09-04,2026-09-05"
        ),
        "the last week must run into the next month: {tree}"
    );
    assert!(
        tree.contains("moved=2026-07-05"),
        "prev_month must answer a different grid, not the same one: {tree}"
    );
    assert!(
        tree.contains("value=2026-08-15"),
        "a single date must round-trip as a plain string: {tree}"
    );
}

/// `pagination_items` lays the page numbers out, gaps and all.
///
/// The layout is the only thing base contributes here — the root is a
/// navigation landmark and the buttons are the script's — so it is what the
/// test asserts, and it asserts a total large enough to need both gaps. An
/// ellipsis naming the pages it covers is the part a script could not work out
/// from the item list alone.
#[gpui::test]
fn pagination_items_lay_out_the_pages_and_their_gaps(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex, Pagination, pagination_items } from "gpui-base";

const describe = (items) =>
  items.map((item) => (item.ellipsis ? `[${item.ellipsis[0]}-${item.ellipsis[1]}]` : item.page)).join(" ");

export default class Pager extends View {
  render(_cx) {
    return v_flex()
      .child(
        Pagination.new("results")
          .accessibility_label("Results")
          .child(div().child(`middle=${describe(pagination_items(10, 20, 7))}`)),
      )
      .child(div().child(`short=${describe(pagination_items(2, 4, 7))}`))
      .child(div().child(`single=${describe(pagination_items(1, 1, 7))}`));
  }
}
"#;
    let view_type = runtime.load_source("pagination", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    draw(&mut context, &view);

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("middle=1 [2-7] 8 9 10 11 12 [13-19] 20"),
        "a current page in the middle of twenty must keep both ends, hold a window of \
         five around it, and collapse each broken run into a gap naming the pages it \
         covers: {tree}"
    );
    assert!(
        tree.contains("short=1 2 3 4"),
        "a total that fits needs no gaps at all: {tree}"
    );
    assert!(
        tree.contains("single="),
        "one page is not a control, and lays out nothing: {tree}"
    );
    assert!(
        tree.contains("Pagination \"results\""),
        "the root must carry the id it was built with: {tree}"
    );
}

/// An `Avatar` renders its image slot, and falls back to the other when there
/// is none.
///
/// The choice is the only thing base's `Avatar` does — it draws no circle, no
/// size and no background — so it is the only thing worth asserting, and both
/// directions have to be, because a root that always rendered the fallback
/// would pass a test that only checked the second.
#[gpui::test]
fn an_avatar_renders_its_image_or_its_fallback(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View } from "gpui-kit";
import { v_flex, Avatar, AvatarImage, AvatarFallback } from "gpui-base";

export default class People extends View {
  render(_cx) {
    return v_flex()
      .child(
        Avatar.new()
          .w(40)
          .h(40)
          .rounded_full()
          .image(AvatarImage.new("avatars/ada.png").size_full())
          .fallback(AvatarFallback.new().child("AL")),
      )
      .child(
        Avatar.new()
          .w(40)
          .h(40)
          .child(AvatarFallback.new().child("GH")),
      )
      .child(Avatar.new().w(40).h(40).fallback(AvatarFallback.new().child("BB")));
  }
}
"#;
    let view_type = runtime.load_source("avatar", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    draw(&mut context, &view);

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("AvatarImage") && tree.contains("avatars/ada.png"),
        "the image slot must carry the path it was built with: {tree}"
    );
    assert!(
        tree.contains("\"AL\"") && tree.contains("\"BB\""),
        "both fallbacks must be described, whichever one base ends up drawing: {tree}"
    );
    assert!(
        tree.contains("\"GH\""),
        "a fallback passed as an ordinary child is still described: {tree}"
    );
}

/// An action nobody claimed carries on to an element further out.
///
/// This is the half of the routing GPUI would have given for free if every
/// script action were its own Rust type. They share one, so one listener per
/// element does the matching, and an action that listener does not handle has
/// to re-open propagation explicitly. Without that, an inner element handling
/// any action at all would silently swallow every other one on its way out.
#[gpui::test]
fn an_unclaimed_action_carries_on_to_an_outer_element(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Nested extends View {
  init(_props, cx) {
    this.handle = cx.focus_handle();
    this.log = [];
    cx.bind_keys([
      { keystroke: "ctrl-shift-i", action: "inner" },
      { keystroke: "ctrl-shift-o", action: "outer" },
    ]);
  }

  render(_cx) {
    return v_flex()
      .w(200)
      .h(120)
      .child(
        div()
          .id("outer")
          .w(200)
          .h(80)
          .on_action("outer", (event, cx) => {
            this.log.push(`outer:${event.action}`);
            cx.notify();
          })
          .child(
            div()
              .id("inner")
              .w(200)
              .h(40)
              .tab_index(1)
              .track_focus(this.handle)
              .on_action("inner", (event, cx) => {
                this.log.push(`inner:${event.action}`);
                cx.notify();
              }),
          ),
      )
      .child(div().child(`log=${this.log.join(" ")}`));
  }
}
"#;
    let view_type = runtime.load_source("action-bubble", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("root view")
        .read_with(&context, |root, _| root.0.clone());

    let handles = runtime.entities().focus_handles();
    context.update(|window, cx| handles[0].focus(window, cx));
    context.update(|window, cx| window.draw(cx).clear(cx));

    // The keyboard is on the inner element for both. The first is its own; the
    // second is not, and has to reach past it.
    context.simulate_keystrokes("ctrl-shift-i ctrl-shift-o");
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("log=inner:inner outer:outer"),
        "an action the inner element does not handle must reach the outer one: {tree}"
    );
}

/// The window answers its own measurements, and refuses to be changed mid-frame.
///
/// The measurements are asserted against the size the test window was actually
/// given, so a stub answering zero would fail. The refusal is asserted in the
/// same test because the two halves are one decision: reading during `render`
/// is the point, and writing during it is a frame arguing with itself.
#[gpui::test]
fn the_window_answers_its_measurements_and_refuses_changes_during_render(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Metrics extends View {
  render(_cx) {
    const viewport = window.viewport_size();
    const rem = window.rem_size();
    let refused = "no";
    try {
      window.set_rem_size(20);
    } catch (_) {
      refused = "yes";
    }
    return v_flex().child(
      div().child(
        `viewport=${viewport.width}x${viewport.height} rem=${rem} ` +
          `appearance=${window.appearance()} refused=${refused}`,
      ),
    );
  }
}
"#;
    let view_type = runtime.load_source("metrics", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("root view")
        .read_with(&context, |root, _| root.0.clone());

    let expected = context.update(|window, _| window.viewport_size());
    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains(&format!(
            "viewport={}x{}",
            f32::from(expected.width),
            f32::from(expected.height)
        )),
        "the viewport a script reads must be the one the window has: {tree}"
    );
    assert!(
        tree.contains("rem=16"),
        "the rem size must be the window's own, not a placeholder: {tree}"
    );
    assert!(
        tree.contains("appearance=light"),
        "the appearance must reduce to one of the two a script can draw for: {tree}"
    );
    assert!(
        tree.contains("refused=yes"),
        "changing the window from inside render() must be refused: {tree}"
    );
}

/// A script hears a press, a release, and a press that landed somewhere else.
///
/// The three are asserted together because they are three different GPUI
/// dispatch paths — bubble on the hitbox, bubble on the hitbox, and capture
/// *off* it — and a mistake in the wiring shows up as one of them firing when
/// another should have. The right-button press is what proves the button
/// argument reaches GPUI rather than being recorded and ignored: a left press
/// on the same element must not trigger it.
#[gpui::test]
fn a_script_hears_presses_releases_and_presses_outside(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Surface extends View {
  init(_props, _cx) {
    this.log = [];
  }

  render(_cx) {
    return v_flex()
      .w(300)
      .h(200)
      .child(
        div()
          .id("target")
          .w(100)
          .h(50)
          .on_mouse_down("left", (event, cx) => {
            this.log.push(`down:${event.button}:${event.click_count}`);
            cx.notify();
          })
          .on_mouse_down("right", (event, cx) => {
            this.log.push(`context:${event.button}`);
            cx.notify();
          })
          .on_mouse_up("left", (event, cx) => {
            this.log.push(`up:${event.button}`);
            cx.notify();
          })
          .on_mouse_down_out((_event, cx) => {
            this.log.push("outside");
            cx.notify();
          }),
      )
      .child(div().child(`log=${this.log.join(" ")}`));
  }
}
"#;
    let view_type = runtime.load_source("mouse", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("root view")
        .read_with(&context, |root, _| root.0.clone());

    // Inside the 100x50 box, then well outside it.
    let inside = point(px(20.), px(20.));
    let outside = point(px(250.), px(160.));
    context.simulate_mouse_move(inside, gpui::MouseButton::Left, Modifiers::default());
    context.simulate_mouse_down(inside, gpui::MouseButton::Left, Modifiers::default());
    context.simulate_mouse_up(inside, gpui::MouseButton::Left, Modifiers::default());
    context.simulate_mouse_move(outside, gpui::MouseButton::Left, Modifiers::default());
    context.simulate_mouse_down(outside, gpui::MouseButton::Left, Modifiers::default());
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("log=down:left:1 up:left outside"),
        "a press and release inside, then a press outside, and no right-button \
         handler firing for a left press: {tree}"
    );
}

/// Input reaches base's own controls, not just a plain element.
///
/// `Button.new("save").on_key_down(...)` is a reasonable thing to write and for
/// two commits it was recorded and never wired — the handler sat in a
/// description while base's `Button` built its own element and hung its own
/// listeners on it.
///
/// The two halves are asserted on different controls on purpose, because they
/// need different things. A key event travels the focus path, so it can only
/// reach a control that accepts a script's focus handle — `Button` and
/// `Checkbox` do. A pointer event travels the hitbox, so it reaches a `Tab`,
/// which accepts no focus handle at all and could never hear a key.
#[gpui::test]
fn input_reaches_a_base_control_not_only_a_plain_element(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex, Button, Checkbox, Tab } from "gpui-base";

export default class Toolbar extends View {
  init(_props, cx) {
    this.button = cx.focus_handle();
    this.check = cx.focus_handle();
    this.log = [];
  }

  render(_cx) {
    return v_flex()
      .child(
        Tab.new("first")
          .w(200)
          .h(40)
          // Right, not left: base's own `Tab` stops propagation on a left
          // press, so a left handler here would be testing that rather than
          // the wiring.
          .on_mouse_down("right", (event, cx) => {
            this.log.push(`tab:${event.button}`);
            cx.notify();
          })
          .child("One"),
      )
      .child(
        Button.new("save")
          .w(200)
          .h(40)
          .tab_index(1)
          .track_focus(this.button)
          .on_key_down((event, cx) => {
            this.log.push(`button:${event.keystroke}`);
            cx.notify();
          })
          .child("Save"),
      )
      .child(
        Checkbox.new("wrap")
          .w(200)
          .h(40)
          .tab_index(2)
          .track_focus(this.check)
          .on_key_down((event, cx) => {
            this.log.push(`checkbox:${event.keystroke}`);
            cx.notify();
          })
          .child("Wrap"),
      )
      .child(div().child(`log=${this.log.join(" ")}`));
  }
}
"#;
    let view_type = runtime.load_source("control-input", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("root view")
        .read_with(&context, |root, _| root.0.clone());

    let handles = runtime.entities().focus_handles();
    assert_eq!(handles.len(), 2, "the script created two focus handles");

    context.update(|window, cx| handles[0].focus(window, cx));
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.simulate_keystrokes("cmd-s");

    context.update(|window, cx| handles[1].focus(window, cx));
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.simulate_keystrokes("space");

    // The tab is the first 40-tall row.
    context.simulate_mouse_move(
        point(px(20.), px(20.)),
        gpui::MouseButton::Right,
        Modifiers::default(),
    );
    context.simulate_mouse_down(
        point(px(20.), px(20.)),
        gpui::MouseButton::Right,
        Modifiers::default(),
    );
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("button:cmd-s"),
        "a key typed at a Button must reach the handler written on it: {tree}"
    );
    assert!(tree.contains("checkbox:space"), "and at a Checkbox: {tree}");
    assert!(
        tree.contains("tab:right"),
        "a press on a Tab must reach it even though a key never could, because a \
         pointer event travels the hitbox rather than the focus path: {tree}"
    );
}

/// `cx.stop_propagation()` keeps an event at the element that handled it.
///
/// GPUI delivers a key event to every handler on the focus path, so a nested
/// element with its own handler fires both by default. That default is what
/// makes the call worth having, and asserting it in the same test is what
/// stops a no-op implementation from passing: the first keystroke must reach
/// both handlers, and the second must reach only the inner one.
#[gpui::test]
fn stop_propagation_keeps_a_key_event_at_the_element_that_handled_it(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Nested extends View {
  init(_props, cx) {
    this.handle = cx.focus_handle();
    this.log = [];
  }

  render(_cx) {
    return v_flex()
      .w(200)
      .h(120)
      .child(
        div()
          .id("outer")
          .w(200)
          .h(60)
          .on_key_down((event, cx) => {
            this.log.push(`outer:${event.key}`);
            cx.notify();
          })
          .child(
            div()
              .id("inner")
              .w(200)
              .h(30)
              .tab_index(1)
              .track_focus(this.handle)
              .on_key_down((event, cx) => {
                this.log.push(`inner:${event.key}`);
                if (event.key === "b") {
                  cx.stop_propagation();
                }
                cx.notify();
              }),
          ),
      )
      .child(div().child(`log=${this.log.join(" ")}`));
  }
}
"#;
    let view_type = runtime.load_source("propagation", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("root view")
        .read_with(&context, |root, _| root.0.clone());

    let handles = runtime.entities().focus_handles();
    context.update(|window, cx| handles[0].focus(window, cx));
    context.update(|window, cx| window.draw(cx).clear(cx));

    context.simulate_keystrokes("a b");
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("log=inner:a outer:a inner:b"),
        "`a` must reach both handlers and `b` must stop at the inner one: {tree}"
    );
}

/// The keyboard actually reaches a script's controls.
///
/// Not "the description carries `tab_index`" — a real window, a real `ShellRoot`
/// with its Tab binding, a real Tab keystroke, and the assertion that the
/// window's focus is now the handle the script created and gave to the second
/// button. Anything less would pass while `tab_index` set a field nobody read.
#[gpui::test]
fn the_tab_key_walks_the_focus_order_a_script_declared(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex, Button, Checkbox, Toggle } from "gpui-base";

export default class Form extends View {
  init(_props, cx) {
    this.handles = [
      cx.focus_handle(),
      cx.focus_handle(),
      cx.focus_handle(),
      cx.focus_handle(),
    ];
  }

  render(cx) {
    return v_flex()
      .w(300)
      .h(200)
      .child(
        Button.new("save")
          .w(200).h(40)
          .tab_index(1)
          .track_focus(this.handles[0])
          .child("Save"))
      .child(
        Checkbox.new("remember")
          .w(200).h(40)
          .tab_index(2)
          .track_focus(this.handles[1])
          .child("Remember"))
      .child(
        Toggle.new("bold")
          .w(200).h(40)
          .tab_index(3)
          .track_focus(this.handles[2])
          .child("Bold"))
      .child(
        div()
          .id("custom")
          .w(200).h(40)
          .tab_index(4)
          .track_focus(this.handles[3])
          .child("Custom"));
  }
}
"#;
    let view_type = runtime.load_source("tab-order", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let (_root, context) = cx.add_window_view(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        let view = cx.new(|_| ScriptView::new(runtime_for_view, object));
        crate::root::ShellRoot::new(view.into(), window, cx)
    });
    context.update(|window, cx| window.draw(cx).clear(cx));

    let handles = runtime.entities().focus_handles();
    assert_eq!(handles.len(), 4, "the script created four focus handles");

    // Nothing is focused until something puts focus in the window, and while
    // nothing is, the root's Tab binding has no dispatch path to reach — a
    // ShellRoot limitation that predates focus reaching scripts at all. So the
    // first step is taken directly; every step after it is a real keystroke.
    assert_eq!(context.update(|window, cx| window.focused(cx)), None);
    context.update(|window, cx| window.focus_next(cx));

    // One keystroke per control, in the order the script numbered them: a
    // Button, a Checkbox and a Toggle through base's own focus builders, and a
    // plain element through GPUI's.
    for (step, expected) in handles.iter().enumerate() {
        context.simulate_keystrokes("tab");
        assert_eq!(
            context.update(|window, cx| window.focused(cx)).as_ref(),
            Some(expected),
            "Tab {} must land on the handle the script declared there",
            step + 1
        );
    }

    context.simulate_keystrokes("shift-tab");
    assert_eq!(
        context.update(|window, cx| window.focused(cx)),
        Some(handles[2].clone()),
        "Shift-Tab must walk the same order backwards"
    );
}

/// `is_focused()` answers about the element the handle was given to.
///
/// The round trip is the whole point of a script-owned handle: the script asks
/// for focus, GPUI moves it, and the next render reads it back. A handle that
/// only ever answered `false` would still make every other assertion here pass.
#[gpui::test]
fn a_tracked_handle_reports_the_focus_it_was_given(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex, Button } from "gpui-base";

export default class Panel extends View {
  init(_props, cx) { this.target = cx.focus_handle(); }

  render(cx) {
    return v_flex()
      .w(300)
      .h(120)
      .child(`focused: ${this.target.is_focused()}`)
      .child(
        div()
          .id("mover")
          .w(300).h(40)
          .on_click((_event, cx) => { this.target.focus(); cx.notify(); })
          .child("Move focus"))
      .child(
        Button.new("target")
          .w(200).h(40)
          .track_focus(this.target)
          .child("Target"));
  }
}
"#;
    let view_type = runtime.load_source("tracked-focus", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));

    let view = window.root(&mut context).expect("view");
    let described = |context: &mut VisualTestContext| {
        context.update(|_, cx| {
            view.read(cx)
                .snapshot()
                .map(crate::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    };
    assert!(
        described(&mut context).contains("text \"focused: false\""),
        "nothing has focus before the script asks for it"
    );

    // The click lands on the plain element above the button, which is not
    // itself focusable — so the focus that arrives is the one the script moved,
    // not one GPUI handed out on a press.
    context.simulate_click(point(px(10.), px(30.)), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));

    let handles = runtime.entities().focus_handles();
    assert_eq!(handles.len(), 1);
    assert_eq!(
        context.update(|window, cx| window.focused(cx)),
        Some(handles[0].clone()),
        "focus() must move the window's focus to the script's handle"
    );
    assert!(
        described(&mut context).contains("text \"focused: true\""),
        "and the next render must read it back: {}",
        described(&mut context)
    );
}

/// A role and a selected state reach the description, and a name that is not a
/// role fails where it was written rather than turning into silence.
#[gpui::test]
fn accessibility_semantics_reach_the_description(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Options extends View {
  init() { this.chosen = 1; }
  render(cx) {
    const names = ["Daily", "Weekly"];
    return v_flex()
      .role("list_box")
      .accessibility_label("Cadence")
      .children(
        names.map((name, index) =>
          div()
            .id(`cadence-${index}`)
            .role("list_box_option")
            .aria_selected(index === this.chosen)
            .when(index === this.chosen, (el) => el.aria_active_descendant())
            .child(name)));
  }
}
"#;
    let view_type = runtime.load_source("options", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("render");

    assert!(
        tree.contains(":role[Str(\"list_box\")]"),
        "the container's role must survive into the description: {tree}"
    );
    assert!(
        tree.contains(":aria_selected[Bool(true)]") && tree.contains(":aria_selected[Bool(false)]"),
        "each option says whether it is the chosen one: {tree}"
    );
    assert_eq!(
        tree.matches(":aria_active_descendant").count(),
        1,
        "exactly one option is the active descendant: {tree}"
    );

    // And it materializes: `role` needs a stateful element, so a plain `div`
    // that never grew an identity would fail here rather than in a story.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// An unknown role is silence in the accessibility tree, which is exactly what
/// calling `role` was meant to prevent — so it fails at the call site.
#[gpui::test]
fn an_unknown_role_fails_where_it_was_written(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";

export default class Wrong extends View {
  render(cx) { return div().role("listbox"); }
}
"#;
    let view_type = runtime.load_source("wrong-role", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a name that is not a role must fail at the script call site");

    assert!(
        error.to_string().contains("unknown accessibility role"),
        "got: {error}"
    );
}

/// The source the popover tests render, with the open state substituted in.
///
/// The trigger sits 100 pixels down so the surface, anchored to its top-left
/// corner, extends past the bottom of the trigger. The click target is in that
/// overhang: it is inside the content and outside the trigger, so a click there
/// can only be reporting that the content is on screen.
const POPOVER: &str = r#"
import { View, div } from "gpui-kit";
import { v_flex, Popover } from "gpui-base";

export default class Menu extends View {
  init() { this.open = OPEN; this.hits = 0; }

  render(cx) {
    return v_flex()
      .w(400)
      .h(400)
      .child(div().w(400).h(100))
      .child(
        Popover.new("menu")
          .anchor("top_left")
          .open(this.open)
          .on_open_change((open, cx) => { this.open = open; cx.notify(); })
          .trigger(div().w(300).h(40).child("Open"))
          .content(
            div()
              .id("body")
              .w(200)
              .h(160)
              .on_click((_event, cx) => { this.hits += 1; cx.notify(); })
              .child(`Body: ${this.hits}`),
          ),
      );
  }
}
"#;

/// Where the content lands: below the trigger, inside the surface.
fn inside_the_surface() -> gpui::Point<gpui::Pixels> {
    point(px(100.), px(200.))
}

/// Where the trigger is.
fn on_the_trigger() -> gpui::Point<gpui::Pixels> {
    point(px(100.), px(120.))
}

/// Renders the popover source with the given open state, clicks where the
/// content sits when it is drawn, and returns the description that came out.
fn popover_tree(cx: &mut TestAppContext, open: bool) -> String {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = POPOVER.replace("OPEN", if open { "true" } else { "false" });
    let view_type = runtime.load_source("menu.js", &source).expect("load");

    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    // Twice: a popup measures its trigger while painting and only places the
    // surface on the frame after it knows where the trigger is.
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.simulate_click(inside_the_surface(), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));

    let view = window.root(&mut context).expect("view");
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    })
}

/// An open `Popover` draws the element in its `content` slot where a click can
/// reach it, and both slots reach the description as slots rather than as
/// children.
#[gpui::test]
fn an_open_popover_draws_its_content_where_it_can_be_clicked(cx: &mut TestAppContext) {
    let tree = popover_tree(cx, true);

    assert!(tree.contains("Popover \"menu\""), "missing popover: {tree}");
    assert!(
        tree.contains("@trigger") && tree.contains("@content"),
        "both slots must be described as slots: {tree}"
    );
    assert!(
        tree.contains("text \"Body: 1\""),
        "an open popover must draw its content, so the click must land: {tree}"
    );
}

/// A closed one describes the same content and draws none of it.
#[gpui::test]
fn a_closed_popover_describes_its_content_without_drawing_it(cx: &mut TestAppContext) {
    let tree = popover_tree(cx, false);

    assert!(
        tree.contains(":open[Bool(false)]"),
        "the closed state must be described: {tree}"
    );
    // The description is open-agnostic: `open` gates what is rendered, not what
    // the script said. The trigger proves the popover itself is on screen.
    assert!(
        tree.contains("@content"),
        "a closed popover still describes its content: {tree}"
    );
    assert!(
        tree.contains("text \"Open\""),
        "the trigger is drawn either way: {tree}"
    );
    assert!(
        !tree.contains("text \"Body: 1\""),
        "a closed popover draws no content, so there is nothing there to click: {tree}"
    );
}

/// The open state goes out through `on_open_change` and comes back in through
/// `open`, which is the whole of what "controlled" means here.
///
/// Nothing else can tell the story: the description carries the content whether
/// or not it is showing, so the proof that the pointer opened the surface is
/// that a click 60 pixels below the trigger started landing, and the proof that
/// pressing outside closed it again is that the same click stopped landing.
#[gpui::test]
fn a_popover_reports_the_open_state_the_pointer_changed(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = POPOVER.replace("OPEN", "false");
    let view_type = runtime.load_source("menu.js", &source).expect("load");

    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = |context: &mut VisualTestContext| {
        let view = window.root(context).expect("view");
        context.update(|_, cx| {
            view.read(cx)
                .snapshot()
                .map(crate::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    };

    context.simulate_click(on_the_trigger(), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    let opened = tree(&mut context);
    assert!(
        opened.contains(":open[Bool(true)]"),
        "pressing the trigger must report the new open state to the script: {opened}"
    );

    context.simulate_click(inside_the_surface(), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    let clicked = tree(&mut context);
    assert!(
        clicked.contains("text \"Body: 1\""),
        "the content the script re-rendered must be on screen and clickable: {clicked}"
    );

    // Below the surface as well as beside it, so this is outside and nothing
    // else.
    context.simulate_click(point(px(300.), px(380.)), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    let dismissed = tree(&mut context);
    assert!(
        dismissed.contains(":open[Bool(false)]"),
        "pressing outside must report the surface closed: {dismissed}"
    );

    context.simulate_click(inside_the_surface(), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    let after = tree(&mut context);
    assert!(
        after.contains("text \"Body: 1\""),
        "a dismissed popover draws nothing, so the second click must not land: {after}"
    );
}

/// The anchor grammar mirrors `gpui::Anchor`, so an unknown corner is a script
/// error rather than a surface that quietly opens in the default one.
#[gpui::test]
fn an_unknown_anchor_is_rejected_at_the_call_site(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { Popover } from "gpui-base";

export default class BadAnchor extends View {
  render(cx) { return Popover.new("menu").anchor("topLeft"); }
}
"#;
    let view_type = runtime.load_source("bad-anchor", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("an unknown anchor must fail at the script call site");

    let message = error.to_string();
    for name in crate::materialize::ANCHOR_NAMES {
        assert!(
            message.contains(name),
            "the error must list every legal corner, and `{name}` is missing: {message}"
        );
    }
}

/// A `HoverCard` opens after the pointer rests on its trigger and closes after
/// it leaves, and the script's content is what is on screen in between.
#[gpui::test]
fn a_hover_card_opens_after_its_delay_and_closes_once_the_pointer_leaves(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex, HoverCard } from "gpui-base";

export default class Card extends View {
  init() { this.hits = 0; }

  render(cx) {
    return v_flex()
      .w(400)
      .h(400)
      .child(div().w(400).h(100))
      .child(
        HoverCard.new("card")
          .anchor("top_left")
          .open_delay(50)
          .close_delay(50)
          .trigger(div().w(300).h(40).child("Hover"))
          .content(
            div()
              .id("body")
              .w(200)
              .h(160)
              .on_click((_event, cx) => { this.hits += 1; cx.notify(); })
              .child(`Body: ${this.hits}`),
          ),
      );
  }
}
"#;
    let view_type = runtime.load_source("card.js", source).expect("load");

    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = |context: &mut VisualTestContext| {
        let view = window.root(context).expect("view");
        context.update(|_, cx| {
            view.read(cx)
                .snapshot()
                .map(crate::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    };
    let settle = |context: &mut VisualTestContext| {
        context
            .executor()
            .advance_clock(std::time::Duration::from_millis(60));
        context.run_until_parked();
        context.update(|window, cx| window.draw(cx).clear(cx));
        context.update(|window, cx| window.draw(cx).clear(cx));
    };

    context.simulate_mouse_move(on_the_trigger(), None, Modifiers::default());
    settle(&mut context);

    context.simulate_click(inside_the_surface(), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    let opened = tree(&mut context);
    assert!(
        opened.contains("HoverCard \"card\""),
        "missing hover card: {opened}"
    );
    assert!(
        opened.contains("text \"Body: 1\""),
        "resting on the trigger must put the content on screen, where a click reaches it: \
         {opened}"
    );

    context.simulate_mouse_move(point(px(350.), px(380.)), None, Modifiers::default());
    settle(&mut context);

    context.simulate_click(inside_the_surface(), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    let closed = tree(&mut context);
    assert!(
        closed.contains("text \"Body: 1\""),
        "the card closes once the pointer leaves, so the second click must not land: {closed}"
    );
}

/// The source the combobox tests render.
///
/// The layout is fixed so the click targets can be named rather than searched
/// for: a 60-pixel button that moves the focus onto the select, a 40-pixel
/// spacer under it, and then the select itself at y=100 with a 300×40 trigger.
/// The list is 200×160 anchored to the popup's top-left corner, so its overhang
/// — inside the list, below and beside the trigger — is the one place a click
/// can only mean "the list is on screen".
const SELECT: &str = r#"
import { View, div } from "gpui-kit";
import { v_flex, Select, Popup } from "gpui-base";

export default class Picker extends View {
  init(_props, cx) {
    this.trigger_focus = cx.focus_handle();
    this.list_focus = cx.focus_handle();
    this.open = false;
    this.chosen = "none";
    this.confirms = 0;
    this.dismisses = 0;
  }

  list() {
    return v_flex()
      .id("list")
      .w(200)
      .h(160)
      .track_focus(this.list_focus)
      .role("list_box")
      .child(
        div()
          .id("option-cn")
          .w(200)
          .h(120)
          .role("list_box_option")
          .aria_selected(this.chosen === "CN")
          .aria_active_descendant()
          .on_click((_event, cx) => { this.chosen = "CN"; cx.notify(); })
          .child("China"));
  }

  render(cx) {
    return v_flex()
      .w(400)
      .h(400)
      .child(
        div()
          .id("focus")
          .w(400)
          .h(60)
          .on_click((_event, cx) => { this.trigger_focus.focus(); cx.notify(); })
          .child("Focus"))
      .child(div().w(400).h(40))
      .child(
        Select.new("country")
          .accessibility_label("Country")
          .open(this.open)
          .track_focus(this.trigger_focus)
          .content_focus_handle(this.list_focus)
          .on_open_change((open, cx) => { this.open = open; cx.notify(); })
          .on_confirm((_event, cx) => { this.confirms += 1; cx.notify(); })
          .on_dismiss((_event, cx) => { this.dismisses += 1; cx.notify(); })
          .child(
            Popup.new(
              "country-popup",
              div()
                .id("trigger")
                .w(300)
                .h(40)
                .on_click((_event, cx) => { this.open = !this.open; cx.notify(); })
                .child("Choose"))
              .anchor("top_left")
              .when(this.open, (el) => el.content(this.list()))))
      .child(
        `open:${this.open} chosen:${this.chosen} confirm:${this.confirms} dismiss:${this.dismisses}`);
  }
}
"#;

/// Where the select's trigger sits.
fn on_the_select_trigger() -> gpui::Point<gpui::Pixels> {
    point(px(100.), px(120.))
}

/// Inside the list and outside the trigger, so a click here can only land when
/// the popup is showing.
fn inside_the_list() -> gpui::Point<gpui::Pixels> {
    point(px(100.), px(200.))
}

/// Opens a window on [`SELECT`] and hands back the pieces the tests drive.
fn select_harness(cx: &mut TestAppContext) -> (VisualTestContext, gpui::Entity<ScriptView>) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source("picker.js", SELECT).expect("load");

    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    // Twice: a popup measures itself while painting and only places the surface
    // on the frame after it knows where it is.
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window.root(&mut context).expect("view");
    (context, view)
}

fn described(context: &mut VisualTestContext, view: &gpui::Entity<ScriptView>) -> String {
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    })
}

/// Two frames, because a popup that has just been given content places it on
/// the frame after the one that measured the trigger.
fn settle_popup(context: &mut VisualTestContext) {
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|window, cx| window.draw(cx).clear(cx));
}

/// The whole controlled loop, driven from the keyboard base actually binds.
///
/// ↓ opens: base reports the new state through `on_open_change`, the script
/// stores it, and the next render fills the popup's `content` slot. The proof
/// that the list is really on screen is that a click 60 pixels below the
/// trigger starts landing — and the proof that Escape closed it again is that
/// the same click stops landing. Escape also reports through `on_dismiss`
/// first, which is what lets a script commit a pending value on the way out.
#[gpui::test]
fn a_select_carries_its_open_state_both_ways_and_draws_its_list_in_a_popup(
    cx: &mut TestAppContext,
) {
    let (mut context, view) = select_harness(cx);

    let before = described(&mut context, &view);
    assert!(
        before.contains("Select \"country\"") && before.contains("Popup \"country-popup\""),
        "both roots must reach the description: {before}"
    );
    assert!(
        before.contains("open:false"),
        "a select starts shut: {before}"
    );

    // The pointer path first: the trigger the script drew owns the press, and
    // the list it drew appears in the popup above the window.
    context.simulate_click(on_the_select_trigger(), Modifiers::default());
    settle_popup(&mut context);
    let opened = described(&mut context, &view);
    assert!(
        opened.contains("open:true"),
        "pressing the trigger must open the select: {opened}"
    );
    assert!(
        opened.contains(":aria_active_descendant[]"),
        "the script marks its own highlighted option; the root cannot: {opened}"
    );

    context.simulate_click(inside_the_list(), Modifiers::default());
    settle_popup(&mut context);
    let chosen = described(&mut context, &view);
    assert!(
        chosen.contains("chosen:CN"),
        "an open select must draw its list where a click reaches it: {chosen}"
    );

    // Escape needs the keyboard, and the root's actions are dispatched down the
    // focus path — a select nothing has focused hears no keys at all.
    context.simulate_click(point(px(10.), px(30.)), Modifiers::default());
    settle_popup(&mut context);
    context.simulate_keystrokes("escape");
    settle_popup(&mut context);
    let dismissed = described(&mut context, &view);
    assert!(
        dismissed.contains("dismiss:1"),
        "Escape must report the dismissal: {dismissed}"
    );
    assert!(
        dismissed.contains("open:false"),
        "and then the close, so the script can stop drawing the list: {dismissed}"
    );

    context.simulate_click(inside_the_list(), Modifiers::default());
    settle_popup(&mut context);
    let after = described(&mut context, &view);
    assert!(
        after.contains("open:false"),
        "a closed select draws no list, so the click in it opens nothing: {after}"
    );

    // And the keyboard opens it again, which is the other half of the loop: the
    // value goes out through `on_open_change` and comes back in through `open`.
    context.simulate_keystrokes("down");
    settle_popup(&mut context);
    let reopened = described(&mut context, &view);
    assert!(
        reopened.contains("open:true"),
        "the down arrow must report the new open state back to the script: {reopened}"
    );
}

/// Enter on an open root confirms; Enter on a shut one opens it instead, which
/// is base's rule rather than ours.
#[gpui::test]
fn enter_confirms_an_open_select_and_opens_a_shut_one(cx: &mut TestAppContext) {
    let (mut context, view) = select_harness(cx);

    context.simulate_click(point(px(10.), px(30.)), Modifiers::default());
    settle_popup(&mut context);

    context.simulate_keystrokes("enter");
    settle_popup(&mut context);
    let opened = described(&mut context, &view);
    assert!(
        opened.contains("open:true") && opened.contains("confirm:0"),
        "Enter on a shut select opens it and confirms nothing: {opened}"
    );

    context.simulate_keystrokes("enter");
    settle_popup(&mut context);
    let confirmed = described(&mut context, &view);
    assert!(
        confirmed.contains("confirm:1"),
        "Enter on an open select confirms, with no payload to carry: {confirmed}"
    );
}

/// A `Popup` has no trigger to fall back on, so an omitted one is refused where
/// it was written rather than drawn as an empty box that anchors nothing.
#[gpui::test]
fn a_popup_without_a_trigger_is_refused_at_the_call_site(cx: &mut TestAppContext) {
    let message = render_error(
        cx,
        "popup-trigger",
        r#"
import { div, View } from "gpui-kit";
import { Popup } from "gpui-base";

export default class NoTrigger extends View {
  render(cx) { return Popup.new("menu"); }
}
"#,
    );
    assert!(
        message.contains("Popup.new(id, trigger)"),
        "the error must name the constructor: {message}"
    );
}

/// The anchor grammar is one table shared by every anchored surface, so a
/// corner spelled the JavaScript way fails on a `Popup` exactly as it does on a
/// `Popover`.
#[gpui::test]
fn a_popup_with_an_unknown_anchor_is_refused_at_the_call_site(cx: &mut TestAppContext) {
    let message = render_error(
        cx,
        "popup-anchor",
        r#"
import { View, div } from "gpui-kit";
import { Popup } from "gpui-base";

export default class BadAnchor extends View {
  render() { return Popup.new("menu", div()).anchor("bottomLeft"); }
}
"#,
    );
    for name in crate::materialize::ANCHOR_NAMES {
        assert!(
            message.contains(name),
            "the error must list every legal corner, and `{name}` is missing: {message}"
        );
    }
}

/// Base's `DatePicker::new` takes the focus handle, so there is no picker to
/// build without one — and no builder to add one afterwards. The message has to
/// say both, because "expects a FocusHandle" alone reads like a call that could
/// be moved down the chain.
#[gpui::test]
fn a_date_picker_without_a_focus_handle_says_why_it_needs_one(cx: &mut TestAppContext) {
    let message = render_error(
        cx,
        "date-picker-handle",
        r#"
import { div, View } from "gpui-kit";
import { DatePicker } from "gpui-base";

export default class NoHandle extends View {
  render() { return DatePicker.new("due"); }
}
"#,
    );
    assert!(
        message.contains("DatePicker.new(id, focus_handle)"),
        "the error must name the constructor: {message}"
    );
    assert!(
        message.contains("cx.focus_handle()"),
        "and where a handle comes from: {message}"
    );
    assert!(
        message.contains("no builder to supply one later"),
        "and why it cannot simply be set afterwards: {message}"
    );
}

/// A picker holds no date, and in the shell it holds no keyboard either.
///
/// What it does hold is the trigger's focus handle and the announced open
/// state, and both are real: Tab lands on the handle the script created, and
/// `open` reaches the element that announces it.
///
/// Enter and Escape are the part that is missing, and the reason is worth
/// pinning down: base's `DatePicker` sets no key context, while every binding
/// base installs is scoped to one. `crates/component` supplies both — its own
/// `"DatePicker"` context and its own bindings — and the shell has no
/// key-binding layer to supply either. So the assertion below is that Escape
/// changes nothing. If it ever starts changing something, this test is the
/// place that says the `.d.ts` needs updating.
#[gpui::test]
fn a_date_picker_carries_focus_and_an_announced_open_state(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex, DatePicker } from "gpui-base";

export default class Due extends View {
  init(_props, cx) {
    this.focus = cx.focus_handle();
    this.open = false;
  }

  render(cx) {
    return v_flex()
      .w(400)
      .h(400)
      .child(
        DatePicker.new("due", this.focus)
          .open(this.open)
          .w(300)
          .h(40)
          .on_open_change((open, cx) => { this.open = open; cx.notify(); })
          .child(`open:${this.open}`));
  }
}
"#;
    let view_type = runtime.load_source("due.js", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let (_root, context) = cx.add_window_view(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        let view = cx.new(|_| ScriptView::new(runtime_for_view, object));
        crate::root::ShellRoot::new(view.into(), window, cx)
    });
    context.update(|window, cx| window.draw(cx).clear(cx));

    let handles = runtime.entities().focus_handles();
    assert_eq!(handles.len(), 1, "the script created one focus handle");

    // The whole of what base wires for a picker: the constructor's handle is
    // its tab stop, so the window's focus order reaches the picker itself.
    context.update(|window, cx| window.focus_next(cx));
    assert_eq!(
        context.update(|window, cx| window.focused(cx)).as_ref(),
        Some(&handles[0]),
        "the constructor's handle must be the picker's tab stop"
    );

    // And the documented gap: no key context, so no binding matches and the
    // controlled open state never moves.
    context.simulate_keystrokes("escape");
    context.simulate_keystrokes("enter");
    context.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        context.update(|window, cx| window.focused(cx)).as_ref(),
        Some(&handles[0]),
        "nothing takes the keyboard away from the picker either"
    );
}

/// A tooltip is the one bound thing whose content outlives the render that
/// described it, so this walks the path rather than the description: a real
/// window with a `ShellRoot`, a real pointer resting on a real button, base's
/// own half-second delay, and the window's overlay reporting that something is
/// now up.
///
/// Base's `TooltipOverlay` exposes no reader for what it is showing, so what is
/// asserted is its notification — which it emits when the delayed show lands
/// and again when the grace period after the pointer leaves runs out, and at no
/// other time. A trigger that never reached the overlay produces neither.
#[gpui::test]
fn a_tooltip_reaches_the_window_overlay_after_the_pointer_rests(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex, Button } from "gpui-base";

export default class Toolbar extends View {
  render(cx) {
    return v_flex()
      .w(400)
      .h(400)
      .child(
        Button.new("save")
          .w(300)
          .h(40)
          .tooltip("Save the document")
          .child("Save"));
  }
}
"#;
    let view_type = runtime.load_source("toolbar.js", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let (root, context) = cx.add_window_view(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        let view = cx.new(|_| ScriptView::new(runtime_for_view, object));
        crate::root::ShellRoot::new(view.into(), window, cx)
    });
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = context.update(|_, cx| {
        root.read(cx)
            .content()
            .clone()
            .downcast::<ScriptView>()
            .ok()
            .expect("the root was given a script view")
            .read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains(r#":tooltip[Str("Save the document")]"#),
        "the label has to survive into the description: {tree}"
    );

    let overlay = context
        .update(|window, cx| crate::root::ShellRoot::tooltip_overlay(window, cx))
        .expect("a ShellRoot owns its window's tooltip layer");
    let shows = Rc::new(Cell::new(0usize));
    let counter = Rc::clone(&shows);
    let _subscription =
        context.update(|_, cx| cx.observe(&overlay, move |_, _| counter.set(counter.get() + 1)));

    // On the button, and still nothing: the delay is what stops a tooltip
    // appearing under a pointer that was only passing over the toolbar.
    context.simulate_mouse_move(point(px(20.), px(20.)), None, Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        shows.get(),
        0,
        "a tooltip must not appear the instant it is hovered"
    );

    context
        .executor()
        .advance_clock(std::time::Duration::from_millis(600));
    context.run_until_parked();
    assert_eq!(
        shows.get(),
        1,
        "resting on the trigger must put the label up"
    );

    // And the layer actually renders what it was handed: the label view, the
    // positioner around it and the enter animation all run here.
    context.update(|window, cx| window.draw(cx).clear(cx));

    context.simulate_mouse_move(point(px(20.), px(300.)), None, Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    context
        .executor()
        .advance_clock(std::time::Duration::from_millis(400));
    context.run_until_parked();
    assert_eq!(
        shows.get(),
        2,
        "leaving the trigger must take the label away again"
    );
}

/// A tooltip that is not a string is refused where it was written.
///
/// Coercing it, or dropping it, would leave an element that looks tooltipped
/// and shows nothing when the pointer rests on it. The second half of the
/// message is the only place a script is told that the element form of a
/// tooltip is not bound yet — a function argument is caught one layer earlier,
/// by the same conversion that refuses one to `id`, and says only that.
#[gpui::test]
fn a_tooltip_that_is_not_a_string_is_refused_at_the_call_site(cx: &mut TestAppContext) {
    let message = render_error(
        cx,
        "label.js",
        r#"
import { div, View } from "gpui-kit";
import { Button } from "gpui-base";

export default class Toolbar extends View {
  render() {
    return Button.new("save").tooltip(42).child("Save");
  }
}
"#,
    );
    assert!(
        message.contains("tooltip(text) expects a string"),
        "the message has to name the method and the type it wanted: {message}"
    );
    assert!(
        message.contains("not bound yet"),
        "and has to say that the element form is the thing that is missing: {message}"
    );

    let message = render_error(
        cx,
        "label.js",
        r#"
import { div, View } from "gpui-kit";
import { Button } from "gpui-base";

export default class Toolbar extends View {
  render() {
    return Button.new("save").tooltip(() => "Save").child("Save");
  }
}
"#,
    );
    assert!(
        message.contains("`tooltip` does not take a function"),
        "a function is the mistake a script is most likely to make here: {message}"
    );
}

/// Renders `source` once and returns the message it failed with.
fn render_error(cx: &mut TestAppContext, name: &str, source: &str) -> String {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source(name, source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("the script must fail where the call was written")
        .to_string()
}
const RESIZABLE: &str = r#"
import { View, div } from "gpui-kit";
import { h_resizable, resizable_panel } from "gpui-base";

export default class Workspace extends View {
  init() { this.sizes = []; }
  render(cx) {
    return h_resizable("workspace")
      .w(400)
      .h(200)
      .on_resize((sizes, cx) => { this.sizes = sizes.map((size) => Math.round(size)); cx.notify(); })
      .child(
        resizable_panel()
          .size(220)
          .size_range(120, 320)
          .child(div().size_full().child("Sidebar")))
      .child(resizable_panel().visible(true).child(`Editor ${this.sizes.join("/")}`));
  }
}
"#;

/// A resizable group is the first component that reads its children as
/// descriptions rather than as elements: `size`, `size_range` and `visible`
/// belong to `ResizablePanel` and have no counterpart on an `AnyElement`, so a
/// panel built out of a finished element would have lost all three.
///
/// The drag is the other half. Nothing in the script holds the panel widths —
/// they live in the window under the group's id — so this asserts both that
/// dragging works with no state on the view and that the sizes reach the script
/// as ordinary numbers.
#[gpui::test]
fn a_resizable_group_sizes_its_panels_and_reports_a_drag(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("workspace.js", RESIZABLE)
        .expect("load");

    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|window, cx| window.draw(cx).clear(cx));

    let view = window.root(&mut context).expect("view");
    let tree = |context: &mut VisualTestContext| {
        context.update(|_, cx| {
            view.read(cx)
                .snapshot()
                .map(crate::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    };

    let described = tree(&mut context);
    assert!(
        described.contains("h_resizable \"workspace\""),
        "the axis is the constructor, so the dump has to name which one: {described}"
    );
    assert!(
        described.contains("resizable_panel :panel_size[Number(220.0)]"),
        "a panel's initial size is the panel's, not a width and a height: {described}"
    );
    assert!(
        described.contains(":size_range[Number(120.0), Number(320.0)]"),
        "both ends of the range must survive into the description: {described}"
    );
    assert!(
        described.contains(":panel_visible[Bool(true)]"),
        "visibility is a panel builder, so it must be described: {described}"
    );

    // The boundary sits at 220; the handle straddles it, four pixels either
    // side. Each event is followed by a frame, because the group reads the
    // panel being dragged out of the state the previous frame left behind.
    context.simulate_mouse_down(
        point(px(218.), px(100.)),
        gpui::MouseButton::Left,
        Modifiers::default(),
    );
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.simulate_mouse_move(
        point(px(240.), px(100.)),
        Some(gpui::MouseButton::Left),
        Modifiers::default(),
    );
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.simulate_mouse_move(
        point(px(300.), px(100.)),
        Some(gpui::MouseButton::Left),
        Modifiers::default(),
    );
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.simulate_mouse_up(
        point(px(300.), px(100.)),
        gpui::MouseButton::Left,
        Modifiers::default(),
    );
    context.update(|window, cx| window.draw(cx).clear(cx));

    let dragged = tree(&mut context);
    assert!(
        dragged.contains("text \"Editor 300/100\""),
        "the drag must reach the script as a plain array of pixel sizes: {dragged}"
    );
}

/// A `resizable_panel()` renders only inside a group — base's panel reads its
/// size out of the group's state and panics without one — so putting it
/// anywhere else is refused where the script can be pointed at the line that
/// did it.
#[gpui::test]
fn a_resizable_panel_outside_a_group_is_refused_at_the_call_site(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { resizable_panel } from "gpui-base";

export default class Loose extends View {
  render(cx) {
    return div().child(resizable_panel().child("Nowhere"));
  }
}
"#;
    let view_type = runtime.load_source("loose.js", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a panel outside a group must fail at the script call site");

    assert!(
        error.to_string().contains("h_resizable"),
        "the error must name where a panel belongs: {error}"
    );
}

#[gpui::test]
fn a_resizable_panel_rejects_a_reversed_size_range(cx: &mut TestAppContext) {
    let message = render_error(
        cx,
        "reversed-panel-range.js",
        r#"
import { div, View } from "gpui-kit";
import { h_resizable, resizable_panel } from "gpui-base";

export default class Workspace extends View {
  render() {
    return h_resizable("workspace")
      .child(resizable_panel().size_range(300, 100))
      .child(resizable_panel());
  }
}
"#,
    );
    assert!(
        message.contains("maximum") && message.contains("minimum"),
        "the range must be refused before a drag reaches Rust clamp: {message}"
    );
}

/// The script every virtual list test below renders, parameterized by what the
/// item renderer does beyond describing a row.
///
/// The list is nested in a box of a known height so the visible range is a
/// number the test can predict, and the range the last prepaint asked for is
/// written back onto the view — which is how a pass that happens inside GPUI's
/// layout is observable from outside it at all.
fn virtual_list_source(extra: &str) -> String {
    format!(
        r#"
import {{ div, View }} from "gpui-kit";
import {{ v_flex, v_virtual_list }} from "gpui-base";

export default class Rows extends View {{
  init() {{
    this.range = [0, 0];
    this.clicked = -1;
    this.refused = "";
  }}

  render(cx) {{
    return v_flex()
      .w(300)
      .h(400)
      .child(
        v_flex()
          .h(200)
          .child(
            v_virtual_list("rows", 500, 20, (index) => String(index), (range) => {{
              this.range = [range.start, range.end];
              const items = [];
              for (let index = range.start; index < range.end; index++) {{
                const row = div().h(20).child(`row ${{index}}`);
                {extra}
                items.push(row);
              }}
              return items;
            }}).on_item_click((key, cx) => {{
              this.clicked = key;
              cx.notify();
            }}),
          ),
      )
      .child(`range ${{this.range[0]}}..${{this.range[1]}} clicked ${{this.clicked}} refused ${{this.refused}}`);
  }}
}}
"#
    )
}

#[gpui::test]
fn a_secondary_press_on_a_row_reports_its_key_and_the_press(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui-kit";
import { v_flex, v_virtual_list } from "gpui-base";

export default class Rows extends View {
  init() {
    this.items = ["alpha", "beta", "gamma"];
    this.pressed = "none";
    this.clicked = "none";
  }
  render(cx) {
    return v_flex()
      .w(300)
      .h(200)
      .child(
        v_virtual_list(
          "rows",
          this.items.length,
          40,
          (index) => this.items[index],
          (range) => {
            const rows = [];
            for (let index = range.start; index < range.end; index++) {
              rows.push(div().h(40).child(this.items[index]));
            }
            return rows;
          },
        )
          .on_item_click((key, cx) => {
            this.clicked = key;
            cx.notify();
          })
          .on_item_secondary_click((key, event, cx) => {
            this.pressed = `${key} ${event.button} at ${Math.round(event.local_position.y)} of ${Math.round(event.bounds.height)}`;
            cx.notify();
          }),
      )
      .child(`pressed ${this.pressed} clicked ${this.clicked}`);
  }
}
"#;
    let view_type = runtime
        .load_source("pressed-list.js", source)
        .expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("view")
        .read_with(&context, |root, _| root.0.clone());

    // The second row spans 40..80; press ten pixels into it.
    context.simulate_mouse_move(
        point(px(150.), px(50.)),
        gpui::MouseButton::Right,
        Modifiers::default(),
    );
    context.simulate_mouse_down(
        point(px(150.), px(50.)),
        gpui::MouseButton::Right,
        Modifiers::default(),
    );
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    // The key names the row, the press is measured from that row's own box,
    // and a right press is not a click.
    assert!(
        tree.contains("pressed beta right at 10 of 40 clicked none"),
        "a right press must reach the list's secondary handler with the row it landed on:\n{tree}"
    );
}

/// Rebuilds the description so that what the item renderer recorded during the
/// last layout shows up in the tree.
///
/// A virtual list writes its state during GPUI's layout pass, which is *after*
/// the render that produced the description being laid out. Nothing about that
/// is a re-render, and it must not be — so the test asks for one.
fn redraw_and_read(context: &mut VisualTestContext, view: &gpui::Entity<ScriptView>) -> String {
    context.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.invalidate();
            cx.notify();
        })
    });
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    })
}

/// The `start..end` the item renderer was last asked for, read out of the tree.
fn reported_range(tree: &str) -> (usize, usize) {
    let text = tree
        .split("range ")
        .nth(1)
        .unwrap_or_else(|| panic!("no reported range in: {tree}"));
    let (start, rest) = text.split_once("..").expect("a start and an end");
    let end = rest
        .split(|character: char| !character.is_ascii_digit())
        .next()
        .expect("an end");
    (
        start.parse().expect("a start index"),
        end.parse().expect("an end index"),
    )
}

fn mount_virtual_list(
    cx: &mut TestAppContext,
    extra: &str,
) -> (
    Rc<ShellRuntime>,
    gpui::WindowHandle<RootedScriptView>,
    gpui::Entity<ScriptView>,
    VisualTestContext,
) {
    mount_list_source(cx, &virtual_list_source(extra))
}

fn scroll_by(context: &mut VisualTestContext, dy: f32) {
    context.simulate_event(gpui::ScrollWheelEvent {
        position: point(px(150.), px(100.)),
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(dy))),
        ..Default::default()
    });
    context.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test]
fn a_virtual_list_describes_only_the_visible_window_and_follows_the_scroll(
    cx: &mut TestAppContext,
) {
    assert_the_visible_window_follows_the_scroll(cx, &virtual_list_source(""));
}

#[gpui::test]
fn a_virtual_lists_handlers_do_not_accumulate_while_it_is_scrolled(cx: &mut TestAppContext) {
    let (runtime, _window, view, mut context) = mount_virtual_list(cx, "");

    let settled = runtime.live_callbacks();
    assert!(
        settled > 0,
        "the list registers its renderer, key resolver and click handler"
    );

    // Forty passes over the item renderer — two per frame, one to measure and
    // one to place. Each describes twenty rows. If a row could register a
    // handler, or if a batch's descriptions outlived the frame that drew them,
    // this is where it would show.
    for step in 0..20 {
        scroll_by(&mut context, if step % 2 == 0 { -60. } else { 40. });
    }

    assert_eq!(
        runtime.live_callbacks(),
        settled,
        "scrolling must not register a single handler; rows are rebuilt every frame, so \
         anything registered from one would never be released"
    );

    // And a real script render does not accumulate either. The count settles at
    // two generations rather than one because the view deliberately keeps the
    // previous snapshot alive — a build that throws still has an interface to
    // show — and that is a fixed two however many renders follow.
    for _ in 0..5 {
        redraw_and_read(&mut context, &view);
    }
    assert_eq!(
        runtime.live_callbacks(),
        settled * 2,
        "a rendered generation and the one it replaced, and nothing older"
    );
}

#[gpui::test]
fn a_row_cannot_register_a_handler_of_its_own(cx: &mut TestAppContext) {
    let (_runtime, _window, view, mut context) = mount_virtual_list(
        cx,
        "try { row.on_click(() => {}); } catch (error) { this.refused = String(error.message); }",
    );

    let tree = redraw_and_read(&mut context, &view);
    assert!(
        tree.contains("refused"),
        "the view reports what the row was told: {tree}"
    );
    let refused = tree
        .split("refused ")
        .nth(1)
        .unwrap_or_else(|| panic!("no refusal in: {tree}"));
    assert!(
        refused.contains("on_item_click"),
        "the refusal must name what to use instead: {refused}"
    );
}

/// The row helpers an application already has all take the `cx` of the render
/// they were written in — `label(text, cx)`, `surface(cx)`, and so on. The item
/// renderer is a closure inside that same `render()`, so that `cx` is what it
/// has in hand, even though GPUI calls it from a layout pass of its own.
#[gpui::test]
fn an_item_renderer_may_use_the_cx_of_the_render_that_registered_it(cx: &mut TestAppContext) {
    let (_runtime, _window, view, mut context) = mount_virtual_list(
        cx,
        "try { row.text_color(cx.theme().colors.foreground); this.refused = \"none\"; } \
         catch (error) { this.refused = String(error.message); }",
    );

    let tree = redraw_and_read(&mut context, &view);
    assert!(
        tree.contains("refused none"),
        "the enclosing render's cx must still speak for a live call here: {tree}"
    );
}

/// And no further than that. Adopting one generation is what keeps the rule
/// checkable: a `cx` from a render that has already been replaced is as dead
/// inside a list as it is anywhere else.
#[gpui::test]
fn an_item_renderer_still_refuses_a_cx_from_an_earlier_render(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex, v_virtual_list } from "gpui-base";

export default class Rows extends View {
  init() {
    this.result = "not reached";
  }

  render(cx) {
    const previous = this.saved;
    this.saved = cx;
    return v_flex()
      .w(300)
      .h(400)
      .child(
        v_flex()
          .h(200)
          .child(
            v_virtual_list("rows", 500, 20, (index) => String(index), (range) => {
              const items = [];
              for (let index = range.start; index < range.end; index++) {
                items.push(div().h(20).child(`row ${index}`));
              }
              if (previous) {
                try {
                  previous.theme();
                  this.result = "accepted";
                } catch (error) {
                  this.result = "refused";
                }
              }
              return items;
            }),
          ),
      )
      .child(`result ${this.result}`);
  }
}
"#;
    let view_type = runtime.load_source("stale-rows.js", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("view")
        .read_with(&context, |root, _| root.0.clone());

    // The first pass has no earlier render to reach for; the second does, and
    // the third is what shows what the second decided.
    redraw_and_read(&mut context, &view);
    let tree = redraw_and_read(&mut context, &view);
    assert!(
        tree.contains("result refused"),
        "only the render that registered the renderer is adopted: {tree}"
    );
}

#[gpui::test]
fn a_virtual_list_reports_which_row_was_clicked(cx: &mut TestAppContext) {
    assert_a_click_reports_the_row_key(cx, &virtual_list_source(""), 50.);
}

/// The hit box belongs to the item it was painted for, not to the position the
/// item happened to occupy. A queued event from the previous snapshot
/// deliberately arrives after the first handler has reordered the collection.
#[gpui::test]
fn a_virtual_list_click_keeps_the_stable_item_key_across_reordering(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex, v_virtual_list } from "gpui-base";

export default class Rows extends View {
  init() {
    this.items = [{ key: "alpha" }, { key: "beta" }];
    this.clicked = [];
  }
  render(cx) {
    return v_flex()
      .w(300)
      .h(100)
      .child(
        v_virtual_list(
          "rows",
          this.items.length,
          40,
          (index) => this.items[index].key,
          (range) => {
            const rows = [];
            for (let index = range.start; index < range.end; index++) {
              rows.push(div().h(40).child(this.items[index].key));
            }
            return rows;
          },
        ).on_item_click((key, cx) => {
          this.clicked.push(key);
          this.items.reverse();
          cx.notify();
        }),
      )
      .child(`clicked ${this.clicked.join(",")}`);
  }
}
"#;
    let view_type = runtime.load_source("stable-list.js", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window.root(&mut context).expect("view");

    let old_callback = context.update(|_, cx| {
        let snapshot = view.read(cx).snapshot().expect("initial snapshot");
        snapshot
            .arena()
            .node(snapshot.root())
            .expect("root")
            .children()
            .iter()
            .find_map(|child| {
                snapshot
                    .arena()
                    .node(*child)?
                    .ops()
                    .iter()
                    .find_map(|op| match op {
                        crate::spec::SpecOp::Callback("on_item_click", callback) => Some(*callback),
                        _ => None,
                    })
            })
            .expect("item click callback")
    });

    context.simulate_click(point(px(150.), px(20.)), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    // Emulate a queued event from the hit box painted by the previous
    // snapshot. Its payload is the key captured before the reorder.
    context.update(|window, cx| runtime.dispatch_item_key(old_callback, "alpha", window, cx));
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("clicked alpha,alpha"),
        "an old event must keep addressing the item that owned its hit box: {tree}"
    );
}

#[gpui::test]
fn item_sizes_are_taken_as_one_extent_or_one_per_item(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex, v_virtual_list, h_virtual_list } from "gpui-base";

export default class Both extends View {
  render(cx) {
    return v_flex()
      .child(v_virtual_list("uniform", 3, 20, (index) => String(index), () => []))
      .child(h_virtual_list("explicit", 3, [10, 20, 30], (index) => String(index), () => []));
  }
}
"#;
    let view_type = runtime.load_source("both.js", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    // One number and three numbers describe the same three items: the count is
    // the list's own argument either way, which is what keeps a hundred
    // thousand uniform rows from crossing the boundary a hundred thousand
    // times.
    assert!(
        tree.contains("v_virtual_list \"uniform\" \u{d7}3"),
        "a uniform extent must still size every item: {tree}"
    );
    assert!(
        tree.contains("h_virtual_list \"explicit\" \u{d7}3"),
        "an explicit list of extents must survive: {tree}"
    );
}

#[gpui::test]
fn item_sizes_that_disagree_with_the_item_count_are_refused(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_virtual_list } from "gpui-base";

export default class Mismatched extends View {
  render() {
    return v_virtual_list("rows", 3, [10, 20], (index) => String(index), () => []);
  }
}
"#;
    let view_type = runtime.load_source("mismatched.js", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("two extents for three items must fail where it was written");
    assert!(
        error.to_string().contains("item sizes"),
        "the error must say which argument disagrees: {error}"
    );
}

#[gpui::test]
fn virtual_lists_share_one_bounded_host_allocation_budget_per_render(cx: &mut TestAppContext) {
    let message = render_error(
        cx,
        "oversized-lists.js",
        r#"
import { div, View } from "gpui-kit";
import { v_flex, v_virtual_list } from "gpui-base";

export default class LargeLists extends View {
  render() {
    return v_flex()
      .child(v_virtual_list("first", 600000, 20, (index) => String(index), () => []))
      .child(v_virtual_list("second", 600000, 20, (index) => String(index), () => []));
  }
}
"#,
    );
    assert!(
        // "lists", not "virtual lists": one budget covers every lazy list in a
        // render, `list` and `uniform_list` included.
        message.contains("lists in one render"),
        "the error must identify the aggregate host allocation boundary: {message}"
    );
}

/// An overlay's content is a function, and what it closes over is another
/// view's state -- the only contract `open_dialog` can have, since it answers a
/// depth and not a handle. So the overlay has to rebuild from the script, not
/// from the description it was opened with: a dialog that looks up what
/// somebody typed would otherwise show the answer to nothing for as long as it
/// is open.
#[gpui::test]
fn a_dialog_rebuilds_from_the_state_it_closes_over(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Probe extends View {
  init(_props, cx) {
    this.answer = "pending";
    cx.timer.after(5, () => {
      window.open_dialog(() => v_flex().child(`dialog:${this.answer}`), {});
    });
    cx.timer.after(30, () => {
      this.answer = "settled";
      window.refresh();
    });
  }
  render() {
    return v_flex().child(`view:${this.answer}`);
  }
}
"#;
    let view_type = runtime
        .load_source("dialog-rebuild.js", source)
        .expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let (root, context) = cx.add_window_view(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        let view = cx.new(|_| ScriptView::new(runtime_for_view, object));
        crate::root::ShellRoot::new(view.into(), window, cx)
    });
    context
        .executor()
        .advance_clock(std::time::Duration::from_millis(10));
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let dialog = context
        .update(|_, cx| root.read(cx).topmost_dialog().cloned())
        .expect("the dialog the script opened")
        .downcast::<ScriptView>()
        .expect("a script dialog");
    let opened = context.update(|_, cx| {
        dialog
            .read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(opened.contains("dialog:pending"), "{opened}");

    // The state the closure reads moves, and nothing here is the dialog's own
    // view to notify -- `window.refresh()` is the call for exactly that.
    context
        .executor()
        .advance_clock(std::time::Duration::from_millis(20));
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let refreshed = context.update(|_, cx| {
        dialog
            .read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        refreshed.contains("dialog:settled"),
        "the dialog must rebuild from the state it closes over:\n{refreshed}"
    );
}

#[gpui::test]
fn a_virtual_list_rejects_a_sparse_item_size_array_without_allocating_it(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { div, View } from "gpui-kit";
import { v_virtual_list } from "gpui-base";

export default class SparseSizes extends View {
  render() {
    return v_virtual_list("rows", 2147483647, new Array(2147483647), (index) => String(index), () => []);
  }
}
"#;
    let view_type = runtime
        .load_source("sparse-sizes.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        context.update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
    }));
    let result = result.expect("a sparse array must not panic or abort the host");
    let error = result.expect_err("the sparse size array must be rejected");
    assert!(
        error.to_string().contains("item_sizes"),
        "the error must identify the oversized argument: {error}"
    );
}

/// An application effect installs native state that outlives the render that
/// asked for it. Retiring the generation that asked must take that state back
/// down: keys are scoped per generation, so a reload cannot replace the
/// previous generation's install, and waiting for the root view to be released
/// would leave a reloading application accumulating one install per reload.
#[gpui::test]
fn retiring_an_application_generation_runs_its_app_effect_cleanups(cx: &mut TestAppContext) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().unwrap();
    let directory = std::env::temp_dir().join(format!(
        "gpui-shell-app-effect-retire-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("main.js"),
        "export default class Effectful { render() { return \"effect\"; } }\n",
    )
    .unwrap();

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view_type = context
        .update(|window, cx| {
            let (_scope, _) = crate::scope::enter_runtime(
                &runtime,
                window,
                cx,
                crate::scope::ScopePhase::Event,
                None,
            );
            runtime.load_app(&directory, "main.js")
        })
        .unwrap();
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .unwrap();
    let application = object.application_generation().unwrap();
    let runtime_for_view = runtime.clone();
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime_for_view, object)));

    let installs = Arc::new(AtomicUsize::new(0));
    let cleanups = Arc::new(AtomicUsize::new(0));
    let install_count = installs.clone();
    let cleanup_count = cleanups.clone();
    context
        .update(|window, cx| {
            runtime.schedule_component_app_effect(
                application.clone(),
                view.downgrade(),
                "menu-bar".to_owned(),
                "revision-1".to_owned(),
                window,
                cx,
                Box::new(move |_| {
                    install_count.fetch_add(1, Ordering::SeqCst);
                    Box::new(move |_: &mut gpui::App| {
                        cleanup_count.fetch_add(1, Ordering::SeqCst);
                    })
                }),
            )
        })
        .unwrap();
    context.run_until_parked();
    assert_eq!(
        installs.load(Ordering::SeqCst),
        1,
        "the effect must install"
    );
    assert_eq!(
        cleanups.load(Ordering::SeqCst),
        0,
        "an installed effect stays installed while its generation is live"
    );

    context.update(|_, cx| runtime.release_application_generation(&application, cx));

    assert_eq!(
        cleanups.load(Ordering::SeqCst),
        1,
        "retiring the generation must run the cleanup, not wait for the view"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

// ---------------------------------------------------------------------------
// `list` and `uniform_list`: GPUI's own lazy lists, driven from script.

fn uniform_list_source() -> &'static str {
    r#"
import { div, View, uniform_list } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Rows extends View {
  init() {
    this.range = [0, 0];
    this.clicked = -1;
  }

  render(cx) {
    return v_flex()
      .w(300)
      .h(400)
      .child(
        v_flex()
          .h(200)
          .child(
            uniform_list("rows", 500, (index) => String(index), (range) => {
              this.range = [range.start, range.end];
              const items = [];
              for (let index = range.start; index < range.end; index++) {
                items.push(div().h(20).child(`row ${index}`));
              }
              return items;
            }).on_item_click((key, cx) => {
              this.clicked = key;
              cx.notify();
            }),
          ),
      )
      .child(`range ${this.range[0]}..${this.range[1]} clicked ${this.clicked}`);
  }
}
"#
}

/// Rows of two heights, so the list has to measure each one: a uniform guess
/// from the first row would place every later row wrong.
fn measured_list_source() -> &'static str {
    r#"
import { div, View, list } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Rows extends View {
  init() {
    this.lo = -1;
    this.hi = -1;
    this.shown = [-1, -1];
    this.clicked = -1;
  }

  render(cx) {
    // What the previous frame's layout asked for; the item renderer runs after
    // this render, from inside layout, so the report is always one frame old.
    this.shown = [this.lo, this.hi];
    this.lo = -1;
    this.hi = -1;
    return v_flex()
      .w(300)
      .h(400)
      .child(
        v_flex()
          .h(200)
          .child(
            list("rows", 500, (index) => String(index), (index) => {
              if (this.lo < 0 || index < this.lo) this.lo = index;
              if (index > this.hi) this.hi = index;
              return div().h(index % 2 === 0 ? 20 : 40).child(`row ${index}`);
            }).on_item_click((key, cx) => {
              this.clicked = key;
              cx.notify();
            }),
          ),
      )
      .child(`range ${this.shown[0]}..${this.shown[1] + 1} clicked ${this.clicked}`);
  }
}
"#
}

/// A 200px box of 20px rows shows about ten of them, and scrolling moves which
/// ten the script is asked for. Shared by the two lists that take a range.
fn assert_the_visible_window_follows_the_scroll(cx: &mut TestAppContext, source: &str) {
    let (_runtime, _window, view, mut context) = mount_list_source(cx, source);

    let (start, end) = reported_range(&redraw_and_read(&mut context, &view));
    assert_eq!(start, 0, "an unscrolled list starts at its first item");
    assert!(
        (10..=13).contains(&end),
        "a 200px box of 20px rows shows about ten of five hundred, not {end}"
    );

    // Ten rows down. The script has to be asked again, with a different range:
    // that it is asked at all is the whole of what separates these components
    // from every other one, and that the range moves is what makes them lists
    // rather than a window onto the first screenful.
    scroll_by(&mut context, -200.);

    let (scrolled_start, scrolled_end) = reported_range(&redraw_and_read(&mut context, &view));
    assert_eq!(
        scrolled_start, 10,
        "200px of 20px rows is ten items; the window must start there"
    );
    assert!(
        scrolled_end > end,
        "the window must have moved down the collection: {scrolled_start}..{scrolled_end}"
    );
}

/// Rows are twenty pixels tall and the list starts at the top of the window, so
/// the third one covers 40..60 and its stable key is `2`.
fn assert_a_click_reports_the_row_key(cx: &mut TestAppContext, source: &str, y: f32) {
    let (_runtime, _window, view, mut context) = mount_list_source(cx, source);

    context.simulate_click(point(px(150.), px(y)), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = redraw_and_read(&mut context, &view);
    assert!(
        tree.contains("clicked 2"),
        "the click must arrive with the item's stable key: {tree}"
    );
}

/// Loads one script source as the window's own root view and draws it once.
///
/// The view has to be the window's own root. A helper that drew it once into a
/// throwaway element would leave every later frame going to the real root
/// instead, and a lazy list only says anything once it has been laid out more
/// than once.
fn mount_list_source(
    cx: &mut TestAppContext,
    source: &str,
) -> (
    Rc<ShellRuntime>,
    gpui::WindowHandle<RootedScriptView>,
    gpui::Entity<ScriptView>,
    VisualTestContext,
) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source("rows.js", source).expect("load");

    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_view
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        RootedScriptView(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window
        .root(&mut context)
        .expect("view")
        .read_with(&context, |root, _| root.0.clone());
    (runtime, window, view, context)
}

#[gpui::test]
fn a_uniform_list_describes_only_the_visible_window_and_follows_the_scroll(
    cx: &mut TestAppContext,
) {
    assert_the_visible_window_follows_the_scroll(cx, uniform_list_source());
}

#[gpui::test]
fn a_uniform_list_reports_which_row_was_clicked(cx: &mut TestAppContext) {
    assert_a_click_reports_the_row_key(cx, uniform_list_source(), 50.);
}

#[gpui::test]
fn a_list_measures_each_item_and_follows_the_scroll(cx: &mut TestAppContext) {
    let (_runtime, _window, view, mut context) = mount_list_source(cx, measured_list_source());

    let (start, end) = reported_range(&redraw_and_read(&mut context, &view));
    assert_eq!(start, 0, "an unscrolled list starts at its first item");
    // 20 + 40 + 20 + 40 + 20 + 40 + 20 fills the 200px box with seven rows, and
    // the list draws a short band past the fold so it has measured ground to
    // scroll into. A list that placed every row by the first one's 20px would
    // put eighteen in the same space.
    assert!(
        (7..=13).contains(&end),
        "a 200px box of alternating 20px and 40px rows shows about seven plus the \
         overdraw band, not {end}"
    );

    scroll_by(&mut context, -200.);

    let (_, scrolled_end) = reported_range(&redraw_and_read(&mut context, &view));
    assert!(
        scrolled_end > end,
        "the window must have moved down the collection: ends at {scrolled_end}, was {end}"
    );
}

#[gpui::test]
fn a_list_reports_which_item_was_clicked(cx: &mut TestAppContext) {
    // Alternating heights: row 0 covers 0..20, row 1 covers 20..60, row 2
    // covers 60..80.
    assert_a_click_reports_the_row_key(cx, measured_list_source(), 70.);
}
