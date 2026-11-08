#![cfg(feature = "components")]
#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;

use gpui_shell::gpui as shell_gpui;

use std::{
    fs,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};

use gpui::{Entity, IntoElement as _, ParentElement as _, TestAppContext, VisualTestContext};

static NEXT_APP: AtomicU64 = AtomicU64::new(0);

struct TempApp(PathBuf);

impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "gpui-shell-public-host-{}-{}",
            std::process::id(),
            NEXT_APP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create temporary application directory");
        fs::write(path.join("main.js"), source).expect("write temporary application entry");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempApp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove temporary application directory");
    }
}

struct Empty;

impl shell_gpui::Render for Empty {
    fn render(
        &mut self,
        _: &mut shell_gpui::Window,
        _: &mut shell_gpui::Context<Self>,
    ) -> impl shell_gpui::IntoElement {
        shell_gpui::div()
    }
}

fn mount(
    cx: &mut TestAppContext,
    source: &str,
) -> (
    VisualTestContext,
    Entity<gpui_shell::ScriptView>,
    std::rc::Rc<gpui_shell::ShellRuntime>,
) {
    cx.update(|cx| {
        gpui_shell::components::init(cx);
    });
    let app = TempApp::new(source);
    let runtime = gpui_shell::components::new_isolated_runtime().expect("runtime");
    let loaded = runtime
        .load_application(app.path(), "main.js")
        .expect("load application");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.mount_application(&loaded, window, cx))
        .expect("mount application");
    (context, view, runtime)
}

#[shell_gpui::test]
fn public_host_api_mounts_and_materializes_registered_component_js(cx: &mut TestAppContext) {
    let source = r#"
import { div, View } from "gpui-kit";
import { Spinner } from "gpui-component";

export default class ComponentApp extends View {
  render() {
    return div().p(2).child("loading").child(new Spinner().size("small"));
  }
}
"#;
    let (mut context, view, _runtime) = mount(cx, source);

    context.draw(
        shell_gpui::Point::default(),
        shell_gpui::size(shell_gpui::px(400.), shell_gpui::px(300.)),
        {
            let view = view.clone();
            move |_, _| view.into_any_element()
        },
    );

    let tree = context.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.build_error(), None);
        view.snapshot().expect("render snapshot").debug_tree()
    });
    assert!(tree.contains("div"), "{tree}");
    assert!(tree.contains("loading"), "{tree}");
    assert!(tree.contains("Spinner"), "{tree}");
    assert!(tree.contains(":size(registered)"), "{tree}");
}

#[shell_gpui::test]
fn registered_component_argument_errors_are_reported_during_render(cx: &mut TestAppContext) {
    let source = r#"
import { View } from "gpui-kit";
import { Spinner } from "gpui-component";

export default class InvalidComponentApp extends View {
  render() {
    return new Spinner().size("enormous");
  }
}
"#;
    let (mut context, view, _runtime) = mount(cx, source);

    context.draw(
        shell_gpui::Point::default(),
        shell_gpui::size(shell_gpui::px(400.), shell_gpui::px(300.)),
        {
            let view = view.clone();
            move |_, _| view.into_any_element()
        },
    );

    let error = context.update(|_, cx| {
        view.read(cx)
            .build_error()
            .expect("invalid method call must fail the render")
            .to_owned()
    });
    assert!(
        error.contains("size(size) expects `xsmall`, `small`, `medium`, `large`"),
        "{error}"
    );
    assert!(error.contains("at render"), "{error}");
}

