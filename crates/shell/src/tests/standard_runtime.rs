use std::ops::Deref;

use gpui::{IntoElement as _, TestAppContext, VisualTestContext};

use crate::ShellRuntime;

const PURE_MODULES: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { Buffer } from "buffer";
import path from "path";
import { URL } from "url";
import { deflateSync, inflateSync } from "zlib";
import { createHash } from "crypto";

export default class Probe extends View {
  render(cx) {
    const input = Buffer.from("shell", "utf8");
    const compressed = deflateSync(input);
    const inflated = inflateSync(compressed).toString("utf8");
    const digest = createHash("sha256").update(input).digest("hex");
    const url = new URL("https://example.com/a?b=1");
    return v_flex().child([
      input.toString("hex"),
      path.join("a", "b"),
      url.hostname,
      inflated,
      digest,
    ].join("|"));
  }
}
"#;

#[gpui::test]
fn llrt_pure_modules_execute_inside_the_shell_runtime(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("standard-runtime.js", PURE_MODULES)
        .expect("load LLRT imports");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate probe");

    let view_to_draw = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view_to_draw.into_any_element(),
    );

    // A missing snapshot means the script threw, so report its error rather
    // than an empty result.
    let rendered = context.update(|_, cx| {
        let view = view.read(cx);
        match view.snapshot() {
            Some(snapshot) => snapshot.debug_tree(),
            None => panic!(
                "the render produced no snapshot; the build failed with: {}",
                view.build_error().unwrap_or("no error was recorded either")
            ),
        }
    });
    // The snapshot quotes text the way `Debug` does, so a Windows backslash comes back escaped.
    let joined_path = format!("a{}b", std::path::MAIN_SEPARATOR)
        .escape_debug()
        .to_string();
    let expected = format!(
        "7368656c6c|{joined_path}|example.com|shell|\
         ce635c4eabff5e4f56dba8fb1e39ca235530aa2b6b18533eef1af3862016c577"
    );
    assert!(
        rendered.contains(&expected),
        "unexpected Standard Runtime result: {rendered}"
    );
}

const TEXT_CODING: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Probe extends View {
  render(cx) {
    const encoder = new TextEncoder();
    const bytes = encoder.encode("hé中😀");
    const target = new Uint8Array(4);
    const into = encoder.encodeInto("a中中", target);
    const lone = [...encoder.encode("\ud800")].join(",");

    const decoder = new TextDecoder();
    const streamed = new TextDecoder("utf8");
    const parts = [
      streamed.decode(bytes.subarray(0, 4), { stream: true }),
      streamed.decode(bytes.subarray(4, 9), { stream: true }),
      streamed.decode(bytes.subarray(9)),
    ];
    let fatal;
    try {
      new TextDecoder("utf-8", { fatal: true }).decode(new Uint8Array([0xff]));
      fatal = "accepted";
    } catch (error) {
      fatal = error instanceof TypeError ? "TypeError" : "other";
    }
    let label;
    try { new TextDecoder("latin1"); label = "accepted"; }
    catch (error) { label = error instanceof RangeError ? "RangeError" : "other"; }
    const bom = new Uint8Array([0xef, 0xbb, 0xbf, 0x61]);

    return v_flex().child([
      encoder.encoding,
      [...bytes].join(","),
      `${into.read}:${into.written}:${[...target].join(",")}`,
      lone,
      decoder.decode(bytes.buffer),
      parts.join("/"),
      decoder.decode(new Uint8Array([0x61, 0xff, 0x62])),
      fatal,
      label,
      decoder.decode(bom).length,
      new TextDecoder("utf-8", { ignoreBOM: true }).decode(bom).length,
      decoder.decode(new DataView(bytes.buffer, 0, 1)),
    ].join("|"));
  }
}
"#;

