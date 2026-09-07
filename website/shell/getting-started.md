---
title: Getting Started
description: Add the runtime to a Rust application, write the script it loads, and check that script without opening a window.
order: 2
---

# Getting Started

`gpui-shell` is first of all a way to give a Rust GPUI application JavaScript extension points: the host builds the runtime, decides what a script may reach, and mounts script Views where it wants them. Running a script directory on its own — the `gpui-shell` binary below — is the development convenience that comes with that, not the point of it.

## Add the runtime to a Rust application

A host does four things: initialize the library, build a runtime, grant the capabilities it is willing to grant, and mount a script View under a `ShellRoot`. The `gpui-shell` binary is itself just a thin host that does exactly this.

```rust
use gpui_kit::shell::{Capabilities, ShellRuntime};

gpui_platform::application()
    .with_assets(gpui_kit::shell::AppAssets::new(root.clone()))
    .run(move |cx| {
        // Initializes gpui-base, the shell's default token palette, and the
        // style reflection table.
        gpui_kit::shell::init(cx);

        let runtime = ShellRuntime::new(cx).expect("script runtime");

        // Nothing is permitted until the host says so.
        gpui_kit::shell::set_store_path(store_directory.join("store.json"));
        gpui_kit::shell::set_capabilities(
            Capabilities::new()
                .read_roots([root.clone()])
                .write_roots([store_directory.clone()])
                .store(true),
        );

        cx.open_window(Default::default(), move |window, cx| {
            runtime.load(&root, window, cx)
        })
        .expect("window");
    });
```

Two of those lines carry rules rather than mechanics.

**`runtime.load(...)` returns the window's `ShellRoot`**, the same role `Root` has in a `gpui-component` window. It owns the dialog stack, the sheet, the toast stack, focus restoration and Tab navigation. A manifest selects the application entry and records capability requests; it never approves those requests. Both manifest-backed and bare directories run under the host's current default policy, and a bare directory uses `main.js`.

**Capabilities default to empty.** `Capabilities::default()` grants nothing at all — no file, no storage, no clipboard, no process. The host decides, because only the host knows how far it trusts the code it is about to run. See [Capabilities](./capabilities.md).

Install a `tracing` subscriber too. The runtime reports script errors, unhandled promise rejections and illegal-phase calls through `tracing`; with no subscriber, every one of them is discarded and the symptom is a View that quietly stopped responding.

## The script it loads

One file is enough. Create a directory with a `main.js` in it:

```js
// hello/main.js
import { View } from "gpui-kit";
import { v_flex, Button } from "gpui-base";

export default class Hello extends View {
  init() {
    this.clicks = 0;
  }

  render(cx) {
    return v_flex()
      .size_full()
      .items_center()
      .justify_center()
      .gap(12)
      .bg(cx.theme().colors.background)
      .child(
        div()
          .text_color(cx.theme().colors.foreground)
          .child(`Clicked ${this.clicks} times`),
      )
      .child(
        Button.new("click")
          .h(28)
          .px(12)
          .items_center()
          .justify_center()
          .border(1)
          .border_color(cx.theme().colors.border)
          .bg(cx.theme().colors.surface)
          .text_color(cx.theme().colors.foreground)
          .on_click((_event, cx) => {
            this.clicks += 1;
            cx.notify();
          })
          .child("Click me"),
      );
  }
}
```

```bash
cargo run -p gpui-shell -- hello
```

Four things in that file are worth naming now, because everything else builds on them.

**One module per crate that provides it.** `"gpui-kit"` holds GPUI's own elements and what the runtime adds — `View`, `div`, `text`, storage, scheduling. `"gpui"` is a compatibility alias for the same module, so `import { div } from "gpui"` is also supported. `"gpui-base"` holds gpui-base's layout helpers, components and theme — `v_flex`, `Button`, `InputState`. `"gpui-fps"` holds its performance overlay. The runtime also supplies a deliberately small JavaScript-standard layer: `buffer`, `path`, `url`, `crypto`, `zlib`, `console`, `process`, `os`, `fs/promises`, `net`, `websocket`, and global `fetch`. Application-relative imports remain confined to the application directory. Node-prefixed aliases such as `node:fs`, package lookup, and CommonJS `require` are not part of the contract.

