//! The JavaScript-facing process adapter.
//!
//! Unit tests in `crate::process` exercise the pipe, timeout and kill mechanics.
//! These tests cross the public script boundary: a real module calls
//! `process.run`, awaits its promise, and publishes what JavaScript observed in
//! a render snapshot.

use std::ops::Deref;

use gpui::{Entity, IntoElement as _, TestAppContext, VisualTestContext};

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
        String(process.env.CARGO_MANIFEST_DIR),
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

#[gpui::test]
fn an_application_learns_where_it_is_and_only_the_environment_it_was_granted(
    cx: &mut TestAppContext,
) {
    let directory =
        std::env::temp_dir().join(format!("gpui-shell-process-paths-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("lib")).expect("application directory");
    std::fs::write(
        directory.join("lib").join("helper.js"),
        "export const helperUrl = import.meta.url;\n",
    )
    .expect("helper module");
    std::fs::write(
        directory.join("main.js"),
        r#"
import { View } from "gpui-kit";
import { v_flex } from "gpui-base";
import os from "os";
import process from "process";
import { helperUrl } from "./lib/helper.js";

export default class Probe extends View {
  render(cx) {
    return v_flex().child([
      import.meta.url,
      helperUrl,
      process.dataDir,
      Object.keys(process.env).join(","),
      os.homedir(),
    ].join("|"));
  }
}
"#,
    )
    .expect("entry module");

    cx.update(crate::init);
    // The home variables are defaults, `CARGO_PKG_NAME` is granted, and `CARGO_MANIFEST_DIR`
    // is set by cargo and neither, so it stays out.
    crate::set_capabilities(Capabilities::new().env_variables(["CARGO_PKG_NAME".to_owned()]));
    let data = directory.join("data");
    crate::policy::update_default(|policy| policy.with_data_dir(data.clone()));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    runtime.set_editor_files(false);
    let view_type = runtime.load_app(&directory, "main.js").expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate script view");
    draw(&mut context, &view);
    let rendered = snapshot_text(&mut context, &view);

    let home = crate::runtime::home_dir().expect("a test machine has a home directory");
    // The snapshot quotes text the way `Debug` does, so Windows backslashes come back escaped.
    let quoted = |path: &std::path::Path| path.to_string_lossy().escape_debug().to_string();
    let fields: Vec<&str> = rendered
        .lines()
        .find(|line| line.contains("file://"))
        .and_then(|line| line.split_once("file://").map(|(_, rest)| rest))
        .map(|line| line.split('|').collect())
        .unwrap_or_default();
    assert_eq!(fields.len(), 5, "{rendered}");
    assert!(fields[0].ends_with("/main.js"), "{rendered}");
    assert!(
        fields[1].starts_with("file://") && fields[1].ends_with("/lib/helper.js"),
        "{rendered}"
    );
    assert_eq!(fields[2], quoted(&data), "{rendered}");
    let names: Vec<&str> = fields[3].split(',').collect();
    // Both home variables are defaults, but only the ones the host sets are
    // exported: a Windows runner may have `USERPROFILE` and no `HOME`.
    let homes: Vec<&str> = ["HOME", "USERPROFILE"]
        .into_iter()
        .filter(|name| std::env::var_os(name).is_some())
        .collect();
    assert!(
        !homes.is_empty(),
        "the host sets neither HOME nor USERPROFILE"
    );
    for home in homes {
        assert!(
            names.iter().any(|name| name.eq_ignore_ascii_case(home)),
            "{home} is a default and set on the host: {rendered}"
        );
    }
    assert!(names.contains(&"CARGO_PKG_NAME"), "{rendered}");
    assert!(!names.contains(&"CARGO_MANIFEST_DIR"), "{rendered}");
    assert!(fields[4].starts_with(&quoted(&home)), "{rendered}");

    crate::policy::update_default(|policy| policy.with_capabilities(Capabilities::new()));
    let _ = std::fs::remove_dir_all(&directory);
}

#[cfg(unix)]
/// Draws until `done` holds for the snapshot, giving the child's threads real
/// time to run between frames.
#[cfg(unix)]
fn draw_until(
    context: &mut VisualTestContext,
    view: &Entity<ScriptView>,
    done: impl Fn(&str) -> bool,
) -> String {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        context
            .executor()
            .advance_clock(std::time::Duration::from_millis(20));
        context.run_until_parked();
        draw(context, view);
        let text = snapshot_text(context, view);
        if done(&text) {
            return text;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "gave up waiting; the view shows: {text}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[cfg(unix)]
const SPAWN_PROBE: &str = r#"
import { View } from "gpui-kit";
import { v_flex } from "gpui-base";
import process from "process";

export default class Probe extends View {
  init(_props, cx) {
    this.lines = [];
    this.exits = 0;
    this.state = "running";
    const child = process.spawn("/bin/sh", ["-c", "printf 'one\\ntw'; sleep 0.05; printf 'o\\n'; printf 'oops' >&2; exit 3"], { lines: true });
    this.pid = child.pid;
    child
      .on("stdout", (line, cx) => { this.lines.push(line); cx.notify(); })
      .on("stderr", (line, cx) => { this.lines.push(`err:${line}`); cx.notify(); })
      .on("exit", (result, cx) => {
        this.exits += 1;
        this.state = `exit ${result.code} ${result.signal} ${result.error ?? "none"}`;
        cx.notify();
      });
  }

  render() {
    return v_flex().child(`${this.lines.join(",")} | ${this.state} | exits ${this.exits}`);
  }
}
"#;

/// Lines arrive in order through handlers that get a `cx`, and the exit
/// comes once, after the last of them.
#[cfg(unix)]
#[gpui::test]
fn spawn_streams_lines_to_handlers_and_exits_once(cx: &mut TestAppContext) {
    let (_runtime, object, mut context) = probe(cx, "/bin/sh", "spawn.js", SPAWN_PROBE);
    let text = draw_until(&mut context, &object, |text| text.contains("exits 1"));
    // The two streams are separate pipes, so only the order within each is
    // meaningful; the exit comes after both.
    let shown = text.split('"').nth(1).expect("the probe's text");
    let (lines, rest) = shown.split_once(" | ").expect("lines, then the exit");
    let lines: Vec<&str> = lines.split(',').collect();
    let stdout: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| !line.starts_with("err:"))
        .collect();
    assert_eq!(stdout, ["one", "two"], "{text}");
    assert!(lines.contains(&"err:oops"), "{text}");
    assert!(rest.contains("exit 3 null none | exits 1"), "{text}");
}

#[cfg(unix)]
const SPAWN_DENIAL_PROBE: &str = r#"
import { View } from "gpui-kit";
import { v_flex } from "gpui-base";
import process from "process";

export default class Probe extends View {
  init() {
    const attempt = (run) => { try { run(); return "allowed"; } catch (error) { return error.message; } };
    this.state = [
      attempt(() => process.spawn("gpui-shell-denied-command")),
      attempt(() => process.spawn("/bin/sh", ["-c", "true"], { cwd: "/" })),
      attempt(() => process.spawn("/bin/sh", ["-c", "true"], { stdin: "inherit" })),
      attempt(() => process.spawn("/bin/sh", ["-c", "true"]).write("x")),
    ].join(" | ");
  }

  render() {
    return v_flex().child(this.state);
  }
}
"#;

#[cfg(unix)]
#[gpui::test]
fn spawn_refuses_what_the_grant_does_not_cover(cx: &mut TestAppContext) {
    let (_runtime, object, mut context) =
        probe(cx, "/bin/sh", "spawn-denial.js", SPAWN_DENIAL_PROBE);
    draw(&mut context, &object);
    let text = snapshot_text(&mut context, &object);
    let shown = text.split_once('"').map_or(text.as_str(), |(_, rest)| rest);
    let reasons: Vec<&str> = shown.split(" | ").map(str::trim).collect();
    assert!(text.contains("gpui-shell-denied-command"), "{text}");
    assert!(
        text.contains("capabilities.fs.read"),
        "a cwd outside the grant: {text}"
    );
    assert!(text.contains("options.stdin"), "{text}");
    assert!(
        text.contains("ChildProcess.write(text) needs a child spawned"),
        "write without a pipe: {text}"
    );
    assert!(!reasons.contains(&"allowed"), "{text}");
}

#[cfg(unix)]
const LONG_RUNNING_PROBE: &str = r#"
import { View } from "gpui-kit";
import { v_flex } from "gpui-base";
import process from "process";

export default class Probe extends View {
  init(_props, cx) {
    const child = process.spawn("/bin/sh", ["-c", "sleep 30 & echo $!; sleep 30"], { lines: true });
    this.pids = `${child.pid}`;
    child.on("stdout", (line, cx) => { this.pids += ` ${line}`; cx.notify(); });
  }

  render() {
    return v_flex().child(`pids ${this.pids} end`);
  }
}
"#;

/// A reload releases the application generation that spawned the child; the
/// child and what it started go with it rather than outliving the code that
/// owned them.
#[cfg(unix)]
#[gpui::test]
fn reloading_the_application_kills_the_children_it_spawned(cx: &mut TestAppContext) {
    cx.update(crate::init);
    crate::set_capabilities(
        Capabilities::new().execute(crate::ExecuteGrant::Allowed(vec!["/bin/sh".to_owned()])),
    );
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let directory =
        std::env::temp_dir().join(format!("gpui-shell-spawn-reload-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("main.js"), LONG_RUNNING_PROBE).unwrap();
    let view_type = runtime.load_app(&directory, "main.js").expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");

    let text = draw_until(&mut context, &object, |text| {
        text.split_whitespace()
            .filter(|word| word.parse::<i32>().is_ok())
            .count()
            == 2
    });
    let pids: Vec<i32> = text
        .split_whitespace()
        .filter_map(|word| word.parse().ok())
        .collect();
    let alive = |pid: i32| unsafe { libc::kill(pid, 0) } == 0;
    assert!(
        pids.iter().all(|pid| alive(*pid)),
        "both are running: {pids:?}"
    );

    let generation = context
        .update(|_, cx| object.read(cx).application_generation())
        .expect("an application generation");
    context.update(|_, cx| runtime.release_application_generation(&generation, cx));

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while pids.iter().any(|pid| alive(*pid)) {
        assert!(
            std::time::Instant::now() < deadline,
            "a child outlived its application: {pids:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    std::fs::remove_dir_all(&directory).ok();
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
        Capabilities::new().execute(crate::ExecuteGrant::Allowed(vec![command.to_owned()])),
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
