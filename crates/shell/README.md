# gpui-shell

`gpui-shell` exists to make a Rust GPUI application
**extensible in JavaScript**.

**The primary goal is plugin extension.** A host application compiles and ships
once. After that, a new panel, a side tool or a piece of business logic arrives
as a script loaded into the same process — no rebuild, no binary to
redistribute, and no fork for a contributor who only wants to add a panel.

**The secondary goal is writing a whole application in JavaScript**, which is
also how a plugin is developed: get the script running standalone, then mount
it in a host.

**It is not an Electron or a Tauri.** There is no WebView, no DOM, no HTML or
CSS, no browser engine, and no Node.js. A script never renders. It describes an
interface once, and Rust replays that description into real GPUI elements on
every frame after it — the same element model a Rust application on `gpui-base`
builds, through the same GPU renderer. JavaScript is the application layer here,
not the rendering layer.

Both goals rest on the same split. Built on
[`gpui-base`](../base/README.md), the host owns rendering, layout, input and
system capabilities; the script owns composition, presentation and business
logic. JavaScript is the default scripting language.

Its design is specified in [`docs/gpui-shell.md`](../../docs/gpui-shell.md).
This crate is at milestone M0: a feasibility baseline, not a stable interface.
A script now contributes dockable panels and draws a dock's chrome —
`gpui_shell::dock` is public, and `DockArea`, `dock_area(...)` and
`DockArea.register_panel` are part of the script surface. What is still missing
above them is the rest of the contribution registry and a CLI that uses
`PluginManager`, so the standalone path is what runs end to end today.

## Base-First: The Script Owns Presentation

`gpui-base` controls carry no visual style. `Button::new("save")` has no
padding, background, radius or size, and that is an API contract rather than a
missing feature. The JavaScript bindings preserve it: `Button.new("save")` with
no styling draws nothing but its children.

```js
function saveButton(cx) {

  // Unstyled: activation, focus and disabled state work, but nothing is drawn.
  Button.new("plain-save").on_click(save).child("Save");

  // Styled: every visual decision is written out, in the script.
  return Button.new("save")
    .h(32)
    .px(14)
    .items_center()
    .justify_center()
    .text_sm()
    .bg(cx.theme().colors.primary)
    .text_color(cx.theme().colors.primary_foreground)
    .rounded(6)
    .on_click(save)
    .child("Save");
}
```

This is the same trade the Rust side makes when an application builds directly
on `gpui-base` instead of `gpui-component`. Colors are named as semantic theme
tokens, so a shared visual language stays available without the runtime making
visual decisions on the application's behalf. Applications that want ready-made
product visuals wait for a `gpui-component` module, a later milestone.

## Quick Start

Run the bundled example from the repository root:

```bash
cargo run -p gpui-shell -- examples/js_todolist
```

The runtime loads `main.js` from the given directory, takes the view class it
default-exports, and mounts one instance of it as the window's root view:

```js
// main.js
import { View } from "gpui-kit";
import { v_flex, Button, InputState } from "gpui-base";

export default class Notes extends View {
  init() {
    this.draft = InputState.new({ placeholder: "What needs doing?" });
    this.draft.on("submit", (_event, cx) => this.add(cx));
    this.items = [];
  }

  add(cx) {
    const caption = this.draft.value().trim();
    if (caption === "") return;
    this.items = [...this.items, caption];
    this.draft.set_value("");
    cx.notify();
  }

  render(cx) {
    return v_flex()
      .size_full()
      .p(24)
      .gap(12)
      .bg(cx.theme().colors.background)
      .children(this.items.map((item) => div().text_color(cx.theme().colors.foreground).child(item)));
  }
}
```

See [`examples/js_todolist`](../../examples/js_todolist) for the complete
version: retained input state, controlled checkboxes, a confirmation dialog, a
toast, icons, and storage that degrades to memory when it is not granted.

