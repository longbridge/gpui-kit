//! The filesystem surface, which is asynchronous.
//!
//! These need a real `App`, because that is where the executors are: a capability
//! check runs on the calling thread and the syscall behind it does not. The
//! denial cases live next to the resolver in `capability.rs` and `host.rs`,
//! where they need no window at all — a refusal never reaches the disk.

use std::ops::Deref;

use crate::{
    Capabilities, RenderSnapshot, ScriptView, ShellRuntime,
    spec::{CallbackId, SpecOp},
};
use gpui::{Entity, IntoElement as _, TestAppContext, VisualTestContext};

/// A view that does its filesystem work in a task and records the outcome, so
/// the assertion can be made on what the script saw rather than on what the host
/// did.
const PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import * as fs from "fs/promises";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";

    cx.spawn(async (cx) => {
      try {
        await fs.writeFile("notes.txt", "hello");
        await fs.writeFile("bytes.bin", new Uint8Array([0, 255, 42]));
        const back = await fs.readFile("notes.txt", "utf8");
        const bytes = await fs.readFile("bytes.bin");
        const names = await fs.readdir(".");
        const entries = await fs.readdir(".", { withFileTypes: true });
        const entryShape = entries.every((entry) =>
          typeof entry.name === "string" && typeof entry.isDirectory() === "boolean"
        );
        const there = await fs.exists("notes.txt");

        await fs.mkdir("nested/deeper", { recursive: true });
        const nested = await fs.exists("nested/deeper");

        await fs.unlink("notes.txt");
        const gone = !(await fs.exists("notes.txt"));

        this.state = `${back}|${bytes instanceof Uint8Array}:${[...bytes].join(",")}|${names.join(",")}|${entryShape}|${there}|${nested}|${gone}`;
      } catch (error) {
        this.state = `failed: ${error.message}`;
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
import * as fs from "fs/promises";
export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try { await fs.readFile("__PATH__"); this.state = "unexpectedly allowed"; }
      catch (error) { this.state = `rejected:${error.message}`; }
      cx.notify();
    });
  }
  render(cx) { return v_flex().child(this.state); }
}
"#;

