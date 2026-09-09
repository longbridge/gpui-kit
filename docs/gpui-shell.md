# GPUI Shell Architecture

> [!WARNING]
> GPUI Shell is experimental. Scripting interfaces, Standard Runtime
> compatibility, capability semantics and module behavior may change between
> minor releases.

## Status and Scope

This document describes the architecture implemented by `crates/shell`. It is a
source-derived reference, not a proposal. The public exports in
`crates/shell/src/lib.rs`, the Rust API documentation, and the generated
`gpui-kit.d.ts` remain authoritative for individual methods.

`gpui-shell` is a scriptable application runtime built on `gpui-base`. The host
owns rendering, layout, text editing, focus, overlays, and system access; the
script owns composition, presentation, and business logic. JavaScript is the
default scripting language, running on QuickJS.

Two documents come before this one: [Architecture](ARCHITECTURE.md) for the
`gpui-base` seam this runtime sits on, and [Styling and
Motion](STYLING-AND-MOTION.md) for the token model it exposes to scripts.

The runtime is real and runnable — `cargo run -p gpui-shell -- examples/js_todolist`
opens a working application — but it is not complete. §26 states plainly what
exists and what does not.

The crate is under active development, so §26 in particular is a snapshot and
will need re-checking against the source. Two modules are complete in Rust with
nothing above them yet: `dock.rs` has no engine binding, so a script cannot reach
a panel, and the CLI does not use the multi-plugin manager, so it does not load
plugins. Both are exercised inside the crate and noted where they appear below.

---

## 1. Overview

GPUI Shell gives an application layer written in JavaScript the same standing
that a Rust application built directly on `gpui-base` has: it composes base
behavior and owns every visual decision. Rust supplies the element model,
layout, text input, the overlay stack, the theme tokens, and the capability
gate. The script supplies the tree and the style.

Four things make up the runtime:

1. an embeddable script runtime — VM, scheduler, error recovery, hot reload —
   with the VM behind an explicit seam (§6.5);
2. bindings over the `gpui` element and style layer and the `gpui-base`
   behavior layer;
3. a capability-gated system API (`fs`, `localStorage`, clipboard,
   `process`, HTTP, TCP, and WebSocket);
4. a command-line host that runs an application directory, checks it, and
   generates its type declarations (§23).

The plugin runtime model — manifests, discovery, isolated policy, storage, load
and unload — is built (§18). Contribution registration, authorization UI,
packaging and distribution are not.

The goal is one sentence: build an application layer at the iteration speed of
a scripting language while keeping Rust and the GPU on the render path.

### 1.1 Standard Runtime installation and removal

The Standard Runtime is built into `gpui-shell`; a script application does not
install LLRT, Node.js, external package tooling, or a second VM. A Rust host enables it by
depending on and initializing GPUI Shell:

```toml
[dependencies]
gpui-shell = { path = "crates/shell" }
```

```rust
gpui_shell::init(cx);
let runtime = gpui_shell::ShellRuntime::new(cx)?;
```

The shell pins every LLRT crate to commit
`7b95c82a9b15e7ddfb2778eca4b5a63111e74f51`, whose modules share
`rquickjs 0.12`. LLRT supplies Standard Runtime implementations and data types;
GPUI Shell remains responsible for the VM, scheduler, Policy and OS authority.

Scripts import bare GPUI Shell module names:

```js
import { Buffer } from "buffer";
import { WebSocket } from "websocket";
import * as fs from "fs/promises";
import process from "process";
import { connect } from "net";
```

There are deliberately no `node:` aliases and no claim of Node.js
compatibility. Unknown bare imports remain errors; only built-in modules and
manifest-declared Git dependencies are resolved.

| Surface                                   | Implementation and authority                                                                                                                            |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `buffer`, `path`, `url`, `crypto`, `zlib` | LLRT implementation in the existing QuickJS context                                                                                                     |
| `console`                                 | LLRT-compatible surface routed to `gpui_shell::script` tracing                                                                                          |
| `process`                                 | Shell adapter: filtered metadata plus async bounded `run` and host-mediated `exit`                                                                      |
| `os`                                      | Honest read-only subset: platform, architecture, and line ending; no invented home/temp paths                                                           |
| `fs/promises`                             | Shell adapter over capability directory handles; no callback-style `fs` module or ambient `std::fs` access                                              |
| global `fetch`                            | Capability-checked HTTP, including every redirect; 30-second timeout and 8 MiB buffered-body limit                                                      |
| `net`                                     | Capability-checked TCP connect; 30-second I/O timeout and 1 MiB per-call read/write limit                                                               |
| `websocket.WebSocket`                     | Capability-checked `connect` plus asynchronous text/binary `read`, `write`, and `close`; not a browser global or constructor; 8 MiB message/frame limit |

The old `gpui.fs` and `gpui.process` exports have been removed. Existing scripts
must migrate to `fs/promises` and `process`; `console` remains global and is
also importable from `console`.

To remove the Standard Runtime from an embedding application, remove the
`gpui-shell` dependency and calls to `gpui_shell::init`/`ShellRuntime`; it does
not install files or a system runtime. To uninstall a standalone script
application, remove its application directory and, if its persistent data is
also unwanted, remove only its generated directory identity under
`<data-home>/gpui-shell/apps/` (§23). Plugin data is under
`<data-home>/gpui-shell/plugins/<plugin-id>`. Never remove the shared
`gpui-shell` data root when uninstalling one application.

---

## 2. Why It Exists

### 2.1 Three costs in this repository

**Compile cost.** Every application-layer adjustment goes through
`cargo build`. This workspace depends on the Zed git `gpui`, tree-sitter,
syntect, and reqwest; a cold build takes minutes and even an incremental one
pays for a link. For "change a gap, change a color, add a filter," the compile
is longer than the thought behind the change.

**Extension cost.** `crates/base/src/dock` already has half of what a plugin
system needs: `PanelRegistry` rebuilds a panel from a `panel_name` string,
`PanelInfo::panel(serde_json::Value)` lets a panel persist private state, and an
unregistered panel is carried by a placeholder so a layout round-trips intact.
The missing half is that a panel's implementation has to be compiled into the
host binary. Nobody can contribute a panel without forking it.

**Generation cost.** Generating Rust UI requires correct types, correct
borrows, and a successful compile; the feedback loop is the compiler.
Generating a script interface executes immediately, draws immediately, and on
failure throws a recoverable exception while the host process survives.

JavaScript sharpens the third point and blunts nothing about the first two. It
is the best-covered language in public training data, and its type declarations
(§14.4) are a format both editors and models already read.

The same coverage is also a liability: a model writing for this runtime will
reach for `document`, full browser `fetch`, `require("fs")`, ecosystem packages, and
`setTimeout`. Only the documented Standard Runtime subset exists. §19.1 answers
unsupported callable globals with named stubs and rejects unknown modules,
rather than silently pretending broad compatibility.

### 2.2 Who it is for

| Audience              | Situation                                                             | What it needs                                                       |
| --------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------- |
| Plugin authors        | Adding a panel, command, or side tool to an existing Rust application | Stable contribution points, a sandbox, dock persistence             |
| Internal tool authors | Dashboards, ops panels, data viewers, one-off tools                   | Low start-up cost, complete system API, packaging                   |
| Generated interfaces  | A model writing the interface and its interactions                    | Common syntax, recoverable errors, hot reload, a typed API contract |

None of these is "rewrite the product core in a script." That distinction
decides nearly every trade-off below.

### 2.3 What the reference projects do and do not prove

VS Code shows that JavaScript extensions over a single host namespace, with
declared capabilities and contribution points, can carry a very large ecosystem.
Figma shows that QuickJS works as a restricted UI plugin VM in production.
Neovim shows the general shape: the host provides capability, the script
provides extension, and `vim.api` is a stable contract.

What none of them proves is the one thing this runtime depends on: their
scripts are not on the path that rebuilds an element tree. This one is (§20).

---

## 3. Scope and Non-Goals

The runtime binds the `gpui` element and style layer, part of the `gpui-base`
behavior layer, the semantic theme tokens, the window-level overlay stack, and a
capability-gated system API. The script holds complete presentation authority:
style, color, spacing, and state styles are all expressed in script.

Seven things are deliberately absent, and will stay absent:

1. **Rust is not replaced for the product core.** The text editing engine,
   syntax highlighting, LSP, virtualization, and animation stay in Rust.
2. **There is no UI DSL, markup, or JSX.** Interfaces come from ordinary
   functions and builder chains (§5.3). JSX needs a compile step, and "edit a
   line, save, see it" is why this runtime exists.
3. **Script never enters the layout or paint path.** Layout, painting, hit
   testing, scrolling, and IME are entirely in Rust (§8.4).
4. **There is no multi-threaded script.** The VM and GPUI's `App` are both
   main-thread only (§12.4). There is no `Worker`.
5. **There is no dynamic native plugin loading.** Rust has no stable ABI, and
   `dlopen`ed native code inside the process defeats the sandbox outright.
6. **`gpui-base` is not modified.** Everything lives in `crates/shell`.
7. **There is no Node.js or browser compatibility layer.** There is no DOM,
   CommonJS `require`, general Node module resolution, or `node:` namespace. The
   Standard Runtime deliberately exposes selected bare modules plus a
   WinterCG-style `fetch`; those individual APIs do not imply browser or Node.js
   compatibility. The shell's narrow `window` and `process` globals expose only
   GPUI/host capabilities.

---

## 4. Relation to the Existing Architecture

### 4.1 Layering

```text
     JS application            main.js · views · styles · business logic
              │  import ... from "gpui-kit" · "gpui-base" · "gpui-fps"
              ▼
     crates/shell ── gpui-shell
     ┌──────────────────────────────────────────────┐
     │ engine/ seam: QuickJS                        │
     ├──────────────────────────────────────────────┤
     │ CallScope · SpecArena · style reflection     │
     │ materialize · theme tokens · capabilities    │
     │ ShellRoot · entities · typings · watch       │
     └──────────────────────────────────────────────┘
              │
              ▼
     gpui-base              behavior · state · infrastructure (unstyled)
              │
              ▼
     gpui / gpui_platform   elements · style · rendering · GPU · platform
```

Against the dependency diagram in [ARCHITECTURE.md](ARCHITECTURE.md),
`crates/shell` plus a script application occupies the **application-owned UI**
branch: parallel to `gpui-component`, not downstream of it. The seam is one thin
line in that picture, and everything above it is language-independent (§6.5).

### 4.2 Why it binds `gpui-base` and `gpui`

**Presentation authority goes to the script, which is the whole point.** Binding
`gpui-component` would leave a script calling visuals somebody else already
decided; changing a button's corner radius would still mean going back to Rust.
Binding base puts style, state style, spacing, and color entirely in script.

**Layer neutrality.** The shell depends on no product visual system, so any
host can embed it, including one with its own design system. The moment the
shell depended on `gpui-component`, it would impose one set of visuals on every
embedder.

**A binding surface an order of magnitude smaller.** `gpui_base::Button` has 13
public functions against `gpui_component::Button`'s 52; base has 18 direct
dependencies against 31. Base's interfaces are narrower and more stable
precisely because they carry no visuals, which is what makes complete coverage
possible at all — and a binding layer that covers only part of its target is the
hardest kind to use.

**Build size and reach.** The runtime's own iteration speed, binary size, and
WebAssembly viability all benefit from a smaller dependency tree. QuickJS is a
full ES engine and not small; every dependency saved elsewhere is worth having.

**A working precedent.** `crates/base/examples/showcase` is a base-only
application: it implements the dock renderer traits itself, supplies its own
`InputEditorStyle` and colors, wires syntect highlighting, and builds for
WebAssembly. `examples/js_todolist` is that same posture with the composition
and styling written in JavaScript instead of Rust, and `ui.js` deliberately
follows the showcase's visual language.

**The layering is visible in the script's import lines.** Each crate that
provides script API gets a module named after it: `"gpui-kit"` for GPUI's own
elements and what this runtime adds, `"gpui-base"` for base's layout helpers,
components and theme, `"gpui-fps"` for its overlay. A name belongs to exactly one
of them, which makes the boundary argued for above checkable rather than merely
intended — a script that reaches for a component says so at the top of the file,
and the day `gpui-component` becomes bindable it arrives as `"gpui-component"`
without a single existing name changing meaning.

### 4.3 What base-first makes the shell carry

Four costs follow from binding base rather than a styled component library.
They are real, and all four are paid in `crates/shell`.

**The default color tokens are transparent, so the shell installs a fallback.**
`gpui_base::Theme`'s `ColorTokens` derives `Default`, meaning every color starts
as `Hsla { h: 0, s: 0, l: 0, a: 0 }` — fully transparent. `RadiusTokens` and
`SpacingTokens` have real defaults; colors do not. A runtime that only called
`gpui_base::init` would paint an invisible window. The library's `theme.rs`
installs a neutral Rust fallback so embedders remain legible without imposing a
product palette. The `gpui-shell` binary separately embeds
`src/bin/default-tokens.json` and installs that CLI-owned light/dark palette
after shell initialization (§13.3).

**There is no `Root`, so the shell provides `ShellRoot`.** `Root` lives in
`crates/component` and belongs to `gpui-component`. Base ships the parts — `Dialog` and
`Sheet` each build their own viewport-sized host, `ToastManager` and
`ToastStackState` own stacking geometry, `FocusTrapElement` owns focus trapping
— but nothing in base decides what happens when two of them are open at once.
`root.rs` is that decision (§16).

**There is no Icon, TitleBar, or Notification component.** `Icon`, `IconName`,
`TitleBar`, and `window_border` are all in `crates/component`. Scripts load icons with
`svg(path)`, resolved against the application directory by `assets.rs`, and draw
their own chrome.

**The dock draws nothing.** A `DockArea` built without a renderer docks, drags,
and persists, but paints no chrome at all. Supplying those renderers is the work
described in §15, and it is not done.

### 4.4 Constraints on existing crates

`crates/base` and `crates/component` are unchanged; `crates/shell` depends on
`gpui-base` (with its `inspector` feature) and `gpui`, and on neither
`gpui-component` nor `crates/component`. Consumers who do not add `crates/shell` see no
change to their build output or dependency tree.

`crates/shell` enables `gpui-base/inspector` unconditionally, which forwards to
`gpui/inspector`. That is not optional: the style reflection tables are behind
`#[cfg(any(feature = "inspector", debug_assertions))]`, so without the feature a
release build would expose an empty style surface (§13.1).

---

## 5. Design Principles

**5.1 The host provides capability; the script composes and presents.** A
script can do exactly what the host registered, no more. Adding capability is an
explicit host action — which is also why quickjs-libc's `std` and `os` modules
are never registered and there is no Node compatibility layer.

**5.2 Elements are values, not objects.** `Button.new("id")` returns an element
_description_ that expires when the render pass ends. This is a direct
consequence of GPUI's element model (§8.1), not a stylistic choice.

**5.3 No DSL, no JSX.** Interfaces are built with builder chains that
correspond one-to-one with the Rust API, so learning one teaches the other. A
DSL would need its own parser, diagnostics, editor support, and version
evolution. JSX would need a compile step, and "edit a line, save, see it" is the
reason this runtime exists. This matches the GPUI builder style in
`CLAUDE.md`: keep one fluent chain and express conditions with `when`.

**5.4 A context is valid only for the duration of a call.** `&mut App`,
`&mut Window`, and `&mut Context<T>` are borrows. `CallScope` turns "am I inside
a legal host call?" into a runtime-checkable fact, so an out-of-scope access
throws a script exception rather than reading a dead stack frame (§9).

**5.5 Binding tables are data.** The no-argument style surface comes from
GPUI's reflection tables with no hand-written names, and `gpui-kit.d.ts` is
generated from the same tables the dispatcher uses (§14.4). The failure mode of
hand-written bindings is not that they are tedious; it is that upstream changes
a signature and the binding does not follow.

**5.6 Presentation belongs to the script and must be replaceable.** The Rust
side installs no visual decision beyond the color tokens, which exist only in
overridable form. Anything more would amount to a third, uncontrolled visual
system on top of base.

**5.7 No capability by default.** `Capabilities::default()` is the empty set,
and every field is private with a builder (`capability.rs`). "No capability by
default" is therefore a fact about the type, not a promise in prose (§19.2).

**5.8 Failure is recoverable.** Every script error collapses into one exception
carrying the script's own stack: it is logged, it is shown as a failure surface
where the interface should have been, the rest of the host keeps working, and no
Rust panic crosses the boundary (§21.1).

**5.9 The engine is a parameter, not part of the architecture.** Anything that
can live above the seam lives above the seam, and anything that lives inside an
engine has to justify why it could not (§6.5).

---

## 6. Runtime Overview

### 6.1 Modules

| Module            | Responsibility                                                                                    | Side of the seam |
| ----------------- | ------------------------------------------------------------------------------------------------- | ---------------- |
| `engine/`         | VM lifecycle, module loading, method dispatch, callbacks, exception conversion                    | below            |
| `engine/quickjs/` | The engine: prelude, host API, scheduler, overlays, entity API, native bridge, theme API, sandbox | below            |
| `scope`           | `CallScope`: the host-context stack, its phases, and generation checks                            | above            |
| `snapshot`        | `RenderSnapshot`: what one script render publishes and every frame replays                        | above            |
| `metrics`         | `RuntimeMetrics`: script renders and materializations, with their timings                         | above            |
| `spec`            | `SpecArena`: element descriptions, single-use checks, `debug_tree`                                | above            |
| `materialize`     | Replays a snapshot into real GPUI elements; pure Rust, non-destructive                            | above            |
| `style`           | The reflected style table, the parametric bindings, spelling suggestions                          | above            |
| `theme`           | The default semantic palette and token-name resolution                                            | above            |
| `value`           | `Bridged`: the neutral script argument value, and color and length coercion                       | above            |
| `error`           | `ShellError`: the neutral error type                                                              | above            |
| `entities`        | `EntityStore`: retained state by handle, one store per runtime                                    | above            |
| `capability`      | The capability set, the installed grant, the path resolver, and denial messages                   | above            |
| `runtime`         | `CallbackArena<T>` in snapshot generations, application-root resolution, the failure surface      | above            |
| `root`            | `ShellRoot`: the window-level overlay stack                                                       | above            |
| `dock`            | `ScriptPanel`, `ScriptDockSkin`, panel registration and name interning                            | above            |
| `native`          | The host-registered native module registry                                                        | above            |
| `view`            | `ScriptView`: the one bridge into GPUI's render loop, and where a snapshot lives                  | above            |
| `assets`          | Application-directory asset source for `svg(path)`                                                | above            |
| `watch`           | Source watching and in-place reload                                                               | above            |
| `typings`         | `gpui-kit.d.ts` generation                                                                        | above            |

The ratio is the argument. Above the seam are the element model, styling,
theming, capabilities, and context safety — the actual design. Below it is what
a script value looks like.

### 6.2 Key Rust types

`ScriptView` is the only way script output reaches GPUI. Every script-defined
view, dialog body, and sheet body is carried by one:

```rust,ignore
pub struct ScriptView {
    runtime: Rc<ShellRuntime>,
    object: ViewObject,
    policy: Rc<Policy>,
    current: Option<RenderSnapshot>,
    dirty: bool,
}

impl Render for ScriptView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.is_dirty() {
            self.rebuild(window, cx);
        }
        materialize(&self.runtime, self.current.as_ref().unwrap(), window, cx)
    }
}
```

The real implementation also overlays a recoverable error on the previous good
snapshot and handles the no-snapshot failure case; the sketch shows the normal
path and, critically, that a clean render only materializes cached data.

`ScriptView` carries `ViewObject` and the render state of §8.4 — the published
snapshot, the one it replaced, the dirty flag. Under QuickJS a `ViewObject` is a
`Persistent<Object>`; `ScriptView` needs to know that no more than it needs to
know what a JavaScript object is. It also exposes `replace_object`, which is what
makes a hot reload keep the window, the focus, and the element identities while
swapping only what the script produced (§21.2), and `invalidate`, which is the
Rust half of `cx.notify()`.

`gpui_shell::init(cx)` must run once at startup. It calls `gpui_base::init`,
installs the default palette, and builds the style reflection table so the first
script call does not pay for it.

### 6.3 The engine choice

QuickJS via `rquickjs` 0.12, with the `macro`, `loader`, `classes`, and
`properties` features. It is the only engine, and it sits behind the seam of
§6.5 anyway.

JavaScript is the choice for one reason, and it is a product reason rather than
a technical one: application code reads better in it, and the model corpus is
the best there is. The costs are real and are not offset. QuickJS is a bytecode
interpreter with **no JIT**, so neither hot loops nor per-call boundary cost will
ever beat a JIT-compiled engine, and a full ES engine — regex, Unicode, the
lot — is not small.

Two properties of QuickJS turned out to matter more than expected. Reference
counting means a host handle — `Persistent<Function>`, `Persistent<Object>` — is
released the moment its last reference goes away, which removes a layer of
uncertainty from the cross-GC cycle problem in §7.4; true cycles still wait for
the collector. And because it emits no machine code it has no W^X problem, which
a tracing JIT does have on Apple Silicon. It also compiles to WebAssembly, being
plain C.

The measurement is in §20.3, and what it now measures is different from what it
originally did: script description cost is no longer frame cost (§8.4), so the
per-call boundary cost that would decide between engines matters less than it
used to. It still decides, and it is still the number that would justify adding a
second engine — but the argument for one is weaker than it was when every frame
paid it.

The constraint the seam imposes on any second engine is that the script API stays
inside the semantic intersection. That does not mean two languages would look
alike — it means the same use case must produce the same description tree, and
the same application activity must produce the same number of script renders.

### 6.4 The JavaScript surface

One import, one module. Components are type tables with a single `.new`:

```js
import { View, text } from "gpui-kit";
import { v_flex, Button } from "gpui-base";

export default class Counter extends View {
  init(props = {}) {
    this.count = props.start ?? 0;
  }

  render() {
    return v_flex()
      .gap(12)
      .child(`${this.count}`)
      .child(
        Button.new("increment")
          .on_click((_event, cx) => {
            this.count += 1;
            cx.notify();
          })
          .child("Increment"),
      );
  }
}
```

The built-in modules are named after the crate that provides the capability:
`"gpui-kit"` for GPUI's own elements and what the runtime adds, `"gpui-base"` for
gpui-base's layout helpers, components and theme, and `"gpui-fps"` for its
performance overlay. A name belongs to exactly one of them, so an import says
which layer a script depends on, and a layer added later — `gpui-component` —
arrives as its own module rather than as more names on `"gpui-kit"`. The Standard
Runtime also provides the selected bare modules listed in §1.1; every other
`import` resolves inside the application directory (§19.1). The entry point is `main.js`, and it must
`export default` a class extending `View`. The host takes that class, constructs
one instance, and mounts it as the window's root view.

Naming follows the Rust side directly. **Where a binding lives in Rust decides
where it lives in JavaScript** — the rule is provenance, not category, so a
binding added later lands in one place and there is nothing to argue about:

| Rust                      | JavaScript                              | Example                                              |
| ------------------------- | --------------------------------------- | ---------------------------------------------------- |
| Method on `App`           | Method on `cx`                          | `App::open_url` → `cx.open_url(url)`                 |
| Method on `Window`        | Method on the `window` global           | `Window::paint_path` → `window.paint_path(path, bg)` |
| Type plus `::new`         | Capitalized type table with only `.new` | `Button::new(id)` → `Button.new(id)`                 |
| Free function             | Lowercase function                      | `div()`, `h_flex()`, `v_flex()`, `s`                 |
| State entity              | Capitalized type table                  | `InputState::new(...)` → `InputState.new({...})`     |
| No GPUI or base original  | Where the web already keeps it          | `localStorage`, `console`                            |
| A standard-runtime module | Lowercase module import                 | `fs/promises`, `process`, `path`                     |
| View base class           | `class X extends View`                  | `export default class Counter extends View`          |

An earlier version of this table mapped by category — "system capability" and
"scheduling" both became module members — and that is what let the script
surface drift away from GPUI's. `open_url`, `spawn`, `read_from_clipboard`,
`write_to_clipboard` and `focus_handle` are `App` methods in Rust, so they are
`cx` methods here; `paint_path` is a `Window` method, so it is on `window`. Note
what the rule also settles: `FocusHandle.new()` was a constructor GPUI does not
have, because `App::focus_handle` is the only way to make one.

An entity is a child wherever a child is taken, exactly as an `Entity<V>` is
renderable in GPUI: `.child(handle)` mounts a retained nested view, and a
`render` may return a handle directly.

#### Style and behavior methods keep their Rust snake_case spelling

`items_center`, `size_full`, `gap_2`, `text_3xl`, `on_click`, and `border_color`
are spelled exactly that way in JavaScript. There are no camelCase aliases, and
that is a deliberate break with JavaScript convention for three reasons.

These names are not hand-written. The whole no-argument style table comes from
GPUI reflection (§13.1); when upstream adds a method, the script surface gets it
for free. Adding camelCase aliases would convert a zero-maintenance table into a
maintained one.

Mechanical conversion is also not well defined over this particular set.
`items_center` → `itemsCenter` is obvious; `gap_2` → `gap2` or `gapTwo`?
`text_3xl` → `text3xl` or `text3Xl`? `rounded_tl` → `roundedTl` or `roundedTL`?
Any single rule produces something awkward across a few dozen names that get
typed every day.

And there should be one spelling for one thing. Two equivalent spellings
immediately split the examples, the type declarations, the documentation, and
the code a model generates.

The cost is honest: a JavaScript author's first `.items_center()` does not look
like JavaScript, and one file then carries two naming conventions. Bound names
are snake_case; anything the author writes — `visible()`, `setFilter`,
`onConfirm` — is camelCase. `examples/js_todolist` reads that way, and in
practice the contrast is useful: a snake_case call is host surface, a camelCase
one is script code.

#### `Button.new(id)`, not `new Button(id)`

The JavaScript habit would be `new Button(id)`. It is not used because the
return value is not an object; it is a description valid for one render pass
(§8.3). `new` implies an instance with identity that can be stored and reused,
which is precisely what an author must not assume here — reusing one throws.
`Button.new(id)` matches Rust exactly and stays neutral about what is being
constructed.

