---
title: GPUI Shell
description: Makes a Rust GPUI application extensible in JavaScript, rendered by GPUI itself — no WebView, no DOM. Plugins first, standalone script applications second.
order: 1
---

# GPUI Shell

`gpui-shell` exists to make a Rust GPUI application **extensible in JavaScript**.

**The primary goal is plugin extension.** A host application compiles and ships once. After that, a new panel, a side tool or a piece of business logic arrives as a script loaded into the same process — no rebuild, no binary to redistribute, and no fork for a contributor who only wants to add a panel.

**The secondary goal is writing a whole application in JavaScript.** The CLI runs an application directory on its own, which is a usable path in itself and also how a plugin is developed: get the script running standalone, then mount it in a host.

**It is not an Electron or a Tauri.** There is no WebView, no DOM, no HTML or CSS, no browser engine, and no Node.js. A script never renders. It describes an interface once, and Rust replays that description into real GPUI elements on every frame after it — the same element model a Rust application on `gpui-base` builds, through the same GPU renderer. JavaScript is the application layer here, not the rendering layer, which is why a repaint costs no JavaScript at all and taking the whole runtime costs [+13.5 MiB of binary](./engine.md#what-linking-it-costs).

Both goals rest on the same split. `gpui-shell` is built directly on [`gpui-base`](/base), with [QuickJS](https://github.com/quickjs-ng/quickjs) running on the host's own thread. The host builds the runtime and grants what a script may reach; the script draws real interface inside the same process. Rust keeps rendering, layout, text editing, virtualization, focus, overlays and every system capability; the script owns composition, presentation and business logic.

```js
import { View } from "gpui-kit";
import { v_flex, Button } from "gpui-base";

export default class Counter extends View {
  init() {
    this.count = 0;
  }

  render(cx) {
    return v_flex()
      .size_full()
      .items_center()
      .justify_center()
      .gap(20)
      .bg(cx.theme().colors.background)
      .child(
        div()
          .text_3xl()
          .text_color(cx.theme().colors.foreground)
          .child(`${this.count}`),
      )
      .child(
        Button.new("increment")
          .h(32)
          .px(14)
          .items_center()
          .justify_center()
          .bg(cx.theme().colors.primary)
          .text_color(cx.theme().colors.primary_foreground)
          .rounded(6)
          .on_click((_event, cx) => {
            this.count += 1;
            cx.notify();
          })
          .child("Increment"),
      );
  }
}
```

## Why plugins come first

`crates/base/src/dock` already holds half of what a plugin system needs: a layout that is pure data, a `PanelRegistry` that rebuilds a panel from a name in a persisted file, and a per-panel `serde_json::Value` that rides along with it. The missing half is that a panel's implementation has to be compiled into the host binary — nobody can contribute one without forking it. `gpui-shell` supplies that half.

Plugins-first is not a positioning statement. It is the reason behind decisions that would each have gone another way for a runtime aimed only at standalone scripts:

| Decision                                                                  | Why it follows from plugins                                                                                              |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `Capabilities::default()` is the empty set, and the host grants           | A plugin is code someone else wrote; the grant has to be the host's, not a self-declaration in the plugin's own manifest |
| A separate `Policy` per plugin, and unload cancels every task carrying it | Several plugins share one runtime, so grants must not bleed between them                                                 |
| A script fault is a recoverable exception, and the host process survives  | One broken plugin should not take the application with it                                                                |
| A repaint replays a Snapshot and never enters the VM                      | The host answers for the frame budget, so a plugin's JavaScript cannot sit on it                                         |
| `HostModule` lends the host's own Rust to a script                        | Only meaningful when the script runs inside a host — a standalone application has no host to borrow from                 |
| Dock panels keep their place and state across an uninstall                | Plugins get installed and removed; a panel comes back where it was, with what it had                                     |
| The foundation ships no presentation, so the script owns all of it        | A plugin has to look like part of its host, which takes control of every pixel                                           |

A standalone script application uses few of these. What it gains is the iteration speed — hot reload, `check`, and a generated `gpui-kit.d.ts` — which is why it sits second: it is where a plugin is developed and proven, rather than the point of the runtime.

Text editing, syntax highlighting, LSP, virtualization and motion sampling stay in Rust. That line is a division of responsibility rather than a limit on the script: the host owns everything that has to sit close to the GPU and the system, so a plugin never becomes a variable in the application's performance or stability.

::: warning Plugins are the goal, not yet the whole interface
The machinery below a plugin is built and tested — manifest parsing and discovery, load and unload, a per-plugin policy and data directory. A script now contributes panels and draws a dock's chrome: `DockArea`, `dock_area(...)` and `DockArea.register_panel` are public, and a layout with a script's panels in it survives a restart. What is still missing is the rest of the contribution registry (`gpui.command`, `gpui.keymap`), the authorization UI, and a CLI that uses `PluginManager`. **What runs end to end today is the standalone path, docks included.** See [Dock and Panels](./dock.md).
:::

## What defines it

### Architecture: the script describes, the host renders

A script never holds a GPUI element. It records a **description** of one — every call in a builder chain writes an operation into an arena, and Rust replays those operations into real elements when a frame needs them. Layout, painting, hit testing, scrolling, IME and text editing stay in Rust and never call back into the script. [How a script becomes an interface](#how-a-script-becomes-an-interface) traces one pass of that.

The engine is a parameter of the design rather than a part of it. QuickJS is the only one today, but everything above the seam — the arena, the materializer, the call scope, the style table, the theme, the capability model, the overlay host, hot-reload — names no VM anywhere in its source. See [The engine seam](./engine.md).

### Capability: a whole application layer, not a widget set

A script gets what a Rust application built on `gpui-base` gets: elements and layout, links and controls, a fluent style surface over semantic theme tokens, View state through `init` / `render` / `cx.notify()`, retained host state such as a text input's rope and selection, dialogs, a sheet and toasts, asynchronous tasks, native transitions and springs, and gated filesystem, storage, clipboard, process, HTTP, TCP and WebSocket surfaces.

Around that: `--watch` hot-reloads on save, `gpui-shell.json` declares identity and least-privilege capabilities before code runs, a generated `gpui-kit.d.ts` describes the whole API to an editor or a model, and `check` reports mistakes before the application runs.

::: tip
`gpui-kit.d.ts` can go in `.gitignore` — it is generated.
:::

### Performance: the script is not in the frame

`render` does **not** run once per frame. It describes the interface once into a Snapshot, and until the next `cx.notify()` every repaint replays that Snapshot in Rust. A pointer crossing a button, a blinking cursor, a scrolling list and a native transition or spring advancing do not run JavaScript.

The runtime counts the two events separately, and the gallery's Shell story (`cargo run -- shell`) puts both counters on screen:

<img class="architecture-light" src="/shell-render-frequency-light.svg" alt="One second of a live panel. With nothing JavaScript reads changing, 60 frames fire and the JavaScript track stays empty. With prices moving every 50 ms, 60 frames fire and JavaScript runs about 20 times.">
<img class="architecture-dark" src="/shell-render-frequency-dark.svg" alt="One second of a live panel. With nothing JavaScript reads changing, 60 frames fire and the JavaScript track stays empty. With prices moving every 50 ms, 60 frames fire and JavaScript runs about 20 times.">

| What the interface is doing                       | Frames a second | JavaScript runs a second |
| ------------------------------------------------- | --------------- | ------------------------ |
| Repainting, with nothing JavaScript reads changed | 60              | 0                        |
| Prices moving every 50 ms                         | 60              | 19                       |

The frame count belongs to the display, the JavaScript count to the data. In the second row the other 41 frames replay a description that already exists.

Cost is therefore paid per user action rather than per frame. On a 443-node panel, running `render` and recording the whole interface into a Snapshot takes 1.1 ms, paid only when state changes; each frame after it takes 1.3 ms, which is rendering itself — turning the Snapshot into elements, laying out, painting, with no JavaScript in it.

|                    | Cost per frame                                                      |
| ------------------ | ------------------------------------------------------------------- |
| Without a Snapshot | 1.1 ms (JS render) + 1.3 ms (Rust render) = **2.4 ms/frame render** |
| With a Snapshot    | **1.3 ms**                                                          |

Growing the panel does not change that. The [benchmark](./engine.md#the-measurement) covers sizes up to 8,403 nodes, no frame at any of them runs JavaScript, and the smallest size is asserted on every CI build.

### Size: a script runtime for +13.5 MiB

A host that runs a real script application ships a **26.1 MiB** binary and holds **81 MiB** resident, QuickJS and the whole Standard Runtime included. Taking the dependency costs **+13.5 MiB of binary and +14 MiB of memory** over the same application without it.

That figure is a constant, not a proportion: the component gallery — five times the size — adds the same 13.5 MiB. [What linking it costs](./engine.md#what-linking-it-costs) gives the pair it was measured on, and where the megabytes go.

All figures here were taken on a MacBook Pro (M3, 8 cores, 24 GB): the frame and run counts from the Shell story, the milliseconds from a release build of the benchmark, the binary and memory figures from release builds of `examples/hello_world` and the `gpui-shell` CLI.

### Security: nothing by default, and a language trimmed to match

`Capabilities::default()` is the empty set — no file access, no storage, no clipboard, no process execution, no network. The host decides the grant before loading a View, which then keeps that grant for its lifetime; every path in the `fs` surface goes through **one** resolver that refuses anything landing outside a granted root.

Below the grants, the sandbox trims the language itself, because one VM will eventually host several plugins: `eval` and all four function compilers are gone, the built-in prototypes are frozen so one plugin cannot change `Object.prototype` for another, module resolution is confined to the application directory, and the heap (256 MiB), interpreter stack (1 MiB) and time in a single call (50 ms in `render`) are capped. That time limit is an interrupt a `catch` block cannot swallow, which is measured by a test. See [Capabilities](./capabilities.md).

## How a script becomes an interface

<img class="architecture-light" src="/shell-architecture-light.svg" alt="How a script becomes an interface: the script describes elements, Rust materializes them, GPUI paints">
<img class="architecture-dark" src="/shell-architecture-dark.svg" alt="How a script becomes an interface: the script describes elements, Rust materializes them, GPUI paints">

The diagram traces one frame, and the shape of it explains most of this documentation.

GPUI elements are values that are **consumed** when used: `RenderOnce::render` takes `self` by value, `.child()` takes its child by value, and a View rebuilds its whole element tree on every redraw. A JavaScript object can therefore never _be_ a GPUI element — there is nothing for it to hold onto.

So the script does not build elements. It **describes** them. Every call in a builder chain records one operation into an arena of element descriptions; the object the script holds carries nothing but an integer index into that arena. When GPUI asks the View to render, Rust replays the recorded operations into real elements, hands them to GPUI, and clears the arena. Layout, painting, hit testing, scrolling and IME never return to the script.

Three consequences follow directly, and each has a page below:

- **Elements are single-use.** The description is gone at the end of the pass, so a stored element throws on its next use rather than drawing something unexpected. See [Elements](./elements.md).
- **The `cx` handed to a call belongs to that call.** It carries a generation number, checked against the live call stack, so a `cx` kept across an `await` reports a clear error instead of touching a dead stack frame. See [State and Views](./state.md).
- **Callbacks belong to the render that registered them.** They are replaced wholesale by the next render, which is what keeps script closures from accumulating in the host. See [Elements](./elements.md).

All three fall out of binding a script to an element model that consumes its values.

## Presentation belongs to the script

Most scripting layers hand a script a set of finished widgets and let it arrange them. This one has none to hand over, because the layer underneath it has none either.

`gpui-base` controls carry no visual style at all. `Button::new("save")` in Rust has no padding, no background, no radius and no size, and that is the contract. The JavaScript bindings preserve it exactly: `Button.new("save")` with no styling draws nothing but its children.

The consequence is the point. **Because the foundation ships no presentation, the script owns all of it** — every colour, every pixel of spacing, every hover state, every corner radius. That is the same trade a Rust application makes when it builds on `gpui-base` instead of `gpui-component`; the difference is that here the trade is made in a file you can save and see the result of immediately, with no `cargo build` in between.

What the script gains in exchange for the extra typing is the whole application layer. Changing a button's radius does not mean going back to Rust.

## Where it fits

- **Adding plugin support to an existing GPUI application — the primary case.** Plugins run inside the host process under capabilities the host grants one at a time, starting from none. Extending the product stops meaning a fork or a new release: interface and business logic ship as script and change without recompiling or redistributing a binary, and a failing plugin surfaces as a recoverable error rather than taking the host down.
- **Writing a complete application in JavaScript on `gpui-shell` — the secondary case.** The whole application layer — elements, styling, View state, overlays and system APIs — while rendering, text editing, virtualization and every animation frame stay in Rust. It is also where a plugin is written and proven before it is mounted in a host.

## Where it sits

```text
  JavaScript application       main.js · Views · styles · business logic
            │  import { … } from "gpui-kit"
            ▼
  gpui-shell                   engine seam · element descriptions · call scope
                               style table · theme tokens · capabilities
                               ShellRoot (dialogs, sheet, toasts) · scheduler
            │
            ▼
  gpui-base                    behavior · state · infrastructure (no style)
            │
            ▼
  gpui                         elements · styling · rendering · GPU · platform
```

`gpui-shell` sits beside `gpui-component` rather than beneath it: both are consumers of `gpui-base`, and both supply a presentation layer that Base does not. `gpui-component` supplies one in Rust, finished and coherent. `gpui-shell` supplies the machinery for a script to supply its own.

## Read next

| Page                                    | What it covers                                                                                         |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| [Getting started](./getting-started.md) | Running the example, the smallest application, `check` and `types`                                     |
| [Examples](./examples.md)               | The two applications in the repository, and what to copy from them                                     |
| [Elements](./elements.md)               | Constructors, `child` / `children` / `when`, and why an element is single-use                          |
| [Styling](./styling.md)                 | The fluent style surface, lengths, colour tokens and state styles                                      |
| [State and Views](./state.md)           | `init` / `render`, `cx.notify()`, retained state, async                                                |
| [Overlays](./overlays.md)               | Dialogs, the sheet, toasts, and the phase rule                                                         |
| [Capabilities](./capabilities.md)       | `gpui-shell.json`, default deny, filesystem, storage, process and network APIs                         |
| [Dependencies](./dependencies.md)       | Shell packages: what makes one, how a manifest names and pins it, and the types an editor gets         |
| [Hosting](./hosting.md)                 | The Rust side in full: mounting, refreshing, metrics, exit, hot-reload                                 |
| [HostModule](./host-module.md)          | Lending the host's own Rust to a script, and the plain-data boundary                                   |
| [Dock and Panels](./dock.md)            | A script View as a dockable panel, the chrome you draw for it, and what survives a restart             |
| [Performance](./performance.md)         | What a script costs: invalidation against description size, the View as the boundary, and the counters |
| [The engine seam](./engine.md)          | QuickJS, why the seam exists, and the measurements that tell script cost from frame cost               |

## Status

The crate is at milestone **M0**: a feasibility baseline, not a stable interface. It is not published to crates.io, and the script API is expected to change. What is documented here exists and works; what is missing is called out on the page where you would go looking for it.

The design is specified in the [GPUI Shell design document](https://github.com/longbridge/gpui-kit/blob/main/docs/gpui-shell.md), and the crate lives at [`crates/shell`](https://github.com/longbridge/gpui-kit/tree/main/crates/shell).