#[shell_gpui::test]
fn all_retained_form_bindings_materialize_and_keep_their_state_across_frames(
    cx: &mut TestAppContext,
) {
    let source = r#"
import { div, View } from "gpui-kit";
import {
  Calendar, CalendarState, ColorPicker, ColorPickerState,
  DatePicker, DatePickerState, Input, InputState, NumberInput,
  OtpInput, OtpState, Slider, SliderState,
} from "gpui-component";
export default class RetainedForms extends View {
  init() {
    this.input = InputState();
    this.otp = OtpState(6);
    this.slider = SliderState();
    this.color = ColorPickerState();
    this.calendar = CalendarState();
    this.date = DatePickerState();
  }
  render() {
    return div()
      .child(new Input(this.input).aria_label("Project").disabled(false))
      .child(new NumberInput(this.input).placeholder("Quantity").disabled(true))
      .child(new OtpInput(this.otp).w(320).groups(3).disabled(false))
      .child(new Slider(this.slider).vertical().reverse().disabled(false))
      .child(new ColorPicker(this.color).label("Accent").accessibility_label("Accent color"))
      .child(new Calendar(this.calendar).number_of_months(2))
      .child(new DatePicker(this.date).placeholder("Choose date").disabled(false));
  }
}
"#;
    let (mut context, view, _runtime) = mount(cx, source);
    for _ in 0..2 {
        context.draw(
            shell_gpui::Point::default(),
            shell_gpui::size(shell_gpui::px(400.), shell_gpui::px(300.)),
            {
                let view = view.clone();
                move |_, _| view.into_any_element()
            },
        );
        let tree = context.update(|_, cx| {
            let view = view.read(cx);
            assert_eq!(view.build_error(), None);
            view.snapshot().expect("retained OTP snapshot").debug_tree()
        });
        for expected in [
            "Input",
            "NumberInput",
            "OtpInput",
            "Slider",
            "ColorPicker",
            "Calendar",
            "DatePicker",
            ":aria_label(registered)",
            ":placeholder(registered)",
            ":groups(registered)",
            ":vertical(registered)",
            ":reverse(registered)",
            ":label(registered)",
            ":accessibility_label(registered)",
            ":number_of_months(registered)",
            ".w[Number(320.0)]",
        ] {
            assert!(tree.contains(expected), "missing `{expected}`:\n{tree}");
        }
    }
}

#[shell_gpui::test]
fn retained_otp_rejects_an_ordinary_child_during_public_host_materialization(
    cx: &mut TestAppContext,
) {
    let source = r#"
import { div, View } from "gpui-kit";
import { OtpInput, OtpState } from "gpui-component";
export default class InvalidOtp extends View {
  init() { this.otp = OtpState(6); }
  render() { return new OtpInput(this.otp).child(div().child("not allowed")); }
}
"#;
    let (mut context, view, _runtime) = mount(cx, source);
    context.draw(
        shell_gpui::Point::default(),
        shell_gpui::size(shell_gpui::px(400.), shell_gpui::px(300.)),
        {
            let view = view.clone();
            move |_, _| view.into_any_element()
        },
    );

    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .expect("invalid OTP still has a script snapshot")
            .debug_tree()
    });
    assert!(tree.contains("OtpInput"), "{tree}");
    assert!(tree.contains("not allowed"), "{tree}");
}

#[shell_gpui::test]
fn retained_state_constructor_rejects_rounded_overflow_from_js(cx: &mut TestAppContext) {
    let source = r#"
import { View } from "gpui-kit";
import { OtpInput, OtpState } from "gpui-component";
export default class InvalidOtpState extends View {
  init() { this.otp = OtpState(18446744073709551616); }
  render() { return new OtpInput(this.otp); }
}
"#;
    cx.update(|cx| {
        gpui_shell::components::init(cx);
    });
    let app = TempApp::new(source);
    let runtime = gpui_shell::components::new_isolated_runtime().expect("runtime");
    let loaded = runtime
        .load_application(app.path(), "main.js")
        .expect("load application");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let error = context
        .update(|window, cx| runtime.mount_application(&loaded, window, cx))
        .expect_err("rounded usize overflow must reject state construction");
    assert!(error.to_string().contains("positive integer"), "{error}");
}

#[shell_gpui::test]
fn loaded_applications_are_single_mount_and_runtime_bound(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_shell::components::init(cx);
    });
    let app = TempApp::new(
        r#"
import { div, View } from "gpui-kit";
export default class Reusable extends View { render() { return div().child("ok"); } }
"#,
    );
    let runtime = gpui_shell::components::new_isolated_runtime().expect("runtime");
    let loaded = runtime
        .load_application(app.path(), "main.js")
        .expect("load application");
    let other = gpui_shell::components::new_isolated_runtime().expect("second runtime");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let error = context
        .update(|window, cx| other.mount_application(&loaded, window, cx))
        .expect_err("another runtime must reject this loaded application");
    assert!(
        error.to_string().contains("different ShellRuntime"),
        "{error}"
    );
    context
        .update(|window, cx| runtime.mount_application(&loaded, window, cx))
        .expect("a foreign rejection does not consume the owner handle");
    let error = context
        .update(|window, cx| runtime.mount_application(&loaded, window, cx))
        .expect_err("a loaded application can only be mounted once");
    assert!(
        error.to_string().contains("already been mounted"),
        "{error}"
    );
}