Views, by contrast, use the standard `class extends View`, because a view really
does have identity and cross-frame state and really is owned by GPUI (§7.3). Two
construction shapes in one file, because the two kinds of thing have different
lifetimes.

### 6.5 The engine seam

`crates/shell/src/engine/mod.rs` defines the contract. An engine module exports
one `ShellRuntime` type plus two handle types, `ViewType` and `ViewObject`, that
are opaque to every caller:

```text
ShellRuntime::new(&mut App) -> anyhow::Result<Rc<Self>>
ShellRuntime::new_isolated() -> anyhow::Result<Rc<Self>>
ShellRuntime::arena_mut(&self) -> RefMut<'_, SpecArena>

ShellRuntime::load_app(&Rc<Self>, &Path, entry: &str) -> anyhow::Result<ViewType>
ShellRuntime::load_source(&Rc<Self>, &str, &str) -> anyhow::Result<ViewType>
ShellRuntime::instantiate(&Rc<Self>, &ViewType, &mut Window, &mut App)
    -> anyhow::Result<ViewObject>
ShellRuntime::instantiate_view(&Rc<Self>, &ViewType, &mut Window, &mut App)
    -> anyhow::Result<Entity<ScriptView>>
ShellRuntime::instantiate_view_with_policy(&Rc<Self>, &ViewType, Rc<Policy>,
    &mut Window, &mut App) -> anyhow::Result<Entity<ScriptView>>
ShellRuntime::instantiate_for_view(&Rc<Self>, &ViewType, Entity<ScriptView>,
    &mut Window, &mut App) -> anyhow::Result<ViewObject>

ShellRuntime::build_snapshot(&Rc<Self>, &ViewObject, Option<Entity<ScriptView>>,
    Rc<Policy>, &mut Window, &mut App) -> anyhow::Result<RenderSnapshot>
ShellRuntime::render_to_spec(&Rc<Self>, &ViewObject, Option<Entity<ScriptView>>,
    &mut Window, &mut App) -> anyhow::Result<String>

ShellRuntime::dispatch_click(&Rc<Self>, CallbackId, &ClickEvent, &mut Window, &mut App)
ShellRuntime::dispatch_change(&Rc<Self>, CallbackId, bool, &mut Window, &mut App)
```

The rest of the crate calls nothing else. That sentence is the definition of the
seam: it is not a trait, it is the fact that the layer above uses only these
entry points. A trait would not work — `ViewType` and `ViewObject` carry their
own lifetimes and `'js` annotations, and forcing them through one would move the
complexity into the type system rather than removing it.

The distinction between `instantiate` and `instantiate_view` is load-bearing.
The former is the low-level object path used by description tests. The latter
constructs the JavaScript object without running `init`, creates the
`ScriptView` entity under its final `Policy`, and only then calls `init` inside a
scope carrying that entity. Work started in `init` therefore inherits the right
runtime, policy and weak owner from birth. `instantiate_for_view` applies the
same rule to hot reload while preserving the existing entity.

`load_app` takes the entry file name rather than assuming `main.js`, because a
plugin declares its own entry in its manifest (§18) and the engine is the only
thing that knows the extension a given engine loads. Each source module is
limited to 8 MiB before it enters QuickJS.

#### Exactly one engine

```rust,ignore
#[cfg(not(feature = "quickjs"))]
compile_error!("enable a scripting engine: `quickjs` is the default and the only one today");
```

An engine exports `ShellRuntime`, `ViewType`, and `ViewObject`; two of them would
make those names ambiguous, which is why the feature is a selection rather than a
set. Building with none fails at compile time rather than producing a crate that
exports nothing.

This means one engine implementation per build, **not one runtime instance per
process**. Several `ShellRuntime`s may coexist on the same UI thread. Each owns
its QuickJS VM, heap limit, globals, module cache, callbacks, retained entities
and pending tasks; dropping one runtime cancels only its work. A continuation
keeps a `Weak<ShellRuntime>` and resumes against the VM that created it rather
than consulting the current global runtime.

The practical boundary is:

```text
Policy       permission and host-capability boundary
ShellRuntime VM, heap, modules and lifecycle boundary
OS process   strong security and crash-isolation boundary
```

The default deployment is one runtime per independent application, with
multiple trusted views or plugins sharing it under separate policies. A host
uses separate runtimes when applications need independent globals, memory
ceilings, module caches, reload teardown or shutdown. Separate runtimes are
stronger VM-state isolation, but they are not a security sandbox: all VMs and
Rust bridges still live in one native process. Truly untrusted code needs a
process boundary.

#### The two sides

