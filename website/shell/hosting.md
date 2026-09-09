---
title: Hosting
description: The Rust side in full — runtime lifetime, mounting script Views, refreshing them from host state, metrics, exit requests and hot-reload.
order: 11
---

# Hosting

[Getting Started](./getting-started.md) shows the four lines that put a script View on screen. This page is the rest of the Rust surface: what to call, when, and the two or three places where the obvious call is the wrong one.

## The runtime

One `ShellRuntime` owns one VM. It is an `Rc` with interior mutability — neither `Send` nor `Sync` — so it lives on the thread that owns the `App`.

```rust
gpui_kit::shell::init(cx);                     // gpui-base, the token palette, the style table

let runtime = ShellRuntime::new(cx)?;     // one VM, installed as this App's default
```

`new(cx)` lets callbacks, HostModule registrations and hot reload find the default runtime without the host threading a handle through every layer. A host deliberately managing more than one VM can create additional runtimes with `new_isolated()` and retain those handles itself.

`gpui-shell` uses GPUI's inspector reflection table to expose the fluent style
methods, including in release builds. Depending on this crate therefore enables
the `gpui-base/inspector` feature for the unified Cargo dependency graph. This
is required for the JavaScript style surface; embedders should account for the
additional release-build instrumentation and dependencies.

## Loading and instantiating

For the usual application window, loading is one operation and returns its
`ShellRoot` directly:

```rust
cx.open_window(options, move |window, cx| {
    let root = runtime.load(&app_root, window, cx);
    #[cfg(debug_assertions)]
    if let Ok(watch) = runtime.watch(&root, window, cx) {
        watch.forget();
    }
    root
})?;
```

If `gpui-shell.json` exists, `load` validates its identity metadata and applies
its entry. Its capabilities are requests, not approval: both paths run
under the host's current default policy, and without a manifest the entry is
`main.js`. Either path refreshes `gpui-kit.d.ts`; a load failure renders the
selectable error surface instead of panicking the host. A host that needs to
handle the structured error itself uses `try_load`. A failure root has no
application to watch, so `watch` returns `Err`; ignoring that error here keeps
the selectable failure surface mounted.

The lower-level methods below are for a host that needs to assemble a script
View into an existing Rust composition.

Loading turns source into a **View type** — the class the script default-exports. Instantiating turns that type into a **View object**, one live instance:

```rust
let view_type = runtime.load_app(&root, "main.js")?;   // a directory
let view_type = runtime.load_source("inline", source)?; // a string, for tests

let object = runtime.instantiate(&view_type, window, cx)?;
```

`load_app` resolves the directory, reads the entry file, and evaluates the module. Every failure here is a `ShellError` carrying the script's own stack — a syntax error, an import that resolves outside the application root, a missing or misshapen default export.

Instantiating runs the script's `init`, which means it needs a live `Window`: it may create retained state such as an `InputState`.

## Mounting

A script View is a GPUI View like any other, and it goes **under a `ShellRoot`**:

```rust
cx.open_window(options, move |window, cx| {
    let object = runtime.instantiate(&view_type, window, cx).expect("view");
    let content = cx.new(|_| ScriptView::new(runtime.clone(), object));
    cx.new(|cx| ShellRoot::new(content.into(), window, cx))
})
```

`ShellRoot` owns the dialog stack, the sheet, the toast stack, focus restoration and Tab navigation — the same role `Root` plays for a `gpui-component` window. `window.open_dialog` and friends reach it, so a script mounted under any other root View gets a refusal naming the reason rather than a silent no-op.

The host can drive the same surfaces directly, which is how a plugin panel and the host's own UI end up in one stack:

```rust
root.update(cx, |root, cx| {
    root.open_dialog(view.into(), window, cx);
    root.push_toast(ToastRequest::new("Saved").with_level(ToastLevel::Success), window, cx);
    root.close_all_dialogs(window, cx);
});
```

## Refreshing a View from host state

This is the one call that is easy to get wrong, and the mistake is silent.

```text
cx.notify()        ── draw this View again          (no script runs)
view.refresh(cx)   ── and its description is stale  (the script runs)
```