#[shell_gpui::test]
fn failed_owner_mount_consumes_the_loaded_application(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_shell::components::init(cx);
    });
    let app = TempApp::new(
        r#"
import { View } from "gpui-kit";
export default class Broken extends View {
  init() { throw new Error("init failed"); }
  render() { return "unreachable"; }
}
"#,
    );
    let runtime = gpui_shell::components::new_isolated_runtime().expect("runtime");
    let loaded = runtime
        .load_application(app.path(), "main.js")
        .expect("load application");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let first = context
        .update(|window, cx| runtime.mount_application(&loaded, window, cx))
        .expect_err("init must fail the first mount");
    assert!(first.to_string().contains("init failed"), "{first}");
    let second = context
        .update(|window, cx| runtime.mount_application(&loaded, window, cx))
        .expect_err("a failed owner attempt consumes the application");
    assert!(
        second.to_string().contains("already been mounted"),
        "{second}"
    );
}

#[shell_gpui::test]
fn public_host_materializes_real_typed_compound_children_in_script_order(cx: &mut TestAppContext) {
    let source = r#"
import { View, div } from "gpui-kit";
import {
  Accordion, AccordionItem, Radio, RadioGroup,
  Stepper, StepperItem, Tab, TabBar,
} from "gpui-component";
export default class TypedCompounds extends View {
  render() {
    return div().v_flex().gap(8)
      .child(new Accordion("faq").w(500).multiple(true)
        .child(new AccordionItem().px(2).title(div().child("Question A")).open(true).child("Answer A"))
        .child(new AccordionItem().title(div().child("Question B")).child("Answer B")))
      .child(new TabBar("sections").w(500).selected_index(1).variant("underline")
        .child(new Tab().label("First"))
        .child(new Tab().label("Second")))
      .child(new Stepper("setup").w(500).selected_index(1)
        .child(new StepperItem().child("Account")).child(new StepperItem().child("Profile")))
      .child(new RadioGroup("density").w(500).selected_index(1)
        .child(new Radio("comfortable").px(2).label("Comfortable"))
        .child(new Radio("compact").label("Compact")));
  }
}
"#;
    let (mut context, view, _runtime) = mount(cx, source);
    context.draw(
        shell_gpui::Point::default(),
        shell_gpui::size(shell_gpui::px(700.), shell_gpui::px(600.)),
        {
            let view = view.clone();
            move |_, _| view.into_any_element()
        },
    );
    let tree = context.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.build_error(), None);
        view.snapshot().expect("typed render snapshot").debug_tree()
    });
    for component in [
        "AccordionItem",
        "Accordion",
        "Tab",
        "TabBar",
        "StepperItem",
        "Stepper",
        "Radio",
        "RadioGroup",
    ] {
        assert!(tree.contains(component), "missing {component}: {tree}");
    }
    let answer_a = tree.find("Answer A").expect("first accordion answer");
    let answer_b = tree.find("Answer B").expect("second accordion answer");
    assert!(answer_a < answer_b, "{tree}");
    let account = tree.find("Account").expect("first step label");
    let profile = tree.find("Profile").expect("second step label");
    assert!(account < profile, "{tree}");
    assert_eq!(tree.matches(".w[Number(500.0)]").count(), 4, "{tree}");
}