| Above the seam (no VM name appears in the source)                                                                | Below the seam (what an engine implements)                                              |
| ---------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `snapshot.rs`: what a script render publishes and frames replay                                                  | Engine value → `Bridged` conversion                                                     |
| `spec.rs`: description arena, single-use checks, `debug_tree`                                                    | Module system (ES modules and a resolver, versus `require` and a path list)             |
| `materialize.rs`: descriptions → real elements, pure Rust                                                        | Method dispatch (a shared prototype, versus an `__index` metamethod and a method cache) |
| `scope.rs`: `CallScope`, phases, generation checks, the crate's only `unsafe`                                    | Callback handle type (`Persistent<Function>`)                                           |
| `style.rs`: reflection table, parametric styles, spelling suggestions                                            | Exception conversion (`ShellError` → the language's own exception)                      |
| `theme.rs`: default palette and token resolution                                                                 | View definition shape (`class extends View`)                                            |
| `capability.rs`: capability set, **the installed grant**, and path resolution                                    | Sandbox specifics: language trimming, intrinsics, promise pumping (§19)                 |
| `value.rs`, `error.rs`, `entities.rs`, `runtime.rs`, `root.rs`, `view.rs`, `watch.rs`, `typings.rs`, `assets.rs` |                                                                                         |

The seam's load-bearing rule is about _when_, not what: `build_snapshot` is the
only entry into script `render`, and nothing calls it per frame (§8.4). An engine
that rendered opportunistically would put script cost back on the frame budget.
Benchmark C (§20.3) is what catches it.

**And it is dependency isolation, not a replaceable-engine contract.**
`ShellRuntime`, `ViewObject` and `ViewType` are re-exports of concrete QuickJS
types rather than associated types behind a trait, so adding a second engine
means editing `engine/mod.rs` and matching a structural surface nothing checks —
a port, not an implementation. The isolation is still worth its keep: nothing
above the directory names a script value, and host configuration either has an
entry here or fails to build. Making it a real contract, with a fake engine to
compile against, is work for when there is a second engine to write.

**Host configuration crosses this line in one direction only.** The grant used to
live inside the QuickJS module, with the crate root calling into it and a silent
no-op compiled in for any other build — so a second engine could compile, run,
and ignore the security configuration without a word. A grant is a decision about
the _application_, not about the interpreter, so it is above the seam now and an
engine can read it but cannot answer it. `set_store_path` and
`set_development_mode` stayed engine-side but are part of the contract above, so
an engine either provides them or does not build. There is no fallback left to be
silent with.

#### Adding capability

Any new capability goes above the seam unless the language genuinely prevents
it. Three questions in order: does it need to know what a script value looks
like? Can it be expressed with `Bridged`, `SpecOp`, and `ShellError`? If it truly
must live in an engine, does it belong to the language or merely happen to have
been written there?

That rule has not held, and with only one engine the pressure to hold it is
weaker rather than stronger — which is exactly why it is written down. The
QuickJS engine has grown `host.rs`, `scheduler.rs`, `sandbox.rs`, `overlay.rs`,
and `entity_api.rs`. Some of that is legitimately engine-specific: promise
pumping and intrinsic trimming have no meaning above the seam. The parts that are
not — the `fs` and storage surfaces, whose bodies are a capability check plus one
`std::fs` call — should have landed above the seam with only argument shuffling
left in the engine. §25 treats this as the standing risk it is.

Asynchrony is the one gap the original design named and the implementation closed
inside the engine rather than in the contract. QuickJS requires the host to drain
its job queue or nothing after an `await` ever runs (§12.2), and that is not a
shape every engine shares. The scheduler therefore lives in
`engine/quickjs/scheduler.rs` and does not appear in the contract above. That is a
defensible place for the pumping, and an indefensible place for the ownership and
cancellation model, which is neither engine-specific nor duplicated anywhere.

The render path no longer drains inline. A snapshot build checks whether QuickJS
has jobs queued and, if it does, hands the drain to the foreground executor;
otherwise it does nothing at all, which is the usual case. That keeps
continuations — application code of unbounded length — off the path a render
took. Every other entry point still drains inline, because an event handler
resuming its own `await` promptly is what an author expects.

---

## 7. Object Model

Every object crossing between script and Rust belongs to exactly one of three
classes.

| Class           | Rust side                   | Script side                                                   | Lifetime                     | Examples                             |
| --------------- | --------------------------- | ------------------------------------------------------------- | ---------------------------- | ------------------------------------ |
| **Value**       | Small `Copy`/`Clone` data   | number, string, boolean, plain object                         | Copied on transfer           | `Pixels`, `Hsla`, `ElementId`, enums |
| **Description** | A node id in an arena       | A lightweight object over a shared prototype, carrying `__id` | **One render pass**          | `div()`, `Button.new(...)`           |
| **Entity**      | `Entity<T>` behind a handle | A handle object with methods                                  | Across frames, owned by GPUI | `InputState`, `ScriptView`           |

### 7.1 Values

`value.rs` owns every coercion, so the rules are defined once. `Bridged` has
four cases — `Nil`, `Bool`, `Number`, `Str` — and everything above the seam sees
only those.

| Script input                         | Target           | Rule                          |
| ------------------------------------ | ---------------- | ----------------------------- |
| `12`                                 | `Pixels`         | `px(12.)`                     |
| `"50%"`                              | `DefiniteLength` | `relative(0.5)`               |
| `"12px"`, `"1rem"`                   | `AbsoluteLength` | Explicit unit                 |
| `"auto"`                             | `Length`         | `Length::Auto`                |
| `"#1e88e5"`, `"#1e88e5cc"`, `"#f00"` | `Hsla`           | Hex parsing, three lengths    |
| `"accent"`                           | `Hsla`           | Semantic token lookup (§13.3) |

`null` and `undefined` both collapse to `Bridged::Nil`, because at a call site
they mean the same thing: the argument was not given.

An error over an enumerated set names the valid members. The implemented
wording is:

```text
unknown color token `surfacee`; expected one of: background, foreground, surface, … — or a #rrggbb literal
```

That is an order of magnitude more useful than `invalid argument #1`, and it is
the reason the token name list is a real constant rather than something derived
at the call site.

The length grammar is narrowed per method rather than parsed per method: the
three GPUI length types nest (`Length` ⊃ `DefiniteLength` ⊃ `AbsoluteLength`),
so `style.rs` parses once and narrows afterwards, which lets the error say
_which_ form was rejected. `.p("auto")` reports that padding needs a definite
length; `.rounded("50%")` reports that radius needs an absolute one.

`line_height` is the single exception in the grammar: a bare number is a
multiplier, not pixels, because `line_height(1.45)` means 1.45× the font size
everywhere else in the industry and 1.45px is never what anyone meant. A string
still goes through the ordinary grammar.

### 7.2 Descriptions

See §8. The constraint is that a description expires when the pass that built it
ends, and reusing one is an error rather than a surprise.

### 7.3 Entities

Retained state lives in `entities.rs`, and a script holds a **handle**, not an
entity reference. The store is a thread-local slot vector; a released slot is
reused before the vector grows, so an application that opens and closes many
inputs does not leak handle space.

```js
const state = InputState.new({ placeholder: "Search" });
state.set_value("hello");
state.value();
state.on("submit", (event, cx) => {
  /* ... */
});
state.release();
```

The rules:

1. A handle that no longer resolves throws — `this input state has been
released` — rather than returning `undefined`. In JavaScript an `undefined`
   travels a long way before it fails, and where it finally fails says nothing
   about where it came from.
2. Creating an entity needs a live host call and is refused during `render` and
   `layout`: `InputState.new(...) cannot run during render; create state in
init() or in an event handler and keep it on the view`.
3. **Subscriptions are owned by the store, not returned to the script.** A
   dropped GPUI `Subscription` stops delivering, and a script has nowhere
   sensible to keep one, so a handler that silently stopped firing would be
   nearly undiagnosable. The store holds them for the lifetime of the handle.
4. Releasing a handle does not release the Rust entity; GPUI still owns it.

`entities.rs` also installs the editor style when it creates an input, for the
same reason `theme.rs` exists: `InputEditorStyle::default()` is entirely
transparent, so an input built without one renders invisible text. The shell
owns the default palette, so it owns this too.

The event names a script can subscribe to are `change`, `submit`, `focus`, and
`blur` — named for what they mean rather than for the key that produced them, so
`submit` covers base's `InputEvent::PressEnter`. An unknown name reports the
full valid set.

### 7.4 Cycles across two garbage collectors

The classic embedded-script leak: Rust holds a script closure, the closure
captures a handle to a Rust entity, and neither collector can see the other's
edge.

Render-bound callbacks (`on_click`, `on_change`) live in `CallbackArena`, in a
generation owned by the snapshot that registered them. Dropping the snapshot
retires the generation, so they never form a long-lived cycle. `CallbackId`
encodes the generation in its high 16 bits; a view keeps two snapshots — the
published one and the one it just replaced — because an event can be dispatched
against a superseded frame.

Long-lived callbacks — entity subscriptions, timers, task continuations — are
bound to an owner. A timer or spawned task holds a `WeakEntity<ScriptView>`, so
when the view goes away the callback is skipped rather than writing into state
nothing will render (§12.3). `ShellRuntime::drop` clears the callback arena,
shuts the scheduler down, and clears the entity store, all before the QuickJS
runtime is torn down — a `Persistent` released after its runtime aborts the
process, which is why field declaration order in `ShellRuntime` is load-bearing
and commented as such.

What is not yet built is observability: there is no `gc_stats`, so a slow leak
would be found by noticing memory rather than by reading a number.

---

## 8. The Render Protocol

This chapter is language-independent: `snapshot.rs`, `spec.rs` and
`materialize.rs` sit above the engine seam and name no script value.

### 8.1 The constraint: GPUI elements are consumed values

```rust,ignore
#[derive(IntoElement)]
pub struct Button { /* ... */ }

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement { /* ... */ }
}
```

Three facts decide everything downstream. `render(self, ...)` consumes the
element by value, so an element value can be used exactly once.
`.child(impl IntoElement)` likewise takes its child by value. And a view's
`Render::render` rebuilds the entire tree from scratch on every redraw.

The third fact is about _elements_, not about descriptions, and conflating the
two was the original mistake in this chapter. Elements must be rebuilt every
redraw. The description they are built from must not be — see §8.4.

A JavaScript object therefore cannot _be_ an element, and a mapping from a
script `Button` object to a Rust `Button` entity does not exist, because
`Button` was never an entity.

### 8.2 Why descriptions rather than a retained tree

Two alternatives were considered and both stay rejected.

A **retained script control tree with a Rust mirror** would mean building a
virtual DOM and a reconciler on top of GPUI, which already rebuilds from scratch
every frame. The reconciler would exist only to undo GPUI's model.

**Plain data object literals** — a script returning nested objects for Rust to
interpret — are exactly equivalent to the builder chain but constitute a second
way to write the same thing. Examples, documentation, type declarations, and
generated code would immediately split into two dialects, and JavaScript makes
that temptation sharp, because object literals are the most natural thing in the
language and React has made "UI is data" reflexive. What it would buy — fewer
host calls per description — is worth much less now that a description is not
rebuilt per frame (§8.4), and what remains is available from virtualization
instead (§20.6).

### 8.3 The description arena

```rust,ignore
pub struct SpecArena {
    nodes: Vec<SpecNode>,
    /// Nodes already attached to a parent. Re-using one is an error.
    parented: Vec<bool>,
    /// Nodes consumed by an op rather than by a parent — a state style's
    /// declarations. They take style ops but can never enter the tree.
    claimed: Vec<bool>,
}

struct SpecNode {
    component: Option<Component>,
    ops: SmallVec<[SpecOp; 8]>,
    children: SmallVec<[SpecId; 4]>,
}

pub enum SpecOp {
    NullaryStyle(u16),                       // index into the reflection table
    ParamStyle(&'static str, SmallVec<[Bridged; 2]>),
    Method(&'static str, SmallVec<[Bridged; 2]>),
    Callback(&'static str, CallbackId),
    StateStyle(&'static str, SpecId),        // hover / active / focus
}
```

A script-side element wraps a `SpecId` and nothing else — in JavaScript, an
`__id` property on an object created from the shared prototype. Each method call
pushes one `SpecOp` and returns the same object, which is what makes the chain
work.

`Component` currently covers `Div`, `HFlex`, `VFlex`, `Text`, `Button`, `Link`,
`Checkbox`, `Switch`, `Svg`, and `Input`. `Link` takes an absolute HTTP(S)
target through `.href(url)` and delegates opening it to the host. `Input` is
addressed by its entity handle rather than by an id, because the state is what
identifies it and the state outlives the description.

**Rust's move semantics survive the trip into a garbage-collected language as an
explicit runtime error.** Attaching a node sets `parented`; touching it again —
adding it to a second parent, or reusing it across frames — reports:

```text
element `Button` was already added to a parent; elements are single-use values
```

A node from an earlier render reports differently, because it is a different
mistake:

```text
this element belongs to a previous render pass; elements are single-use values
and must be rebuilt each time render runs
```

`claimed` covers the third case: the detached node that collects a state style's
declarations (§10.4) takes style operations but can never enter the tree, and
says so if a script tries.

`debug_tree` renders the arena as text, which is what makes interface structure
assertable without a GPU (§22.1):

```text
v_flex .size_full .items_center .gap[Number(12.0)] .bg[Str("background")]
  text "Count: 0" .text_size[Number(12.0)] .text_color[Str("foreground")]
  Button "increment" :accessibility_label[Str("Increment")] .h[Number(28.0)] .bg[Str("primary")] :hover(.opacity[Number(0.9)]) :on_click(fn)
    text "Increment"
```

### 8.4 The render snapshot

A GPUI render is not a script render, and the distinction is the load-bearing
one in this runtime.

GPUI repaints for reasons the script knows nothing about: a pointer crossing a
button, a text cursor blinking, a list scrolling, an animation advancing. None of
those is a reason to enter the VM. So a script `render` does not describe _this
frame_. It describes the interface once, into a `RenderSnapshot`, and every
frame after that replays the snapshot in Rust:

```text
      SCRIPT WORLD                     NATIVE WORLD

  script state changes
        │
        │ cx.notify()  →  script_dirty = true, GPUI notify
        ▼
  ScriptView::render ── dirty? ──no──▶ materialize(snapshot) ──▶ GPUI
        │                                        ▲
       yes                                       │
        ▼                                        │
  CallScope(Render) · script render(cx)          │
        │                                        │
        ▼                                        │
  ┌──────────────────┐                           │
  │ RenderSnapshot   │───────────────────────────┘
  │  SpecArena       │        replayed by every frame
  │  root SpecId     │        until something invalidates it
  │  callback gen    │
  └──────────────────┘
```

Building one:

```text
ScriptView::render(window, cx), snapshot dirty
        ├─ script_dirty = false, before the script runs               (see below)
        ├─ SpecArena::reset() on the runtime's scratch arena
        ├─ CallbackArena::begin() → a fresh generation, not yet callable
        ├─ CallScope::enter(phase = Render)                            §9
        ├─ call the script's render(cx)  →  root SpecId
        ├─ CallScope::exit()
        │
        ├─ on failure: CallbackArena::abort() · arena reset · keep the old
        │              snapshot · record the message · draw it over the
        │              interface that still works                      §21
        │
        ├─ on success: CallbackArena::commit()
        ├─           mem::take the arena → RenderSnapshot
        ├─           publish: previous ← current ← new
        └─ CallScope::enter(phase = Task) · drain the job queue        §12.2
        │
        ▼
materialize(snapshot) → AnyElement                            (pure Rust)
        │
        ▼
GPUI layout / paint (never re-entering script)
```

**Publication is transactional.** The scratch arena and the open callback
generation are staging; a script that throws half-way discards both and leaves
the previously published snapshot — and the handlers registered with it —
untouched. A failed render loses the _new_ interface, never the old one.

**The dirty flag is cleared before the script runs, not after.** Draining the
job queue at the end of a build can notify the same view, and that notify has to
survive into the next frame rather than be erased by the build already in
flight.

**A failed build is not retried until something invalidates the view again.** A
render that throws on every call would otherwise be exactly as frame-coupled as
one that succeeds.

**A failure is reported over the interface, not instead of it.** Because the old
snapshot survived, there is usually still something correct to draw; blanking it
would cost the reader their scroll position and their focus in exchange for a
message that fits in a strip. `error_banner` is that strip. The full-screen
`error_overlay` is kept for the one case with nothing to preserve — a view whose
very first render failed.

#### What invalidates a snapshot

- `cx.notify()` from an `Event` or `Task` phase — it marks the view dirty _and_
  notifies GPUI, leaving the scheduling and coalescing of the repaint to GPUI.
  Three notifies before the next frame rebuild one snapshot, not three.
- `ScriptView::replace_object` — hot reload (§21.3). The entity, the window and
  the focus survive; the description does not.
- `ScriptView::refresh` — the Rust half of the same request, for a host that
  changed state the script reads through a native module (§17.6). **A bare
  `cx.notify()` on a `ScriptView` is a repaint and nothing more**, which is the
  right call for a host-side animation and the wrong one for host data. The two
  are separate methods because they are separate requests, and conflating them
  was the one behaviour change this lifecycle forced on embedders.
- A palette change. `bg(cx.theme().colors.surface)` records a concrete `Hsla` while the
  script runs, so the palette is baked into the snapshot and a repaint cannot
  pick up a new one. `theme::generation()` is compared against the generation the
  snapshot was built at.

Nothing else does. In particular hover, focus, active, scrolling, cursor blink,
input editing and animation are native throughout.

#### Materialization

`materialize` is a depth-first walk in pure Rust: it reads each node from the
snapshot's arena, replays its ops in order, recurses into children, and produces
an `AnyElement`. It is **non-destructive** — that is what makes a snapshot
replayable — and it never calls script, which is what lets it be benchmarked and
snapshot-tested independently of the VM (§20.3, §22.1).

Two things materialization decides that the description cannot.

**Text color is inherited while walking the description.** GPUI resolves
inherited text color at paint time, but an `svg` will not paint at all unless
the color is on its _own_ style — and materialization is the last point at which
the description is available. So `materialize_node` carries a color down the
tree: each node passes its own `text_color` if it set one and the ambient color
otherwise, and an `svg` writes the result into its own style. That is what makes
an icon inside a dark button come out light without the script saying so twice,
and it is the reason `examples/js_todolist` can write `icon("check", 11)` and
have it follow its row.

**An element becomes stateful only when a state style needs identity.** GPUI's
`hover` works on any interactive element, but `active` and `focus` need a stable
element identity. A plain `div` therefore stays identity-free unless a state
style demands one, at which point it takes an `ElementId` derived from its
position in the description. That position is a _snapshot-local address_: it is
stable for as long as the snapshot lives, and stable across rebuilds only while
the script builds the same tree in the same order — a conditional child appearing
above it shifts every address below.

So a script can name an element instead: `div().id("toolbar")` makes the name the
identity, and it survives the script reordering its own tree. `Button`, `Link`,
`Checkbox` and `Switch` already take an identity from `new(id)` and say so with
a warning rather than ignoring a second one in silence.

`...` materializes as a `div` carrying a string child rather than as a
distinct element type, so every style method works on it unchanged. `Input`
materializes as an `InputBase` frame — not a bare `div`, because `InputBase`
carries the input semantics, the focused state style, and the accessibility role
— with three defaults applied before the script's own styling: a centered row,
full width, and click-anywhere-to-focus. A script can override all three and
does not have to remember any of them.

A component that cannot honor a state style says so rather than dropping it
silently: a state style on a `Switch` logs a warning, because `Switch` itself is
not the interactive element (`SwitchTrack` is) and the style has nowhere to land.

#### State styles resolve natively

`.hover(...)`, `.active(...)` and `.focus(...)` record their declarations into a
detached node while the script runs, and materialization turns them into
`StyleRefinement`s that GPUI applies itself. A pointer moving across the
interface executes no script at all. The same rule is what any future animation
API has to follow: the script creates a native animation once, and receives at
most a completion callback — never a per-frame one.

#### The invariants

These are what §22 tests, and what a change to this chapter has to preserve.

1. `Render::render` on a `ScriptView` does not inherently execute script.
2. A clean snapshot may be materialized any number of times without entering
   the VM.
3. Script `render` executes only when the view has been invalidated.
4. `cx.notify()` invalidates script render state and delegates scheduling and
   coalescing of the repaint to GPUI.
5. Native interaction — hover, focus, cursor blink, scroll, animation — stays
   native unless an explicit script event is required.
6. Materialization is a pure native operation over frozen description data.
7. Render-bound callbacks live with the snapshot that produced them, not with
   a frame.
8. Building a replacement snapshot is transactional; a failed script render
   does not corrupt the currently valid snapshot.

### 8.5 Re-entrancy

Several base components call application code back during GPUI's layout and
prepaint phases to render one item: `VirtualList`, `Tree`'s `TreeEntry`,
`Calendar`'s `CalendarItem`, table cells, and the dock renderers of §15. Those
callbacks happen outside `ScriptView::render`.

`ScopePhase::Layout` exists for them. It permits reading state and building
elements, and refuses `notify`, entity creation, and spawning, because changing
state during layout produces either an inconsistent frame or a recursive
invalidation.

`ScriptDockSkin` is the first thing to use it (§15). Every chrome callback runs
inside `in_layout_scope`, which pushes a _new_ frame rather than reusing an
enclosing one — a dock area nested in a script view already has an outer scope —
so each callback starts on a fresh render-time budget and a `cx` captured during
an earlier call is still rejected. The scope inherits the enclosing view, because
chrome is drawn on behalf of whatever view is rendering and owns no view of its
own. `VirtualList`'s item renderer already takes exactly this shape, and
`Tree`'s and `Calendar`'s will when they are bound.

### 8.6 Memoization

Descriptions are now cached across frames wholesale (§8.4), which removes the
problem memoization was first proposed for: repainting an unchanged view no
longer costs a script render at all.

What is left for `gpui.memo` is _sub_-tree granularity — skipping the part of a
rebuild whose data has not changed when the rest of the view has. That is a
smaller win than it used to be, and it is not implemented. §20.6 places it
against the other levers, and §20.7 states its complement: memo reuses a subtree
whose values did not change, a template cache reuses a structure whose values
did.

---

## 9. Context Safety: CallScope

`scope.rs` is the crate's only `unsafe` module.

### 9.1 The problem

GPUI's core contexts are borrows:

```rust,ignore
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement
fn on_click(&mut self, event: &ClickEvent, window: &mut Window, cx: &mut App)
```

A script object's lifetime is decided by the script's collector and cannot carry
a Rust borrow. A `cx` stashed in a module-level variable and used later from a
timer points at a stack frame that is long gone. JavaScript makes this easy to
do, because an arrow function captures its enclosing scope with no explicit act
at all.

### 9.2 The design

```rust,ignore
pub fn enter(window: &mut Window, app: &mut App, phase: ScopePhase,
             view: Option<Entity<ScriptView>>) -> (CallScopeGuard, u64);
pub fn with_context<R>(generation: u64,
             f: impl FnOnce(&mut Window, &mut App) -> R) -> Result<R, StaleContext>;
```

Every Rust → script entry point — render, event dispatch, timer, task
resumption, view construction — pushes a frame with a fresh generation, and
`CallScopeGuard` pops it on return. The script-side `cx` is an ordinary object
carrying nothing but that generation; every use compares it against the top of
the stack, and a mismatch throws:

```text
cx is no longer valid: it was captured during an earlier call and used later.
Use cx.spawn or take cx from the callback arguments instead.
```

#### Two flavors, the way GPUI has two

GPUI draws the same line with the borrow checker: `App` and `Context<T>` are
borrows that cannot outlive their call, and `AsyncApp` is the one flavor you may
hold across an `await` — `Context::spawn` says so in as many words. A script has
no borrow checker, so the line is drawn at run time by `ContextBinding`:

- **`Context`** names one call by generation, and is what `render` and every
  event handler receive. A `cx` stashed in a closure still reports clearly.
- **`AsyncContext`** names no call. It resolves whichever frame is live when a
  member is used, and refuses only when none is. It is what `init` receives and
  what `cx.spawn` and `cx.timer` hand their callbacks.

That is the _whole_ difference: every member gates on the binding's check and
then does ordinary ambient work, so the two cannot drift apart. Nor is ambient
resolution a new mechanism here — it is the majority one. `scope::with_context`
has two call sites outside `scope.rs`; `with_current` and `with_current_app`
back entity creation, the overlays, storage, the clipboard, the theme and every
native module. The overlays were themselves _moved off_ `cx` onto the ambient
`window` global for exactly this reason, and `overlay.rs` records the argument.

Nothing is lost by it. `notify` already reads its view from the ambient
`current_view()`, and every re-homed member was ambient underneath to begin
with, so an `AsyncContext` used from a later call is not working by luck — it is
working the way the operation always worked. What the generation refuses is a
`cx` used in a frame that is merely _different_; what an `AsyncContext` still
refuses is one used with no frame at all.

The `unsafe` is confined to this one module and its preconditions are written
into the module header: the VM and `App` are both main-thread only, so no other
thread can observe the stack; frames are strictly last-in-first-out, enforced by
the guard; and a frame's pointers are only reachable while its guard is alive.

A script cannot forge a generation. The `cx` object exposes no field carrying
it — the number exists only in the Rust closures the object's methods were built
from — so even `Object.keys(cx)` shows nothing but the methods.

Three accessors read the stack without a generation, and each exists for a
specific reason. `with_current_app` lets value conversion resolve a theme token
without threading a context through every coercion. `with_current` gives entity
creation the `Window` and `App` it needs at a point the host does not control.
`current_view` is what a callback registration records so a later `notify` knows
what to reach.

### 9.3 What each phase permits

| Phase    | Permits                                                               | Refuses                                       |
| -------- | --------------------------------------------------------------------- | --------------------------------------------- |
| `Render` | Reading state and the theme, building elements, registering callbacks | `notify`, creating entities, opening overlays |
| `Event`  | Everything: mutating state, `notify`, `spawn`, overlays               | Blocking                                      |
| `Task`   | Same as `Event`                                                       | Blocking                                      |
| `Layout` | Reading and building elements (§8.5)                                  | `notify`, creating or destroying entities     |

Every refusal is a specific message, not undefined behavior:

```text
cx.notify() is not allowed during the `render` phase;
request a re-render from an event handler instead
```

```text
window.open_dialog(content, options) is not allowed during the `render` phase;
overlays may only be opened or closed while handling an event or a task
```

The phase is also what gives the interrupt handler a per-call deadline (§19.3):
render runs on a tighter budget than an event handler, and the change of
generation is what tells the handler a new call has begun.

---

## 10. Events and Callbacks

### 10.1 Registration and lifetime

```js
Button.new("save")
  .child("Save")
  .on_click((event, cx) => {
    this.saved = true;
    cx.notify();
  });
```

The arrow function is not a style preference. It does not bind its own `this`,
so `this` remains the view instance and the handler can mutate view state
directly. A `function () {}` handler gets the wrong `this` — the mistake
JavaScript authors and models make most often here — which is why every example
and every declaration comment uses an arrow function.

`on_click` stores the function in the open `CallbackArena` generation and records
only a `CallbackId` in the description; the high 16 bits are the generation and
the low 16 the index. At materialization Rust builds a closure holding a
`Weak<ShellRuntime>` and that id.

**A callback belongs to the snapshot that produced it, not to a frame.** A
button described by snapshot #42 may stay on screen through hundreds of repaints,
and its handler has to remain callable for all of them. So the generation is
retired when the snapshot is dropped — which is also what keeps two views sharing
one runtime from retiring each other's handlers, since each owns its own
generations.

A view keeps the snapshot it just replaced for one more generation, because an
event can be dispatched against a frame that has already been superseded. An
event that arrives later than that is dropped with a `debug` log rather than an
error — the author did nothing wrong.

A generation that was open when a script render failed is discarded rather than
committed, so a handler registered by a render that then threw is never
callable.

### 10.2 Event objects

Events arrive as plain objects whose field names mirror the Rust structs:

```js
.on_click((event, cx) => {
  // event.click_count === 1
  // event.modifiers === { shift: false, control: false, alt: false, platform: true }
});
```

Only semantics base has already normalized are exposed; platform events are not.
Base has already collapsed "Enter activates the button" and "click the button"
into one callback, and the script should not see that difference.

### 10.3 Controlled values

Base's controlled components report intent rather than mutating their own state
(see "Controlled values" in [ARCHITECTURE.md](ARCHITECTURE.md)), and the
bindings preserve that:

```js
Checkbox.new("agree")
  .checked(this.agreed) // the value comes from script state
  .on_change((checked, cx) => {
    // this is only a request
    this.agreed = checked;
    cx.notify();
  });
```

The shell never quietly maintains a checked state on the script's behalf. Doing
so would give script authors and Rust authors different mental models of the
same control, in the same application.

### 10.4 State styles

Hover, active, and focus styles reuse the ordinary style methods on a detached
node, so there is no second grammar for what a style is:

```js
function saveButton(cx) {
  return Button.new("save")
    .bg(cx.theme().colors.primary)
    .hover((style) => style.opacity(0.9))
    .active((style) => style.opacity(0.8))
    .focus((style) => style.border_color(cx.theme().colors.ring));
}
```

The declaring function receives a detached element; its return value is ignored,
so both a chain and a block body work. In the arena this is
`SpecOp::StateStyle(name, node)` pointing at a node marked `claimed`, and
`materialize` resolves it into a `StyleRefinement` applied through GPUI's native
`hover`, `active`, and `focus` modifiers.

The semantic state styles base offers through `state_style::resolve_style` —
checked, selected, disabled — are **not** bound. A script expresses those
conditionally instead:

```js
function saveButton(cx, disabled, selected) {
  return Button.new("save")
    .when(disabled, (el) => el.opacity(0.4))
    .when(selected, (el) =>
      el.bg(cx.theme().colors.muted).border_color(cx.theme().colors.foreground),
    );
}
```

That is a real gap rather than a simplification, because it means the semantic
precedence rules in [Styling and Motion](STYLING-AND-MOTION.md) are not
available to a script; the script re-derives them with `when`.

### 10.5 Input events

`on_key_down`, `on_key_up`, `on_modifiers_changed`,
`on_mouse_down(button, …)`, `on_mouse_up`, `on_mouse_down_out` and
`on_scroll_wheel` are GPUI's own `InteractiveElement` builders, installed
together by `materialize::with_input_handlers`.

```js
div()
  .track_focus(this.handle)
  .on_key_down((event, cx) => {
    if (event.keystroke === "cmd-s") {
      this.save();
      cx.stop_propagation();
    }
  });
```

Modifier-only presses and releases use GPUI's native event rather than a
synthetic key event:

```js
div()
  .track_focus(this.handle)
  .on_modifiers_changed((event, cx) => {
    this.primaryModifierDown = event.modifiers.control;
    this.capslockOn = event.capslock.on;
    cx.notify();
  });
```

The event preserves GPUI's `ModifiersChangedEvent` shape. Its `modifiers`
object contains `shift`, `control`, `alt`, `platform`, and `function`; its
`capslock` object contains `on`. Like key events, it travels the focus path.

**Where they are installed.** `div`, `h_flex`, `v_flex`, `Button`, `Link`,
`Checkbox`, `Switch`, `Radio`, `Toggle`, `Tabs` and `Tab`. Every other
component builds its own base type and hangs its own listeners on it, so a
handler written there is recorded in the description and never reaches GPUI —
reported by `warn_unhonoured_input`, the way `tooltip` reports the same shape
of gap. Widening the list is one call per component, not a new mechanism.

The two kinds route differently and it shows: a key event travels the focus
path, a pointer event travels the hitbox. So a component that accepts no
script focus handle — `Tab` — hears presses and never hears keys, however well
both are wired. That is a property of the component, not of the wiring.

**One function for both kinds, and that is load-bearing.** An earlier version
factored out only the keyboard and left the pointer inline in the `div` arm, so
every component the helper was applied to answered keys and silently ignored
presses — while the table naming which components honour input claimed all of
it. Half a family is worse than none, because the table is then wrong rather
than incomplete.

**The keystroke spelling is normalized.** `keystroke` is the whole chord, but
not `Keystroke::unparse`: GPUI spells the platform modifier for the platform it
was built for — `cmd-`, `super-`, `win-` — which is right for a keymap a person
reads and wrong for a string a program compares. A script is one file running
on all three, so `event.keystroke === "cmd-s"` has to mean the same thing
everywhere. `materialize`'s `script_keystroke` spells it `cmd` on every
platform, which is a spelling `Keystroke::parse` accepts everywhere, so a
binding and the event it produces agree by construction. `key` and `modifiers`
carry the same chord taken apart.

`cx.stop_propagation()` and `cx.propagate()` mirror `App`'s own pair, and are
not specific to the keyboard: GPUI delivers every event to every handler on the
path, so an element nested inside another that handles the same event fires
both by default.

### 10.6 Actions and key bindings

GPUI actions are types generated by `actions!` at compile time, and a script
cannot produce a Rust type, so the whole family collapses into one
[`crate::action::ShellAction`] carrying the script's own id. `cx.bind_keys`
installs `KeyBinding`s over it, `key_context(...)` declares where a binding
applies, `on_action(id, handler)` handles one, and `window.dispatch_action(id)`
sends one without a keyboard.

```js
init(_props, cx) {
  cx.bind_keys([{ keystroke: "cmd-s", action: "save", context: "Editor" }]);
}

render(_cx) {
  return div()
    .key_context("Editor")
    .track_focus(this.handle)
    .on_action("save", (event, cx) => this.save(cx));
}
```

**One type is not a free simplification, and the place it bites is dispatch.**
GPUI matches an action listener by `TypeId`, and during the bubble phase it
stops after the first listener that matches: an action is handled once, by the
innermost element claiming it. That is exactly right when every action is its
own type. It is wrong when they all share one — two `on_action` registrations
on one element would be two listeners with the same `TypeId`, and the first
would swallow every action including the ones meant for the second.

So `materialize` installs **one** GPUI listener per element however many
actions the script registered, and does by id what GPUI would have done by
type: find the handler, or call `cx.propagate()` so an unclaimed action carries
on outward. `ShellAction::partial_eq` compares ids for the same reason —
comparing types would make every script action equal to every other, which is
the question GPUI asks when it looks up the chord currently bound to one.

`Action::name()` returns `&'static str`, so every `ShellAction` answers one
name for the family. Nothing downstream needs it to distinguish two of them:
dispatch is by `TypeId`, and the id is compared at the listener. The name is
for the keymap's JSON form and for anything that shows an action to a person.

What is still missing is a contribution registry (§18.4): bindings are
installed by a running script rather than declared in a manifest, so they
arrive when `init` does.

### 10.7 Entity subscriptions

```js
const state = InputState.new({ placeholder: "What needs doing?" });
state.on("submit", (event, cx) => this.add(cx));
```

A subscription is a long-lived callback and is owned by the entity store rather
than the script (§7.3). The valid event names come from `InputEventName`, and a
misspelling reports all of them.

---

## 11. State

### 11.1 Three layers

| Layer            | Where it lives                             | Suited to                                  | After a change                |
| ---------------- | ------------------------------------------ | ------------------------------------------ | ----------------------------- |
| View-local       | Fields on the view instance (`this.count`) | Expansion, filters, drafts                 | `cx.notify()`                 |
| Host entity      | `Entity<T>` behind a handle (`InputState`) | Text, and later trees, tables, dock layout | The entity notifies itself    |
| Application-wide | `localStorage` (§17.3) or module scope     | Settings, caches                           | Subscribers notify explicitly |

### 11.2 There is no automatic dependency tracking

No signals, no observables, no automatic `notify`. Three reasons, and the first
is the one that matters: GPUI is an explicit `cx.notify()` model, and two mental
models coexisting in one application interfere with each other. Automatic
tracking would also mean wrapping every view instance in a `Proxy`, a permanent
cost on the render path that QuickJS has no JIT to amortize — the measured price
of one diagnostic `Proxy` is in §13.2. And a forgotten `notify` has a definite
symptom (the interface does not update) that costs far less to diagnose than
over-triggering does.

This has to be said loudly in JavaScript, because the entire front-end ecosystem
assumes the opposite: **there are no signals here, no
`useState`, and no dependency arrays. Change state, then call `cx.notify()`.**

### 11.3 View definition

```js
import { View } from "gpui-kit";

export default class Counter extends View {
  init(props = {}) {
    // once, at construction; phase = Event
    this.count = props.start ?? 0;
  }

  render(cx) {
    // phase = Render; returns exactly one element
    return v_flex().gap(12).child(`${this.count}`);
  }
}
```

`View`'s constructor does one thing: if the subclass defines `init`, it calls
it. Authors do not write `constructor` directly because a `constructor` must
call `super(props)` before touching `this`, and every forgotten `super` is a red
exception; `init` has no such trap.

A view constructed for a dialog or a sheet is built with `new Class(props)`
directly, and `View`'s constructor forwards the argument to `init` — so the same
protocol covers both the root view (constructed with no properties) and an
overlay body (§16).

---

## 12. Asynchrony

### 12.1 Executors

This workspace does not depend on tokio. GPUI supplies a foreground executor
(main thread, same thread as the UI, can reach `App`) and a background executor
(a thread pool for `Send` computation and IO). Script runs only on the
foreground and never enters the background.

### 12.2 Promises, `await`, and the job queue

Script code is asynchronous in the ordinary JavaScript way. What
`engine/quickjs/scheduler.rs` supplies is the half a bare QuickJS runtime does
not have: a clock, an owner for pending work, and somebody to pump the queue.

```js
cx.spawn(async (cx) => {
  await cx.sleep(200);
  // The same `cx` the body was handed. It names no call, so the `await` does
  // not take it away.
  this.ready = true;
  cx.notify();
});
```

**Nothing after an `await` runs until the host drains the queue.** QuickJS keeps
promise reactions in a job queue that only runs on request, so every script
entry point ends with `drain_jobs`: render, click dispatch, change dispatch,
input events, and every resumption the scheduler drives. Two placement rules are
load-bearing and are written into the function's own documentation. The drain
must happen _outside_ `Context::with`, because `execute_pending_job` takes the
runtime lock that `Context::with` already holds. And it must happen _inside_ the
entry point's scope guard, because a resumed continuation is script code that
will ask for a `cx` of its own — which is why a render pass opens a fresh
`ScopePhase::Task` scope around its drain rather than draining under `Render`.

A job that throws is reported and the drain continues; one broken continuation
must not stop the others. The drain is bounded at 100,000 jobs so that
`for(;;) Promise.resolve().then(f)` cannot wedge the frame loop, and hitting the
bound is itself an error log.

**A call-scoped `cx` must not be held across an `await`.** After an `await` the
generation has moved and the old token produces the §9.2 error. This is easy to
get wrong in JavaScript, because the code before and after an `await` shares one
lexical scope and the old `cx` is simply in reach — which is why the contexts
whose job is to survive that are a different flavor. The `cx` a `cx.spawn` or
`cx.timer` callback receives is an `AsyncContext` and stays usable; `render`'s
and a handler's do not, and should not be captured. There is no third way to
obtain one: a module's top level and a bare `constructor` are handed no context
and cannot ask for one, which is deliberate — GPUI has no module top level, and
work started there would belong to no view. `init` is where a view is handed
its context, and where its work starts.

**An unhandled rejection must be visible.** A failed promise with no `catch` is
silent by default in JavaScript. `cx.spawn` adopts the promise it is given and
attaches reporting handlers, so a rejection reaches `tracing::error!` with the
script's own stack rather than vanishing. A body that throws synchronously is
absorbed the same way.

**Top-level `await` is not supported.** Module evaluation must complete
synchronously; anything needing asynchronous start-up does it from `init` with
`cx.spawn`.

### 12.3 Ownership and cancellation

```js
const task = cx.timer.every(1000, (cx) => {
  /* ... */
});
task.cancel();
task.is_done();
```

Every task belongs to a view: `opts.owner`, or the view whose call is in
progress. The task holds a `WeakEntity<ScriptView>`, so when the panel that
started the work closes, the callback is skipped instead of writing into state
nothing will ever render again. That failure mode is worse in script than in
Rust, because it does not panic — it silently mutates an object nobody will look
at.

This includes work started in `init`. The runtime constructs the JavaScript
object first, creates its `ScriptView`, and invokes `init` only after entering a
scope with that entity and its policy. Calling `init` from the JavaScript base
constructor would be too early: the entity would not exist yet, so a promise
continuation could retain the policy but have no view to invalidate or owner to
cancel with.

`opts.owner: null` is the deliberate opt-out for work that must outlive every
view. Any _other_ view is refused rather than silently ignored, because the
engine can only resolve the current view's script instance back to its entity,
and a task that quietly took the wrong owner is exactly the bug ownership
exists to prevent.

There is no language-level way to interrupt a JavaScript function that is
already inside an `await`, so cancellation means the runtime stops resuming it:
a cancelled timer does not fire again, a cancelled `sleep` leaves its promise
pending forever, and a cancelled `spawn` stops having its outcome adopted. A
cancelled task reports itself done and its registry entry is reaped immediately,
so a long-running application does not accumulate one entry per elapsed timer.
Host operations that support physical cancellation add a cancellation hook.
`process.run` uses it to kill and reap its child when the task, owner or runtime
goes away; cancellation is not merely dropping the JavaScript continuation.
The registry accepts at most 1,024 outstanding host tasks per runtime. A call
over that ceiling fails instead of allowing timers or I/O promises to grow the
registry without bound.

### 12.4 No background script

Running script on the background executor is deliberately not offered. Only
host-implemented Rust work can be dispatched there, and its arguments and
results must be plain, thread-transferable data. There is no `Worker`.

### 12.5 Timers

`cx.timer.after(ms, fn, opts?)` and `cx.timer.every(ms, fn, opts?)`, both
owner-bound. The interval on `every` is measured from the end of one callback to
the start of the next wait, so a slow callback delays the next tick rather than
piling ticks up behind it.

There is no global `setTimeout` or `setInterval`. They are not part of the
JavaScript language — a host provides them — and they have no owner, so a
`setInterval` keeps running after the panel that started it is closed, which is
exactly what §12.3 exists to prevent. The names are present as throwing stubs
that point at the replacement (§19.1); a name that errors usefully beats a name
that is simply missing.

---

## 13. Styling and Theme

Presentation authority sits in script, so this is a core chapter rather than a
supporting one. All of it is above the seam: `style.rs` is engine-independent and
`theme.rs`.

### 13.1 No-argument styles: reflected, zero maintenance

`crates/component/src/inspector.rs` already does something this runtime needs:

```rust,ignore
let table: Vec<_> = [
    gpui_base::styled_ext_reflection_methods::<StyleRefinement>(),
    gpui::styled_reflection::methods::<StyleRefinement>(),
]
.into_iter()
.flatten()
.collect();
```

That yields a name → style-method table at runtime. The shell uses the same pair
of APIs — one from `gpui-base`, one from `gpui`, neither requiring
`gpui-component` — and exposes **3,148 no-argument style methods** to script:
`flex`, `flex_col`, `items_center`, `gap_2`, `rounded_md`, `text_sm`,
`size_full`, and the rest. When upstream GPUI adds one, script gets it with no
change here. They are addressed by a `u16` index, so recording a style call
costs the arena two bytes rather than a string.

Three constraints follow.

`FunctionReflection::invoke` takes only a receiver, so reflection covers exactly
the `fn(self) -> Self` shape. Anything taking an argument has to be bound by
hand (§13.2).

Reflection is behind `#[cfg(any(feature = "inspector", debug_assertions))]` in
both crates, so `crates/shell` enables `gpui-base/inspector` unconditionally and
`style.rs` carries a test asserting the table has more than a hundred entries —
a test that is meaningful only when CI runs it in release.

Nine names reflection cannot see are added by hand: `gpui-base` generates its
font-weight helpers with a macro, and the reflection pass does not see
macro-expanded trait methods, so `font_thin` through `font_black` would
otherwise be missing entirely. They are appended after the reflected table and
addressed by the same `u16`.

The QuickJS engine does one more thing at start-up: it hands the name list to a
JavaScript prelude, which loops over it and installs one small function per name
on the element prototype, each forwarding to a single Rust entry point:

```js
const define = (name) => {
  methods[name] = function (...args) {
    __apply(this.__id, name, args);
    return this;
  };
};

for (const name of __styleNames) define(name);
```

Three thousand small JavaScript closures, not three thousand registered Rust
closures — which would cost both memory and one cross-language registration
each.

### 13.2 Styles that take arguments

Fifty-seven methods are bound by hand in `style.rs`, and they are indistinguishable
from the reflected ones at the call site:

```js
function panel(cx) {
  return v_flex()
    .size_full()
    .items_center() // reflected
    .bg(cx.theme().colors.surface)
    .p(12)
    .rounded(8)
    .gap(8); // hand-bound
}
```

They divide as 9 size, 7 padding, 7 margin, 5 position, 6 flex, 6 paint, 8
border, and 9 radius. Five families are deliberately unbound, with the reason
recorded at the head of the array: `shadow` takes a `Vec<BoxShadow>` and belongs
with the animation and token work rather than as a positional argument list;
`cursor`, `text_align`, `text_overflow`, and `font_weight` take GPUI enums and
would each need a name mapping when every variant already has a nullary form
(`cursor_pointer`, `text_center`, `font_bold`); and `scrollbar_width` is
not exposed because the shell owns native overflow behavior but does not yet
offer configurable scrollbar presentation.

Overflow itself is an element behavior rather than a `StyleRefinement` entry:
`.overflow_scroll()`, `.overflow_x_scroll()`, and `.overflow_y_scroll()` wrap a
bounded element in GPUI's retained native scroll container. The element gains
an identity when needed, and an explicit `.id(...)` keeps its scroll offset
attached to the same viewport when the script description changes around it.

#### The cost of a good diagnostic

Prototype dispatch gives no diagnostic by itself: a name that is not on the
prototype never reaches `__apply`, and QuickJS reports it as
`TypeError: not a function` without naming the property. A mistyped style name
would arrive with no clue at all — and giving the call site a real diagnostic is
the entire reason the style surface is methods rather than a string of class
names.

A `Proxy` prototype solves it, and the M0 benchmark measured what it costs:
**1.09 ms → 1.42 ms for 443 nodes, about 30% of the whole description pass.**

So the implementation keeps two prototypes. The plain one is the default. A
render that fails with "not a function" is re-run once against a `Proxy`
prototype whose `get` trap returns a function that throws with a "did you mean"
suggestion, purely to produce the message; the arena is reset between the two
passes and the flag is cleared afterwards. Errors are rare; a 30% tax on every
frame is not.

The cost of _that_ is a string match — `error.to_string().contains("not a
function")` — deciding whether to re-run. It is a fragile hinge, and a QuickJS
wording change would silently degrade the diagnostic to the bare `TypeError`
rather than break anything visibly.

The suggestion itself is Levenshtein distance over the full name list, with a
budget of two edits relaxed to a third of the name for longer identifiers,
because a wrong suggestion is worse than none:

```text
unknown element method `items_centre` (did you mean `items_center`?)
```

Two further guarantees hold regardless of engine. `gpui-kit.d.ts` plus `// @ts-check`
catches the same class of mistake in the editor, before anything runs (§14.4).
And nothing is ever silently ignored: any name reaching `__apply` that the
dispatcher does not recognize throws.

### 13.3 Semantic tokens and the default palette

`gpui_base::Theme`'s `ColorTokens` derives `Default`, so its colors are all
zero — fully transparent. `RadiusTokens` and `SpacingTokens` have real defaults.
Calling `gpui_base::init(cx)` without supplying colors paints an invisible
window.

The shell library does not own a palette registry or a JSON theme format. The
embedding application supplies the active `gpui_base::Theme`. The standalone
`gpui-shell` binary embeds `src/bin/default-tokens.json` as its own product
default so it remains self-contained without turning that file into a library
contract.

And it resolves token names for script. Seventeen colors — `background`,
`foreground`, `surface`, `primary`, `muted`, `accent`, `destructive`, `border`,
`input`, `ring`, and the `*_foreground` pairs — plus seven spacing steps and six
radius steps. A test compares each name list against the serialized field names
of the corresponding token struct, so adding a token upstream without adding its
name here fails.

Token lookup is cached in a thread-local rather than read from the `App` on every
access, and that is a bug fix rather than an optimization. Lookups happen in two
places with different access to the host: while a script records a style (inside
a call scope, `App` reachable) and again while the description is materialized
(outside any scope, `App` _not_ reachable). Reading only through the scope made
every color silently resolve to `None` during materialization — a window that
painted nothing but a black rectangle. The palette changes at most once per theme
switch, so caching it is both correct and cheaper.

Rules for script:

- Prefer a value read during the current call:
  `el.bg(cx.theme().colors.surface)`. Semantic token-name
  strings remain accepted for compatibility. Hex literals (`#rgb`, `#rrggbb`,
  `#rrggbbaa`) are accepted for one-off tools and bypass the theme, so they do
  not follow a theme switch.
- An unknown token is an error listing the valid set, never a transparent
  fallback — that would reproduce the exact failure this module exists to
  prevent.
- This matches `CLAUDE.md`: the theme API exposes semantic tokens, not a growing
  set of component-specific fields.

The preferred read is call-scoped and explicit at each use site, for example
`cx.theme().colors.surface`, `cx.theme().spacing.md`, or
`cx.theme().radius.lg` inside `render(cx)`. Do not destructure or alias the
theme snapshot: keeping the complete access path makes later API refactors
mechanical. It returns every color both as a direct semantic role and under
`colors`, the `spacing` and `radius` scales, and `appearance` plus `is_dark`.
`gpui.theme()` remains a compatibility accessor to the same snapshot. The
snapshot and all three nested token groups are frozen, so scripts can select
tokens but cannot mutate the palette object. The prelude caches that deeply
read-only object only while the serialized palette is unchanged; a theme switch
produces a fresh snapshot on the next read.

`gpui.set_theme({ appearance, tokens })` replaces the active semantic snapshot.
It needs a live host call, validates the complete token object, refreshes windows,
and returns whether anything changed. Applications own named theme registries;
they can keep those objects in JavaScript or load and parse a JSON file.

### 13.4 The preset module

There is no `gpui/preset` module. `examples/js_todolist/ui.js` is what a preset
would be — `button`, `iconButton`, `checkbox`, `field`, `label`, `muted`,
`surface`, `emptyState` — written as ordinary JavaScript in the application, and
it is instructive that it stayed there. Three disciplines still apply to
anything that does ship:

1. it must be script source, replaceable or forkable wholesale;
2. the Rust side installs no visual decision (§5.6), or the shell becomes a
   third, uncontrolled visual system on top of base;
3. it is not a reproduction of `gpui-component` and promises no visual parity
   with it.

The seam's real cost surfaces here: Rust above the seam is written once, but
script shipped with the runtime has to be written per engine. The only control
is keeping any preset thin.

### 13.5 Animation

Bound as target declarations on every element. A script applies the ordinary
target style, then attaches a native policy:

```js
div()
  .id("sidebar")
  .w(this.expanded ? 320 : 64)
  .transition("width", { duration: 180, easing: "ease-out" });
```

`transition` and `spring` support `opacity`, `width`, `height`, `left`, and
`top`. Length motion currently requires pixel targets. The snapshot stores only
the target and policy; `gpui-base::motion` owns keyed retained state, sampling,
interruption, reduced-motion handling, and animation-frame requests. A target
change therefore enters JavaScript once, while every intermediate frame stays
in Rust. The Shell story's standalone Motion section is the runnable example.

---

## 14. Bindings and Generated Declarations

### 14.1 The surface, measured

The bound surface today is small and deliberately so: `div`, `h_flex`, `v_flex`,
`text`, `svg`, `Button`, `Link`, `Checkbox`, `Switch`, `Input`, and `InputState`,
plus
`child`, `children`, `when`, `on_click`, `on_change`, `disabled`, `selected`,
`checked`, `accessibility_label`, `href`, `id`, `transition`, `spring`, and the
three state styles — over 3,148 no-argument and 57 parametric style methods.

That is a small prefix of `gpui-base`. What makes completing it plausible is the
size difference measured in §4.2: base's `Button` has 13 public functions to
`gpui-component`'s 52.

### 14.2 What will and will not be bound

Base's semantic elements, compound behavior roots, and stateful systems are all
in scope: Checkbox, Radio, Switch, Toggle, Link, Input/Textarea, Select,
Combobox, Tabs, Dialog, Sheet, Popover, Tooltip, Scrollbar, Tree, Table,
VirtualList, Dock, and the rest of the [module
families](ARCHITECTURE.md#module-families).

Two things are bound as _state without their element_, and the reason is
render cost rather than cross-language cost. `Calendar` walks its month grid
calling a renderer once per cell — up to forty-two crossings into the VM per
frame, from inside GPUI's layout pass, for cells that carry no behavior at all;
the exception §8 records for a virtual list is one batched call per frame, not
forty-two unbatched ones. What a script cannot work out for itself is the grid,
and `CalendarState::month_days` is public, so the state is bound and the
element is not. `AlertDialog`'s four parts are the opposite case and the same
answer: they are `div`s with fixed ids, `window.open_dialog` already exists,
and binding them would add names without adding behavior.

`input::Editor`'s LSP, folding, diagnostics, and highlighting interfaces are
not, and will not be. They are built from Rust traits and generics —
`InputHighlighter`, `CompletionProvider`, `HighlightStyleResolver` — where
cross-language mapping is both expensive and lossy, and they are exactly the
part that belongs in Rust (§3). An editor should be exposed through a narrow
"here is the text, here is the language, here is the read-only flag" interface
instead.

### 14.3 The binding table, for registered components only

The design called for bindings declared as data and expanded by a macro. For
the built-in `gpui-base` components it still does not exist: their methods are
matched by name in each engine's `apply`, and the behavior name list is a
literal array in `install_globals`.

For registered components it does exist, and it is the descriptor
(§14.7). One `MethodDescriptor` is the runtime's validator, the generated
TypeScript, and the documentation — one source of truth read three times,
which is exactly what the table was for. The built-in surface is the part
still waiting to move onto it.

The style surface already works the way the whole binding layer should: it is a
table, it is generated, and nothing about it is written by hand.

### 14.4 `gpui-kit.d.ts`

`typings.rs` generates TypeScript declarations for the script API.
`gpui-shell types <directory>` writes `gpui-kit.d.ts` next to an application, and the
output is deterministic — no timestamps, no reflection order — so regenerating
after a runtime upgrade produces a reviewable diff.

One file, one ambient module per crate that provides the capability: `"gpui-kit"`,
`"gpui-base"`, `"gpui-fps"`. A name belongs to exactly one of them. The
dependency runs upward only — `"gpui-base"` imports the element and
component-factory types it is built out of from `"gpui-kit"`, and `"gpui-kit"` refers
down only where one shared element prototype forces it: `track_scroll`, `mode`,
`axis` and `cx.theme()` name their argument types with an inline
`import("gpui-base").X`.

What makes the declarations trustworthy is that they are generated from **the
tables the runtime dispatches through**, not transcribed from documentation:

- style methods come from `style::known_names()`, the same list the prelude
  loops over, so a name that type-checks is a name the dispatcher accepts;
- a parametric method's argument type is _probed_: `argument_of` hands
  `style::apply_param` one literal of each shape and sees which are refused, so
  the difference between `Length`, `DefiniteLength`, `AbsoluteLength`, a color,
  and a bare number is decided by the code that enforces it. `.p("auto")` and
  `.rounded("50%")` are type errors for the same reason they throw;
- the color union comes from `theme::color_token_names()`, so a mistyped token
  is a compile error;
- the phase union comes from `ScopePhase` itself.

An unrecognized argument shape is emitted as `never` rather than `any`, so a
style method added without a matching probe literal fails loudly at the first
call site rather than silently accepting anything.

Four things the declarations deliberately do not express, each stated in the
generated file's own preamble. Capability grants: every `fs`, storage,
`clipboard`, and `process` call type-checks, and whether it is _granted_ is a
runtime question types cannot carry. Element and `cx` lifetimes: TypeScript has
no affine types, so reusing an element still type-checks and still throws.
Which methods suit which component: every element shares one prototype, so
`.checked(true)` is declared on all of them and is simply inert on a `div`;
narrowing it would mean inventing a type hierarchy the runtime does not have.
And retained entities.

The application stays plain JavaScript with no compile step; `.d.ts` is an
annotation for the editor and for `// @ts-check`. It is also the form in which
the API is handed to a model, which is an explicit audience.

### 14.5 Drift

There is no automated drift check. The intended one — read `crates/base`'s
public API with `cargo rustdoc --output-format json` and compare it against the
binding table — needs the table of §14.3 to exist first.

Drift within the crate is caught: `typings.rs` has tests asserting that the
declared element methods and the runtime's style table have not diverged, that
no style method collides with an element method, that every parametric method is
classified, that every color token is in the union, and that no internal name
(`__id`, `__apply`, `__gpui`) leaks into the declarations.

The generated declarations now include `Link`/`href`, `cx.theme()`, the deeply
read-only semantic token shapes, motion policies, overlays, retained input,
native modules, the Standard Runtime surfaces, and `set_theme`.
The remaining limitation is structural: without the component binding table of
§14.3 there is no general proof that every future hand-bound export was also
added to the declarations. A check that every `MODULE_EXPORTS` name appears in
the output would close that gap.

### 14.6 A `gpui-component` module

The second step was a `gpui-component` binding as a _second registry_ sharing
the same render protocol, call scope, event model, and arena. It is built;
§14.7 describes the registry it is built on.

```js
import { text } from "gpui-kit";
import { v_flex } from "gpui-base"; // base: the script owns presentation
import { Button } from "gpui-component"; // product visuals, ready-made
```

Four points decided it then, and all four held. The protocol is one thing and
the registries are two, which is exactly what separating the render protocol
from component bindings bought. The crate dependency stays out of the runtime
entirely: it lives in `crates/component-shell`, so linking `gpui-shell` alone
keeps `gpui-component` out of the tree. The two module names are distinct,
because both export `Button` with overlapping method names and different
semantics, and in JavaScript they can be imported into the same file — the
module name is the only thing that can distinguish them. And migration is a
change of import rather than a rewrite: the functions that build the interface
change, the business logic and the state do not.

### 14.7 The component registry

§14.6 described a second registry as the natural next step. It exists.
`crates/component-shell` registers the `gpui-component` catalog against a
registry API that `crates/shell` owns and that names no component. The
dependency runs one way and only one way: the adapter uses both the runtime and
the component library, and the runtime uses neither. `gpui_shell::init`
installs the base layer; `gpui_component_shell::init` installs the catalog and
then the runtime. A host that wants the protocol without the catalog links
`gpui-shell` alone and gets an empty registry, which declares no module at all.

**A registry is built, then frozen.** `ComponentRegistry::new(api_version,
module_specifier)` opens one; `register` and `register_state` add to it;
`freeze` consumes it and returns a `FrozenComponentRegistry`, which is cheap to
clone and shared by every runtime built from it. `freeze` takes `self` so that
"a frozen catalog cannot be registered into" is a fact about the type rather
than a flag checked at run time.

The specifier is the adapter's to choose. The runtime holds no opinion about
which component library it is carrying, so the module name a script imports
from is data on the registry, not a literal in the engine.
`DEFAULT_COMPONENT_MODULE` is `"gpui-component"`, which is what the shipped
adapter picks. A registry may not claim one of the runtime's own module names —
those resolve first, so the catalog would be unreachable rather than overriding.

**Descriptors are built, not written as literals.** `ComponentDescriptor::new`
takes the component's name and its materializer; `with_constructors`,
`with_methods`, and `with_documentation` fill in the rest. The fields are
private for the reason §7 of the coding guides gives: a descriptor crosses the
seam between the runtime and an adapter, and a later field must be an added
`with_` method rather than a break in every adapter that registers anything.

Every method must carry documentation. `register` refuses one that does not,
rather than filling in a default sentence, because a default would make the
published `gpui-kit.d.ts` look documented without anyone having described it.

**Arguments are schemas, and the schema is the validator.** An
`ArgumentDescriptor` pairs a name with an `ArgumentSchema` — string, number,
boolean, element, an entity of a named kind, a callback with a TypeScript
signature, an enum of literals, an array, or an optional. The engine validates
a script's call against that schema before the adapter sees it, and
`typings.rs` emits the matching TypeScript from the same value. A registered
method's declared type and its enforced type cannot drift, because they are one
value read twice.

**Recording and materializing are separate.** A method call from script is
_recorded_: the descriptor's recorder decodes the validated arguments once, into
an adapter-owned `ComponentPayload`, which is stored on the node. Materializing
then downcasts that payload. Nothing matches on a method name per frame; the
string comparison happens once, when the script calls, not once per render.

**`MaterializeRequest` is the whole of what an adapter is handed.** It carries
the constructor payload, the recorded methods, the node's style, its children,
its named slots, and the common behavior the shell resolved (`disabled`,
`selected`, `on_click`). Each part is _taken_, and a part left untaken is
reported when the request drops — a slot an adapter never read is an element
the script wrote and nobody rendered, which is worth a line in the log.

Children come in two mutually exclusive lanes. `take_children` returns
already-materialized `AnyElement`s, which is what an ordinary container wants.
`take_typed_children` returns opaque tokens instead, for a component whose Rust
builder needs the concrete child value rather than an element; the adapter asks
which component each token is and materializes it by hand. Mixing the lanes is
an adapter bug and is reported as an error, not answered with an empty list.

Slots come in two representations, also exclusive. `take_slot` returns the
element eagerly. `take_slot_factory` returns a repeatable
`ComponentElementFactory` for a slot that a deferred GPUI callback — a popover's
content, a tooltip — has to build later, possibly more than once. A factory
leases its snapshot, so a deferred build cannot retain an arena that has already
been cleared, and it is created only when an adapter asks for one by name.
`take_slot` distinguishes absence from failure: `Ok(None)` means the script
wrote no such slot, and a slot that exists but cannot be built is an error,
because collapsing the two would let a script's element disappear with no
diagnostic at all.

**Retained state.** `register_state` publishes a factory that runs on the
application thread and returns an adapter-owned value — a native `Entity`, an
input state, anything that must survive between renders. Script receives an
opaque frozen object; the registry's generated module maps it back to a handle
through a `WeakMap` it alone holds. Because retained state travels as an
ordinary object, the way elements and entities already do, the handle carries a
proof the module stamps on it. The proof comes from `RandomState`'s OS-seeded
keys: anything a script can observe — a process id, a clock, a counter — would
be reconstructible, and a forged handle would reach another application's state.
Retained values are owned by the application generation that created them and
are released with it.

**Effects.** Some native surfaces change state that outlives the render asking
for it: an application menu bar, a window title. `MaterializeRequest::app_effects`
returns a keyed capability — `replace(key, revision, …)` installs, returns a
cleanup, and reinstalls only when the revision changes. Effects belong to the
application generation that installed them and are torn down when it retires,
not when the window closes: keys are scoped per generation, so a reload cannot
replace the previous generation's install, and waiting for the root view would
leave a reloading application accumulating one install per reload. A revision
should be a hash of what the effect renders, never `Debug` output, whose format
is explicitly not stable to key on.

`ComponentWindowEffects` is the shorter-lived counterpart: a transaction within
one event, where `run_once(key, …)` is idempotent for the length of that event
and no longer.

**What is registered, and what is not.** `component-inventory.json` lists every
public `crates/component` module and every Story, and a test fails if that list drifts
from the public exports. Each entry is either registered — naming its
descriptor, its exports, and any companion parts — or explicitly deferred with a
reason. That file is the single answer to "why is X not bound"; a prose list
beside the source would be a second answer, and the one nothing checks.

---

## 15. Dock and Panels

`dock.rs` is what lets a panel come from somewhere other than the host binary,
and `engine/quickjs/dock_api.rs` is the script face of it. Base already had the
half that is hard to build — a layout that is pure data, a `PanelRegistry` that
rebuilds a panel from a name in a persisted file, and a per-panel
`serde_json::Value` that rides along with it — and what it lacked was a way to
point that machinery at a script.

Three halves, and they are independent of each other.

**`ScriptPanel`** is a `gpui_base::dock::Panel` whose body is a `ScriptView`.
It implements behavior only, not the presentation trait one layer up: a script
panel's title, toolbar, and menus are drawn by the skin from the script's own
elements, which is what "the script owns presentation" means here. Its `dump`
writes the script's `serialize()` payload into `PanelInfo::panel`, and
`register_panel` teaches the registry to rebuild it — `PanelScript::build`
first, then `deserialize` with whatever the last save wrote. Three more hooks
carry the rest of a panel's life across the seam: `set_active`, `set_zoomed`,
and `release`, which is where the engine frees the retained handle its own
`build` produced.

**`ScriptDockSkin`** is the appearance. It implements all three renderer traits
and forwards each callback to a `DockChrome`, whose every method has a default
reproducing base's own no-chrome behavior — so an application that draws no
chrome implements none of them and still gets a dock that docks, drags, resizes,
and persists. Base keeps the drag source, drop-target hit testing, keyboard
actions, and focus; a chrome implementation never sees a drag event, a mouse
position, or a hit test, only resolved state through `TabGroupContext`,
`DockContext`, and `TileContext`.

**The script binding** is `DockArea` (a retained entity), `dock_area(area)` (one
description of it), and `dock_content()` (where a dock's own content goes inside
the chrome drawn around it).

### 15.1 Why the area is retained rather than described

The layout is what the _user_ changed. A drag, a resize, a closed tab and a
collapsed dock all happen without a script render, so an area rebuilt from a
description would put every one of them back the way the last render described
it. It is therefore an entity in the store, exactly as an input's text is, and
`dock_area(...)` mounts it.

### 15.2 Commands, not callbacks

A chrome handler runs when its callback or resolved native state changes; the
resulting spec is cached and replayed in Rust on unchanged frames. A callback
registered inside one would therefore have no sound event lifetime and could be
duplicated by later state changes, so it is refused by the same
`ScopePhase::Layout` check used for virtual-list row descriptions.

So a chrome element carries a `DockCommand` instead: `SelectTab { node, index }`,
`ClosePanel { node, panel }`, `MoveTile { panel }` and nine more. A command names
a container and what to ask it, carries no script value, and is resolved against
`DockContexts` — the table the skin files each context in as base hands it past,
cleared once per frame by `materialize` before anything is recorded again. The
contexts are all `Clone` over `Rc` handlers, which is what makes filing them
possible at all.

That is also why a command is only wired onto the generic stateful `div` path:
it needs `on_click`, `on_drag` and `on_drop`, which a `Button` — building its own
interior — does not expose.

### 15.3 The chrome slots

A skin is installed when the area is created and `DockArea` offers no way to
replace it; the handlers belong to whichever snapshot is currently published.
`DockChromeSlots` is the join. `materialize` writes the current six callback ids
as it replays a `dock_area(...)` description — once per frame, before base asks
the skin for anything — and the engine's `DockChrome` reads them when base does
ask. Writing them every frame rather than only on change is deliberate: a
callback id is meaningful only while its snapshot lives, and materialization is
the one place that always runs against the live one.

`ScriptChrome` caches the `SpecArena` and root produced for each native
container, together with the callback id and resolved JSON payload. A match
replays that spec without entering QuickJS; a changed callback or payload
replaces it. The cache is per retained dock and hard-bounded. It stores no
`AnyElement`: the current frame still materializes the spec, which is how
`dock_content()` consumes the native content handed over for that frame.

`dock` is the only hook handed an element. An `AnyElement` cannot cross into
script, so the engine installs it in `ContentSlot` for the length of the call and
the script's `dock_content()` takes it. A chrome that never placed it gets its
content drawn after what it returned, with a warning, rather than losing it.

### 15.4 Every edit is deferred

`add_panel` is handed the token `cx.new(Class)` returned, and the view behind it
has not been constructed yet; `load` rebuilds panels through the registry, which
constructs more. Neither can happen while QuickJS holds its runtime lock, which
is exactly where both calls run. So all three edits — add, remove, load — go
through `PendingNestedOperation::EditDock` and are applied at the same unlocked
boundary a `cx.new` is, in the order the calls were made. Removal does not need
the deferral; it is queued so that ordering holds.

The visible consequence is that `panels()` and `dump()` read the layout as it was
before the current turn's edits, and `on("layout_changed", …)` is where a script
reads it afterwards.

### 15.5 Re-entering the VM for `serialize()`

`Panel::dump` is a read with only an `&App`, so `PanelScript::serialize` opens no
call scope and a script `serialize()` must be a plain value-returning method.
But `dock.dump()` is itself a call _from_ script, so reaching the panel's
`serialize()` means entering the VM while its runtime lock is already held —
which `Context::with` answers with a re-entrant borrow panic.

`ShellRuntime::with_js_nested` is the answer: `with_js` records the context it is
executing under in a field, and a nested body runs against that borrow instead of
asking for a second one. A field rather than a thread-local, so two runtimes on
one thread cannot hand each other a context; installed by a frame on the stack and
cleared before that frame returns, exactly as `crate::scope`'s pointers are.

### 15.6 Two constraints that were known in advance

`Panel::panel_name` returns `&'static str` while a script's panel name is only
known when the application loads, so there is no way to satisfy the signature
without a leak. It is made once per distinct name through a process-wide intern
table: calling `panel_name("mail", "inbox")` twice returns the same pointer. The
bound is applications loaded × panels each, in the hundreds, at tens of bytes
apiece. Unloading does **not** reclaim a name, deliberately: reclaiming would let
a name be freed while a persisted layout still refers to it, and outliving the
load that produced it is the whole purpose of the name.

The name is namespaced `shell:<application>/<panel>`. The prefix is `shell:`
rather than an engine name so that one layout file still restores after the host
switches engines. The application half comes from `Policy::application()` —
`set_bundle_id` for a single-application host, the manifest id for a plugin —
because a policy already answers "under whose authority?", and "filed under whose
name?" is the same question asked of persistence.

### 15.7 The property that makes it worth building

When a panel's name is not in the registry, `DockArea` substitutes a
draw-nothing placeholder that answers `dump` with the `PanelState` it was handed,
so the next save writes the panel — name, payload, and position — back out
unchanged; a user can uninstall an application and reinstall it and its panels
return to where they were. `dock.rs` extends that by one step: a panel that _is_
registered but whose script throws on construction gets a `RetainedPanel` with
the same behavior, so a broken script costs the user that panel's contents for
the session rather than its place in the layout or its saved state.

Registering a panel class also has to survive the runtime outliving neither more
nor less than it should. `register_panel` files the builder in an `App` global,
which outlives the runtime, while the class is a `Persistent` QuickJS value that
must be released while its context still exists. `ScriptPanelClass::retire`,
called from `ShellRuntime::drop`, drops the class and leaves the registration in
place — which turns those panels into exactly the placeholder case above.

---

## 16. ShellRoot: the Window and its Overlays

### 16.1 Base ships the parts, not the host

`Root` belongs to `gpui-component`. Base ships the pieces: `Dialog` and `Sheet`
each build their own viewport-sized host, `ToastManager` and `ToastStackState`
own stacking geometry and lifecycle, `FocusTrapElement` owns focus trapping,
`Popup` and `Positioner` own placement and collision. What base does not decide
is what happens when two of them are open at once.

`ShellRoot` is that decision, and it is the only reason `root.rs` exists: a
stacking order plus a dismissal order, with the smallest presentation that makes
them visible. The first view of a shell window is always a `ShellRoot`, the same
way the first view of a `gpui-component` window is always a `Root`, and a script
reaches it only through `ShellRoot::update` — never by constructing overlays
itself.

**Stacking**, painted back to front: the script's content; at most one sheet,
anchored to a viewport edge; the dialog stack in open order, each deferred at
`10 + index` so a later dialog always paints over an earlier one regardless of
element build order; and toasts above everything at `POPUP_PRIORITY + 1`. A
sheet sits _below_ the dialog stack because a sheet is a place in the window: a
dialog raised from inside a sheet must be readable, and a sheet raised under a
dialog must not cover it. Only the topmost dialog draws a backdrop, so a stack
of three dims the window once rather than three times, and that single backdrop
is what separates the live dialog from the inert ones behind it.

**Dismissal** is always one layer, never a cascade. Escape closes the topmost
dialog only; lower dialogs render with keyboard handling disabled, so repeated
presses unwind the stack one dialog at a time and never reach the sheet while a
dialog is open. `escape_dismissable: false` withdraws the _key binding_, not the
underlying cancel action, so a close control the script put inside the dialog
still works — which is what makes an undismissable dialog one the user must
answer rather than one they cannot leave. A backdrop press closes the topmost
dialog only if it was opened `backdrop_dismissable`. Enter does nothing at this
layer: base's dialog host treats it as "confirm and close", which belongs to the
dialog's own primary button, so the root vetoes the built-in confirmation rather
than guessing what the content wants.

**Focus** is recorded on open and restored on close, so a stack restores through
its own history: closing the second dialog returns focus to the first, and
closing the first returns it to wherever the window was. `close_all_dialogs`
restores to where the _first_ dialog took focus from, because restoring through
each in turn would flicker focus across views about to be dropped. Tab and
Shift-Tab honor base's focus trap, with a wrap-around loop bounded at 100 steps
so a trap with no focusable child cannot spin.

**Toasts** are data, not views — a title, an optional description, a level, a
timeout, and an optional id — which is what lets the root own the geometry and
lifecycle without asking the script to render anything. Pushing the same id
twice replaces rather than stacks, so a repeated "Saved" reads as one event.
Three are mounted at a time and the rest wait, so a burst is throttled rather
than lost. A 50 ms clock advances the lifecycle, paused while hover or focus
holds the stack expanded — the user is reading it. An inactive window does not
pause it: activation says nothing about whether the window is on screen, and a
side-by-side or second-monitor window would otherwise hold its toasts forever.

### 16.2 The script surface

```js
const depth = window.open_dialog(() => confirmClear(count, onConfirm), {
  escape_dismissable: false,
});
window.close_dialog(); // -> was anything open?
window.close_all_dialogs(); // -> how many closed
window.has_active_dialog();

window.open_sheet(() => filtersPanel(filters)); // right, the default side
window.open_sheet_at("left", () => navigation());
window.close_sheet();
window.has_active_sheet();

window.push_toast({
  title: "Saved",
  description: "3 files",
  level: "success",
  timeout: 4000,
  id: "save",
});
window.remove_toast("save");
window.clear_toasts();
```

`window` is a global, like `cx`, and for the same reason: it names the window the
script is already inside, which is not something a file opts into by importing
it. Nothing collides — this runtime has no DOM.

These are on `window` rather than on `cx` because a dialog belongs to the window,
not to the view that opened it: `cx.notify()` re-renders one view,
`window.open_dialog()` changes what the user is looking at. `gpui-component`
draws the same line, so the two halves of an application read as one vocabulary.
`window` is also somewhere to grow — overlays are what it carries today, and
`Window` in Rust also answers focus, size and appearance.

The content is a **function returning an element**, not an element: an element
belongs to the arena of the render pass that built it, and a dialog outlives the
call that opened it. The function runs when the dialog draws and again whenever
it redraws — the same contract `render` has — and whatever it closes over is the
dialog's state. That is what removed `props`, which existed only because the
dialog used to be a class the script handed over.

Four details are worth stating because each was a decision.

`open_dialog` and `open_sheet` take the view _class_, not an instance and not an
element. The runtime constructs it, passing `props` to the constructor, which
`View` forwards to `init`.

`open_dialog` returns the new stack depth rather than a handle. The root
addresses dialogs by position and never by identity, so a handle would have to
promise "close _this_ dialog", which is not an operation that exists. The depth
is what a script can actually use.

A misspelled option is refused, not ignored. `{ escapeDismissable: false }`
throws and names both the offending key and the valid set — a silently ignored
option would leave the dialog dismissable while the call looked like it worked.
This applies to `props` too, which is a named key rather than a bare object:
passing `{ count: 3 }` where `{ props: { count: 3 } }` was meant is an error at
the call site.

An absent `timeout` keeps the default; an explicit `null` asks for a toast that
stays until dismissed. The two cannot be collapsed into one optional.

Every entry point checks the phase before doing anything, and refuses `Render`
and `Layout` (§9.3). The check exists in both `overlay.rs` and `ShellRoot`
because the two refusals are different: the root logs and ignores, which is right
for host code that got it wrong, while a script gets a thrown `TypeError` naming
the phase it called from — the only shape an author can act on.

### 16.3 Windows and window decoration

`gpui.open_window` is not bound; the host opens exactly one window (§23). There
is no `TitleBar` or `window_border` component, so a script that wants one draws
it; the behavior bindings for drag regions, double-click maximize, and window
buttons are not built either.

---

## 17. System Capabilities

Everything here is denied by default and gated on the capability set in force
(§19.2). Capability decisions and path resolution live above the seam in
`capability.rs`; the engine holds only the argument shuffling.

```js
import * as fs from "fs/promises";
import process from "process";
// `localStorage`, `sessionStorage` and `console` are globals; the clipboard is
// `cx.read_from_clipboard` / `cx.write_to_clipboard`.
```

Two rules keep this honest. **There is one path resolver:** every filesystem
path goes through `Capabilities::resolve`, never through `std::fs` directly, so
`fs/promises` and every later path-taking entry point share one policy and there is
no second place for a traversal bug to hide. **A denial names its manifest
key:** the error a script sees is the instruction for fixing it.

```text
`/etc/passwd` is outside every granted read root; add its directory to
capabilities.fs.read in the manifest
```

```text
running `curl` is not granted; add it to capabilities.fs.execute in the manifest
```

### 17.1 Filesystem

The public module is `fs/promises`; the callback-style `fs` name is deliberately
not registered. `readFile(path)` resolves to `Uint8Array`, while an explicit
`"utf8"` or `{ encoding: "utf8" }` resolves to text. `writeFile` accepts a string
or `Uint8Array`. `readdir(path)` resolves to sorted names; pass
`{ withFileTypes: true }` for `Dirent` objects with `isDirectory()`. The remaining
subset is `exists`, `unlink`, `rmdir`, and `mkdir`. Streams, file descriptors,
watches, and sync operations are not part of the experimental contract.

Paths resolve against the granted roots; traversal is rejected by lexical
normalization and each operation executes through a `cap-std` directory handle,
not by joining an ambient host path after authorization.

Three shapes are deliberate. `readdir` sorts by name, so a script rendering a
listing does not inherit the filesystem's arbitrary order and does not have to
sort. `exists` _throws_ on a denied path rather than answering `false`, because
"you may not look" and "it is not there" are different facts and collapsing them
would let a script probe outside its roots one boolean at a time. `remove` is
not recursive: write access is granted per root, so a recursive remove would turn
one mistyped path into the loss of an application's whole data directory.

**These calls return promises.** The syscall runs on the background executor,
because a disk has no bound on how long it takes and blocking the main thread
blocks the frame and the VM together — somewhere the interrupt budget cannot see,
because the time is spent in the kernel rather than in script.

The capability check does _not_ move. It is cheap, it needs the ambient scope,
and it stays on the calling thread, so **a denial throws at the call site rather
than rejecting**: a rejected promise nobody awaited is a denial nobody sees. That
split is also why the symlink and denial tests need no executor at all.

`readFile` refuses a file over 64 MiB by name. The alternative to a ceiling is a
string that has to fit in the JavaScript heap, which is itself capped — so the
failure without one is an out-of-memory inside the VM instead of a sentence
naming the file. `writeFile` is capped at 8 MiB per call. `readdir` stops at
10,000 entries or 1 MiB of UTF-8 name bytes, whichever comes first.

Storage is deliberately not like this: it is a cache with a write-through, so
`getItem` and `setItem` answer from memory. `localStorage.flush()` returns a promise that
settles when the current value is durable. Its serialized file is capped at
8 MiB, with at most 4,096 keys and 1 MiB per JSON value. At most 1,024 pending
flush barriers may wait for durability at once.

### 17.2 Network

Three experimental surfaces are implemented. Global `fetch(url, options?)`
supports bounded requests of any HTTP method -- which methods actually reach a
host is the capability policy's decision -- string or `Uint8Array` bodies, safe
request headers, and resolves to `{ status, ok, url, , json() }`; both
body readers return promises. The `gpui-kit` module exports
`WebSocket.connect(url, { headers }?)`, returning a socket with asynchronous
text/binary `read`, `write`, and `close`. Bare module `net` provides raw
`connect(host, port)` with bounded async `read` and `write`; its synchronous
`close(): void` immediately shuts down the cloned transport handles. Neither
socket API provides a listening/server surface.

All three are **capability-gated**, and their potentially blocking operations
are asynchronous. The host allowlist is
checked at the call site before background work starts. `fetch` handles
redirects itself and authorizes every target against its method, host, and path
grant. It refuses HTTPS downgrade. A non-GET request is never replayed across
origins; neither Authorization nor any other caller-supplied header crosses an
origin boundary. Same-origin redirects may retain method, body, and headers.
HTTP requests time out after 30 seconds; request and response bodies are capped
at 8 MiB.

WebSocket connect resolves only after a successful handshake and does not
follow a redirect. Optional ordinary/custom protocol headers are allowed, but
Authorization, Proxy-Authorization, Cookie, Host, Connection, Upgrade, and all
`Sec-WebSocket-*` handshake-control headers are rejected before connecting.
Frames/messages are capped at 8 MiB. Connect, TLS/Upgrade handshake, and writes
have a 30-second transport deadline. A pending `read()` has no public deadline
and waits for the next text/binary message, peer close, local close, or
transport error; each socket accepts only one outstanding read, rejecting a
second immediately. The actor's short transport read slice is an internal
scheduling mechanism, not a user-visible timeout; between slices it services
writes and close, so a heartbeat write or close can complete while a read is
pending. The actor's command queue holds eight combined read/write/close
operations; enqueueing another rejects immediately. Raw socket reads/writes
remain capped at 1 MiB per call.

The fetch subset remains deliberately small: GET and POST are the only methods,
client-managed framing headers are refused, and streaming bodies, abort signals,
cookies, and browser proxy semantics are not promised. The name follows the
WinterCG ecosystem, while this subset and the absence of DOM or package
resolution mean GPUI Shell does not claim browser compatibility.

### 17.3 Storage

The [Web Storage API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API):
`localStorage` and `sessionStorage`, each with `length`, `key`, `getItem`,
`setItem`, `removeItem` and `clear`. Globals, and also on `window`, because that
is where the web has them and this surface is the web's rather than GPUI's — the
naming rule in §17.1 sends a binding to wherever its original lives, and here the
original is a browser.

**The two differ only in lifetime.** `localStorage` is one flat JSON object per
application, in a file the host placed; `sessionStorage` is the same structure
with no file behind it. That difference is also why only `localStorage` is a
capability: nothing `sessionStorage` holds ever leaves the process, so there is
nothing to grant, and it works on a host that granted nothing. `localStorage` is
granted by default at the manifest layer — a browser gives every origin one
without being asked, storage here is keyed by bundle id and cannot name its own
file, and defaulting it to denied had a trap: an application that added a
manifest to declare a network host silently lost its settings. The Rust
`Capabilities` still deny it until a host says otherwise.

Values are strings. That is the Web Storage definition rather than a limitation
we chose, and it is the better contract for this store: it makes the encoding
the script's own visible act, so what lands in the file is what the script wrote
rather than whatever our JSON bridge decided about `undefined`, `NaN` or a
reference cycle. A script with structure calls `JSON.stringify`, exactly as it
would in a browser.

Reads are cached in memory because `getItem` is called from `render` and a file
read per frame would be absurd.

Every mutation writes through immediately, to a temporary file that is renamed
over the target, so a crash mid-write leaves the previous settings intact rather
than a truncated file. The store holds small configuration data, and losing a
setting because a script forgot to call `flush` is a worse failure than one extra
rename; `flush` is the one member the web does not have, and it exists only as
the durability barrier — a browser never needs it because its storage is
synchronous all the way down.

A missing file is an empty store — a first run is not an error. A _malformed_
file is an error, because silently discarding a user's settings is worse than
refusing to start.

### 17.4 Clipboard and logging

`cx.read_from_clipboard` and `cx.write_to_clipboard`, named after the `App`
methods they call, with read and write as separate grants so a denial names the
half that was missing. Both need a live host call, and a clipboard call from a
context that has none reports that rather than panicking.

`log.debug/info/warn/error` need no capability: a script that can run can already
say something, and denying it would only cost the author their diagnostics.
Output goes to `tracing` under the `gpui_shell::script` target, so script output
is separable from host output in a filter. Extra arguments are appended
space-separated the way `console.log` behaves, and any value prints — structured
values as JSON, an unprintable one as a placeholder rather than aborting the call
it was describing.

`console.debug/log/info/warn/error` use the same tracing sink as `gpui.log`;
they do not create a second logging subsystem or bypass filtering.

### 17.5 Processes

`process.nextTick(callback, ...args)` queues a QuickJS job without creating a
timer or another scheduler. `process.run(command, args?)` resolves to
`{ code, stdout, stderr }` and requires
the command to be on the `capabilities.fs.execute` allowlist. A promise, for a
sharper version of §17.1's reason: a file read has no bound and a child process
has less — it can compute for minutes, wait on input that never comes, or outlive
the window — so waiting for one on the main thread stops the frame and the VM
together, in the kernel, where the interrupt budget cannot see it. Output is
captured rather than inherited, because a script that runs a command wants what
it said and a child writing to a windowed host's stdout is writing nowhere. The
grant is checked on the calling thread, so a denial throws at the call site
rather than rejecting a promise nobody awaited. Execution is bounded to 30
seconds and 8 MiB for each output stream; crossing a bound kills and reaps the
child. The two pipes are drained concurrently, so a child cannot deadlock by
filling stderr while the host waits on stdout. Owner loss, task cancellation and
runtime shutdown use the same kill-and-reap path. The child environment is
cleared before spawn, and the public surface has no option for restoring host
variables. `process.exit(code?)` requires
`capabilities.process.exit` and is a
**request**: it hands the code to a handler the host
installed with `gpui_shell::on_exit_request`, which decides what to do with it —
close the panel, close the window, end the process. It is never `exit(2)` inside
the runtime, because one plugin must not be able to take down an application the
user is working in.

The handler is not optional. A host that grants the capability without
installing one gets a **failure at the call**, naming the omission, rather than
a script that reports success while nothing happens — which is what an earlier
version did: the code went into a cell no production caller ever read, and the
window stayed open. A request nobody answers is worse than a denial, because the
script cannot tell the two apart.

The `gpui-shell` binary installs the obvious policy for a host that _is_ the
process: it ends it, with the code the script asked for.

`process` is installed as a global as well as a `gpui-kit` module member, because
`process` is the name a JavaScript author, or a model writing JavaScript, will
reach for.

There is no streaming subprocess: pipe semantics conflict with the asynchronous
model, and a case that needs streaming output belongs behind a host-registered
module that can return a structured result and a timeout.

### 17.6 Host modules

A script cannot load a native extension. `dlopen`ed Rust has no stable ABI and,
once inside the process, holds every permission the process holds — a sandbox
that permits it does not mean anything. So the direction is reversed: **the host
registers, at compile time, the Rust it is willing to expose**, and a script
reaches exactly that and nothing else. The cost — a third party who needs native
capability must fork the host or send a patch — is deliberately retained.

```rust,ignore
gpui_shell::export_module(
    HostModule::new("workspace")
        .function("project_name", |_| Ok(HostValue::from("gpui-component")))
        .function("version", |_| Ok(HostValue::from("0.1.0"))),
)?;
```

```js
import { project_name } from "workspace";

project_name();
```

**A registered module is an ES module, not a lookup.** The alternative
considered was a `native("workspace")` call answering with a frozen bag of
functions. It loses twice, both times on when a mistake surfaces. A lookup puts
every misspelled export on the run-time path, where an import fails while the
module graph is linked. And a lookup leaves the generated declarations with
nothing to say — only the host knows what it registered, so the best `gpui-kit.d.ts`
can offer for `native(name)` is `Record<string, (...args) => any>`, with any real
types hand-written in a `.d.ts` that nothing checks against the registry. A
module specifier is a name declarations can be written against, so §21 emits
them from the registry itself.

The import fixes the set of _names_, not the functions behind them: each export
is a stub that resolves through `dispatch` on every call, so withdrawing a module
still refuses the next call through an already-imported name. The consequence for
a host is one ordering rule — `export_module` before `load_app`.

**The runtime's own specifiers are refused at registration.** A host module
shares one namespace with the built-ins and the Standard Runtime, and the
resolver chain reaches those first, so a host registering `path` would register a
module nothing can import and never find out. `RESERVED_SPECIFIERS` names them
and `validate` reports every bad name at once; an engine test asserts the list
against the resolvers themselves, so a module added to one and not the other is a
failing test rather than a name a host can silently lose.

**The boundary is plain data.** A host function receives `HostArguments` and
returns a `HostValue` — null, boolean, number, string, array, or
insertion-ordered object — and never receives a script handle. That is not a
convenience. A handle would let the host keep a reference to a script value past
the call that produced it and past the scope frame that made the surrounding
context valid. It is also what lets one registry serve any engine, since
neither engine's value type appears in `host_modules.rs`. It is also what rules
out exporting a _class_: a constructor hands the script a live host object, and
object identity across the seam is the thing this boundary exists to prevent.

**A host function may not re-enter the engine.** A host call happens inside
a script call, which is itself inside a host call; calling back into the VM from
there would run script with an engine frame already on the stack, re-entering
the render pass currently building an element tree. Holding no script handle
makes that impossible to express, and `dispatch` additionally refuses a nested
call outright, so a host that finds another route gets a diagnosable error rather
than undefined behavior. Reading and writing host state is fine and is the point:
a function may reach the ambient `App` and request a re-render, which is delivered
after the call unwinds.

**Reaching host modules is itself the grant.** The default registry is empty
and every entry point fails while it stays that way — the same shape as
`Capabilities::default()`. There is deliberately no per-module capability: the
host chose the module list, so the list _is_ the grant. The two failures get
different sentences, because they are different facts: a host that registered
nothing has not wired its extension surface up, and telling that author "unknown
module" would send them hunting for a typo that is not there.

```text
host module `workspace` is not available: this host registered none. Host
modules are granted by the embedding application, with gpui_shell::export_module(...).
```

```text
unknown host module `workspaces`; this host registered: editor, workspace
```

```text
host module `editor` has no function `line_cont`; it provides: line_count
```

Argument readers (`string`, `number`, `integer`, `boolean`) report which
position was wrong and what arrived there, so a host writing a function does not
write that sentence itself. A `HostError` carries only a sentence; the engine
adds the module and function names when it turns one into a script exception.

**A function may be asynchronous, in two halves.** `HostModule::async_function`
takes a closure that runs on the main thread and returns a `Send + 'static`
future that does not; the script gets a promise, driven on GPUI's background
executor through the same `scheduler` machinery `fs.readFile` uses, and slow
work stops holding the thread that renders. The split is not a concession to
`Send`: the synchronous half may read host state because it runs inside the
caller's scope, and the future cannot re-enter the engine because on another
thread there is no `Ctx` to re-enter it with — the rule above, made physical
rather than guarded. Cancellation follows `cx.sleep`: a call whose view has gone
away leaves its promise pending rather than inventing an error for code that was
asked to stop. On the QuickJS side an asynchronous export is an arrow calling a
free function rather than a bound stub, because `Promise<'js>` borrows the
context lifetime and a closure cannot be polymorphic over a lifetime appearing
in both its parameter and its return type.

**A module describes its own TypeScript face.** `HostModule::declarations` takes
the body of a `declare module`, and `validate` compares the exports it declares
with the functions actually registered. Putting it beside the registration is
what makes the check possible at all: a `.d.ts` next to the script is a second
file, in a second language, with nothing holding it to the registry, and the
drift would surface as an editor completing a function the host had deleted.

The QuickJS side is a resolver, a loader, and exactly the two conversions the
seam forbids the registry from knowing about. Resolving a registered name yields
generated source — one `export const name = __host_function(module, name)` per
function — tagged with the registry's generation, because QuickJS caches a linked
module by name for the life of the runtime and two plugins importing `workspace`
would otherwise share whichever linked first. A miss is a plain resolving error
rather than a thrown exception: this resolver is not last in the chain, and a
thrown one leaves the exception pending so the file resolver behind it never
answers. Argument conversion is depth-limited at 16, which turns "the host was
handed a 100,000-deep list" from a blown Rust stack into a message at the call
site.

---

## 18. The Plugin Model

A host that runs one application from a directory may omit the manifest and get
the CLI's narrow local defaults, with storage keyed by the path (§23). When
`gpui-shell.json` is present, the CLI parses it before loading code, checks its
`shell-version`, and installs its declared capabilities; an invalid manifest is
reported and its entry module is not executed.
A host that runs _several_
applications cannot do any of that, because identity, permission, and storage
become per-plugin questions — and all three have to be answerable **before** the
plugin's code runs. That is the whole reason a manifest exists.

`plugin.rs` implements it: manifest parsing with a generated JSON Schema,
discovery, load and unload, per-plugin policies, capabilities and data
directories. Compatibility is manifest metadata rather than executable API:
`shell-version` is validated during discovery, before the entry can run.
The common single-application host calls `ShellRuntime::load`, which consumes a
single manifest directly while keeping the host's default policy as the
permission ceiling. Directory discovery and id-based lifecycle remain the
multi-plugin manager's separate job; its `load` requires an explicit
authorization callback before requested capabilities become a grant.
Its integration test does load and run a plugin, including asynchronous `init`
under the manifest-derived policy. The rest of this section describes what it
does, because the shape is what the design is about.

### 18.1 The manifest

Seven recognized fields: `id`, `name`, `version`, `shell-version`, `entry`,
`dependencies`, and `capabilities`. Only `id`, `name`, and `entry` are required. An omitted
`version` is reported as `unknown`; an omitted `shell-version` accepts the
current runtime; omitted `capabilities` grants nothing. The file is
`gpui-shell.json` — the name makes the owning runtime explicit.

```json
{
  "id": "com.example.inbox",
  "name": "Inbox",
  "version": "1.2.0",
  "shell-version": "0.6.0",
  "entry": "main.js",
  "dependencies": {
    "omarchy-ui": "huacnlee/omarchy-ui#main"
  },
  "capabilities": {
    "fs": {
      "read": ["${pluginDir}", "${dataDir}"],
      "write": ["${dataDir}"],
      "execute": ["git"]
    },
    "network": {
      "hosts": ["quotes.example.com"],
      "http": [
        {
          "scheme": "https",
          "host": "api.example.com",
          "methods": ["GET"],
          "paths": ["/v1/account"],
          "path_prefixes": ["/v1/quotes/"]
        }
      ]
    },
    "storage": true,
    "clipboard": { "write": true }
  }
}
```

`dependencies` maps a bare JavaScript module name to gpui-shell Git dependency
syntax. A string may be strict GitHub shorthand (`owner/repository` or
`owner/repository#ref`) or a full Git URL with optional `#ref`. GitHub shorthand
without a fragment selects `main`; a full URL without one selects the remote's
HEAD. A ref may name a branch, tag, or commit-ish such as a commit ID. Branches,
tags, and remote HEAD are fetched and resolved on each load; a commit ID keeps
selecting that exact commit.

Once the immutable checkout exists, gpui-shell reads its root `package.json`.
A string `main` selects the entry; if the file or field is absent, `index.js` is
used. Invalid JSON, a non-string `main`, or a path that is missing, not a file,
or outside the checkout fails the load before application code executes.

The original object form remains supported unchanged. It requires exactly one
explicit `branch` or `tag`, and its optional repository-relative `entry`
defaults to `index.js`. Existing manifests need no migration. Authors choosing
the string form publish their entry through `package.json` `main` or root
`index.js`.

Before linking the application module graph, the host fetches each repository
into `~/.gpui-shell/cache/dependencies/`. A per-remote lock serializes local
mirror updates; the fetched commit is atomically published as an immutable,
commit-addressed checkout and registered with the application's module
generation. The exact fragment-free URL is the remote and cache identity. The
raw configured origin is verified, while Git's `url.*.insteadOf` rules may
still choose the effective fetch URL. Git is non-interactive and each command
has a 30-second timeout. Relative imports from dependency code remain inside
its checkout, and runtime or Standard Runtime module names cannot be shadowed.
This host-side acquisition step requires `git`; it does not grant the fetched
script any network capability.

`network.hosts` is the backwards-compatible broad grant: every supported
network API may reach that host. Use `network.http` when a plugin only needs
selected REST operations. An HTTP request must match the rule's scheme,
effective port, host, method, and either an exact `paths` entry or a
`path_prefixes` entry. The scheme defaults to HTTPS and the port defaults to
that scheme's standard port; a manifest writes `port` only for a non-default
endpoint. Redirects are
checked again with the same rule, so an allowed endpoint cannot redirect a
credentialed request onto an unlisted path or host. An HTTP-only grant does not
also grant TCP or WebSocket access to its host.

**Capability is permission; contribution is behavior.** Commands, panels, key
bindings, settings, and themes are registered from script, never declared a
second time here. A permission has to be shown to a user and approved before any
code runs, so it belongs in data; a contribution is code, so it belongs in code.
Declaring both would create a class of bug — manifest and script disagreeing —
while producing no information the script did not already carry. The schema is
generated from the types with `schemars`, following
`crates/component/src/theme/schema.rs`, so the schema and the parser cannot disagree.

Every parse failure names the field and says what was expected, because this is
the first thing a plugin author meets and usually the only diagnostic they get:
nothing has run, so there is no stack to fall back on. Three validation
decisions are worth stating.

**Unknown fields are rejected before missing ones**, so a typo reports itself
rather than reporting the field it was meant to be. This is the case the design
is most exposed to: a manifest that misspells `capabilities` looks like one without any, and accepting it would hand
the plugin an empty grant while its author believes everything listed was
granted.

**`id` is validated strictly** — lowercase letters, digits, `.`, `-`, `_`, not
beginning or ending with a separator, no `..`. Two of those rules are security
rather than taste: no path separators and no `..`, because `<data home>/<id>`
must stay inside the data directory; and no uppercase, because two ids differing
only in case are one directory on a case-insensitive filesystem and two
everywhere else. When present, `version` must be semver-shaped, because it is
compared across an upgrade. An explicit `shell-version` must also be SemVer and
names the oldest compatible runtime; any runtime at or above that version is
accepted. `entry` must be a path that cannot leave the plugin directory, which
is the same rule the module resolver applies to every `import` — so a manifest
cannot ask for a file the resolver would refuse anyway.

Absent `capabilities` and `{}` both mean the empty grant; requiring the key
would add a line saying "nothing" to every plugin that wants nothing.

The manifest writes `${pluginDir}` and `${dataDir}` rather than real paths, for
the same reason a plugin cannot name its own storage location: a path chosen by
the plugin is a path the plugin can point anywhere. The _shape_ of the grant
comes from the manifest and nowhere else; the two directories it is anchored to
come from the host and nowhere else. A relative path is anchored to the plugin
directory; an absolute path is taken as written, and is exactly the case a host
policy or an approval prompt exists to gate.

`execute` is either an allowlist of command names or the string `"*"`.
Unrestricted execution has to be spellable — a host that cannot express it pushes
its users toward granting a wildcard read root instead, which is worse — but it
is spelled differently from an allowlist so a permission sheet can show it at the
severity it deserves.

### 18.2 Lifecycle

**Discovery executes nothing.** `PluginManager::discover` reads manifests and
stops, so a host with thirty installed plugins lists thirty names, versions, and
permission sets without starting thirty programs. Directories are searched in
order and an earlier one wins a duplicate `id`, which is what lets a user's own
copy shadow a bundled one. Only `load` evaluates script.

Lazy loading belongs at the module level, not the plugin level. The entry module
runs at load and its only job is registration; the real implementation lives in
other modules that a handler pulls in with a dynamic `import()` when it is
triggered. That is why dynamic `import()` is deliberately left callable by the
sandbox (§19.1) — it is the mechanism, not a hole. It also gives every plugin a
non-zero start-up cost that has to be budgeted (§20.8), and makes "do real work
in the entry module" a convention violation tooling can flag.

Data lives under the plugin's `id` rather than under its path, so it survives an
upgrade that moves the plugin directory — which is exactly what §23's path digest
cannot do for a directory run from the command line.

### 18.3 Authority travels with the code, not with the moment

Each loaded plugin holds a `Policy` — its grant, its storage, its native modules —
built once from its manifest at load. The policy rides on the **call frame**
(`scope::Frame`), so every call to `fs` or `localStorage` answers with the
grant of whichever plugin owns the code that is running. Two plugins loaded at
once hold two different grants at the same time, and neither can see the other's
files.

This replaced a single process-wide slot with a guard installed around each call
into a plugin. The guard could not be made correct, and the reason is worth
recording: a plugin that `await`s hands control back before its guard drops, so
the grant in force during the continuation was whichever plugin happened to be
running when the promise resolved. **`await` crosses time, and a time-scoped
guard cannot describe authority that outlives the moment.** Neither does moving
the slot onto the runtime help — two plugins share one runtime.

A `ScriptView` captures its policy at construction rather than reading one when
it renders, which is what makes a callback that fires three seconds later still
run under the grant its own script was loaded with. Construction precedes
`init`, so a filesystem, process or timer promise started during initialization
captures the same policy and a weak reference to the final view. Every async
resumption also carries a `Weak<ShellRuntime>`; it never consults whichever
runtime happens to be global when it wakes up.

Plugin unload first cancels every scheduler entry carrying the plugin's `Policy`,
including tasks that deliberately opted out of view ownership. Only then does
the manager drop the plugin, so owner-less work cannot retain or exercise
authority after unload.

### 18.4 What is still missing

The complete authorization product — `granted` / `denied` / `prompt`, a
permission sheet shown before the first run, a decision persisted in host
configuration rather than in the plugin directory, and re-asking when an update
adds a capability — is not built. The API boundary is present: a
single-application `ShellRuntime::load` never promotes manifest requests beyond
the host policy, and `PluginManager::load` requires the host to authorize the
inert manifest before code runs. A contribution registry is also missing: there
is no `gpui.command`, `gpui.keymap`, `gpui.register_panel`, or
`gpui.register_theme` for a script to register into.

Today the grant comes from the host directly. Running a directory from the
command line is an explicit act of trust, the same as `node app.js`, and
`gpui-shell` grants read access to the application root and its storage
directory, write access to the storage directory, `store`, and the right to
request that this standalone host exits. It grants no network, child-process
execution, or clipboard access (§23).

---

## 19. The Sandbox

The language surface is engine-specific and this chapter describes QuickJS;
another engine's trimming would be a different list because it would be a
different attack surface. Capability decisions and path resolution exist once,
above the seam, so no engine's sandbox can be looser than the policy.

### 19.1 Language trimming

JavaScript's advantage here is that its standard library has no IO: apart from
`eval`, the language itself cannot reach a file, a process, or the network. The
exposure is therefore concentrated in four places — what the host injected, paths
from a string to executable code, module resolution, and the shared built-in
prototypes.

| Treatment            | Target                                                                                              | Notes                                                                                                                                                                                                                                                                                                                                                                 |
| -------------------- | --------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Never added**      | quickjs-libc's `std` and `os`                                                                       | These provide `open`, `exec`, `getenv`, and `popen`; registering either is full access. `rquickjs` does not inject them and the shell never registers them. This is "never added" rather than "removed", which is an order of magnitude more reliable — and `rquickjs-sys` does not compile that file at all, so a test asserts their absence as a guard on the build |
| **Withheld**         | `eval` and every function constructor                                                               | `globalThis.eval` is deleted outright; the `Function`, `AsyncFunction`, `GeneratorFunction`, and `AsyncGeneratorFunction` constructors are replaced with throwing stubs                                                                                                                                                                                               |
| **Replaced**         | The module resolver (static and dynamic `import` alike)                                             | Resolves `gpui-kit`, `gpui-base`, `gpui-fps`, the listed Standard Runtime bare modules, and paths inside the application root. `node:` names and unknown packages are refused before reaching the filesystem. Dynamic `import()` stays callable — it is how §18 does lazy loading                                                                                     |
| **Frozen**           | `Object`, `Array`, `Function`, `String`, and `Number` prototypes                                    | One VM hosts several plugins, so the built-ins are shared mutable state                                                                                                                                                                                                                                                                                               |
| **Capability-gated** | `fs/promises`, `process.run`, `process.exit`, `fetch`, `net.connect`, `websocket.WebSocket.connect` | §17; each async operation captures the caller's policy before leaving the VM                                                                                                                                                                                                                                                                                          |
| **Throwing stub**    | `setTimeout`, `setInterval`, `clearTimeout`, `clearInterval`, `require`                             | Present, and throwing a message that names the replacement                                                                                                                                                                                                                                                                                                            |

Three of these are worth more than a table row.

**The `Function` constructor is replaced, not deleted, and all four of them are
swapped.** Deleting `globalThis.Function` would achieve nothing:
`(function(){}).constructor` is the same object, and each of the async,
generator, and async-generator function prototypes carries its own constructor
which is an independent compiler. The replacement also keeps the real
`Function.prototype` as its `.prototype`, because `x instanceof Function` and
`Function.prototype.{call,apply,bind}` are ordinary, legitimate JavaScript that
has nothing to do with `eval`. `eval` itself is deleted rather than stubbed,
because a `ReferenceError` cannot be mistaken for a working `eval` by feature
detection while a throwing stub can.

**This is the weaker of the two available layers, deliberately.** QuickJS makes
evaluation an _optional intrinsic_: a context assembled with
`Context::custom::<(Date, RegExpCompiler, RegExp, Json, Proxy, MapSet,
TypedArrays, Promise, Performance, WeakRef)>` — that is, `intrinsic::All` minus
`intrinsic::Eval` — has no `eval` and no compiler to reach in the first place.
The runtime uses `Context::full` instead, because `Ctx::eval` _is_ `JS_Eval` and
the same intrinsic gates it: dropping it also disables the engine's own
`ctx.eval`, which is how the JavaScript prelude and the two policy snippets are
installed. Reaching intrinsic level means converting the prelude to
`Module::evaluate` or precompiled bytecode first. Until that happens, the
withholding layer above is what is actually in force.

**The DOM names are absent rather than stubbed.** `window`, `document`, and
`localStorage` are deliberately _not_ given throwing stubs, even though
`setTimeout` and `fetch` are. Every bundle that does environment detection reads
them through `typeof`, and `typeof window === "undefined"` is the answer that
makes such a bundle take its non-browser branch; a throwing getter would turn a
working feature test into a crash.

Ordering is load-bearing and stated in the module header. The policy is
installed **after** the runtime's own globals — the prelude, the host API, the
scheduler — and **before** any application module is evaluated. Earlier, and the
prelude's own writes would land on prototypes meant to be frozen, and a later
subsystem could re-add a global this module means to withhold. Later, and
application code would already have had its turn with `eval` and a mutable
`Object.prototype`.

The freeze is switchable, and the trade is stated: a library that patches
`Array.prototype` — a polyfill, an older utility bundle — stops working, at
import time, with a `TypeError` that points at the library rather than at this
policy. A host that knowingly runs one can turn the freeze off and keep every
other part of the sandbox. Turning it off does not hand back a compiler, which
is asserted.

Freezing also does not make a sloppy-mode write throw; ECMAScript discards it
silently. That is the language's rule, not a hole, and the test asserts the
outcome that matters: the property never appears.

The module resolver is the one piece that had to be written rather than
configured. `rquickjs`'s `FileResolver` is unusable here because it tests
candidate paths against the process working directory, so an absolute
application path never matches. Owning the resolver also puts the module policy
in one place: a module must live inside the canonicalized application root, which
is what stops `import "../../../etc/passwd"` before the filesystem is touched.

### 19.2 Capability grants

`Capabilities::default()` is the empty set, every field is private, and
construction goes through a builder — so "no capability by default" is a fact
about the type rather than a promise in prose. The grant lives on the calling
frame's `Policy` and is checked at every call. A view freezes its grant when it
is loaded; changing the default policy affects later views rather than silently
changing the authority of code that is already running.

The three-state `granted` / `denied` / `prompt` model, the authorization UI, and
persisting a decision in host configuration are all part of §18 and not built.

### 19.3 Resource limits

| Limit             | Mechanism                                                | Value                                                                                                                                                          |
| ----------------- | -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Runaway execution | `Runtime::set_interrupt_handler`, on a per-call deadline | 50 ms in `Render`/`Layout`, 500 ms in `Event`/`Task`, 5 s outside any scope                                                                                    |
| Memory            | `Runtime::set_memory_limit`                              | 256 MiB — a leak reports as a catchable exception on the offending allocation rather than an OOM kill of the host                                              |
| Stack             | `Runtime::set_max_stack_size`                            | 1 MiB against QuickJS's 256 KiB default, so deep recursion is a `RangeError` a script can report rather than a native stack overflow, which is a process abort |
| Microtask storms  | Bounded drain (§12.2)                                    | 100,000 jobs per drain                                                                                                                                         |
| Host task fan-out | Per-runtime scheduler registry                           | 1,024 outstanding tasks                                                                                                                                        |
| Module source     | Bounded module loader                                    | 8 MiB per module                                                                                                                                               |
| Filesystem output | Bounded adapters                                         | 8 MiB per `writeFile`; `readdir` stops at 10,000 entries or 1 MiB of names                                                                                     |
| Store             | Bounded cache and barriers                               | 8 MiB total; 4,096 keys; 1 MiB per value; 1,024 pending flush waiters                                                                                          |
| Assets            | Bounded asset source                                     | 16 MiB per asset; listing stops at 10,000 entries or 1 MiB of names                                                                                            |
| WebSocket queue   | Bounded actor channel                                    | 8 combined read/write/close commands                                                                                                                           |
| Child processes   | Bounded adapter plus cancellation hook                   | 30 seconds; 8 MiB stdout and 8 MiB stderr; kill and reap on timeout, overflow, cancellation, owner loss or runtime shutdown                                    |

The budget is per host call, not global: every `scope::enter` mints a fresh
generation, and a change of generation is the signal that a new call has begun
and the clock restarts. That is what lets the render path have a tighter budget
than an event handler without reinstalling the handler between calls.

**An interrupt is not catchable from script.** The design flagged this as an
assumption that had to be measured rather than assumed: if a script could
swallow the interrupt with `try { while (true) {} } catch {}`, the interrupt
would not be a defence at all and the policy would have to escalate to discarding
the plugin's entire execution context. It was measured, and it cannot —
`an_interrupt_cannot_be_swallowed_by_a_catch_block` in `sandbox.rs` asserts it.
The interrupt is a real boundary, and the policy stays as it is.

### 19.4 Development mode

`--dev` restores `eval` and leaves the built-in prototypes writable, which a
REPL needs and a shipped application must not have. Capability gating is _not_
relaxed: development mode makes the language easier to poke at and never hands
out access the manifest did not declare, because a grant nobody wrote down is a
grant that will be missing in production.

It is not wired up. `gpui_shell::set_development_mode` exists and the binary does
not call it — a stale `TODO` in `bin/gpui-shell.rs` still describes the wrapper
as missing — so `--dev` today enables source watching and nothing else, and says
so with a warning at start-up. The visible development-mode marker the design
requires does not exist either.

Signature verification, an installation-time capability listing, and re-prompting
when an update adds a capability all belong to §18.

---

## 20. Performance

This chapter is language-independent and engine-sensitive, which is exactly why
the seam of §6.5 exists.

### 20.1 When rendering happens

Two frequencies, and the whole chapter turns on keeping them apart.

GPUI calls `Render::render` when a view is notified, when an entity it depends on
changes, or when the window is invalidated — which during continuous interaction
means close to frame rate. **Script `render` is not called then.** It runs only
when the view's snapshot has been invalidated (§8.4), which is a function of
application activity.

```text
GPUI render frequency   ×   script render cost      ← the old model
GPUI render frequency   ×   materialization cost    ← what is actually paid
script invalidations    ×   script render cost      ← what the script costs
```

The case that used to be the dangerous one — dragging, scrolling, typing,
animating — is now mostly native: a scroll offset, a hover, a text cursor and an
animation frame all repaint without a script render. What remains genuinely
frequent is application state that _is_ changing quickly, which is legitimate. A
view fed by a 10 Hz market data feed costs ten script renders a second, and at
around 1.4 ms each (§20.3) that is 14 ms of CPU per second. The failure this
design prevents is the same view costing 120 script renders a second because the
window happened to be repainting at 120 Hz.

Where per-frame cost is genuinely inherent — a realtime chart, a canvas, a very
long list, a drag interaction — the answer is a native component the script
configures, not a faster script render. `InputState` is the shape to copy.

### 20.2 The cost model

```text
T_render ≈ N_nodes × (C_new + K_ops × C_op) + N_nodes × C_materialize + C_scope
```

`C_op` is one script → Rust method call including argument conversion;
`C_materialize` is pure-Rust element construction.

Under QuickJS one `C_op` is: a prototype property lookup (an ordinary lookup,
not a proxy trap — §13.2 is what buys that); one JavaScript function call; **one
rest-parameter array allocation**, because the prelude's forwarder is
`function (...args)`; one host call into Rust; and a `Value` → `Bridged`
conversion plus a `SmallVec` push.

The third item is JavaScript-specific and was not in the original cost model:
`...args` allocates an array on every call, and no-argument style methods — the
most common kind by a wide margin — pay for it and use nothing. Specialized
zero- and one-argument forwarders are the most direct optimization available and
are not implemented.

Base-first has its own cost that must be counted: presentation authority in
script means more operations per node than a `gpui-component` binding would
need, where one `.primary()` replaces five or six style calls. And because style
has exactly one expression (§13.2), there is no batching escape hatch. That
leaves three levers: reduce `C_op` itself, memoize, and virtualize.

### 20.3 Measured

`tests/benchmark.rs` reports three numbers, because they are three different
costs and reporting one was what hid the coupling — and a fourth, D, which takes
one recorded call apart so that A can be acted on rather than only watched:

```bash
cargo test -p gpui-shell --release --test benchmark -- --nocapture
```

|       | Path                    | 443 nodes                        | Paid                         |
| ----- | ----------------------- | -------------------------------- | ---------------------------- |
| **A** | script → snapshot       | **1.4 ms**                       | per application invalidation |
| **B** | snapshot → `AnyElement` | **0.7 ms**                       | per frame                    |
| **C** | a full cached repaint   | **1.8 ms**, **0 script renders** | per frame                    |
| **D** | one recorded call       | stage by stage, in nanoseconds   | inside A                     |

| Metric                              | Target       | Measured                                               |
| ----------------------------------- | ------------ | ------------------------------------------------------ |
| 120 Hz frame budget                 | 8.3 ms       | Everything, including layout and paint                 |
| Script description per invalidation | **< 1.5 ms** | **1.4 ms** for 443 nodes (A)                           |
| Materialization, per frame          | —            | 0.7 ms for 443 nodes (B)                               |
| Typical panel node count            | 200 – 800    | 443 in the benchmark (40 rows × 5 cells plus wrappers) |
| Operations per node, base-first     | 6 – 12       | ~10                                                    |
| Implied `C_op` ceiling              | ≈ 150 ns     | ~320 ns measured                                       |

**C is an assertion, not a timing.** Fifty repaints of an unchanged view enter
the VM zero times; the test fails outright if that number moves, rather than
merely getting slower. It is the regression gate for §8.4's invariants and the
most important number in this chapter.

Two caveats carried over. The figures are one machine's, and the _shape_ — A
inside budget, B well under it, C exactly zero — is what should be read from them
rather than the digits. And the measured `C_op` is above the ceiling the budget
implies while the total is under it, which means the budget is met with fewer
operations per node than the model assumed, not with a cheaper operation.

The in-test assertions on A and B are deliberately loose (200 ms), because the
real budget is a release-build figure and the assertion must also hold in a debug
build. The gate for those two is the printed number, read by a person; the gate
for C is the assertion itself.

The first Standard Runtime release baseline was recorded on 2026-08-25 on
Apple Silicon macOS with a release build:

| Standard Runtime build metric                       |                    Baseline |
| --------------------------------------------------- | --------------------------: |
| `gpui-shell` executable                             | 28,540,608 bytes (27.2 MiB) |
| `gpui-shell check examples/js_todolist` wall time   |                      1.19 s |
| Maximum resident set reported by `/usr/bin/time -l` | 65,781,760 bytes (62.7 MiB) |

The pre-LLRT branch did not record equivalent startup or size figures, so this
is the reproducible comparison point for subsequent LLRT revision or feature
changes rather than a retroactively estimated delta. Measurements are platform
and linker specific; CI correctness gates do not assert these absolute values.

### 20.4 The view as the invalidation boundary

A script `render` describes one view **completely**. There is no partial
rebuild inside it: `cx.notify()` on a view whose description is four hundred
nodes rebuilds four hundred nodes, whatever prompted it. So the cost model of
§20.2 is not paid per window or per panel. It is paid per **view that was
invalidated**, and the application decides how large that is.

`cx.new(Class, props)` is what makes it smaller. A nested view is its own
`ScriptView` entity with its own snapshot and its own dirty flag (`view.rs`), so
three things hold:

| Event                          | What enters the VM                                                                                                                                                     |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A child calls `cx.notify()`    | That child's `render`                                                                                                                                                  |
| The parent calls `cx.notify()` | The parent's `render`. Each child answers the frame from the snapshot it already published — `ScriptView::render` materializes without rebuilding when it is not dirty |
| `entity.set_props(props)`      | That child's `update` and `render`. The parent is not rebuilt, which `tests/render.rs` asserts directly                                                                |

The middle row is the load-bearing one. Mounting a child is recording a handle
(`Component::ChildView`), and materializing that handle is a GPUI entity render,
not a script render. Rebuilding a five-panel window therefore costs the parent's
own description plus four entity renders that never reach the engine.

This is the granularity lever, and unlike everything in §20.6 it is already
built. It also settles a question that keeps being asked the wrong way round:
splitting an application for performance means splitting it into **views**, not
into plugins, applications, or processes. A second application is how a second
_authority_ is obtained (§18), not how a second cache is.

What is missing is attribution rather than mechanism. `RuntimeMetrics` counts
the runtime, not the view, so "which boundary is being rebuilt, and how large is
it" has to be assembled from `ScriptView::is_dirty` and `snapshot().len()` by
hand. Per-view counters are the diagnostic this chapter is short of; §20.5 is
the other half of the same gap.

### 20.5 Rendering frequency and presentation latency

§20.1 separates two frequencies. Diagnosis needs a second separation, between
two _latencies_, because the design's own success at the first one hides the
second:

```text
rendering FPS          is the frame smooth?
state → presentation   how long after state changes does the reader see it?
```

Explicit invalidation has a symmetric pair of failure modes, and only one of
them is visible to any frame measurement:

- **Notifying too often** costs script renders that describe things nobody can
  see. It shows up as `script_renders` per second above the rate the data
  changes, and eventually as dropped frames.
- **Notifying too late, or not at all** costs nothing at all on the frame
  budget. GPUI keeps replaying the last good description at full rate, so the
  HUD reads a steady 120 FPS while the interface shows something that stopped
  being true, and then jumps when something unrelated invalidates the view.

The second is the one this architecture makes newly possible. Under a
render-per-frame model a stale view could not exist for longer than a frame; here
it can persist indefinitely and every rendering metric will call it healthy.
Benchmark C (§20.3) asserts one direction — a clean view enters the VM zero
times — and nothing asserts the other, because "a render that should have
happened" is a statement about application intent that the runtime cannot
derive.

What follows is a reporting rule rather than a mechanism: performance work on a
script application must state which of the two numbers it moved. An FPS reading
that never dropped is not evidence that invalidation is correct, and the
`gpui-shell` metrics surface should grow a state-to-presentation figure — a
timestamp carried from the host state change to the materialization that first
showed it — before it grows anything else.

### 20.6 What is left

**Virtualization stays in Rust.** `VirtualList` and `Tree` call back only for
visible items, so a ten-thousand-row list costs the same in script as a
hundred-row one. `VirtualList` is bound; `Tree` is not. (`Table` is not on this
list: base's `Table` is a plain composition of `Stateful<Div>`s and renders
every row it is given. The virtualized one is `crates/component`'s `DataTable`.)

**Reduce `C_op` itself — done for styles and for `child`.** The specialized
forwarders this section used to propose are now what the prelude binds.
`__applyNullaryStyle`, `__applyParamStyle` and `__attach` take the table index
the prelude already knows instead of a method name, and take their one argument
positionally instead of in a rest array — so a style call no longer allocates a
JavaScript array, no longer copies a method name into a Rust `String`, and no
longer arrives at a dispatcher that has to look that name back up. Benchmark D
prices the difference stage by stage. On one Linux x86-64 machine a recorded
`items_center()` went from 196 ns to 94 ns and `bg("surface")` from 360 ns to
208 ns, and benchmark A for the 443-node panel went from 0.96 ms to 0.62 ms.
Lending the semantic palette instead of copying it per token lookup
(`theme_tokens::with_tokens`) is part of the same total.

What is left is the floor: about 60 ns of QuickJS interpreting the builder
method and about 30 ns for the crossing itself, per recorded call. Both are
inherent to one builder call being one crossing, and batching them would trade
the crossings for JavaScript array allocations that cost nearly as much.
**QuickJS has no JIT, so all of this is manual** — there is no trace to hope will
remove part of the dispatch.

**`gpui.memo`** would skip script construction for a _subtree_ whose data has not
changed, within a rebuild the rest of the view still needs. Whole-view caching is
already done (§8.4), which took most of what memoization was originally for; what
is left is worth doing only for views where one small region changes constantly
and the rest is large. Not implemented (§8.6).

**A template cache** would split a description into a reusable structure and the
dynamic slots inside it, so a value-only change writes slots instead of running
the builder again. It is the largest unspent lever here and the one that
addresses the path §8.4 left alone — an invalidated view whose structure did not
change. §20.7 states what it would be, the four problems it has to solve, and
what to measure before building any of it.

**Reuse argument objects.** The context objects handed to item renderers and
dock renderers should be pre-allocated and reused rather than built per row.

**Never let script participate** in layout, text shaping, scroll offsets,
animation interpolation, or hit testing.

### 20.7 The template cache

Built, measured, and deliberately not exposed. This section states what a
template is, what has to be true for it to pay, what it turned out to be worth,
and what is left.

**The conclusion first**, because the rest of the section is how it was reached.
All figures are release builds, best of seven batches of fifty, on one Linux
x86-64 machine:

| Question                                       |                                                                Answer | Where                    |
| ---------------------------------------------- | --------------------------------------------------------------------: | ------------------------ |
| Does a dirty render usually repeat its shape?  |                                     **40 of 40** on a live quote feed | `stories/shell_story.rs` |
| How much of a repeating description varies?    |                     **80 of 1,045** positions — half of them handlers | `tests/structure.rs`     |
| What does filling cost against rebuilding?     | 0.315 → 0.036 ms, **8.7×**; with the handlers it still pays, **5.3×** | `tests/structure.rs`     |
| What is it worth to a script written for it?   |                                            0.310 → 0.090 ms, **3.5×** | `tests/template.rs`      |
| What is it worth with **no** authoring change? |                                           0.339 → 0.272 ms, **1.25×** | `tests/template.rs`      |
| What can the recorder save on its own?         |                                              0.628 → 0.573 ms, **8%** | benchmark A              |

The last two rows are why there is no `template(...)` in the script surface. A
template a script has to be written for is a performance annotation in the
source — two ways to describe one interface, and a decision nobody should be
making while writing a panel — and the automatic version of it is worth a
quarter, not a factor of three.

**What the snapshot cache does and does not cover.** §8.4 removed the cost of
_no change_: an unchanged view is replayed in Rust and enters the VM zero times
(benchmark C). It did not touch the cost of a _small_ change. A snapshot holds
structure and values in the same nodes:

```text
StockRow
├── Text("AAPL")
├── Text("230.42")
└── Text("+1.42%")
```

When the price becomes `230.51` the structure is identical and one leaf differs,
but a description is produced by running the builder, so the whole view is
described again — every `div()`, every `.gap()`, every `.bg()`, every crossing
recorded into a fresh `SpecArena`. That is the dirty-render path, and on a feed
it is the path that runs.

So there are three levels rather than two:

| Level              | Condition                        | Cost                                               | State                     |
| ------------------ | -------------------------------- | -------------------------------------------------- | ------------------------- |
| 1 — snapshot cache | Nothing invalidated the view     | Materialization only, zero VM entries              | Built (§8.4)              |
| 2 — template cache | Invalidated, structure unchanged | Write the changed values into a retained structure | Built, reaching no script |
| 3 — full render    | Invalidated, structure changed   | `render` runs, a description is recorded           | Built                     |

#### What a template would be

The IR already has the right shape to cut. A `SpecArena` is `Vec<SpecNode>`, and
a `SpecNode` is a `Component`, a `SmallVec<[SpecOp; 8]>` and a child list
(`spec.rs`). A template is that arena with holes:

```rust,ignore
struct Template {
    /// Structure and every constant op, recorded once.
    arena: SpecArena,
    /// Where this render's values go, in the order the script supplies them.
    slots: Box<[Slot]>,
}

struct Slot {
    node: SpecId,
    site: SlotSite,
}

enum SlotSite {
    /// A payload carried by the constructor: `Component::Text(String)`,
    /// `Component::Button(String)`.
    Component,
    /// One `Bridged` argument of a recorded `ParamStyle` or `Method`.
    OpArgument { op: u16, argument: u8 },
    /// A `CallbackId` in a `Callback` or `ActionCallback`.
    Handler { op: u16 },
}
```

Filling it is a walk over `slots` writing `slots.len()` values, not a walk over
`arena.len()` nodes. That is the entire proposition: a 443-node panel with 4,430
recorded operations has, on a quote tick, on the order of _tens_ of varying
values.

#### The four problems, in the order they bite

**1. Validity has to be O(slots), not O(nodes).** A template hit that is
established by comparing this render's description against the cached one has
already paid for producing this render's description, which is the cost being
removed. Validity must therefore come from _which template the script selected_,
before any builder call is made — not from a comparison after the fact. Every
design decision below follows from this one.

**2. The win only exists if the builder calls are not made.** §20.6 puts the
floor at roughly 60 ns of QuickJS interpreting a builder method and 30 ns for
the crossing, per recorded call. Reusing the arena on the Rust side while
JavaScript still runs `div().flex().gap(4).p(8).bg(…)` removes the smaller half
and leaves the interpreter cost untouched. A template cache that saves 30% is
not worth the machinery; one that removes the builder calls is. This is what
makes it a _language surface_ question rather than an arena optimization.

**3. Handlers are not values.** Every `on_click(() => …)` allocates a closure
that captures this render's state and registers a `CallbackId` retired with the
snapshot generation (§8.4, `snapshot.rs`). A template can hold the handler
position as a slot, but something still has to produce a closure per render
unless the capture is lifted into an argument. A row with a handler on it will
not be free, and the achievable win is bounded by how much of a description is
handlers rather than structure — which is worth counting on a real panel before
anything else.

**4. Some ops look constant and are not.** `bg(cx.theme().colors.surface)`
records a resolved colour, so a palette change alters an op that no script
expression appears to vary (§13.3, §8.4). The same holds for any value read from
a HostModule while describing, and for `t()`-style locale lookups. Either those
become slots too — which grows the slot list well past the values the author
thinks of as dynamic — or a template is invalidated wholesale by a theme or
locale change, which is the honest and much simpler rule.

#### Loops and conditionals are not the obstacle

A repeated structure is the _best_ case, not the worst. In

```js
items.map((item) => h_flex().child(text(item.symbol)).child(text(item.price)));
```

the item count varies while the body's structure does not, so one row template
is instantiated _n_ times. Virtual list rows, table cells, tree items and menu
items are all this shape, and they are the highest-density recorded-call sites
in a real application.

A conditional is a small set of stable structures rather than an unstable one:

```text
Variant 0 → loading
Variant 1 → content
```

A template cache does not require one structure forever. It requires a _bounded_
number of them, selected by something cheaper than describing them. That is the
same requirement as problem 1, stated from the other side.

#### Where the separation could come from

Four shapes, and the design's existing commitments eliminate two of them.

| Shape                                                                                                               | How structure and values separate         | Verdict                                                                                                                                               |
| ------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Build-time transform** — lift each `render` body's static chains into a template constant, leave a value function | Automatic, full win                       | **Out of scope.** It is a compile step, which §5.3 and §24 reject by name, and it costs the source-map-free line numbers of §21.1                     |
| **Engine call-site keys** — key a template on the QuickJS bytecode position of the builder chain                    | Automatic, no source change               | Reaches into the engine's internals from above the seam (§6.5), and the seam exists to keep exactly this out of the contract                          |
| **Author-declared templates** — an explicit form the script opts into, carrying its own key and its values          | Explicit, full win, no compile step       | The only shape that fits the design as written. Costs an API and asks the author to mark hot paths — see the next section for what it would look like |
| **Record and compare** — record as today, hash the structure, reuse on a match                                      | No win on the JavaScript side (problem 2) | Not a shipping design, but the right _instrumentation_ — see below                                                                                    |

The author-declared shape is also the one that composes with `gpui.memo`
(§8.6, §20.6) rather than replacing it. The two cache opposite halves: memo
reuses a subtree whose **values** did not change, a template reuses a structure
whose values **did**. A panel wants both — memo the chrome, template the rows.

#### The surface: a template discovers its own slots

> Written before the mechanism was built. It is accurate about how discovery
> works — that part is implemented and tested — and wrong about the conclusion:
> the surface below is not shipped, because measuring it showed that what an
> author has to write for is worth 3.5× and what a wrapper can take on its own
> is worth 1.25×. Read it for the mechanism, then read what follows.

The author-declared shape is the only one the table leaves standing, and what it
left open is what that shape _looks like_ — because a form asking the author to
write a second language is §5.3's DSL under another name.

It does not have to be one. The separation can be discovered at run time, by
calling the body once with values that are not values:

```js
import { template } from "gpui-kit";

const Row = template((symbol, price, onSelect) =>
  h_flex().gap(6).py(2)
    .child(div().w(80).text_sm().child(symbol))
    .child(div().w(80).text_sm().child(price))
    .child(Button.new("trade").on_click(onSelect).child("Trade")));

render() {
  return v_flex().children(
    this.quotes.map((quote) =>
      Row(quote.symbol, quote.price, () => this.select(quote))),
  );
}
```

The body is ordinary builder code. On its first call the runtime runs it once
with a **sentinel** in each parameter position, records the description exactly
as it records any other, and notes every place a sentinel came to rest — a text
child, a style argument, a handler. What is left over is the structure, and the
notes are the `slots` list of the IR above. Every call after that grafts the
structure and writes the arguments into the slots: no builder call is made,
nothing crosses the bridge, and no JavaScript runs beyond the caller's own
`map`.

Three properties are what make this work where a call-site key would not:

- **Validity is O(slots).** The template is selected by _which function was
  called_, before any builder call — which is problem 1's requirement, met by
  construction rather than by a comparison after the fact.
- **There is no compile step.** `template(...)` is a function, its body is
  JavaScript, and a reported line number is still a source line number
  (§5.3, §21.1).
- **A variant is a second template.** A conditional inside a body would freeze
  at discovery, so the rule is that a body has none: a loading state and a
  content state are two templates. That is the variant story written by the
  author rather than inferred, and it is the honest version of it.

And two rules the runtime has to _enforce_ rather than document:

- **An argument may be passed through, not computed on.** `price` may be handed
  to `.child(...)`; `` `${price}` `` would consume the sentinel during discovery
  and bake a constant into the structure. So the sentinel refuses to become a
  primitive — `Symbol.toPrimitive`, `toString` and `valueOf` all throw a message
  naming the rule — and the mistake is a diagnostic at first use rather than a
  panel that silently stops updating. Formatting belongs at the call site, where
  it is a value being computed rather than a structure being described.
- **A handler must arrive as an argument.** A closure written inside the body is
  created once, at discovery, and would capture that call's values for the life
  of the template. A body that registers one is refused for the same reason.

The second rule is also where the ceiling sits. A handler passed in is allocated
and registered per call, which is exactly the cost the census below says a
template cannot remove.

#### Why this comes before a QuickJS JIT

A JIT makes the same JavaScript run faster. It does nothing to the rest of the
path — the crossing, the `Bridged` conversion, the `SmallVec` push, the arena —
and §20.6 measures roughly a third of a recorded call in exactly those. A
template removes the work instead of accelerating it, and it removes the part a
JIT cannot reach. Given that §20.6 has already taken the cheap wins on `C_op`
and reports the remainder as a floor, "do less" is the only lever left with an
order of magnitude in it.

#### What was measured

Steps 1 and 2 below are built and have run. The instrumentation is
[`StructureFingerprint`](../crates/shell/src/spec.rs) — a hash of a
description's shape accumulated _while it is recorded_, with payloads and
`CallbackId`s deliberately left out — surfaced as
`RuntimeMetrics::structure_repeats`, `structure_changes` and
`structure_repeat_rate`, compared in `ScriptView::rebuild`, and reported live
under the Shell story's counters. **Nothing acts on it.** It is a reading, and
§20.7's first problem is exactly why it cannot become a cache as it stands.

Three results.

**The assumption holds.** On the Shell story's own board — twenty rows of six
cells fed by a live market entity, written before this question was asked —
**40 of 40** quote-driven rebuilds produced the structure they replaced
(`stories/shell_story.rs`). A moving price is a value, not a
structure, and the runtime rebuilds all of it anyway.

**The slot ceiling is about 4%, and half of it is handlers.** On a 40-row
watchlist — 361 nodes, 684 recorded operations, 1,045 addressable positions —
a value-only tick differs in **80** of them (`tests/structure.rs`):

|                                                  |  Count |              Share |
| ------------------------------------------------ | -----: | -----------------: |
| Nodes                                            |    361 |                    |
| Recorded operations                              |    684 |                    |
| Component payloads that differ (the prices)      |     40 |     11.1% of nodes |
| Argument values that differ                      |      0 |   0% of operations |
| Handler operations, which differ by construction |     40 | 5.8% of operations |
| **Positions a rebuild actually changes**         | **80** |  **7.7% of 1,045** |

So a template would reuse 96% of the description and write 4% of it — and half
of what it writes is handler registration, which problem 3 says a template
cannot fill. The reusable share is real; the _saving_ is bounded by the closure
allocation and callback registration that stay.

**The measurement is free.** Benchmark A on the 443-node panel, best of seven
batches of fifty, release build, one Linux x86-64 machine: **0.628 ms** before
the fingerprint and **0.623 ms** after. Two or three mixes per recorded
operation sit below the noise of the measurement they are inside, which is why
the counter is always on rather than behind a flag.

**And filling is worth about 5× on that panel, not 30×.** The fill path is
priced without the surface, by replaying the same watchlist into a fresh arena
through `push` / `push_op` / `attach` — the exact calls an instantiation would
make, with no JavaScript, no bridge crossing and no `Bridged` conversion
(`tests/structure.rs`). Release build, same machine and method:

|                                                           |                                  Per build |
| --------------------------------------------------------- | -----------------------------------------: |
| Rebuild: script → snapshot                                |                               **0.315 ms** |
| Fill: replay the structure and write the slots            |                        **0.036 ms** — 8.7× |
| The same panel rebuilt with its forty `on_click`s removed |                                   0.292 ms |
| …so the handlers cost                                     | **0.023 ms**, and a template pays them too |
| **Fill + handlers**                                       |                    **0.060 ms** — **5.3×** |

The 8.7× is a floor with the hard part left out; the 5.3× is the number to plan
against, and the gap between them is problem 3 priced rather than argued.
Neither includes selecting the template and its variant, which the surface above
makes a property lookup rather than a search — but which is not zero.

#### What a template costs to keep

A template outlives every render that uses it, which is the point of it and also
the reason nothing in a render would ever free one. Left alone, the store would
grow by one recorded arena per `template(...)` call site **per hot reload**: a
reload re-evaluates the module, the closure's cached id is gone with it, and
discovery runs again.

So a template holds the `ApplicationGeneration` of the script that defined it,
and `release_application_generation` — the same release that retires that
application's callbacks and tasks — empties its templates. The slot in the store
stays, because a template's id is its index and a closure in a still-loaded
module may hold one; what is freed is the arena, which is all of the memory. A
script reaching a retired id is told so rather than handed another
application's structure.

That leaves the runtime with no cache that grows with time. A snapshot is two
descriptions per live view (§8.4), retired with the view; callbacks are retired
with the snapshot that registered them; templates are retired with the
application that defined them.

#### What that licenses, and what it does not

It licenses designing the surface. The assumption the whole idea rested on is
not merely plausible on a real script — it was unanimous on the workload
measured, which is the outcome that makes an author-declared template worth an
API rather than a note.

It does not license expecting the full `C_op` back. The census and the fill
measurement size problem 3 rather than dissolving it: a panel with one button
per row spends half its write set on handlers, and _that_ half is not builder
calls a template skips — it is closure allocation and registration that happen
whether or not the structure was reused. On the watchlist they are 7% of the
rebuild and 38% of what the fill would still cost. A panel with fewer handlers
per row does better; one that is mostly controls does worse.

Two things are still unmeasured. The Longbridge terminal of §20.3 is a larger
and less obliging workload than the story's board, and its rate is the one that
should decide the API. And nothing has yet established that an author-declared
template can be made to read like ordinary builder code — which is the whole
reason §5.3's refusal of a DSL is not also a refusal of this.

#### What was built, and what it turned out to be worth

The mechanism is implemented in `engine/quickjs/template.rs` and
`SpecArena::graft` / `write_slot`. **It is not part of the script surface**, and
the measurements below are why.

Sentinel discovery works exactly as sketched. A body is run once with a sentinel
in each parameter position; wherever a sentinel comes to rest is a slot; every
call after that grafts and fills. `tests/template.rs` pins the behaviour and the
five refusals — a computed argument, an inline handler, a parameter that fills
nothing, a nested template, a slot in a position a template cannot fill — and
one test asserts that a filled template's description is byte-identical to the
builder chain it replaces.

**Explicitly written for, it is worth 3.5×.** Two 40-row watchlists describing
the same rows, release build, best of seven batches of fifty:

|               |    Per build |
| ------------- | -----------: |
| Builder chain | **0.310 ms** |
| Template      | **0.090 ms** |

Slightly under the 5.3× the fill measurement predicted, and the gap is what the
fill measurement left out: the JavaScript call itself, its argument array, and
one bridge crossing per argument.

**Automatically, with no authoring change, it is worth 1.25×.** This is the
number that decides the chapter, because a template a script has to be written
for is a performance annotation in the source — two ways to describe one
interface, and a decision nobody should be making while writing a panel.

An automatic wrapper is safe by construction if it refuses any body where a
sentinel is read and does not land in the description: a conditional on an
argument reads without landing, a computed one throws on coercion, and both are
caught. The question is what survives that rule on code nobody wrote for it. The
Shell story's own `ui.js` answers it:

| Helper                                                                 | Automatic?                                                          |
| ---------------------------------------------------------------------- | ------------------------------------------------------------------- |
| `title`, `label`, `muted`, `rule`                                      | **Yes.** One varying value handed straight to a builder call        |
| `cell(width, options)`, `watchMarker(watched)`, `action(…, {primary})` | No. An argument decides _structure_, through a ternary or a `when`  |
| `quoteRow(quote, onClick, cx)`                                         | No. ``Button.new(`quote-${quote.symbol}`)`` computes on an argument |

Measured on a board of that shape — twenty rows of six cells, with only the
first group templated:

|                            |            Per build |
| -------------------------- | -------------------: |
| Helpers as plain functions |         **0.339 ms** |
| Helpers templated          | **0.272 ms** — 1.25× |

So the automatic ceiling is a quarter, not a factor of three. The gap is not the
mechanism; it is that ordinary presentation code interpolates strings and
branches on its arguments constantly, and both are exactly what a template
cannot hold.

For comparison, the other automatic lever — reusing the recorder's own work
rather than the script's — is smaller still. Removing the eager
`style::apply_param` check entirely, which is the largest single thing a
same-shape rebuild could skip, moves benchmark A from 0.628 ms to 0.573 ms: 8%.
That is the shape of everything on the Rust side of the crossing, because
§20.6's floor says roughly 90 ns of a 140 ns recorded call is the interpreter
and the crossing, and neither is reachable while JavaScript is still driving the
description.

#### Where that leaves it

The mechanism stays, unexposed, reachable as `globalThis.__template` for the
tests that pin it. Three reasons not to delete it and not to ship it:

1. **The measurement is the deliverable.** "A template cache is worth 3.5× if
   written for and 1.25× if not" is a fact about this runtime that had to be
   built to be known, and it is what any later attempt should start from.
2. **The one place it pays automatically is a loop the runtime already owns.**
   A virtual list's item renderer is called from the layout pass, twice a frame
   per list, and the runtime — not the script — decides how many times. A row
   template discovered there costs the author nothing and is paid back per
   frame rather than per invalidation. That is the remaining piece of work worth
   doing, and it needs no script surface at all.
3. **Composition is the limit, and it is liftable.** A template body may not
   call another template today, so a template can only be a leaf. Lifting it —
   letting an outer sentinel flow into an inner template's slot during discovery
   — is small, and it is what would let a whole panel be one template rather
   than a row. It does not change the 1.25×, because what blocks composition on
   real code is string interpolation rather than the restriction.

#### What is left to do, in order

3. **Discover row templates inside the virtual list.** The one place the win
   is automatic _and_ large, because the runtime owns the loop and the rows are
   already on the frame budget rather than on an invalidation. No script
   surface, and the fallback rule above is the safety net.
4. **Lift the nesting restriction** if a whole panel is ever worth templating,
   which the 1.25× says it is not yet.
5. **Re-run the census on the terminal** before spending anything more here, and
   record the number beside the story's.

### 20.8 Start-up

The start-up budget — under 2 ms for VM creation and sandbox trimming, under
5 ms for global registration including the reflection table, under 1 ms for the
palette — has not been measured, and the one line that most needs a number is
the prelude: building roughly 3,200 JavaScript closures is a one-time cost, but
it happens on the start-up path. If it turns out to be expensive, the
alternatives are caching the prototype as QuickJS bytecode or defining methods on
first use — the latter moving the cost back into `C_op`, which makes it worth
doing only if the measurement supports it.

---

## 21. Errors, Diagnostics, and Hot Reload

### 21.1 Failure is recoverable

Every Rust → script call catches at the boundary and carries the script's own
stack. `describe` flattens a QuickJS `Exception` into `message + stack`, which
ends as an ordinary `anyhow::Error`, because nothing above the seam should
recognize a VM's error type.

A failure during render becomes a **failure surface** where the interface should
have been. `runtime.rs` renders it: one heading, the message and stack, one
recovery line, and a copy control, on the same semantic tokens as every other
screen. Three details are deliberate. It takes its colors from tokens rather than
hardcoding red, because a failure surface that hardcodes red is unreadable in
half the themes it will be seen in; `destructive` appears once, as a hairline
rule, because emphasis is a budget and the message is already the focal point.
It has square corners, because it is not a card floating in the window — it _is_
the window's content for as long as the failure lasts. And a stack trace exists
to be pasted somewhere else, so copying it is a first-class action rather than
something the reader retypes.

The same surface serves an application that fails to load, so a failed start-up
still opens a window with the reason in it rather than only a line in a terminal
the user may not be watching.

A failure during an event is logged and the state is left alone. An unhandled
promise rejection is reported as an event-time failure (§12.2) — the
JavaScript-specific case, because without the adoption hook it would be entirely
silent.

Because there is no compile step, a reported line number is a source line
number. No source map is needed, which is one concrete benefit of refusing JSX
and TypeScript compilation.

What is missing is a toast on an event-time failure: today it is a `tracing`
line, and a host without a subscriber installed sees nothing at all — which is
why `bin/gpui-shell.rs` installs one before doing anything else.

### 21.2 Hot reload

```text
a source file changes → debounce 200 ms → re-evaluate the module →
construct a new view instance → swap the object in and notify
```

`ShellRuntime::watch` drives reloads whose implementation does **all** of its fallible
work before it touches the live
entity: re-evaluating the module can throw, and constructing the view can throw,
so both happen first and the swap is a single statement at the end. A save that
does not compile returns an error and changes nothing on screen — the previous
working view keeps running, with the error reported to the caller.

The entity survives, and so do the window, the focus, and the element
identities; only the script object behind the view is replaced. That is what
makes a reload invisible to the host.

**A reload re-reads every module, entry point included.** This is the one thing
about hot reload that had to be discovered rather than designed. QuickJS caches
an evaluated module by name and an ES module cannot be unloaded, so re-evaluating
`main.js` alone left every module it imports at the version that was on disk the
first time — a hot reload that silently ignores every file except the entry
point, which is worse than no hot reload because it looks like it worked. The
fix is a generation counter, incremented on every `load_app` and appended to
every resolved module name as `?v=N`, which makes each reload a different module
as far as the cache is concerned. The entry carries the tag too, because a
reload that re-read every import but served a stale `main.js` is the same bug one
level up. `tests/render.rs` covers exactly this.

The cost is that the previous generation stays in the cache until the runtime
shuts down. That is a development-only leak, and it is a grade coarser than the
clean form, which is to discard and rebuild the whole context — that belongs
behind the seam and is not built.

State does not survive a reload. The design routes preservation through the same
`serialize()` / `deserialize()` round trip as layout persistence, which both
saves a mechanism and continuously tests the serialization; that path does not
exist, so `watch.rs` does not invent a second one. The new instance starts from
its constructor, and the swap is one statement precisely so the restore can be
inserted before it.

`SourceWatcher` polls rather than subscribing, because the crate deliberately
takes no dependency on `notify`. Polling is honest for the job: a 250 ms tick,
one `stat` per source file in a directory that holds a handful, bounded at depth
8 and 4,096 files so a symlink farm or a vendored tree cannot turn one poll into
an unbounded walk. The stamp is three aggregates —
newest modification time, file count, total bytes — each covering a case the
others miss, and what it cannot see is a change that preserves all three, such as
swapping two files' names. A `notify`-based watcher would cut latency from one
poll interval to milliseconds, stop scaling with file count, and see renames;
an event-backed watcher could replace this internal detector without changing
the public `ShellRuntime::watch` lifecycle.

### 21.3 Checking, declarations, and DevTools

`gpui-shell check <directory>` is what a compiler would be for a language that
had one. The script surface is dynamic — an unknown style method, a wrongly typed
argument, a reused element are all runtime facts — so the only honest check is to
build the application and render one frame. The window is real and never shown,
because rendering is where those facts surface. It exits 0 or 1, reports syntax
errors, unresolved imports, a missing or malformed default export, unknown style
methods with a suggestion, wrongly typed style arguments, and an element used
twice. `--print-spec` additionally prints the description tree.

`gpui-shell types <directory>` writes `gpui-kit.d.ts` (§14.4), which moves a second
class of mistake earlier still, into the editor.

There is no DevTools panel. The intended form — a debug panel written in the
script language itself, showing VM memory, live views, persistent handle count,
last frame's node count and duration, style table hits, error history, and a REPL
— is also the best available dogfood, and the REPL depends on the development-mode
`Eval` intrinsic of §19.4.

---

## 22. Testing

### 22.1 Description snapshots, with no GPU

What a script produces is a plain-data `SpecArena`, so interface structure can be
asserted without a window:

```rust,ignore
let tree = runtime.render_to_spec(&object, None, window, cx)?;
assert!(tree.contains("Button \"increment\""));
```

`render_to_spec` runs the script; `RenderSnapshot::debug_tree` reads a
description that has already been built, without entering the VM. Tests that mean
"what does this script produce" use the first; tests that mean "what is this view
currently showing" use the second, and a test that confused them would be
asserting on something production never does.

This is the extra return on choosing descriptions over a retained tree (§8.2) and
the main regression defence for the script layer.

### 22.2 Sandbox escape tests

`sandbox.rs` carries a set of scripts that must fail: every path back to a
compiler (seven of them, including each function-prototype constructor), writing
to a frozen prototype, `process.run` without a grant, `process.exit` without a
grant, and the interrupt-swallowing case of §19.3. `host.rs` covers the path
resolver: a read outside the granted root, a read with no grant at all, a storage
call without the capability, and a clipboard denial naming the half that was
missing. Every one of these asserts on the _message_, because the message is the
instruction for fixing it.

These are security assertions and are not subject to the "avoid trivial tests"
exemption in `.claude/COMPONENT_TEST_RULES.md`.

Two of them are regression guards on the build rather than on this code:
quickjs-libc's `std` and `os` are asserted absent because `rquickjs-sys` does not
compile that file at all, and dynamic `import()` is asserted to stay callable
because confining it is the resolver's job and closing it would remove lazy
loading.

### 22.3 The render-frequency regression suite

`tests/snapshot.rs` is where §8.4's invariants stop being prose. The coupling it
guards against is easy to reintroduce — one `arena.reset()` in the wrong place,
one caller that rebuilds unconditionally — and invisible until it shows up as a
frame-rate problem in an application nobody has written yet. So it is asserted
directly, by counting entries into script `render`:

| Test                                                  | What it proves                                                                                                                |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| 65 repaints of a clean view                           | exactly 1 script render — invariants 1–3                                                                                      |
| one `on_change`, then a frame                         | exactly 2 — `notify` rebuilds, once                                                                                           |
| three events, then one frame                          | exactly 2, and all three events reached the script: GPUI does the coalescing, and the runtime does not add a second scheduler |
| a handler dispatched 32 frames later                  | still callable — invariant 7                                                                                                  |
| a render that throws                                  | the previous description survives — invariant 8                                                                               |
| 16 frames after a failed render                       | no further script renders; a broken render is not frame-coupled either                                                        |
| two views on one runtime, one rebuilding              | the other's handlers stay callable                                                                                            |
| a palette change                                      | rebuilds, because tokens are resolved into the snapshot                                                                       |
| a bare `cx.notify()` from Rust, then `refresh`        | the first repaints, the second re-runs the script                                                                             |
| the shell story's quote feed against its repaint feed | the same distinction end to end, at 50 ms, through a native module and a real script                                          |

`ShellRuntime::metrics()` is what these read — `script_renders` against
`materializations`, with the time each took. It exists for this suite and for the
readout in the shell story, whose quote board ticks every 50 ms and shows the
same two numbers as live rates. A claim about render frequency that cannot be observed
cannot be regression-tested, and one nobody can watch is hard to believe.

Benchmark C in `tests/benchmark.rs` (§20.3) makes the same assertion under a
release build and a realistic node count.

`tests/render.rs` keeps the end-to-end protocol cases — the counter's description
shape, an element added to two parents, a mistyped style name's suggestion, input
events, reload, and a real paint that must not panic.

### 22.4 JavaScript host-API integration tests

The system APIs are not considered covered merely because their Rust adapters
have unit tests. `tests/standard_runtime.rs`, `tests/fs.rs`, `tests/process.rs`,
`tests/network.rs`, and `tests/host_api.rs` load
real JavaScript modules, create real `ScriptView` entities, drive the GPUI event
loop, and assert the snapshot JavaScript published after its promise resumed.
They cover:

- filesystem calls and the store flush path;
- LLRT-backed buffer, path, URL, crypto and compression modules inside the same
  VM, bare module resolution, rejection of `node:` aliases, and removal of the
  old `gpui.fs` / `gpui.process` exports;
- process success, non-zero status, spawn failure, both captured streams, and
  output-limit rejection; the cross-platform case also proves denial, filtered
  environment, absent signal/identity mutation, and `nextTick` execution;
- local HTTP and TCP success, default denial for both surfaces, redirect
  reauthorization, response and socket size ceilings, WebSocket handshake,
  header filtering, text/binary traffic, pending-read concurrency, and two
  simultaneous runtime policies, without depending on the public Internet;
- clipboard read/write, timer cancellation, and a host-registered native module;
- an async context still reaching its own view after `await`.

The plugin integration test adds the boundary above them: manifest declaration
to `Policy`, async work started from `init`, filesystem access under that policy,
and notification of the final view. This is why view construction is split into
object construction, entity creation, and initialization (§6.5): a test that
manually calls `render_to_spec` after the promise settles would miss a lost
owner or a lost `cx.notify()`.

### 22.5 Interaction tests

`overlay.rs` and `root.rs` drive `TestAppContext` and `VisualTestContext`
directly: opening and stacking dialogs, Escape unwinding one layer, focus
restoration through a stack, sheet replacement, every layer drawing at once,
toast timeout and dismissal, and phase refusal from a render pass. These are the
tests that catch a duplicated element id or a missing hitbox.

### 22.6 Relation to the repository's testing rules

Following `.claude/COMPONENT_TEST_RULES.md`: no tests assert presentation
dimensions, and coverage concentrates on complex logic — call-scope validity,
arena reuse errors, snapshot lifecycle and render frequency, callback lifetime,
value conversion, style table non-emptiness, sandbox boundaries, overlay ordering
and focus, and task ownership and cancellation.

### 22.7 CI split

The repository-wide OS matrix excludes `gpui-shell`; otherwise every ordinary
workspace run would repeat its relatively expensive QuickJS/LLRT suite. Shell
has two explicit jobs:

- **GPUI Shell Core** runs once on Linux and excludes the Standard Runtime,
  filesystem, process, network and benchmark groups.
- **GPUI Shell Standard Runtime** runs the focused JavaScript integration groups
  on macOS, Linux and Windows. Network cases use loopback servers only.

The equivalent local commands are the commands in `.github/workflows/ci.yml`.
Adding a Standard Runtime surface means adding its black-box JavaScript test to
the focused job; adding render, scheduler or bridge behavior belongs in the core
job. Benchmarks remain explicit release-only measurements rather than per-commit
correctness gates.

---

## 23. Running an Application

```text
gpui-shell <directory> [--watch] [--dev]
gpui-shell check <directory> [--print-spec]
gpui-shell types <directory>
gpui-shell --help | --version
```

The binary is a thin host: it parses a command line, installs a log sink, builds
one runtime, opens one window, and drives the source watcher when asked.
Everything that outlives one invocation lives in the library.

The directory argument may name the application root or the `main.js` inside it,
and the root resolver handles both — along with being pointed at the _parent_ of
the real application directory, which is the other common way to start. Its error
names what was expected and, where it can tell, where the application actually
is.

An unknown flag is reported rather than taken as a path, because silently
treating a mistyped `--watch` as a directory would report a missing `main.js`
instead of the typo. `--help` and `--version` are answered before anything else
can fail, since a caller who mistyped a flag is exactly the caller who needs
`--help` to work.
Usage errors exit 2; a runtime that fails to start exits 1.

**Storage is per application, under the user's data directory, keyed by the
canonical path of the application root.** The path is
`<data home>/gpui-shell/apps/<directory name>-<16 hex digits>/store.json`, where
the digits are an FNV-1a hash of the canonical root — not a security boundary,
just enough to keep two directories from sharing a folder. Keeping the directory
name in the path makes the folder recognizable; the digest disambiguates it, so
two checkouts of the same application are genuinely different installations.
`<data home>` honors `XDG_DATA_HOME`, and otherwise follows the platform
convention — `~/Library/Application Support` on macOS, `%APPDATA%` on Windows,
`~/.local/share` elsewhere.

Storage lives outside the application directory because that directory may be
read-only, is often a git checkout, and is not where a user expects their data.
When the plugin model lands, a manifest `id` should replace the digest, so an
installed plugin keeps its data across an upgrade that moves it.

Assets — the files `svg(path)` names — are served from the application directory
and nowhere else, with the same traversal check as the module resolver. Note the
asymmetry, because it surprises people: `import "./counter.js"` resolves against
the _importing file_, the way every JavaScript module system does, while
`svg("icons/check.svg")` resolves against the _application root_, the way a web
application's public directory does. The runtime cannot tell which module called
`svg`, so per-file asset paths are not available to it. A missing asset is not an
error — GPUI asks for assets it may not need — but it is warned about once per
path, saying exactly where it was looked for, because an icon that silently does
not appear is among the hardest mistakes to find. One asset may contain at most
16 MiB, and walking the asset tree stops at 10,000 entries or 1 MiB of UTF-8
name bytes.

When present, the manifest's `shell-version` is the oldest compatible
gpui-shell release the application requires. Omitting it accepts the current
runtime. It is checked before the entry module executes, so a
mismatch is a load error rather than an exception in the middle of an
interface. A runtime is compatible when it is not older than the requirement.

The engine belongs in neither the version number nor the manifest: one version
number must mean the same capabilities and the same behavior under either engine.

No packaging or distribution format exists. The intended one is the simplest
thing that works — a `.tar.zst` plus an `index.json`, hosted on any static file
server or git repository, installed by URL with a signature and checksum check,
carrying pure script source with no generated dependency tree. Building
a registry service before the plugin count justifies one would be pure
liability.

---

## 24. Alternatives, and Why They Stay Rejected

This is a record, not a debate. Each of these was decided and none should be
reopened without new information.

| Alternative                                            | Why it is not used                                                                                                                                                                                                                                                                         |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **A second engine (LuaJIT and similar)**               | Removed rather than carried. A fallback nobody exercises rots, and this one had: no `svg`, `Input`, `InputState`, state styles, `accessibility_label`, scheduler, host API, sandbox, or overlays. The seam it justified is kept, because it is what makes the rest of the crate name no VM |
| **The WASM component model**                           | Every call crosses a serialization boundary, which is the worst possible fit for high-frequency fine-grained UI calls. Heavy toolchain, poor debugging                                                                                                                                     |
| **Embedding Node.js or Deno**                          | The process model, native dependency surface, and size do not match an in-process, main-thread, embedded runtime. VS Code's approach requires a separate extension process to work at all                                                                                                  |
| **A pure-Rust scripting language** (Rhai, Steel, Koto) | Almost no ecosystem, a new language for every author, and a thin corpus — which is disqualifying for generated interfaces                                                                                                                                                                  |
| **Rust dylib plugins**                                 | No stable ABI, no sandbox, and the compile cost remains, so it solves none of §2.1                                                                                                                                                                                                         |
| **Rust hot reload**                                    | Solves only the compile time. It does not address plugin distribution or third-party extension, and state preservation is fragile                                                                                                                                                          |
| **A UI DSL or JSX**                                    | A DSL is a second language with its own parser, diagnostics, editor support, and versioning. JSX needs a compile step, which returns the "edit, save, see it" property this runtime exists for (§5.3)                                                                                      |
| **Object-literal element descriptions**                | Exactly equivalent to the builder chain and therefore a second dialect of the same thing (§8.2)                                                                                                                                                                                            |
| **Automatic dependency tracking**                      | A second mental model beside GPUI's explicit `notify`, plus a permanent `Proxy` cost on the render path with no JIT to amortize it (§11.2)                                                                                                                                                 |

---

## 25. Standing Risks

| Risk                                                                                                                                                                                     | Impact    | State                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Cross-boundary call cost exceeds the budget.** Base-first raises operations per node and style has no batching form                                                                    | Contained | Measured under budget at 443 nodes (§20.3), and no longer paid per frame (§8.4); the levers if it regresses are specialized call forms, subtree memoization, virtualization, and finer view granularity                                                                                                                                                                                                                                                                                                                                                                        |
| **Script render couples to frame rate again.** A repaint that enters the VM puts the whole description cost on the frame budget                                                          | Fatal     | Prevented by the snapshot lifecycle (§8.4) and asserted by benchmark C plus `tests/snapshot.rs` (§20.3). It is a regression test rather than a convention precisely because the coupling is easy to reintroduce and invisible until it is a frame-rate problem                                                                                                                                                                                                                                                                                                                 |
| **Presentation authority in script means uneven interface quality**                                                                                                                      | High      | Mitigated by the default palette and by `examples/js_todolist/ui.js` as a worked example; a shipped preset (§13.4) and a `gpui-component` module (§14.6) are the real answers                                                                                                                                                                                                                                                                                                                                                                                                  |
| **Bindings drift from upstream**                                                                                                                                                         | High      | The style surface is immune by construction; component bindings have no drift check at all (§14.5)                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| **A continuation resumes against the wrong runtime.** Several runtimes can share one UI thread, so consulting a process/thread global after `await` would cross VM and policy boundaries | Fatal     | **Closed.** Tasks retain `Weak<ShellRuntime>` plus their `Policy` and weak view owner. Runtime shutdown filters the task registry by runtime identity; destroying runtime A leaves runtime B's work intact. Scheduler and failed-render continuation tests cover both cases (§12.3)                                                                                                                                                                                                                                                                                            |
| **Cycles across two collectors leak**                                                                                                                                                    | Medium    | Render-bound callbacks are retired with their snapshot and long-lived ones are owner-bound (§7.4); retained state is a per-runtime `EntityStore` that drops with the runtime. Native module registries live on `Policy`, and runtime-owned persistent values are released before QuickJS is dropped                                                                                                                                                                                                                                                                            |
| **A symlink escape from a filesystem grant**                                                                                                                                             | Fatal     | **Closed.** The resolver returns an open directory handle rather than a path, and every operation runs against it, so no name is resolved twice — `cap-std`, which is `openat2(RESOLVE_BENEATH)` on Linux and a per-component `openat` walk elsewhere. Two earlier attempts were not enough: comparing strings missed a link entirely, and comparing strings then canonicalizing caught a link that was already there but not one planted between the check and the syscall. `a_symlink_planted_after_the_check_is_still_refused` is the test for the case neither could cover |
| **Sandbox escape**, with `Eval`, quickjs-libc, and prototype pollution as the largest surfaces                                                                                           | High      | quickjs-libc is not compiled in; prototypes are frozen; every compiler path is closed at the JavaScript level, but the stronger intrinsic-level fix is not done (§19.1). The escape suite is real and asserts on messages                                                                                                                                                                                                                                                                                                                                                      |
| **Generated code assumes Node or a browser**                                                                                                                                             | Medium    | Named stubs fail with the available replacement or capability boundary, and `gpui-kit.d.ts` moves unsupported API errors into the editor. A complete browser/Node standard library is deliberately outside Core (§3, §19.1)                                                                                                                                                                                                                                                                                                                                                    |
| **Per-plugin capability grants.** Two loaded plugins must be able to hold different permissions at once                                                                                  | High      | Closed. The grant lives on a `Policy` carried by the call frame, so the engine reads the grant of whichever plugin owns the running code, and it survives an `await` (§18.3)                                                                                                                                                                                                                                                                                                                                                                                                   |
| **Interned `&'static str` accumulates** for script-registered names                                                                                                                      | Low       | Reachable now that panel names are interned (§15). Bounded by applications loaded × panels each, tens of bytes apiece, and never reclaimed — deliberately, because a persisted layout may still refer to a name                                                                                                                                                                                                                                                                                                                                                                |

---

## 26. What Is Built and What Is Not

This is the section most likely to be out of date; check it against the source.

### Built and reachable from script

The engine seam, with QuickJS behind it and `compile_error!` guarding an engineless
build. The render protocol: descriptions into `SpecArena`, published as a
`RenderSnapshot` that outlives the frame that materialized it, materialization in
pure Rust, single-use enforcement, and the text-color inheritance the description
walk resolves. `CallScope` with four phases,
generation checks, and the crate's only `unsafe`. Retained state by handle, with
store-owned subscriptions, for `InputState`. The full style surface — 3,148
reflected no-argument methods, 57 hand-bound parametric ones, 9 hand-added font
weights — with Levenshtein suggestions and a two-prototype diagnostic strategy.
The default semantic palette in light and dark, exposed as a deeply read-only
snapshot through `cx.theme()` (with `gpui.theme()` retained for compatibility)
and switchable with `gpui.set_theme()`. Native target-value transitions and
springs for opacity and pixel geometry, without per-frame script callbacks.
Callbacks with per-pass lifetime and
generation-checked dispatch. State styles for hover, active, and focus.
Asynchrony: promises bridged to GPUI tasks, job-queue draining, `spawn`,
`cx.sleep`, `cx.timer.after`/`every`, owner-bound cancellation, and
unhandled-rejection reporting. Multiple runtimes may coexist; tasks retain their
originating runtime and policy, and initialization runs only after the final
`ScriptView` exists. `Link` with an absolute HTTP(S) `.href(...)` opened by the
host. `ShellRoot` with the dialog stack, one sheet, the
toast stack, focus restoration, and Tab navigation, reached through `cx`. System
capabilities for asynchronous `fs`, storage, clipboard, `process`,
scoped HTTP, TCP, and WebSocket, all default-denied. HTTP redirect
reauthorization and the bounded text/binary WebSocket actor are part of that
surface. Host-registered native modules through
`native(name)`. Manifest-level `shell-version` compatibility. The sandbox:
module confinement, compiler withholding, frozen prototypes, absent-global
stubs, interrupt and memory limits. Hot reload with per-generation module
invalidation. `gpui-kit.d.ts` generation from the dispatch tables. The CLI, with
`check` and `types`. The three-way benchmark of §20.3, including the cached-render
regression gate.

### Built in Rust, with no engine binding above it

The dock: `ScriptPanel`, `ScriptDockSkin`, the three renderer traits forwarded
to a `DockChrome`, panel-name interning, registry round-trip, and the
JSON projections of each renderer context (§15). A host can drive all of it; a
script cannot reach any of it.

The plugin model: manifest parsing and its generated schema, discovery, load and
unload, per-plugin policies, capabilities and data directories (§18). The CLI
uses one local application's manifest directly, as does the public
`ShellRuntime::load` convenience path; `PluginManager` remains available for
hosts that actually need discovery and id-based unload. Integration tests cover
both direct loading and asynchronous initialization under a manifest policy.

### Not built

`gpui.memo` and every other memoization. The template cache of §20.7 is a
partial exception and is described there: its instrumentation reports through
`RuntimeMetrics`, and its mechanism is implemented and tested but reaches no
script — measuring it showed an automatic wrapper would be worth 1.25×, and
only a script written for one is worth 3.5×. Of base's components, `Tree` and the
higher-level `List` are not bound, nor is `Calendar`'s element (§14.2 —
`CalendarState` is), nor `AlertDialog`'s parts (§14.2), nor `ColorPicker`.
Semantic state styles (checked, selected, disabled) with base's precedence rules.
`gpui.open_window` and multi-window applications. A contribution registry — no
`gpui.command`, `gpui.keymap`, `gpui.register_panel`, or `gpui.register_theme`;
key bindings exist (§10.6) but are installed by a running script rather than
declared in a manifest. The capability authorization model: prompting,
persistence, host policy, and re-asking on upgrade. The binding table and the
rustdoc-JSON drift check. Packaging and distribution. The intrinsic-level
`Eval` withholding. DevTools and `gc_stats`. State preservation across a
reload. A shipped preset module. The `gpui-component` binding registry.

Drag and drop is not bound either, and that is a measurement rather than an
omission: `crates/story`, which is an application written with the library and
so is what a script author is, uses `on_drag` and `on_drop` once each, against
94 uses of `on_action`. The library's own internals use it eighteen times, all
of them inside `table`, `list` and `dock`.

---

## 27. Open Questions

1. **How thick should a preset module be?** Too thin and every author writes
   button styling from scratch; too thick and it becomes a third visual system
   in practice (§13.4). `examples/js_todolist/ui.js` — button, icon button,
   checkbox, field, label, surface, empty state — is the current answer and a
   reasonable starting scope. Whatever ships also has to be written per engine.

2. **Do `ShellRoot` and `Root` eventually merge?** Once `gpui-component` is
   bound, `ShellRoot` could delegate to `Root` and reuse its dialog, sheet, and
   notification stacks, or keep its own. `ShellRoot` has since grown decisions
   `Root` does not make — per-dialog dismissal options, only-the-topmost
   backdrop, vetoing Enter — so the merge is less obviously free than it looked.

3. **Can a script define modules other plugins can import?** Cross-plugin
   dependency brings version resolution, load ordering, and cycles. Reuse within
   one plugin only, until there is evidence otherwise.

4. **VM granularity across windows.** One VM for all windows (shared state,
   simple) or one per window (isolated, but state synchronization becomes the
   problem)? One VM is the working assumption, and it is the premise of freezing
   the built-in prototypes (§19.1). The host opens exactly one window today, so
   this is untested.

5. **What does a narrow Editor interface look like?** The full LSP, folding, and
   highlighting surface is explicitly out of scope (§14.2), but "here is the
   text, here is the language, here is the read-only flag" is worth prototyping.

6. **Where do plugin settings live?** A host settings interface driven by a
   script-declared schema (consistent) or drawn by the plugin (flexible)? The
   former, with `gpui.register_settings(schema)`, is the working preference.

7. **Where is the compatibility-stub boundary?** `setTimeout` errors and points
   at `cx.timer`; `fetch` errors and points at a capability. What about
   `structuredClone`, `TextEncoder`, `URL`, `crypto.randomUUID`? The draft
   criterion is that anything mapping exactly may be provided and anything
   mapping approximately may not — the same rule that refuses to name the HTTP
   API `fetch` (§17.2). `console` is the proven exact-mapping case: its methods
   forward to `gpui.log` without adding a second logging subsystem.

8. **When does a second engine become worth adding?** The Lua fallback was
   removed rather than repaired: it had fallen far enough behind (no `svg`,
   `Input`, state styles, scheduler, host API, sandbox, or overlays) that
   "compilable fallback" had stopped being true in any useful sense. The seam
   stays, because it is what keeps the rest of the crate free of VM names. The
   criterion for a second engine is a platform or an embedder that QuickJS
   cannot serve — not a benchmark, now that description cost is not frame
   cost (§8.4).

9. **Does the seam still pay for itself?** It was built because the engine
   choice could not be settled on paper. It has been settled, on measurement,
   in QuickJS's favor. What it still buys is discipline — 90% of the crate cannot
   name a VM — and that discipline is the thing worth keeping even if the second
   engine is not.

---

## 28. Appendices

### Appendix A: A worked example

`examples/js_todolist` is the reference application, and it exists to exercise
the whole runtime rather than to be minimal: retained input state, controlled
checkboxes, a dialog, a toast, capability-gated storage, and a filter that must
survive every mutation. It is four files — `main.js` for the view, `ui.js` for
the presentation layer, `storage.js` for persistence, `confirm.js` for the
dialog body — and a test loads and renders it, because if it stops rendering the
quickstart is wrong.

```js
import { View } from "gpui-kit";
import { h_flex, v_flex, InputState } from "gpui-base";

export default class TodoList extends View {
  init() {
    this.draft = InputState.new({ placeholder: "What needs doing?" });
    // Enter is how a list like this is actually used; the Add button is for
    // the pointer, not the primary path.
    this.draft.on("submit", (_event, cx) => this.add(cx));
    this.items = [];
    this.filter = "all";
  }

  add(cx) {
    const caption = this.draft.value().trim();
    if (caption === "") return;
    this.items = [...this.items, { caption, done: false }];
    this.draft.set_value("");
    cx.notify();
  }

  render(cx) {
    return v_flex()
      .size_full()
      .bg(cx.theme().colors.background)
      .p(24)
      .gap(16)
      .child(this.composer(cx))
      .children(this.items.map((item) => this.row(item, cx)));
  }
}
```

Five things in that code are the shapes this document has been describing.

**Event handlers are always arrow functions**, because they need `this` to be
the view (§10.1).

**`children` takes an array**, so `map` is the natural list form.

**`when(condition, fn)` keeps the chain in one piece**, matching the GPUI
builder style `CLAUDE.md` requires, instead of splitting into a temporary and a
sequence of `if`s:

```js
label(item.caption, cx).when(item.done, (el) =>
  el.text_color(cx.theme().colors.muted_foreground).line_through(),
);
```

**Bound methods are snake_case and the author's own are camelCase** —
`visible()`, `setFilter`, `clearCompleted` against `.items_center()`,
`.on_change()`. That contrast is §6.4's trade in real code.

**A capability that was not granted is absorbed where it is used, not checked
at every call site.** `storage.js` wraps `localStorage` in try/catch and the
interface says "Not saved" rather than failing:

```js
export function save(items) {
  try {
    localStorage.setItem(KEY, JSON.stringify(items));
    return true;
  } catch (error) {
    console.warn(`todolist: could not save (${error.message})`);
    return false;
  }
}
```

One correction to the shipped example: `main.js` used to open its confirmation
dialog by handing over a view class and a `props` object. Both are gone —
`confirm.js` exports a function, and what the dialog shows is closed over:

```js
window.open_dialog(confirmClear(count, onConfirm));
```

### Appendix B: Crate layout

```text
crates/shell/                 # gpui-shell — depends on gpui-base + gpui only
  Cargo.toml                  # features: quickjs (default)
  src/
    lib.rs                    # init, capability and storage entry points, re-exports
    engine/                   # ← the seam
      mod.rs                  #   contract, compile_error! guard, cfg forwarding
      quickjs/
        mod.rs                #   prelude, dispatch, module resolver, callbacks
        host.rs               #   fs · storage · clipboard · console
        scheduler.rs          #   promises · timers · task ownership · job draining
        sandbox.rs            #   language trimming · process · limits
        overlay.rs            #   dialog · sheet · toast on the script-side cx
        entity_api.rs         #   the script face of retained state
        native.rs             #   value conversion for native(name)
        theme_api.rs          #   theme · set_theme
    scope.rs                  # CallScope — the crate's only unsafe module
    snapshot.rs               # RenderSnapshot — one script render, many frames
    metrics.rs                # script renders vs materializations, and timings
    spec.rs                   # SpecArena / SpecNode / SpecOp
    materialize.rs            # snapshot → real elements, pure Rust, non-destructive
    style.rs                  # reflection table + 57 parametric styles + suggestions
    theme.rs                  # the default palette and token resolution
    value.rs                  # Bridged, plus color and length coercion
    error.rs                  # ShellError
    capability.rs             # Capabilities / path resolution / denials
    entities.rs               # EntityStore — retained state by handle, per runtime
    runtime.rs                # CallbackArena<T> · root resolution · failure surface
    root.rs                   # ShellRoot
    dock.rs                   # ScriptPanel · ScriptDockSkin · panel registration
    native.rs                 # the host-registered native module registry
    plugin.rs                 # manifest · discovery · isolated policy · load/unload
    view.rs                   # ScriptView — snapshot ownership and invalidation
    assets.rs                 # application-directory asset source
    watch.rs                  # source watching and in-place reload
    typings.rs                # gpui-kit.d.ts generation
    bin/gpui-shell.rs         # run / check / types
  bin/default-tokens.json     # the CLI host's semantic palette, light and dark
  tests/
    render.rs                 # end-to-end description tests
    snapshot.rs               # the render-frequency invariants (§22.3)
    benchmark.rs              # script build · materialization · cached render
    fs.rs                     # JS → async filesystem/storage → ScriptView
    process.rs                # JS → bounded process Promise → ScriptView
    host_api.rs               # JS clipboard · timer cancellation · native bridge
examples/js_todolist/         # the reference application
```

### Appendix C: Naming

Following `CLAUDE.md`:

- No `Kind` suffix: `ScopePhase` rather than `ScopeKind`, `ExecuteGrant` rather
  than `CapabilityKind`, `SpecOp` rather than `SpecOpKind`.
- Public types crossing the seam keep private fields and are built with a
  builder, so adding a field is not a breaking change: `Capabilities`,
  `DialogOptions`, `ToastRequest`. An all-boolean type names its setters after
  the field and reads with `is_`/`has_` (`DialogOptions::escape_dismissable` and
  `is_escape_dismissable`); a type with non-boolean fields prefixes setters with
  `with_` (`ToastRequest::with_description`). `Capabilities` is inconsistent
  here — `read_roots` and `with_execute` beside a bare `storage(bool)` and
  `clipboard_read(bool)` — and should be brought in line.
- `Context` is spelled out: `PanelBuildContext`, `TabGroupContext`, never
  `…Ctx`. `cx` is reserved for GPUI's `App`, `Context<T>`, and `AsyncApp`, and
  for the script-side object of the same name.
- **Rust type names above the seam carry no language**: `ScriptView`,
  `ScriptPanel`, `ScriptDockSkin` — never `JsView`. They do not know what the
  language is. Types inside an engine may, because there the language is
  singular.
- Bound script method names match Rust exactly, in snake_case, with no camelCase
  renaming (§6.4). Names an author writes follow the host language's convention.
- One namespace prefix for everything a script contributes to the host, and it
  names the shell rather than the engine: `shell:<application>/<panel>` (§15).
  `plugin.rs` still documents it as `script:<id>/<panel>`, which `dock.rs` does
  not implement; the two should be reconciled before either is public.

Module documentation in `crates/shell/src` should say "script" wherever it means
"whatever the engine is", and name QuickJS only inside `engine/quickjs`. The
batch that used to say "Lua" has been corrected.