Because a script `render` is [not a frame render](./state.md#when-render-runs), a plain `cx.notify()` repaints the Snapshot that already exists. If the host changed something the script _reads_ — an entity behind a HostModule, a setting, a document — the View must be told the description itself is out of date:

```rust
runtime.refresh(&root, cx)?;
```

The runtime checks that `root` contains one of its applications, then invalidates that script View and schedules a repaint. Keeping the typed `ScriptView` private prevents host code from downcasting the root content or refreshing a View from another runtime by mistake.

Getting it wrong in the other direction is visible immediately — the interface simply does not update — which is the same failure mode as a forgotten `cx.notify()` in GPUI itself.

## What a script may reach

The three host settings have different lifetimes. Capabilities are frozen into each newly loaded View. The store handle and HostModule registry are live host configuration shared with that View, so replacing either affects its next call:

```rust
gpui_kit::shell::set_capabilities(
    Capabilities::new()
        .read_roots([app_root.clone()])
        .write_roots([data_dir.clone()])
        .store(true),
);
gpui_kit::shell::set_store_path(data_dir.join("store.json"));
gpui_kit::shell::export_module(market_module(&market))?;
```

All three default to nothing: no file access, no storage location, no HostModule registrations. See [Capabilities](./capabilities.md) and [HostModule](./host-module.md).

The standalone binary also checks `<root>/gpui-shell.json`. Its recognized fields supply application identity, optional application/Shell version metadata, the entry point, and capability requests; only `id`, `name`, and `entry` are required. Embedders may instead construct a `Policy` directly when each loaded application needs a distinct grant and module registry.

## Watching what it costs

The runtime counts two events separately, and the gap between them is the point:

```rust
let reading = runtime.read_metrics();
reading.script_renders();      // follows cx.notify(), reloads, theme changes
reading.materializations();    // follows frames
reading.script_render_time();  // total time inside script `render`
reading.native_time();         // of which, inside HostModule registrations
reading.slowest_script_render();
reading.structure_repeat_rate();  // how often a rebuild described the shape it replaced
```

`RuntimeMetrics::since(&earlier)` gives the delta between two readings, which is how a per-second rate is built. There is no reset: the counters belong to the runtime, and zeroing them would move them under anything else that is reading. To measure one stretch, keep a baseline and subtract — the Shell story takes one whenever its feed changes, so its readout answers "what is this feed costing" rather than "what has this window done since it opened".

A regression test can assert on `script_renders` directly; that is what keeps [the benchmark's third figure](./engine.md#the-measurement) honest.

`structure_repeats()` and `structure_changes()` answer a different question: of the rebuilds that had a previous description to compare with, how many produced the same _shape_ — the same components, the same builder methods, the same tree — and differed only in the values inside it. Nothing in the runtime acts on the answer; it is there to size [where the Snapshot cache stops](./performance.md#where-the-snapshot-cache-stops). A View's first build has no predecessor and is counted in neither.

## Building for development

A debug build of a host is roughly **three times slower per script render** than a release
build, and the whole difference is in two dependencies. Measured on a live application —
a market terminal re-rendering on every quote tick — with the runtime's own
[`RuntimeMetrics`](#watching-what-it-costs):

| `[profile.dev.package]`          | mean script render | mean materialize |
| -------------------------------- | ------------------ | ---------------- |
| nothing, or `rquickjs` alone     | 31.5 ms            | 3.9 ms           |
| `rquickjs-sys` + `rquickjs-core` | **11.3 ms**        | **1.2 ms**       |
| release, for comparison          | 11.0 ms            | 1.2 ms           |

So:

```toml
[profile.dev.package]
rquickjs-sys = { opt-level = 3 }
rquickjs-core = { opt-level = 3 }
```

**`rquickjs` on its own does nothing**, which is the trap: it is a thin facade that
re-exports `rquickjs-core`, so naming it optimises neither the interpreter nor the
bindings. `rquickjs-sys` compiles QuickJS itself — C source, built through `cc`, which
reads the profile's optimisation level for _that_ package — and `rquickjs-core` is where
every value that crosses the boundary is converted. An unoptimised interpreter is what
makes an unoptimised build feel like a different product.

The `llrt_*` crates do **not** need this. They were measured with the same application and
made no difference beyond the noise: `fs`, `net`, `crypto` and the rest are not on the
render path, so optimising them buys nothing a script author would feel.

These settings only take effect in the **workspace root that builds the binary**. A library
cannot impose a profile on the application that depends on it, so `gpui-shell` cannot set
this for you — every host has to write it down itself.

## Exit requests

`process.exit(code)` from a script is **a request, never `exit(2)`**. One plugin must not be able to take the host process down, and the host may have unsaved state. The runtime hands the request to the host, and the host decides:

```rust
gpui_kit::shell::on_exit_request(|request, window, cx| {
    match request.view() {
        Some(view) => close_the_panel_showing(view, window, cx),
        None => cx.quit(),
    }
});
```

`request.code()` is the exit code the script asked for, and `request.view()` names the View it came from, when there is one — a plugin host closes _that_ plugin's panel, where one that quit the window would let a plugin end someone else's work.

**A host that grants exit without installing a handler is told at the call**, not never: `process.exit()` throws, naming `on_exit_request`. A request nobody answers is a lie told in the flattering direction — the script gets a success and nothing happens.

## Hot-reload

One call starts it, and it is the same one the `--watch` flag uses:

```rust
runtime.watch(&root, window, cx)?.forget();
```

`runtime.watch` reads the resolved directory and manifest entry retained by the loaded root, so there is no second copy of that metadata to drift. It has no hidden build-mode policy: the CLI enables watching after `--watch`, while an embedded host can put the call behind `#[cfg(debug_assertions)]`. The returned `Watcher` owns the watch: dropping it stops the loop, which is what a host unmounting a panel wants, while `.forget()` lets it run for as long as the View does. The loop also ends on its own when the View, the runtime or the window goes away, because it holds all three weakly.

A reload re-reads **every** module, entry point included — a hot-reload that quietly served a stale import would be worse than none, because it looks like it worked. It does all of its fallible work before touching the live View: if the new code fails to load, the previous View keeps running, the error goes to `tracing`, and a toast with a stable id reports it in the window. The next successful reload retracts that toast.

The View survives a reload. `ScriptView::replace_object` swaps what the script produced while keeping the entity, and with it the window, the focus and the element identities.

Plugin unload is a stronger lifecycle boundary than removing one View: the manager cancels every outstanding task carrying that plugin's `Policy`, including owner-less work, before dropping the plugin. No continuation may retain or exercise an unloaded plugin's authority.

## When a script fails

A script that throws does not take the interface with it. The last good Snapshot stays mounted and the failure is reported over it, so the reader keeps their scroll, their focus, and whatever they were reading. The runtime does not re-run a failing `render` until something invalidates the View again.

Install a `tracing` subscriber. The runtime reports script errors, unhandled promise rejections and illegal-phase calls through `tracing` with the target `gpui_kit::shell::script`; with no subscriber every one of them is discarded, and the symptom is a View that quietly stopped responding.

## Not there yet

- **A supervisor for scripts that hang.** The interpreter's own interrupt cuts a call off, but nothing restarts a runtime that keeps hitting it.