#[shell_gpui::test]
fn component_state_exports_do_not_shadow_same_named_gpui_base_exports(cx: &mut TestAppContext) {
    let source = r#"
import { View } from "gpui-kit";
import { InputState as BaseInputState } from "gpui-base";
import { Input, InputState as ComponentInputState } from "gpui-component";
export default class CoexistingStates extends View {
  init() {
    this.base_state = BaseInputState.new({ placeholder: "Search" });
    this.component_state = ComponentInputState();
  }
  render() { return new Input(this.component_state).aria_label("Name"); }
}
"#;
    let (mut context, view, _runtime) = mount(cx, source);
    context.draw(
        shell_gpui::Point::default(),
        shell_gpui::size(shell_gpui::px(400.), shell_gpui::px(120.)),
        {
            let view = view.clone();
            move |_, _| view.into_any_element()
        },
    );
    context.update(|_, cx| {
        assert_eq!(view.read(cx).build_error(), None);
        assert!(
            view.read(cx)
                .snapshot()
                .expect("render snapshot")
                .debug_tree()
                .contains("Input")
        );
    });
}

/// What the host's own component was handed, in materialization order.
static HOST_BADGE_LABELS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static HOST_INIT_CALLS: AtomicUsize = AtomicUsize::new(0);

struct HostBadge;

impl gpui_shell::ComponentMaterializer for HostBadge {
    fn materialize(
        &self,
        request: gpui_shell::MaterializeRequest<'_>,
    ) -> gpui_shell::anyhow::Result<shell_gpui::AnyElement> {
        let label = request
            .payload()
            .downcast_ref::<String>()
            .ok_or_else(|| gpui_shell::anyhow::anyhow!("HostBadge payload"))?
            .clone();
        HOST_BADGE_LABELS.lock().unwrap().push(label.clone());
        Ok(shell_gpui::div().child(label).into_any_element())
    }
}

fn host_badge(name: &'static str, export: &'static str) -> gpui_shell::ComponentDescriptor {
    gpui_shell::ComponentDescriptor::new(name, Arc::new(HostBadge))
        .with_constructors(vec![gpui_shell::ConstructorDescriptor::new(
            export,
            vec![gpui_shell::ArgumentDescriptor::new(
                "label",
                gpui_shell::ArgumentSchema::String,
            )],
            |args| match args {
                [gpui_shell::ComponentArgument::String(label)] => {
                    Ok(gpui_shell::ComponentPayload::new(label.clone()))
                }
                _ => Err("HostBadge expects a label".into()),
            },
        )])
        .with_methods(vec![])
}

/// The catalog's own startup, then the host's, as `components::registry`
/// tells a host to chain them.
fn host_init(cx: &mut shell_gpui::App) {
    gpui_component::init(cx);
    HOST_INIT_CALLS.fetch_add(1, Ordering::SeqCst);
}

#[shell_gpui::test]
fn an_embedding_host_registers_its_own_component_beside_the_catalog(cx: &mut TestAppContext) {
    let mut registry = gpui_shell::components::registry()
        .expect("catalog")
        .with_initializer(host_init);
    assert!(matches!(
        registry.register(host_badge("HostSpinner", "Spinner")),
        Err(gpui_shell::RegistryError::DuplicateExport("Spinner"))
    ));
    registry
        .register(host_badge("HostBadge", "HostBadge"))
        .expect("register the host component");
    let components = registry.freeze().expect("freeze");

    cx.update(|cx| gpui_shell::init_with_components(cx, &components));
    assert_eq!(HOST_INIT_CALLS.load(Ordering::SeqCst), 1);

    let source = r#"
import { div, View } from "gpui-kit";
import { HostBadge, Spinner } from "gpui-component";

export default class ExtendedCatalog extends View {
  render() {
    return div().child(new Spinner().size("small")).child(new HostBadge("ready"));
  }
}
"#;
    let app = TempApp::new(source);
    let runtime =
        gpui_shell::ShellRuntime::new_isolated_with_components(components).expect("runtime");
    let loaded = runtime
        .load_application(app.path(), "main.js")
        .expect("load application");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.mount_application(&loaded, window, cx))
        .expect("mount application");
    context.draw(
        shell_gpui::Point::default(),
        shell_gpui::size(shell_gpui::px(400.), shell_gpui::px(120.)),
        {
            let view = view.clone();
            move |_, _| view.into_any_element()
        },
    );

    let tree = context.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.build_error(), None);
        view.snapshot().expect("render snapshot").debug_tree()
    });
    assert!(tree.contains("Spinner"), "{tree}");
    assert!(tree.contains("HostBadge"), "{tree}");
    assert_eq!(*HOST_BADGE_LABELS.lock().unwrap(), ["ready"]);
}
