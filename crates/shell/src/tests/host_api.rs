//! Script-level contracts for small host APIs.
//!
//! These deliberately render a real `ScriptView`: success means the API was
//! installed in the `gpui-kit` module, ran inside a live host scope, and produced
//! state JavaScript could publish to a snapshot.

use std::ops::Deref;

use gpui::{Entity, IntoElement as _, TestAppContext, VisualTestContext};

use crate::{Capabilities, HostError, HostModule, HostValue, ScriptView, ShellRuntime};

const CLIPBOARD_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Probe extends View {
  init(_props, cx) {
    cx.write_to_clipboard("written by JavaScript");
    this.value = cx.read_from_clipboard();
  }
  render() { return v_flex().child(this.value); }
}
"#;

const CANCELLED_TIMER_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Probe extends View {
  init(_props, cx) {
    this.fired = false;
    this.timer = cx.timer.after(0, () => { this.fired = true; });
    this.timer.cancel();
  }
  render() {
    return v_flex().child(`${this.timer.is_done()}|${this.fired}`);
  }
}
"#;

const WITHDRAWN_MODULE_PROBE: &str = r#"
import { View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { increment } from "calculator";

export default class Probe extends View {
  render(cx) {
    try {
      return v_flex().child(`answer:${increment(41)}`);
    } catch (error) {
      return v_flex().child(`refused:${error.message}`);
    }
  }
}
"#;

const ASYNC_MODULE_PROBE: &str = r#"
import { View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { double, refuse } from "slow";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      const answer = await double(21);
      let refusal = "none";
      try {
        await refuse("nope");
      } catch (error) {
        refusal = error.message;
      }
      this.state = `answer:${answer}|refused:${refusal}`;
      cx.notify();
    });
  }

  render() {
    return v_flex().child(this.state);
  }
}
"#;

const HOST_MODULE_PROBE: &str = r#"
import { View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { increment } from "calculator";

export default class Probe extends View {
  init() { this.answer = increment(41); }
  render(cx) { return v_flex().child(`answer:${this.answer}`); }
}
"#;

#[gpui::test]
fn javascript_round_trips_text_through_the_host_clipboard(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(
        cx,
        Capabilities::new()
            .clipboard_read(true)
            .clipboard_write(true),
        "clipboard.js",
        CLIPBOARD_PROBE,
    );
    let _keep_runtime_alive = runtime;

    draw(&mut context, &view);
    let rendered = snapshot_text(&mut context, &view);
    assert!(
        rendered.contains("written by JavaScript"),
        "JavaScript did not read back the clipboard value it wrote: {rendered}"
    );
}

#[gpui::test]
fn cancelling_a_javascript_timer_prevents_its_callback(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(
        cx,
        Capabilities::new(),
        "cancelled-cx.timer.js",
        CANCELLED_TIMER_PROBE,
    );
    let _keep_runtime_alive = runtime;

    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot_text(&mut context, &view);
    assert!(
        rendered.contains("true|false"),
        "the cancelled timer either stayed live or fired: {rendered}"
    );
}

#[gpui::test]
fn javascript_imports_a_host_registered_module(cx: &mut TestAppContext) {
    cx.update(crate::init);
    crate::export_module(
        HostModule::new("calculator").function("increment", |arguments| {
            Ok(HostValue::from(arguments.number(0)? + 1.))
        }),
    )
    .expect("`calculator` is not a reserved name");
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("host_module.js", HOST_MODULE_PROBE)
        .expect("load script");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate script view");

    draw(&mut context, &view);
    let rendered = snapshot_text(&mut context, &view);
    assert!(
        rendered.contains("answer:42"),
        "the host argument/result bridge did not round-trip: {rendered}"
    );

    crate::clear_exported_modules();
}

/// An asynchronous host function answers with a promise, and its work runs off
/// the main thread.
///
/// End-to-end because that is where the parts have to agree: the registry's
/// two-half closure, the generated arrow rather than a bound stub, the
/// scheduler's promise, and the scope the continuation is resumed in.
#[gpui::test]
fn javascript_awaits_an_asynchronous_host_function(cx: &mut TestAppContext) {
    cx.update(crate::init);
    crate::export_module(
        HostModule::new("slow")
            .async_function("double", |arguments| {
                let value = arguments.number(0)?;
                Ok(async move { Ok(HostValue::from(value * 2.)) })
            })
            // A failure inside the future rejects the promise, so the script
            // catches it the way it catches any other async refusal.
            .async_function("refuse", |arguments| {
                let reason = arguments.string(0)?.to_owned();
                Ok(async move { Err(HostError::new(format!("refused: {reason}"))) })
            }),
    )
    .expect("`slow` is not a reserved name");

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("async.js", ASYNC_MODULE_PROBE)
        .expect("load script");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate script view");

    draw(&mut context, &view);
    assert!(
        snapshot_text(&mut context, &view).contains("pending"),
        "the call answered before it was awaited"
    );

    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot_text(&mut context, &view);
    assert!(
        rendered.contains("answer:42"),
        "the promise did not resolve with the future's value: {rendered}"
    );
    assert!(
        rendered.contains("refused: nope"),
        "a failing future did not reject its promise: {rendered}"
    );

    crate::clear_exported_modules();
}

/// An import fixes the *names* a module exports, not the functions behind them.
///
/// This is the one semantic an import could plausibly have cost, so it is the
/// one worth pinning: a script holding a function it imported before the host
/// withdrew the module gets a refusal on the next call, not the withdrawn
/// closure. Every export is a stub that resolves through the registry.
#[gpui::test]
fn withdrawing_a_module_refuses_a_call_through_an_already_imported_name(cx: &mut TestAppContext) {
    cx.update(crate::init);
    crate::export_module(
        HostModule::new("calculator").function("increment", |arguments| {
            Ok(HostValue::from(arguments.number(0)? + 1.))
        }),
    )
    .expect("`calculator` is not a reserved name");
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("withdrawn.js", WITHDRAWN_MODULE_PROBE)
        .expect("load script");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate script view");

    draw(&mut context, &view);
    assert!(
        snapshot_text(&mut context, &view).contains("answer:42"),
        "the module did not answer while it was registered"
    );

    crate::clear_exported_modules();
    // The view redraws from its cached snapshot unless something asks it to
    // describe itself again; withdrawing a module is a host act it cannot see.
    context.update(|_, cx| view.update(cx, |view, cx| view.refresh(cx)));
    draw(&mut context, &view);
    let rendered = snapshot_text(&mut context, &view);
    assert!(
        rendered.contains("refused:"),
        "the withdrawn module still answered through the imported name: {rendered}"
    );
    assert!(
        rendered.contains("registered none"),
        "the refusal should say the host has no modules: {rendered}"
    );
}

fn script_view(
    cx: &mut TestAppContext,
    capabilities: Capabilities,
    name: &str,
    source: &str,
) -> (
    std::rc::Rc<ShellRuntime>,
    VisualTestContext,
    Entity<ScriptView>,
) {
    cx.update(crate::init);
    crate::set_capabilities(capabilities);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source(name, source).expect("load script");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate script view");
    (runtime, context, view)
}

fn draw(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view.into_any_element(),
    );
}

fn snapshot_text(context: &mut VisualTestContext, view: &Entity<ScriptView>) -> String {
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    })
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