#[gpui::test]
fn text_encoder_and_decoder_follow_the_encoding_standard(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("text-coding.js", TEXT_CODING)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate probe");
    let view_to_draw = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view_to_draw.into_any_element(),
    );
    // A missing snapshot means the script threw, so report its error rather
    // than an empty result.
    let rendered = context.update(|_, cx| {
        let view = view.read(cx);
        match view.snapshot() {
            Some(snapshot) => snapshot.debug_tree(),
            None => panic!(
                "the render produced no snapshot; the build failed with: {}",
                view.build_error().unwrap_or("no error was recorded either")
            ),
        }
    });
    let expected = "utf-8|\
        104,195,169,228,184,173,240,159,152,128|\
        2:4:97,228,184,173|\
        239,191,189|\
        h\u{e9}\u{4e2d}\u{1f600}|\
        h\u{e9}/\u{4e2d}/\u{1f600}|\
        a\u{fffd}b|\
        TypeError|\
        RangeError|\
        1|2|h";
    assert!(rendered.contains(expected), "{rendered}");
}

#[test]
fn node_prefixed_modules_are_not_part_of_the_shell_contract() {
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    let error = runtime
        .load_source(
            "node-prefix.js",
            r#"import { Buffer } from "node:buffer"; export default Buffer;"#,
        )
        .expect_err("the shell must not advertise Node.js module names");
    assert!(error.to_string().contains("node:buffer"), "{error:#}");
}

#[test]
fn callback_style_fs_module_is_not_part_of_the_shell_contract() {
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    let error = runtime
        .load_source(
            "callback-fs.js",
            r#"import fs from "fs"; export default fs;"#,
        )
        .expect_err("Promise-only filesystem calls belong to fs/promises");
    assert!(error.to_string().contains("fs"), "{error:#}");
}

#[gpui::test]
fn safe_host_standard_modules_replace_the_old_gpui_exports(cx: &mut TestAppContext) {
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import console from "console";
import os from "os";
import process from "process";

export default class Probe extends View {
  render(cx) {
    console.log("standard runtime", os.platform());
    return v_flex().child([
      typeof process.run,
      typeof process.cwd,
      typeof process.env,
      typeof os.homedir,
      typeof os.tmpdir,
    ].join("|"));
  }
}

"#;
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("host-standard.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    let view_to_draw = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view_to_draw.into_any_element(),
    );
    // A missing snapshot means the script threw, so report its error rather
    // than an empty result.
    let rendered = context.update(|_, cx| {
        let view = view.read(cx);
        match view.snapshot() {
            Some(snapshot) => snapshot.debug_tree(),
            None => panic!(
                "the render produced no snapshot; the build failed with: {}",
                view.build_error().unwrap_or("no error was recorded either")
            ),
        }
    });
    assert!(
        // `process.env` holds only granted variables and `os.homedir` names a
        // path without granting it; `cwd` and `tmpdir` stay absent.
        rendered.contains("function|undefined|object|function|undefined"),
        "{rendered}"
    );

    let error = runtime
        .load_source(
            "old-host-exports.js",
            r#"import { div, fs, process } from "gpui-kit"; export default fs || process;"#,
        )
        .expect_err("the old gpui.fs/process exports must be removed");
    assert!(
        error.to_string().to_ascii_lowercase().contains("not find"),
        "{error:#}"
    );
}

#[gpui::test]
fn synchronous_unawaited_host_calls_hit_the_runtime_task_limit(cx: &mut TestAppContext) {
    let source = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "unlimited";
    for (let index = 0; index < 2000; index += 1) {
      try {
        cx.sleep(60000);
      } catch (error) {
        this.state = `limited:${error.message}`;
        break;
      }
    }
  }
  render(cx) { return v_flex().child(this.state); }
}
"#;
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("host-task-limit.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    let view_to_draw = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view_to_draw.into_any_element(),
    );
    // A missing snapshot means the script threw, so report its error rather
    // than an empty result.
    let rendered = context.update(|_, cx| {
        let view = view.read(cx);
        match view.snapshot() {
            Some(snapshot) => snapshot.debug_tree(),
            None => panic!(
                "the render produced no snapshot; the build failed with: {}",
                view.build_error().unwrap_or("no error was recorded either")
            ),
        }
    });
    assert!(
        rendered.contains("limited:") && rendered.contains("outstanding host task limit"),
        "synchronous calls must be stopped by a per-runtime hard limit: {rendered}"
    );
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