**`main.js` must `export default` a class extending `View`.** `init` runs once when the View is created; `render` returns one element, retained `Entity` or string, and runs when the View is invalidated rather than on every frame — see [When `render` runs](./state.md#when-render-runs).

**Style methods are `snake_case`, your own code is `camelCase`.** `items_center`, `on_click`, `text_color`, `gap_2` keep their Rust spelling, because the no-argument style surface is generated from GPUI's reflection table rather than written by hand. Anything the application declares itself — variables, methods, object keys — is ordinary JavaScript. The contrast is deliberate: a `snake_case` call is host surface, a `camelCase` one is your code.

**Nothing repaints on its own.** There are no signals, no `useState`, no dependency arrays. Change state, then call `cx.notify()`.

## Running a script on its own

A script directory can also be run directly, without writing a host. This is how the bundled example runs, and how a script is usually developed before it is loaded by the application that will own it. `gpui-shell` is not published to crates.io, so clone the repository and run it from the root:

```bash
cargo run -p gpui-shell -- examples/js_todolist
```

That opens a window with a working todo list: a text field with retained state, controlled checkboxes, a confirmation dialog, a toast, icons loaded from the application's own directory, and storage that falls back to memory when it has not been granted. It exists to exercise the runtime rather than to be minimal — if something is broken, it shows there first.

The argument is a **directory**, not a file. The runtime resolves that directory, reads `main.js` by default, takes the class that module default-exports, constructs one instance, and mounts it as the window's root View. If the directory contains `gpui-shell.json`, the binary validates that manifest first and uses its declared `entry` and capabilities.

## Check a script without running it

JavaScript has no compiler, and this runtime does not add one. What it adds is the thing a compiler would have done for you:

```bash
cargo run -p gpui-shell -- check hello
```

`check` loads the application and renders one frame into a window that is never shown, then exits `0` on success and `1` on failure. Because the script surface is dynamic — an unknown style method, a wrongly typed argument and a reused element are all runtime facts — building and rendering once is the only honest way to check it. What it catches:

- syntax errors, with the script's own stack;
- unresolved imports, and imports that escape the application directory;
- a missing or malformed default export;
- unknown style methods, with a `did you mean` suggestion;
- wrongly typed style arguments, such as `.p("auto")`;
- an element used twice.

It opens no window, so it is usable from an editor, from CI, or from an agent loop.

Add `--print-spec` to print the element description that was built:

```bash
cargo run -p gpui-shell -- check hello --print-spec
```

That output is the arena's own debug dump — the tree of components and recorded operations, before anything is materialized. It is useful when the question is "what did my chain actually record?".

## Generate TypeScript declarations

```bash
cargo run -p gpui-shell -- types hello
```

This writes `gpui-kit.d.ts` next to the application. Put `// @ts-check` at the top of a script and an editor will complete the whole API and reject a mistyped style method, a colour token that does not exist, or `.p("auto")` — at the call site, before it runs.

It also sets up everything else the editor needs: each Git dependency the manifest declares is fetched and linked into `node_modules` under its declared name, so `import { style } from "omarchy-ui"` resolves to the same files the runtime will execute and carries the package's own types, parameters and JSDoc; and a `jsconfig.json` is scaffolded when the directory has neither that nor a `tsconfig.json`. See [Dependencies](./dependencies.md).

The declarations can be trusted because they are **generated from the tables the runtime dispatches through**, not transcribed from this documentation:

- style method names come from the same list the JavaScript prelude loops over to build the element prototype;
- each parametric method's argument type is _probed_ — the generator asks the runtime which literals that method accepts, so the difference between a length, a definite length, an absolute length, a colour and a bare number is decided by the code that enforces it;
- the colour union comes from the installed palette's token names.

Three things the declarations deliberately do not express, because no type could: whether a capability is **granted** (a denied `fs.readFile` still type-checks), the **lifetime** of an element or a `cx` (TypeScript has no affine types, so reusing an element still type-checks and still throws), and **which component a method suits** (every element shares one prototype, so `.checked(true)` is declared on all of them and is simply inert on a `div`).

Regenerate the file after upgrading the runtime; the output is deterministic, so the diff is reviewable.

## Hot-reload

```bash
cargo run -p gpui-shell -- hello --watch
cargo run -p gpui-shell -- hello --dev      # implies --watch
```

`--watch` polls the application directory four times a second, debounces a burst of writes for 200 ms, and reloads. A reload re-reads **every** module, entry point included — a hot-reload that quietly served a stale import would be worse than none, because it looks like it worked.

A reload does all of its fallible work before it touches the live View. If the new code fails to load, the previous View keeps running, the error goes to stderr, and a toast with a stable id reports it in the window; the next successful reload retracts that toast. A broken save never costs you the window.

`--dev` implies `--watch` and enables development mode before the runtime is constructed. It restores dynamic-code constructors and leaves built-in prototypes writable, while capability checks remain unchanged. See [Capabilities](./capabilities.md#the-sandbox).

## Command reference

```text
gpui-shell <directory> [--watch] [--dev]
gpui-shell check <directory> [--print-spec]
gpui-shell types <directory>
gpui-shell --help | --version
```

| Argument       | Meaning                                                                  |
| -------------- | ------------------------------------------------------------------------ |
| `<directory>`  | The application root, or the `main.js` inside it                         |
| `check`        | Load and render once without a window; exit `0` or `1`                   |
| `types`        | Write `gpui-kit.d.ts`, link the manifest's dependencies, scaffold config |
| `--watch`      | Reload when the sources change                                           |
| `--dev`        | Development mode; implies `--watch`                                      |
| `--print-spec` | With `check`, also print the element description that was built          |
