//! The JavaScript-facing process adapter.
//!
//! Unit tests in `crate::process` exercise the pipe, timeout and kill mechanics.
//! These tests cross the public script boundary: a real module calls
//! `process.run`, awaits its promise, and publishes what JavaScript observed in
//! a render snapshot.

use std::ops::Deref;

use gpui::{Entity, IntoElement as _, TestAppContext, VisualTestContext};

#[cfg(unix)]
use crate::ExecuteGrant;
use crate::{Capabilities, ScriptView, ShellRuntime};

#[cfg(unix)]
const OUTPUT_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import process from "process";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try {
        const output = await process.run("/bin/sh", [
          "-c",
          "printf out; printf err >&2; exit 7",
        ]);
        this.state = `${output.code}|${output.stdout}|${output.stderr}`;
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      cx.notify();
    });
  }

  render() {
    return v_flex().child(this.state);
  }
}
"#;

#[cfg(unix)]
const FAILURE_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import process from "process";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try {
        await process.run("/gpui-shell-command-that-does-not-exist");
        this.state = "unexpectedly resolved";
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      cx.notify();
    });
  }

  render() {
    return v_flex().child(this.state);
  }
}
"#;

#[cfg(unix)]
const OUTPUT_LIMIT_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import process from "process";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try {
        await process.run("/bin/sh", ["-c", "yes x | head -c 8388609"]);
        this.state = "unexpectedly resolved";
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      cx.notify();
    });
  }

  render() {
    return v_flex().child(this.state);
  }
}
"#;

const DENIAL_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import process from "process";

export default class Probe extends View {
  init(_props, cx) {
    try {
      process.run("gpui-shell-denied-command");
      this.state = "unexpectedly allowed";
    } catch (error) {
      this.state = [
        error.message,
        typeof process.kill,
        typeof process.setuid,
        typeof process.setgid,
        typeof process.env,
      ].join("|");
      process.nextTick((suffix) => {
        this.state += suffix;
        cx.notify();
      }, "|tick");
    }
  }
  render(cx) { return v_flex().child(this.state); }
}
"#;

#[cfg(unix)]
#[gpui::test]
fn process_promise_exposes_status_and_both_streams(cx: &mut TestAppContext) {
    let (_runtime, object, mut context) = probe(cx, "/bin/sh", "process-output.js", OUTPUT_PROBE);

    draw(&mut context, &object);
    assert!(snapshot_text(&mut context, &object).contains("pending"));
    context.run_until_parked();
    draw(&mut context, &object);

    let settled = snapshot_text(&mut context, &object);
    assert!(
        settled.contains("7|out|err"),
        "JavaScript did not observe the complete process result: {settled}"
    );
}

#[cfg(unix)]
#[gpui::test]
fn process_start_failure_rejects_the_javascript_promise(cx: &mut TestAppContext) {
    let command = "/gpui-shell-command-that-does-not-exist";
    let (_runtime, object, mut context) = probe(cx, command, "process-failure.js", FAILURE_PROBE);

    context.run_until_parked();
    draw(&mut context, &object);

    let settled = snapshot_text(&mut context, &object);
    assert!(
        settled.contains("rejected:"),
        "promise did not reject: {settled}"
    );
    assert!(
        settled.contains(command) && settled.contains("failed"),
        "the rejection did not identify the failed command: {settled}"
    );
}

#[cfg(unix)]
#[gpui::test]
fn process_output_limit_rejects_the_javascript_promise(cx: &mut TestAppContext) {
    let (_runtime, object, mut context) =
        probe(cx, "/bin/sh", "process-output-limit.js", OUTPUT_LIMIT_PROBE);

    context.run_until_parked();
    draw(&mut context, &object);

    let settled = snapshot_text(&mut context, &object);
    assert!(
        settled.contains("rejected:") && settled.contains("stdout") && settled.contains("exceeded"),
        "JavaScript did not observe the bounded-output rejection: {settled}"
    );
}

#[gpui::test]
fn process_is_denied_and_ambient_authority_is_absent_on_every_platform(cx: &mut TestAppContext) {
    cx.update(crate::init);
    crate::set_capabilities(Capabilities::new());
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("process-denial.js", DENIAL_PROBE)
        .expect("load script");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate script view");
    draw(&mut context, &view);
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot_text(&mut context, &view);
    assert!(
        rendered.contains("capabilities.fs.execute")
            && rendered.contains("|undefined|undefined|undefined|undefined")
            && rendered.contains("|tick"),
        "{rendered}"
    );
}

#[cfg(unix)]
fn probe(
    cx: &mut TestAppContext,
    command: &str,
    name: &str,
    source: &str,
) -> (
    std::rc::Rc<ShellRuntime>,
    Entity<ScriptView>,
    VisualTestContext,
) {
    cx.update(crate::init);
    crate::set_capabilities(
        Capabilities::new().execute(ExecuteGrant::Allowed(vec![command.to_owned()])),
    );

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source(name, source).expect("load script");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate script view");
    (runtime, object, context)
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
