#[path = "../src/shell/support.rs"]
mod support;

#[path = "../src/shell/typed_child.rs"]
mod typed_child;

#[path = "../src/shell/command/mod.rs"]
mod command;

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
            "command-{}-{}",
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
    command::register(&mut registry).unwrap();
    let runtime =
        gpui_shell::ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).unwrap();
    let loaded = runtime.load_application(&app.0, "main.js").unwrap();
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let slot = mounted.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime.mount_application(&loaded, window, cx).unwrap();
        *slot.borrow_mut() = Some(view.clone());
        gpui_component::Root::new(view, window, cx)
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
fn command_catalog_is_closed() {
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    command::register(&mut registry).unwrap();
    let frozen = registry.freeze().unwrap();
    assert_eq!(frozen.states().count(), 1);
    assert_eq!(
        frozen
            .descriptors()
            .map(|item| item.name())
            .collect::<Vec<_>>(),
        [
            "CommandItem",
            "CommandGroup",
            "CommandSeparator",
            "Command",
            "NativeMenuItem",
            "NativeMenuSeparator",
            "NativeMenuTrigger"
        ]
    );
}

#[gpui::test]
fn retained_command_typed_entries_query_and_confirm_callbacks_are_native(cx: &mut TestAppContext) {
    let source = r#"
import { View, div } from "gpui-kit";
import { CommandState, Command, CommandItem, CommandGroup, CommandSeparator } from "gpui-component";
export default class App extends View {
 init(){this.state=CommandState();this.query="";this.confirm="none";this.actions=0;}
 render(){return div().on_action("open",(_event,cx)=>{this.actions++;cx.notify();})
  .child(new Command(this.state).placeholder("Find command").max_height(240).p(2).header(div().child("Header factory")).footer(div().child("Footer factory"))
   .on_query((query,cx)=>{this.query=query;cx.notify();})
   .on_confirm((section,row,cx)=>{this.confirm=`${section}:${row}`;cx.notify();})
   .child(new CommandItem("Alpha").keyword("first").action("open").content(div().child("Alpha custom row")))
   .child(new CommandSeparator())
   .child(new CommandGroup("Group").child(new CommandItem("Beta").checked(true).action("open").content(div().child("Beta custom row")))))
  .child(`State: ${this.query}|${this.confirm}|${this.actions}`);}
}
"#;
    let (mut context, view, _app) = mount(cx, source);
    let assert_builds = |phase: &str, builds: &[&str], header, footer, item| {
        assert_eq!(
            builds.iter().filter(|kind| **kind == "header").count(),
            header,
            "{phase} header builds: {builds:?}"
        );
        assert_eq!(
            builds.iter().filter(|kind| **kind == "footer").count(),
            footer,
            "{phase} footer builds: {builds:?}"
        );
        assert_eq!(
            builds.iter().filter(|kind| **kind == "item").count(),
            item,
            "{phase} item builds: {builds:?}"
        );
        assert_eq!(
            builds.len(),
            header + footer + item,
            "{phase} unexpected probe labels: {builds:?}"
        );
    };
    let mount_builds = command::command_probe::take();
    assert_builds("mount", &mount_builds, 1, 1, 5);
    draw(&mut context);
    let first_draw_builds = command::command_probe::take();
    assert_builds("first draw", &first_draw_builds, 1, 1, 5);
    draw(&mut context);
    let second_draw_builds = command::command_probe::take();
    assert_builds("second draw", &second_draw_builds, 1, 1, 5);
    context.update(|_, cx| assert_eq!(view.read(cx).build_error(), None));
    context.simulate_click(point(px(40.), px(50.)), Modifiers::default());
    context.simulate_keystrokes("b");
    draw(&mut context);
    let queried = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(queried.contains("State: b|none|0"), "{queried}");
    context.simulate_keystrokes("down enter");
    draw(&mut context);
    let confirmed = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(confirmed.contains("State: b|1:0|1"), "{confirmed}");
    command::command_probe::take();
    context.update(|_, cx| view.update(cx, |view, cx| view.refresh(cx)));
    draw(&mut context);
    let refresh_builds = command::command_probe::take();
    assert_builds("refresh", &refresh_builds, 2, 2, 4);
    let refreshed = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(refreshed.contains("State: b|1:0|1"), "{refreshed}");
    context.simulate_keystrokes("enter");
    draw(&mut context);
    let live = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(live.contains("State: b|1:0|2"), "{live}");
}