#[gpui::test]
fn every_fs_call_settles_through_a_promise(cx: &mut TestAppContext) {
    let directory = std::env::temp_dir().join(format!("gpui-shell-fs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a granted root");

    cx.update(|cx| crate::init(cx));
    crate::set_capabilities(
        Capabilities::new()
            .read_roots([directory.clone()])
            .write_roots([directory.clone()]),
    );

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source("probe.js", PROBE).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");

    // Nothing has happened yet: the calls returned promises and the work is on
    // a background thread. That is the property this whole change is about.
    draw(&mut context, &view);
    let before = snapshot_text(&mut context, &view);
    assert!(
        before.contains("pending"),
        "the first render should have found the work still in flight, got: {before}"
    );

    context.run_until_parked();
    draw(&mut context, &view);

    let after = snapshot_text(&mut context, &view);
    assert!(
        after.contains("hello|true:0,255,42|bytes.bin,notes.txt|true|true|true|true"),
        "the round trip did not settle as expected: {after}"
    );

    let _ = std::fs::remove_dir_all(&directory);
}

/// A file over the ceiling is refused by name rather than by an out-of-memory
/// somewhere inside the VM.
#[gpui::test]
fn an_oversized_read_is_refused_by_name(cx: &mut TestAppContext) {
    let directory = std::env::temp_dir().join(format!("gpui-shell-fs-big-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a granted root");

    // Sparse where the filesystem allows it: the test is about the check, not
    // about moving sixty-five megabytes.
    let big = directory.join("big.bin");
    let file = std::fs::File::create(&big).expect("a large file");
    file.set_len(65 * 1024 * 1024).expect("a large length");
    drop(file);

    cx.update(|cx| crate::init(cx));
    crate::set_capabilities(Capabilities::new().read_roots([directory.clone()]));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = PROBE.replace(
        r#"await fs.writeFile("notes.txt", "hello");"#,
        r#"await fs.readFile("big.bin");"#,
    );
    let view_type = runtime.load_source("big.js", &source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    context.run_until_parked();
    draw(&mut context, &view);

    let rendered = snapshot_text(&mut context, &view);
    assert!(
        rendered.contains("big.bin") && rendered.contains("limit"),
        "an oversized read should name the file and the limit: {rendered}"
    );

    let _ = std::fs::remove_dir_all(&directory);
}

#[gpui::test]
fn an_oversized_write_is_refused_before_reaching_disk(cx: &mut TestAppContext) {
    let directory =
        std::env::temp_dir().join(format!("gpui-shell-fs-big-write-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a granted root");
    cx.update(crate::init);
    crate::set_capabilities(Capabilities::new().write_roots([directory.clone()]));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = PROBE.replace(
        r#"await fs.writeFile("notes.txt", "hello");"#,
        r#"await fs.writeFile("notes.txt", "x".repeat(8 * 1024 * 1024 + 1));"#,
    );
    let view_type = runtime.load_source("big-write.js", &source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    context.run_until_parked();
    draw(&mut context, &view);

    let rendered = snapshot_text(&mut context, &view);
    assert!(
        rendered.contains("write") && rendered.contains("limit"),
        "{rendered}"
    );
    assert!(!directory.join("notes.txt").exists());
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui::test]
fn fs_is_denied_by_default_through_the_javascript_module(cx: &mut TestAppContext) {
    let rendered = denial_probe(cx, Capabilities::new(), "notes.txt");
    assert!(
        rendered.contains("rejected:") && rendered.contains("capabilities.fs.read"),
        "{rendered}"
    );
}

#[cfg(unix)]
#[gpui::test]
fn fs_module_cannot_follow_a_symlink_out_of_a_granted_root(cx: &mut TestAppContext) {
    use std::os::unix::fs::symlink;

    let base = std::env::temp_dir().join(format!("gpui-shell-fs-link-{}", std::process::id()));
    let root = base.join("root");
    let outside = base.join("outside");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&root).expect("granted root");
    std::fs::create_dir_all(&outside).expect("outside root");
    std::fs::write(outside.join("secret.txt"), "secret").expect("secret");
    symlink(&outside, root.join("escape")).expect("escape symlink");

    let rendered = denial_probe(
        cx,
        Capabilities::new().read_roots([root]),
        "escape/secret.txt",
    );
    assert!(rendered.contains("rejected:"), "{rendered}");
    assert!(!rendered.contains("unexpectedly allowed"), "{rendered}");
    let _ = std::fs::remove_dir_all(&base);
}

fn denial_probe(cx: &mut TestAppContext, capabilities: Capabilities, path: &str) -> String {
    cx.update(crate::init);
    crate::set_capabilities(capabilities);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = DENIAL_PROBE.replace("__PATH__", path);
    let view_type = runtime.load_source("fs-denial.js", &source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    context.run_until_parked();
    draw(&mut context, &view);
    snapshot_text(&mut context, &view)
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

/// The store answers from memory and reaches the disk on its own.
///
/// `set` stays synchronous — a setting a script can read during `render` without
/// awaiting is the whole point of the cache — and the write it makes necessary
/// happens on a background thread. `flush` is for a script that has to know the
/// write landed.
const STORE_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Probe extends View {
  init(_props, cx) {
    // Synchronous, against the cache. A burst of these is one file, not four.
    // Values are strings, so anything with structure goes through JSON — the
    // browser's bargain, and the reason `setItem` takes what it is given.
    localStorage.setItem("window", JSON.stringify({ title: "Notes", size: [640, 480] }));
    localStorage.setItem("theme", "dark");
    localStorage.setItem("scratch", "1");
    localStorage.removeItem("scratch");

    // Readable immediately, with nothing awaited.
    const keys = [];
    for (let i = 0; i < localStorage.length; i += 1) keys.push(localStorage.key(i));
    this.state = `${JSON.parse(localStorage.getItem("window")).title}|${keys.join(",")}`;

    // Memory only, and gone with the process: it never reaches the file this
    // test is about, which is the whole distinction between the two.
    sessionStorage.setItem("scratch", "kept for this run");

    cx.spawn(async (cx) => {
      await localStorage.flush();
      this.state += "|flushed";
      cx.notify();
    });
  }

  render(cx) {
    return v_flex().child(this.state);
  }
}
"#;

#[gpui::test]
fn the_store_answers_from_memory_and_persists_off_thread(cx: &mut TestAppContext) {
    let directory = std::env::temp_dir().join(format!("gpui-shell-store-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a directory for the store");
    let file = directory.join("store.json");

    cx.update(|cx| crate::init(cx));
    crate::set_capabilities(Capabilities::new().storage(true));
    crate::set_storage_path(file.clone());

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source("store.js", STORE_PROBE).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");

    // The cache answered during `init`, before anything reached the disk.
    draw(&mut context, &view);
    let immediate = snapshot_text(&mut context, &view);
    assert!(
        immediate.contains("Notes|window,theme"),
        "the store should answer from memory without awaiting: {immediate}"
    );

    context.run_until_parked();
    draw(&mut context, &view);

    let settled = snapshot_text(&mut context, &view);
    assert!(
        settled.contains("flushed"),
        "flush never resolved: {settled}"
    );

    // It reached the disk, atomically, and the removed key did not. The value
    // is the JSON the script serialized, stored as the string it is — a
    // `localStorage` file holds strings, so the braces here are the script's
    // own text rather than structure the store understood.
    let written = std::fs::read_to_string(&file).expect("the storage file exists");
    assert!(written.contains("Notes"), "{written}");
    assert!(written.contains("\"theme\": \"dark\""), "{written}");
    assert!(
        !written.contains("scratch"),
        "the removed key must not land, and neither must the session one: {written}"
    );
    assert!(
        !directory.join("store.json.tmp").exists(),
        "the temporary file should have been renamed away"
    );

    let _ = std::fs::remove_dir_all(&directory);
}

const STORE_RETRY_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex, Checkbox } from "gpui-base";

export default class Probe extends View {
  init() {
    this.state = "failed write pending";
    localStorage.setItem("attempt", "1");
  }

  render(cx) {
    return v_flex()
      .child(this.state)
      .child(Checkbox.new("retry").on_change((_checked, cx) => {
        cx.spawn(async (cx) => {
          try {
            await localStorage.flush();
            this.state = "flushed";
          } catch (error) {
            this.state = `rejected:${error.message}`;
          }
          cx.notify();
        });
      }));
  }
}
"#;

/// A failed automatic write must park rather than drive the same revision in a
/// tight loop. A later explicit durability request is allowed to retry it.
#[gpui::test]
fn a_failed_store_write_parks_until_flush_explicitly_retries_it(cx: &mut TestAppContext) {
    let directory =
        std::env::temp_dir().join(format!("gpui-shell-store-retry-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a directory for the store test");
    let file = directory.join("store.json");

    cx.update(crate::init);
    crate::set_capabilities(Capabilities::new().storage(true));
    crate::set_storage_path(file.clone());
    // The store's startup read has already observed a normal first run. Change
    // the target afterwards so only the asynchronous persistence step fails.
    std::fs::create_dir(&file).expect("a directory that makes the first rename fail");

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("store-retry.js", STORE_RETRY_PROBE)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");

    draw(&mut context, &view);
    context.run_until_parked();
    assert!(
        file.is_dir(),
        "the failed revision should park without replacing its target"
    );

    std::fs::remove_dir(&file).expect("make the store path writable");
    let callback = first_change_callback(&mut context, &view);
    context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));
    context.run_until_parked();
    draw(&mut context, &view);

    let rendered = snapshot_text(&mut context, &view);
    assert!(
        rendered.contains("flushed"),
        "flush did not retry: {rendered}"
    );
    let written = std::fs::read_to_string(&file).expect("the retried storage write landed");
    assert!(written.contains("\"attempt\": \"1\""), "{written}");

    let _ = std::fs::remove_dir_all(&directory);
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

fn first_change_callback(context: &mut VisualTestContext, view: &Entity<ScriptView>) -> CallbackId {
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .and_then(first_change_callback_in_snapshot)
            .expect("the retry control should have a change handler")
    })
}

fn first_change_callback_in_snapshot(snapshot: &RenderSnapshot) -> Option<CallbackId> {
    (0..snapshot.len() as u32)
        .filter_map(|id| snapshot.arena().node(id))
        .flat_map(|node| node.ops())
        .find_map(|op| match op {
            SpecOp::Callback("on_change", id) => Some(*id),
            _ => None,
        })
}
