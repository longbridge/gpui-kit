---
title: Getting Started
description: Install gpui-shell, write and run a standalone app in JavaScript, check it without opening a window, and move it under a Rust host when it needs more.
order: 2
maturity: [preview]
---

# Getting Started

Start as a gpui-shell standalone app: write it in JavaScript, run it with hot reload, check it without opening a window and ship it, with no Rust and no build step. That covers most of what a product needs early on. When it needs more — a native library, heavy computation, logic already written in Rust — the same scripts move under a Rust binary that embeds the crate, unchanged; [Move to a Rust host](#move-to-a-rust-host) at the end of this page shows how. A Rust GPUI application that wants JavaScript extension points takes the same last step from the other side.

## Install gpui-shell

Install the `gpui-shell` binary first, whichever way you use the runtime. An application written entirely in JavaScript runs on it; a Rust host that embeds the library still uses it to `check` scripts, generate their TypeScript declarations with `types`, and write instructions for coding agents with `agents`.

### From a release

Each GitHub release carries a prebuilt `gpui-shell` for macOS (Apple silicon and Intel), Linux x86-64 and Windows x86-64. It is built with the [`gpui-component`](./examples.md) catalog, so an application may import styled components from `"gpui-component"` as well as `"gpui-base"`.

```bash
curl -fsSL https://gpui-kit.com/shell/install.sh | sh
```

Each release also carries the same script, so `https://github.com/longbridge/gpui-kit/releases/latest/download/install.sh` works too.

The script installs the binary to `~/.local/bin` and creates `~/.gpui-shell/apps`. `GPUI_SHELL_VERSION=v0.7.1` pins a release. `GPUI_SHELL_HOME` moves the `~/.gpui-shell` directory — installed applications and the Git dependency cache; `gpui-shell` reads it as well, so keep it exported. Asset names carry no version (`gpui-shell-aarch64-apple-darwin.tar.gz`, with a `.sha256` beside it), so a Homebrew formula or a mise `ubi` backend can download `releases/latest/download/<asset>` too.

### Build from crates.io

```bash
cargo install gpui-shell --version 0.8.1 --locked
```

### The latest from Git

To follow `main` ahead of a release — a fix that has landed but not shipped, or a Rust host built against the repository rather than a tagged version — build the binary from Git with Cargo:

```bash
cargo install --git https://github.com/longbridge/gpui-kit gpui-shell --locked
```

It installs to `~/.cargo/bin`, with the same `gpui-component` catalog as a release. Run the same command again to update; Cargo rebuilds when `main` has moved. `--branch <name>`, `--tag <tag>` or `--rev <commit>` builds a specific point instead. This needs a Rust toolchain, and on Linux the system libraries GPUI links against — `script/install-linux.sh` in the repository lists them for Ubuntu.

If `~/.local/bin` comes before `~/.cargo/bin` on your `PATH`, a release installed by the script shadows the Git build; `command -v gpui-shell` shows which one runs.

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
gpui-shell run ./hello
```

Four things in that file are worth naming now, because everything else builds on them.

**One module per crate that provides it.** `"gpui-kit"` holds GPUI's own elements and what the runtime adds — `View`, `div`, `text`, storage, scheduling. `"gpui"` is a compatibility alias for the same module, so `import { div } from "gpui"` is also supported. `"gpui-base"` holds gpui-base's layout helpers, components and theme — `v_flex`, `Button`, `InputState`. `"gpui-fps"` holds its performance overlay. The runtime also supplies a deliberately small JavaScript-standard layer: `buffer`, `path`, `url`, `crypto`, `zlib`, `console`, `process`, `os`, `fs/promises`, `net`, `websocket`, and the globals `fetch`, `TextEncoder` and `TextDecoder`. Application-relative imports remain confined to the application directory. Node-prefixed aliases such as `node:fs`, package lookup, and CommonJS `require` are not part of the contract.

**`main.js` must `export default` a class extending `View`.** `init` runs once when the View is created; `render` returns one element, retained `Entity` or string, and runs when the View is invalidated rather than on every frame — see [When `render` runs](./state.md#when-render-runs).

**Style methods are `snake_case`, your own code is `camelCase`.** `items_center`, `on_click`, `text_color`, `gap_2` keep their Rust spelling, because the no-argument style surface is generated from GPUI's reflection table rather than written by hand. Anything the application declares itself — variables, methods, object keys — is ordinary JavaScript. The contrast is deliberate: a `snake_case` call is host surface, a `camelCase` one is your code.

**Nothing repaints on its own.** There are no signals, no `useState`, no dependency arrays. Change state, then call `cx.notify()`.

## Running a script on its own

Running a directory needs no host of your own. This is how the bundled examples run, and how an application is developed whether or not it later moves under a Rust host. From a clone of the repository:

```bash
gpui-shell run examples/js_todolist
```

That opens a window with a working todo list: a text field with retained state, controlled checkboxes, a confirmation dialog, a toast, icons loaded from the application's own directory, and storage that falls back to memory when it has not been granted. It exists to exercise the runtime rather than to be minimal — if something is broken, it shows there first.

The argument is a **directory**, not a file. The runtime resolves that directory, reads `main.js` by default, takes the class that module default-exports, constructs one instance, and mounts it as the window's root View. If the directory contains `gpui-shell.json`, the binary validates that manifest first and uses its declared `entry` and capabilities.

An argument that contains a path separator — `./hello`, `examples/js_todolist` — is a directory. A bare name is an **installed** application instead, described below; `-d` / `--dir <directory>` always means a directory.

## Installed and packaged applications

`gpui-shell` runs one application per process and has no interface of its own. Where that application comes from is decided by the command line:

| Command                         | Runs                                                                  |
| ------------------------------- | --------------------------------------------------------------------- |
| `gpui-shell run clock`          | the installed application `~/.gpui-shell/apps/clock/`                 |
| `gpui-shell run -d ~/dev/clock` | that directory                                                        |
| `gpui-shell`                    | the application packaged beside the executable; without one, the help |

To install an application, copy or link its directory into `~/.gpui-shell/apps/`; the directory name is the name it runs by. `gpui-shell list` prints every installed application with the capabilities its manifest asks for, running none of them. An application keeps its data under its manifest `id` however it is run — installed, packaged, or with `--dir` — so moving or reinstalling it does not lose it, and the copy you are editing shares its data with the installed one. Only a directory without a manifest is keyed by its path.

With no argument, the executable looks for an application next to itself, and prints its help when it finds none — so a freshly installed `gpui-shell` typed at a prompt explains itself: `Contents/Resources/app` when it is `Contents/MacOS/gpui-shell` inside a macOS bundle, or `app` in the executable's own directory. A standalone desktop application can therefore be the `gpui-shell` binary plus a directory of scripts, with no launcher script to pass a path:

```text
Clock.app/Contents/
├── Info.plist          # CFBundleExecutable = gpui-shell
├── MacOS/gpui-shell
└── Resources/app/
    ├── gpui-shell.json
    └── main.js
```

`gpui-shell bundle -d clock` makes the desktop application for you. On macOS and Linux it leaves the binary out — a `.app`, or a `.tar.gz` with an `install.sh` beside it: on the user's machine they use an installed `gpui-shell` at least as new as the one that made them, or download that release, and run the application with `run --packaged`, which keeps its data under its `id` like an installed one. On Windows it is a portable `.zip` whose executable is `gpui-shell.exe` itself, with `app\` beside it. [Packaging](./packaging.md) covers both ways, the options, and signing and notarization.

## Starting a new application

```bash
gpui-shell new clock
gpui-shell dev -d clock
```

`new` creates the directory where the command runs, the way `cargo new` does — `clock` here is `./clock`, not an installed application. It writes a `gpui-shell.json` whose `id` comes from the directory name, a `main.js` with a title bar and a button, a `.gitignore` (which also ignores `dist/`, where `bundle` writes), `jsconfig.json`, `gpui-kit.d.ts` and `AGENTS.md`, and refuses a directory that already has files in it.

## Instructions for coding agents

```bash
gpui-shell agents                 # print them
gpui-shell agents -d ./hello      # write ./hello/AGENTS.md
```

`agents` prints the rules a coding agent needs to write an application for this runtime and the `check` loop that verifies its work: no DOM, state assigned in `init`, `cx.notify()` after a change, single-use elements, capabilities denied by default. The module list and the component catalog in it come from the binary that prints it. Given an application, it writes `AGENTS.md` there, and only ever replaces a file it generated — the first line marks it — so instructions written by hand are left alone.

## Check a script without running it

JavaScript has no compiler, and this runtime does not add one. What it adds is the thing a compiler would have done for you:

```bash
gpui-shell check ./hello
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
gpui-shell check ./hello --print-spec
```

That output is the arena's own debug dump — the tree of components and recorded operations, before anything is materialized. It is useful when the question is "what did my chain actually record?".

## Generate TypeScript declarations

```bash
gpui-shell types ./hello
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
gpui-shell dev ./hello           # reload, with the debug log
gpui-shell run ./hello --watch    # reload only
```

`dev` is the way to work on an application: it reloads on change, as `run --watch` does, and logs at `DEBUG`. Watching polls the application directory four times a second, debounces a burst of writes for 200 ms, and reloads. A reload re-reads **every** module, entry point included — a hot-reload that quietly served a stale import would be worse than none, because it looks like it worked.

A reload does all of its fallible work before it touches the live View. If the new code fails to load, the previous View keeps running, the error goes to stderr, and a toast with a stable id reports it in the window; the next successful reload retracts that toast. A broken save never costs you the window.

Neither relaxes the sandbox. `--dev`, with `run` or `dev`, enables development mode before the runtime is constructed: it restores dynamic-code constructors and leaves built-in prototypes writable, while capability checks remain unchanged. It does not reload or change the log level. See [Capabilities](./capabilities.md#the-sandbox).

## Move to a Rust host

When the application needs more than the scripts can reach, it gets a Rust binary of its own. None of the JavaScript is rewritten: the scripts, the manifest, the imports and the stored data stay as they are. There are two depths to this, and the first is usually enough.

### Keep the command, add Rust

The `gpui-shell` command lives in the library, so a binary of your own can be that command, under its own name, with Rust functions registered for the scripts. Depend on the published crate:

```toml
[dependencies]
gpui-shell = "0.8.1"
```

```rust
use gpui_shell::{HostModule, HostValue, host::HostBrand};

fn main() {
    gpui_shell::export_module(
        HostModule::new("native")
            .declarations("export function greet(name: string): string;")
            .function("greet", |arguments| {
                let name = arguments.string(0)?;
                Ok(HostValue::from(format!("Hello, {name}")))
            }),
    )
    .expect("register the native module");

    let components = gpui_shell::components::components().expect("component catalog");
    gpui_shell::host::main_with_brand_and_components(
        HostBrand::new("clock", env!("CARGO_PKG_VERSION")),
        components,
    );
}
```

```js
import { greet } from "native";

greet("world"); // "Hello, world"
```

Register modules before calling `main_with_brand_and_components`: an import is linked when the application loads. Everything else is the shipped command — `clock dev -d ./clock`, `clock check`, `clock types` (which now declares `native` too) and `clock agents` behave as `gpui-shell`'s do, and the application reads the same manifest, receives the same grants and keeps its data under the same `id`. `clock` with no arguments runs the `app` directory beside the executable, so a [standalone package](./packaging.md#standalone-packages) is that binary plus the application directory. `bundle` is the one command it refuses, because its launchers install `gpui-shell` releases, which do not carry your modules. [HostModule](./host-module.md) covers what may cross the boundary and how a function reaches host state.

### Build the runtime yourself

A host that owns more of the process — its own windows, a script View beside Rust views, several applications with different grants — builds the runtime itself. It does four things: initialize the library, build a runtime, grant the capabilities it is willing to grant, and mount a script View under a `ShellRoot`. The `gpui-shell` command is itself a thin host that does exactly this.

```rust
use gpui_shell::{Capabilities, ShellRuntime};

gpui_platform::application()
    .with_assets(gpui_shell::AppAssets::new(root.clone()))
    .run(move |cx| {
        // Initializes gpui-base, the shell's default token palette, and the
        // style reflection table.
        gpui_shell::init(cx);

        let runtime = ShellRuntime::new(cx).expect("script runtime");

        // The data directory the gpui-shell command used: the manifest `id`.
        let data = gpui_shell::set_bundle_id("com.example.clock").expect("data directory");

        // Nothing is permitted until the host says so.
        gpui_shell::set_capabilities(
            Capabilities::new()
                .read_roots([root.clone()])
                .write_roots([data.clone()])
                .storage(true),
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

## Command reference

```text
gpui-shell run [<name> | -d <directory>] [--watch] [--dev] [--packaged]
gpui-shell dev [<name> | -d <directory>] [--dev]
gpui-shell
gpui-shell new <directory>
gpui-shell list
gpui-shell check <name> | -d <directory> [--print-spec]
gpui-shell types <name> | -d <directory>
gpui-shell agents [<name> | -d <directory>]
gpui-shell bundle [<name> | -d <directory>] [--platform <platform>] [--icon <png>]
                  [--out <directory>] [--download-url <url>]
gpui-shell --help | --version
```

| Argument            | Meaning                                                                                     |
| ------------------- | ------------------------------------------------------------------------------------------- |
| `<name>`            | An installed application, `~/.gpui-shell/apps/<name>`                                       |
| `-d`, `--dir <dir>` | The application root, or the `main.js` inside it                                            |
| _(none)_            | The application packaged beside the executable, or else this help                           |
| `run`               | Run one application in a window                                                             |
| `dev`               | Run it while developing: reload on change, log at `DEBUG`                                   |
| `new`               | Create a blank application in a new directory                                               |
| `list`              | Print the installed applications and their requested capabilities                           |
| `check`             | Load and render once without a window; exit `0` or `1`                                      |
| `types`             | Write `gpui-kit.d.ts`, link the manifest's dependencies, scaffold config                    |
| `agents`            | Print, or write as `AGENTS.md`, the instructions for coding agents                          |
| `bundle`            | Make a desktop application in `dist/`; see [Packaging](./packaging.md)                      |
| `--packaged`        | With `run`, run a directory as a bundle's launcher does: manifest required, no editor files |
| `--watch`           | With `run`, reload when the sources change; `dev` always does                               |
| `--dev`             | With `run` or `dev`, development mode: relax the sandbox                                    |
| `--print-spec`      | With `check`, also print the element description that was built                             |