const NATIVE_MENU_SOURCE: &str = r#"
import { View, div } from "gpui-kit";
import { NativeMenuTrigger, NativeMenuItem, NativeMenuSeparator } from "gpui-component";
export default class App extends View { init(_props,cx){this.hits=0;this.errors=0;this.focus=cx.focus_handle();this.focus.focus();} render(){return div().size_full().track_focus(this.focus).on_action("open",(_event,cx)=>{this.hits++;cx.notify();})
 .child(new NativeMenuTrigger("native","Actions").absolute().left(0).top(0).w(140).h(40).on_effect_error((_message,cx)=>{this.errors+=10;cx.notify();}).on_effect_error((_message,cx)=>{this.errors++;cx.notify();})
  .child(new NativeMenuItem("Open","open")).child(new NativeMenuSeparator()).child(new NativeMenuItem("Disabled","disabled").disabled(true)))
 .child(`Menu: ${this.hits}|${this.errors}`);}}
"#;

/// What the trigger owes on every platform: one click, one keyed show effect,
/// and a generation that survives a refresh.
///
/// What the menu does once it is open is platform business — see the test
/// below.
#[gpui::test]
fn native_menu_trigger_runs_one_keyed_show_effect_per_click(cx: &mut TestAppContext) {
    let (mut context, view, _app) = mount(cx, NATIVE_MENU_SOURCE);
    draw(&mut context);
    command::test_probe::take_shown();

    context.simulate_click(point(px(20.), px(20.)), Modifiers::default());
    draw(&mut context);
    assert_eq!(
        command::test_probe::take_shown(),
        1,
        "one click must execute one keyed native-menu show effect"
    );

    let tree = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(
        tree.contains("NativeMenuItem :disabled[Bool(true)]"),
        "{tree}"
    );
    assert!(
        tree.contains("Menu: 0|0"),
        "no effect error may be reported: {tree}"
    );
}

/// Selecting an item dispatches its `ShellAction`, and a disabled item does not.
///
/// Only where the menu is drawn in the window. On macOS and Windows
/// `NativeMenu::show` hands the items to the platform and runs off GPUI's call
/// stack, so there is nothing in the test window for `simulate_keystrokes` to
/// reach — the assertions below would be measuring the harness, not the
/// binding. Selection on those platforms is the OS's to deliver.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[gpui::test]
fn native_menu_selection_dispatches_a_shell_action(cx: &mut TestAppContext) {
    let (mut context, view, _app) = mount(cx, NATIVE_MENU_SOURCE);
    draw(&mut context);
    command::test_probe::take_shown();

    context.simulate_click(point(px(20.), px(20.)), Modifiers::default());
    draw(&mut context);
    context.simulate_keystrokes("down enter");
    draw(&mut context);
    let tree = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(tree.contains("Menu: 1|0"), "{tree}");

    context.simulate_click(point(px(20.), px(20.)), Modifiers::default());
    draw(&mut context);
    context.simulate_click(point(px(40.), px(77.)), Modifiers::default());
    draw(&mut context);
    let disabled = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(
        disabled.contains("Menu: 1|0"),
        "disabled native item dispatched: {disabled}"
    );

    // A refresh rebuilds the callbacks and the effect generation. Dismissing
    // first is what makes the next click reach the trigger again, and only the
    // in-window menu can be dismissed from inside the test window — which is
    // the other reason this assertion lives here rather than above.
    context.simulate_keystrokes("escape");
    draw(&mut context);
    context.update(|_, cx| view.update(cx, |view, cx| view.refresh(cx)));
    draw(&mut context);
    command::test_probe::take_shown();
    context.simulate_click(point(px(20.), px(20.)), Modifiers::default());
    draw(&mut context);
    assert_eq!(
        command::test_probe::take_shown(),
        1,
        "refreshed trigger callback/effect generation must remain live"
    );
    context.simulate_keystrokes("down enter");
    draw(&mut context);
    let refreshed = context.update(|_, cx| view.read(cx).snapshot().unwrap().debug_tree());
    assert!(refreshed.contains("Menu: 2|0"), "{refreshed}");
}