For a whole product rather than a demonstration — OAuth, a live WebSocket quote
feed, a virtualized watchlist, retained nested views, and its own Rust host —
see [longbridge/longbridge-lite](https://github.com/longbridge/longbridge-lite),
the largest application written against this runtime.

### Checking an application without running it

JavaScript has no compiler, so the runtime provides what would otherwise be
missing:

```bash
cargo run -p gpui-shell -- check examples/js_todolist    # exit 0 or 1
cargo run -p gpui-shell -- check examples/js_todolist --print-spec
cargo run -p gpui-shell -- types examples/js_todolist    # writes gpui-kit.d.ts
```

`check` loads and renders the application once without showing a window. It
reports syntax errors, unresolved imports, a missing or malformed default
export, unknown style methods with a suggestion, wrongly typed style arguments,
and an element used twice — each with the script's own stack. `types` writes
TypeScript declarations generated from the same tables the runtime dispatches
through, so an editor catches a mistyped style method before it runs.

### Working on an application

```bash
cargo run -p gpui-shell -- examples/js_todolist --watch
cargo run -p gpui-shell -- examples/js_todolist --dev    # implies --watch
```

A reload re-reads every module, entry included. If the new code fails to load,
the previous view keeps running and the error is reported — a broken save never
costs you the window.

## Naming

Method names on the bindings keep their Rust snake_case spelling —
`items_center`, `on_click`, `gap_2`, `text_sm`. They are not a style choice:
the no-argument style surface is generated from GPUI's reflection table, so the
name in JavaScript is the name in Rust, and a method GPUI adds upstream appears
here without anyone renaming it. Everything an application declares itself —
its own variables, functions, classes and object keys — is ordinary camelCase
JavaScript. The contrast is the point: a snake_case call is host surface, a
camelCase one is script code.

## API Surface

Each module carries what its own crate provides:

```js
import { View, div, svg, image } from "gpui-kit";
import { h_flex, v_flex, Button, Link, Checkbox, Switch } from "gpui-base";
import { fps_monitor } from "gpui-fps";
```

| API | Form | Description |
| --- | --- | --- |
| `div()` | function | An element with no layout of its own |
| `h_flex()` / `v_flex()` | function | A row / column flex element |
| `value` | function | A text element |
| `svg(path)` / `image(path)` | functions | A theme-tinted vector icon / full-colour application asset |
| `fps_monitor()` | function | The native `gpui-fps` performance HUD; passive by default, with `.continuous(true)` for a sustained-frame test |
| `Button.new(id)` | type | A base `Button`: activation, focus, disabled and selected state, no styling |
| `Link.new(id)` | type | A base external link; pair it with `.href("https://…")` |
| `Checkbox.new(id)` / `Switch.new(id)` | type | A base controlled toggle, no styling |
| `InputState.new(options)` / `Input.new(state)` | types | Retained text state and its rendered input |
| `View` | class | Base class of every view; subclass it and default-export the subclass |
| `cx.open_url(url)` | `cx` method | `App::open_url` — hands a URL to the system browser |
| `cx.read_from_clipboard()` / `cx.write_to_clipboard(s)` | `cx` methods | `App::read_from_clipboard` / `write_to_clipboard` |
| `cx.focus_handle()` | `cx` method | `App::focus_handle` — a focus target the script keeps |
| `cx.spawn(body, opts?)` | `cx` method | `App::spawn` — the body's `cx` survives an `await` |
| `cx.sleep(ms)` / `cx.timer.after` / `cx.timer.every` | `cx` methods | Work on the foreground executor |
| `window.paint_path(path, bg)` | `window` method | `Window::paint_path` |
| `localStorage` / `sessionStorage` | globals, also on `window` | The Web Storage API, where the web keeps it |

Where a binding lives in Rust decides where it lives here: an `App` method is a
`cx` method, a `Window` method is on the `window` global, a type's `::new` is
`Type.new(...)`, and a free function stays a free function. What has no GPUI or
base original goes where the web already keeps it: storage is `localStorage`
and `sessionStorage`, and diagnostics are JavaScript's own global `console`.

### Elements

| Method | Description |
| --- | --- |
| `.child(element)` | Adds one child. The child is consumed; using it again is an error |
| `.child(viewHandle)` | Mounts a retained nested view, the way an `Entity<V>` is a child in GPUI |
| `.children([a, b])` | Adds several children |
| `.when(condition, el => el)` | Applies the function only when `condition` holds, keeping the chain in one piece |
| `.href(url)` | Gives a `Link` an absolute HTTP(S) target opened by the host |
| `.transition(property, policy)` | Animates a later target change in native Rust code |
| `.spring(property, policy?)` | Springs a later target change in native Rust code |
| `.overflow_scrollbar()` | Scrolls both axes and paints native scrollbars |
| `.overflow_x_scrollbar()` / `.overflow_y_scrollbar()` | Scrolls one axis and paints its native scrollbar |

### Styling

Every element accepts the no-argument GPUI style surface as methods
(`.size_full()`, `.items_center()`, `.justify_center()`, `.flex_col()`,
`.rounded_md()`, `.text_sm()`, `.font_semibold()`, and the rest), plus about
fifty-seven methods that take arguments:

| Method | Argument |
| --- | --- |
| `.w(n)` `.h(n)` `.size(n)`, and the `min_` / `max_` forms | length |
| `.p(n)` `.px(n)` `.py(n)` `.pt(n)` `.pb(n)` `.pl(n)` `.pr(n)` | length |
| `.m(n)` `.mx(n)` `.my(n)` `.mt(n)` `.mb(n)` `.ml(n)` `.mr(n)` | length |
| `.inset(n)` `.top(n)` `.bottom(n)` `.left(n)` `.right(n)` | length |
| `.gap(n)` `.gap_x(n)` `.gap_y(n)` `.flex_basis(n)` | length |
| `.flex_grow(n)` `.flex_shrink(n)` `.opacity(n)` | number |
| `.border(n)` and its per-edge forms, `.rounded(n)` and its per-corner forms | length |
| `.bg(color)` `.text_color(color)` `.text_bg(color)` `.border_color(color)` | color |
| `.text_size(n)` `.line_height(n)` | length |
| `.font_family(name)` | string |

A number is pixels. A string length is `"auto"`, `"50%"`, `"12px"` or `"1rem"`;
which of those a given method accepts follows the Rust signature, so `.p()`
rejects `"auto"` and `.rounded()` rejects percentages. A color is either a
semantic token name — `background`, `foreground`, `surface`,
`surface_foreground`, `primary`, `primary_foreground`, `secondary`,
`secondary_foreground`, `muted`, `muted_foreground`, `accent`,
`accent_foreground`, `destructive`, `destructive_foreground`, `border`, `input`,
`ring`, `selection` — or a `#rrggbb` literal. Passing values from
`cx.theme().colors` is preferred. Semantic token name strings remain
accepted for compatibility; a literal bypasses the theme.

A style name that is neither reflected nor bound is an error at the call site,
not a silently ignored no-op.

Read semantic values directly at the use site, such as
`cx.theme().colors.surface` or `cx.theme().spacing.md`; do not destructure or
alias the theme snapshot. The returned light/dark snapshot contains direct color roles plus `colors`,
`spacing`, `radius`, `appearance`, and `is_dark`; it and all nested token groups are
frozen. `gpui.theme()` remains a compatibility accessor. Calling
`set_theme({ appearance, tokens })` replaces the active `gpui-base` theme
snapshot and refreshes the windows. Applications own theme names and may load
their token objects from JSON.

Motion is target-based, not a JavaScript frame callback. `transition` and
`spring` accept `opacity`, `width`, `height`, `left`, or `top`; length targets
are currently pixels. JavaScript publishes the new target once, while retained
state, sampling, interruption, reduced motion, and frame requests stay native.

### Components

| Method | On | Description |
| --- | --- | --- |
| `.disabled(bool)` | all | Blocks activation and reports the disabled state |
| `.selected(bool)` | `Button` | Reports the selected state |
| `.on_click(handler)` | `Button` | `handler(event, cx)`, called on click and on keyboard activation |
| `.checked(bool)` | `Checkbox`, `Switch` | The controlled value |
| `.on_change(handler)` | `Checkbox`, `Switch` | `handler(checked, cx)`; the script stores the new value and notifies |

Disabled, selected and checked appearance is the caller's to draw; the base
layer only reports the state.

### Views

```js
export default class Counter extends View {
  init(props) {}   // called once, when the view is created
  render(cx) {}    // returns one element, retained Entity, or string
}
```

`cx.notify()` requests a re-render. It is legal inside an event callback or a
task; calling it during `render` throws, because notifying yourself while
rendering is a loop.

**`render` does not run every frame.** It runs when the view has been
invalidated — a `notify`, a hot reload, a theme change — and publishes a
description that every frame after that replays in Rust. A hover, a scroll, a
blinking cursor or an animation repaints without entering the VM at all, so
script cost follows what your application does rather than the frame rate.

Elements are single-use values. Build them in `render` and never store one on
the instance — a stored element belongs to a render that has already ended, and
reusing it throws rather than drawing something unexpected.

## Capabilities and Asynchronous I/O

System access is denied by default. A local application can declare the exact
grant the CLI installs in `gpui-shell.json`:

```json
{
  "id": "com.example.viewer",
  "name": "Viewer",
  "version": "1.0.0",
  "shell-version": "0.6.0",
  "entry": "main.js",
  "dependencies": {
    "omarchy-ui": "huacnlee/omarchy-ui"
  },
  "capabilities": {
    "fs": { "read": ["${pluginDir}"], "write": ["${dataDir}"] },
    "network": {
      "hosts": ["stream.example.com"],
      "http": [{
        "scheme": "https",
        "host": "api.example.com",
        "methods": ["GET"],
        "paths": ["/v1/profile"],
        "path_prefixes": ["/v1/items/"]
      }]
    },
    "storage": true,
    "clipboard": { "write": true }
  }
}
```

Git dependencies are fetched before the entry module is evaluated. Import the
map key as a bare module:

```js
import { label, style } from "omarchy-ui";
```

A string value may be strict GitHub shorthand (`"owner/repository"` or
`"owner/repository#ref"`) or a full Git URL
(`"https://github.com/owner/repository#ref"`). GitHub shorthand without a
fragment selects `main`; a full URL without one selects the remote's HEAD. A
fragment may name a branch, tag, or commit-ish such as a commit ID. Moving
references are fetched on every application load, while a commit ID keeps
selecting the same commit.

For string dependencies, gpui-shell reads the root `package.json` after the
immutable checkout is ready. A string `main` selects the package entry; a
missing file or missing `main` defaults to `index.js`. Malformed metadata, a
non-string `main`, or an entry that is missing, not a file, or escapes the
checkout fails the application load before its JavaScript entry executes.

The legacy object form remains supported without changes. It requires exactly
one explicit `branch` or `tag`, and its repository-relative `entry` still
defaults to `index.js`:

```json
{
  "omarchy-ui": {
    "git": "https://github.com/huacnlee/omarchy-ui",
    "tag": "v1.2.0",
    "entry": "src/public.js"
  }
}
```

Dependencies live below `~/.gpui-shell/cache/dependencies/`. A per-remote lock
serializes updates to the local mirror; immutable commit-addressed checkouts
keep concurrent launches and older hot-reload generations isolated. The exact
fragment-free URL is the cache and remote identity. Its raw configured origin
is verified even when Git's `url.*.insteadOf` changes the effective fetch URL.
Git commands are non-interactive and bounded to 30 seconds. Relative imports
inside a package remain confined to its checkout. Fetching requires `git` on
the host and happens before script capabilities apply.

`network.hosts` grants the host to HTTP, raw TCP, and WebSocket clients;
`network.http` narrows HTTP to a scheme, effective port, listed methods and
paths without granting TCP or WebSocket access. Its default scheme is `https`
and its default port is the scheme's standard port; specify `port` only for a
non-default endpoint. `fetch` supports GET/POST, safe headers, string or
`Uint8Array` bodies, a 30-second request timeout, and 8 MiB request/response
limits. Every redirect target must be granted; HTTPS downgrade is refused, as
are cross-origin POST replays and cross-origin redirects carrying Authorization
or any caller-supplied header.

Import `WebSocket` from `websocket`; `WebSocket.connect(url, { headers })` resolves after the handshake and returns
async `read`, `write`, and `close` methods for text and binary messages. Frames
and messages are limited to 8 MiB. Connect/handshake and writes have 30-second
transport deadlines. A pending `read()` has no public timeout and waits for a
message, close, or error; only one read may be outstanding, while writes and
close are still serviced as it waits. Credential and handshake-control headers
are refused. Each socket has an 8-command queue shared by reads, writes, and
close; a new operation rejects when that queue is full.

`fs/promises` exposes the promise-only filesystem subset; there is deliberately
no callback-style `fs` module. `readFile(path)` resolves to `Uint8Array`, while
`readFile(path, "utf8")` resolves to text. `writeFile` accepts text or bytes.
`readdir(path)` resolves to sorted names; pass `{ withFileTypes: true }` for
standard-shaped `Dirent` values with `isDirectory()`. The remaining calls are
`exists`, `unlink`, `rmdir`, and `mkdir`. Capability
checks happen at the call site, then filesystem work runs off the UI/VM thread.
There are no synchronous filesystem calls. `writeFile` is capped at 8 MiB;
`readdir` at 10,000 entries or 1 MiB of UTF-8 name bytes.

Resource ceilings are per boundary: a JavaScript module is at most 8 MiB; an
asset is at most 16 MiB, and asset listing stops at 10,000 entries or 1 MiB of
names. A runtime may have 1,024 outstanding host tasks. `localStorage` is capped at
8 MiB total, 4,096 keys, 1 MiB per value, and 1,024 pending `flush()`
waiters. Plugin unload cancels every task carrying that plugin's `Policy`, even
owner-less work. `process.run` starts its child with a cleared environment, so
host environment variables are not inherited.

## The Engine Seam

The scripting engine sits behind one internal interface,
[`src/engine/mod.rs`](src/engine/mod.rs). Everything above it — the spec arena,
the materializer, the call scope, the style table, the theme, the capability
model — is engine independent, and only the engine module knows what a script
value is.

QuickJS is what ships, via `rquickjs`, and is the only engine today. JavaScript
is the choice because application code reads better in it and the language is
more widely known.

**Call it dependency isolation, not a replaceable-engine contract.**
`ShellRuntime` and the two handle types are re-exports of QuickJS types, not
associated types behind a trait, so a second engine would be a port rather than
an implementation of something already written down.

What the isolation buys is still worth having: no module above `engine/` names a
script value, host configuration cannot be silently dropped by an engine that
does not implement it, and `build_snapshot` is the single enforcement point for
the rule that a repaint never enters the VM. Turning it into an actual contract —
an internal trait with opaque handles, and a fake engine to compile it against —
is worth doing when there is a second engine to write, and is make-work before
that.

## Not Here Yet

Present today: the element and style surface, state styles (`hover` / `active` /
`focus`), `Button`, `Checkbox`, `Switch`, retained `InputState` with input
events, icons through `svg()`, dialogs, sheets and toasts on `window`, promises and
timers, `fs` / `localStorage` / clipboard / `process` behind capabilities,
capability-gated HTTP and text/binary WebSocket clients, native target-value
transitions and springs, hot reload, `check`, and generated TypeScript
declarations.

Deliberately absent:

- `gpui.open_window` and multi-window applications; the host opens the window.
- Select, combobox, tabs, list, table and tree bindings.
- Charts, the code editor and its LSP surface, and WebView — these stay in Rust
  on purpose; binding a trait-and-generics interface across a language boundary
  costs more than it returns.
- Packaging and installing an application as a distributable archive.

The design, what is implemented, and what is not are in
[`docs/gpui-shell.md`](../../docs/gpui-shell.md).

## Types for the Script

`import ... from "gpui-kit"` is opaque without declarations, and the style surface
is far too large to memorize. **There is nothing to run.** Every `gpui-shell`
invocation — running an application, `check`, `types` — writes `gpui-kit.d.ts` into
each directory that imports a built-in module, from the runtime it is about to
use:

```bash
cargo run -p gpui-shell -- path/to/app           # runs it, and writes them
cargo run -p gpui-shell -- check path/to/app     # checks it, and writes them
cargo run -p gpui-shell -- types path/to/app     # writes them and nothing else
```

One file, built-in modules for each runtime layer: `"gpui-kit"` for GPUI's own elements and what the runtime
adds, `"gpui-base"` for gpui-base's layout helpers, components and theme, and
`"gpui-fps"` for its performance overlay. `"gpui"` is a compatibility alias for
`"gpui-kit"`, so `import { div } from "gpui"` uses the same binding.

Add `gpui-kit.d.ts` to `.gitignore`; the file's own first line says so.

### Dependencies an editor can see

The declarations describe the runtime. A Git dependency the manifest declares is
the rest of what a script imports, and an editor answers `import { style } from
"omarchy-ui"` by walking `node_modules` up from the importing file — it knows
nothing about `gpui-shell.json`. Left alone, a correct import is underlined as a
module it cannot find, and every name behind it has no type, no parameter hints
and no documentation.

So the same invocations link each materialized checkout into the application's
`node_modules` under the name the manifest gave it, and scaffold a
`jsconfig.json` when the directory has neither that nor a `tsconfig.json`. The
editor reads the same files the runtime is about to execute, so a package's own
JSDoc is what it shows and cannot drift from what runs.

Only entries gpui-shell wrote are ever replaced or removed — a symlink into its
dependency cache, or a directory carrying its marker file — so an installed
package of the same name is left alone, and a link whose dependency the manifest
no longer declares goes away. Where the platform refuses a symlink, such as an
unprivileged Windows process, a small package that re-exports the checkout is
written instead: a bare import types the same way, and a package-subpath import
stays unresolved.

The directory is called `node_modules` because that is the one place every
editor looks — no package manager is involved and nothing comes from a
registry. It also buys quiet: TypeScript treats what it resolves there as an
external library, so a dependency's own implicit-`any` diagnostics stay out of
your build. Ignore it, the way `gpui-kit.d.ts` is ignored.

The style methods, their argument types and the colour-token union are generated
from the tables the runtime dispatches through, so a name that type-checks is a
name the dispatcher accepts.

HostModule registrations are generated too, one `declare module` per registered name, so
`import { quotes } from "market"` is checked the way a built-in import is. A
module describes its own TypeScript face in Rust, beside the registration:

```rust
module.declarations(r#"
    export interface Quote { symbol: string; watched: boolean }
    export function quotes(): Quote[];
    export function watch(symbol: string): boolean;
"#);
```

`export_module` compares that against what was actually registered and refuses
a mismatch, so renaming a function on one side is a sentence at start-up rather
than an editor completing something that is gone. Declaring nothing is allowed
and yields `(...args: any[]) => any` signatures, which still check the module
name and every export name.

`crates/story/js/quotes/` has a `jsconfig.json` that turns on checking, and is
the shape to copy.

### Keeping it current

`gpui-kit.d.ts` is an **output**, not a source, and a stale one is worse than none:
it completes methods that no longer exist and refuses ones that do, and nothing
about editing against it feels wrong until the script runs. So it is not
something to write down and remember — it is rewritten by whatever is about to
run the script.

| Situation | What keeps it current |
| --- | --- |
| The `gpui-shell` binary | Every run, `check` and `types` refreshes every directory that imports the module. Nothing to remember. |
| An application embedded in a host | `ShellRuntime::load` refreshes declarations while loading the application. Nothing else to call. |

Nothing is written when the file already matches, so an editor watching the
directory is not woken on every launch and a read-only checkout is not an error.
A directory that refuses the write is logged, never fatal.

Do not commit it. This repository ignores `gpui-kit.d.ts` everywhere, including
beside its own example and story scripts — a committed copy could only ever be
the stale one. What *is* committed is the part that has no machine in it: a
`jsconfig.json` that turns checking on. An application that has none is given
one on its first launch, in the shape `examples/js_todolist/jsconfig.json`
uses; from then on it belongs to whoever edits it and is never rewritten.

The header names the gpui-shell version that generated it. Application/runtime
compatibility is declared separately by `shell-version` in `gpui-shell.json`
and checked before the entry module executes.

## Embedding It

Three host-side calls carry most of the weight:

```rust,ignore
// Editing a script changes the window, with no rebuild and no button.
// An embedded host normally places this call behind #[cfg(debug_assertions)].
let root = runtime.load(&app_root, window, cx);
let watch = runtime.watch(&root, window, cx)?;
// Keep the handle for as long as the view is mounted; dropping it stops the
// watcher, so an unmounted panel does not leave one polling for it.

// The script changed something on screen? GPUI already knows. But when *Rust*
// changes state the script reads, say so — a bare notify is only a repaint.
runtime.refresh(&root, cx)?;

// What it is costing: script renders against frames, with the time each took.
let reading = runtime.read_metrics();

// HostModule closures capture host entity handles, so a host that goes away
// clears them. GPUI's leak check catches a host that forgets.
gpui_shell::clear_exported_modules();
```

## How It Works

GPUI elements are values that are consumed when used: `RenderOnce::render`
takes `self` by value, `child` takes its child by value, and a view rebuilds its
entire element tree on every redraw. A JavaScript object can therefore never
*be* an element. Instead, a script builder records its calls into an arena of
element descriptions, and Rust replays those recorded calls into real elements.

The description, though, is **not** rebuilt every redraw. It is published as a
snapshot when the script says its state moved, and every frame after that
replays the same snapshot in Rust — so a hover, a scroll, a blinking cursor or
an animation repaints without entering the VM. Script cost follows what the
application does rather than the frame rate. Elements are still single-use and
callbacks still belong to the render that registered them; what changed is how
long that render's output lives.

## Related Resources

- [GPUI Shell design document](../../docs/gpui-shell.md)
- [`gpui-base`](../base/README.md), the foundation this runtime binds
- [Architecture](../../docs/ARCHITECTURE.md) and [Styling and Motion](../../docs/STYLING-AND-MOTION.md)
- GPUI

## License

Apache-2.0. See [`../../LICENSE-APACHE`](../../LICENSE-APACHE).
