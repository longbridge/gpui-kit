---
title: UI Automation Testing
description: Write complete headless UI tests for GPUI Kit applications, from dependency setup and real input events to layout assertions and CI.
order: -2.3
example: false
---

# UI Automation Testing

Use `gpui_kit::test` to exercise a GPUI Kit view through real GPUI event dispatch,
then assert its rendered state and application result with ordinary Rust
assertions. A test creates a headless window, finds controls by `ElementId`,
clicks and enters text, and checks focus, values and layout.

This guide covers in-process behavior and layout automation. The harness does
not launch your packaged application or inspect pixels. Keep native-window,
platform integration and visual checks alongside these tests when those are
part of the behavior you need to verify.

## Set up a test project

UI testing is part of `gpui-kit`, behind its `test-support` feature. The
example below uses a Kit source checkout containing these helpers. There is
no additional testing crate, GPUI fork or Cargo patch to install.

Prepare the platform dependencies described in [Installation](./installation.md).
Headless tests still compile GPUI's native dependencies. For a standalone test
project next to the checkout, use this layout:

```text
workspace/
  gpui-kit/
  ui-tests/
    Cargo.toml
    tests/ui.rs
```

Put the following in `ui-tests/Cargo.toml`:

```toml
[package]
name = "ui-tests"
version = "0.1.0"
edition = "2024"
publish = false

[dev-dependencies]
gpui-kit = { path = "../gpui-kit/crates/kit", features = ["test-support"] }
```

For an existing application, add this development dependency to its package.
Its normal `gpui-kit` dependency must resolve to the same source and version;
features then unify for tests. Keep `test-support` in development dependencies
so ordinary application builds do not enable observation. An application that
uses the component crate directly can enable `gpui-component/test-support`.

## A complete test

Copy the following into `tests/ui.rs`. The example uses GPUI Kit's facade,
initializes the component library and wraps the view in `Root`. It retains the
input state on the view, as a real application should.

The test enters a Unicode name, edits it with Backspace, clicks Save, checks
the status text and layout, and verifies the saved application value. The same
source is compiled and run in GPUI Kit's integration suite.

<<< ../../crates/kit/tests/ui.rs{rust}

In your own application, import the production view and its constructor from
your library crate. Keeping a second implementation of the view in the test
would allow the test and application to diverge. This example defines its view
inline only so the entire test can be copied into a new package.

From `ui-tests/`, run:

```sh
cargo generate-lockfile
cargo test --test ui --locked
```

Commit `Cargo.lock` with the test project. Inside the GPUI Kit checkout, run
this exact example with:

```sh
cargo test -p gpui-kit --features test-support --test ui --locked
```

## Choose stable test targets

With `test-support` enabled, these controls register their existing native element;
observation adds no layout container:

| Control | Reported state beyond geometry and visibility |
| --- | --- |
| Button | Label text, focus, disabled, selected |
| Input | Unmasked text/value, focus, disabled |
| Checkbox | Checked, indeterminate, focus, disabled |
| Switch / Toggle | Checked, focus, disabled |
| Radio | Checked, selected, focus, disabled |
| Tab | Selected, disabled |
| Select | Selected title as value, expanded, focus, disabled |
| ListItem | Selected, disabled |
| SidebarMenuItem | Label text, selected, expanded, disabled |

Use constructor IDs where available. Input and Select accept `.id("name")`;
their defaults include the state entity ID. Tabs inside a TabBar use their
index as ID. Select's existing `"input"` child identifies its trigger:
`window.within("language").click("input", cx)`.

Native divs opt in through a single fluent `test_state` closure:

```rust
use gpui_kit::TestStateExt as _;

let status = div()
    .id("status")
    .test_state(|state| state.text(message.clone()))
    .child(message);
```

Here `message` is a `SharedString`. `TestStateExt` is always available, so render
chains need no conditional compilation, temporary element or temporary focus clone.
Without `test-support`, the closure is not called, the original native element
is returned, and configuration storage is zero-sized. Put test-only conversions
inside the closure; Rust still creates and drops captured values normally.

The closure's `state` offers `text`, `focus`, `disabled`, `checked`,
`indeterminate`, `selected`, `expanded` and `value`. These report facts without
changing control behavior: `state.disabled(true)` does not disable handlers.
The name `test_state` distinguishes reporting test facts from running a test
or subscribing to reactive changes.

Observation preserves the div's identity, layout, events and accessibility;
it does not discover arbitrary child text or IDs. For geometry and visibility
alone, use `.test_state(|state| state)`.

IDs only need to be unique within their GPUI identity scope. Window-wide queries
panic on ambiguity. Use existing scopes without adding test containers:

```rust
window.within("toolbar").click("save", cx);
window.within("dialog").click("save", cx);
let save = window.within("dialog").within("footer").find("save");
assert!(!save.disabled());
```

A parent scope need not itself be observed: its ID is part of its observed
children's GPUI paths. `within` requires a unique painted path. Composite row
IDs such as `("row", record_id)` preserve record identity after reordering.

## Interact and assert

Import `gpui_kit::test::TestWindowExt` for the following methods:

| API | Behavior |
| --- | --- |
| `window.find(id)` | Requires an `ElementSnapshot` from the last completed frame; missing targets panic with registered paths and troubleshooting hints. |
| `window.try_find(id)` | Returns `None` when absent; ambiguity still panics. |
| `window.click(id, cx)` | Native mouse move/down/up at the target center. |
| `window.click_at(id, offset, cx)` | Click at a pixel offset from the target's top-left corner, useful for partial clipping. |
| `window.right_click(id, cx)` / `double_click(id, cx)` | Native right-button or two-click sequences. |
| `window.hover(id, cx)` | Move the pointer without pressing a button. |
| `window.scroll(id, delta, cx)` | Native wheel event; `ScrollDelta` retains GPUI units and sign. |
| `window.drag(from, to, cx)` | Left-button drag between window-local points, through GPUI drag creation and drop hit testing. |
| `window.press("backspace", cx)` | Named key or shortcut using GPUI's keystroke parser. |
| `window.input(text, cx)` | Per-character text input to the current focus; does not focus or replace the whole value. |

Scoped queries support `find`, `try_find`, nested `within`, `click` and `click_at`.
For a coordinate drag, query scoped targets and pass their `bounds().center()`
to `window.drag`.

`ElementSnapshot` is an owned, immutable record of a completed paint. Its readers
are `path()`, `bounds()`, `visible()`, `focused()`, `disabled()`, `text()`,
`value()`, `checked()`, `indeterminate()`, `selected()` and `expanded()`.
The last four state readers return `Option<bool>`: `None` means unreported,
not false. Text/value are also optional. Re-query after interactions:

```rust
let before = window.find("agree");
window.click("agree", cx);
assert_eq!(before.checked(), Some(false)); // The original frame.
assert_eq!(window.find("agree").checked(), Some(true)); // The new frame.
```

Assert rendered facts and application results together. Checking saved model
state or an emitted result is a useful part of an integration test; it should
not replace verifying the relevant visible control state.

Text input does not model complete OS IME composition. Masked inputs report
neither text nor value; verify sensitive results through application state.

## Complete the frame before querying

Call `window.render_frame(cx)` before the first query and after direct external
state/focus changes or resizing. Interaction helpers refresh around synchronous
dispatch, including `press`. They cannot finish deferred callbacks while the
surrounding window update is still borrowed.

```rust
cx.update_window(handle.into(), |_, window, cx| {
    window.render_frame(cx);
    window.click("name", cx);
    window.input("Ada", cx);
    window.press("backspace", cx);
    assert_eq!(window.find("name").value(), Some("Ad"));
}).unwrap();
```

Use `TestAppContext::update_window`; typed `WindowHandle::update` already borrows
the root entity and cannot safely redraw it in the same callback.

For asynchronous work or deferred selection commits, use an async
`#[gpui_kit::test]` and wait **outside** the window update:

```rust
use gpui_kit::test::TestAppContextExt;
use std::time::Duration;

cx.wait_for(handle.into(), Duration::from_millis(200), |window, _| {
    window.try_find("result").is_some_and(|snapshot| snapshot.visible())
}).await;
```

`wait_for` refreshes frames and polls every 10 ms using GPUI's test executor
clock, with registered paths in timeout errors. This is a bounded condition
wait, not an OS event loop or a network-service simulator. Provide controlled
responses for external dependencies. A parked executor alone does not imply
that timers or deferred work have completed.

Snapshots never update in place. Cached views keep their painted facts until
invalidated. Unmounted targets disappear after the frame releasing their
element state; virtualized rows become queryable when painted after scrolling.

## Coverage and failure cases

This is an incrementally growing headless interaction API, not automatic
coverage of every Kit component. Table, Menu, Dialog and Dock do not yet have
comprehensive automatic semantic observation or dedicated end-to-end workflows.
Native scrolling and drag/drop primitives are tested, including a real virtual
list and delayed HoverCard opening/closing; this does not establish every Table or Dock behavior. Add observation to
an existing native element in a custom view when necessary, and supply actual
state rather than a test-only constant.

Missing or invisible click targets panic. Disabled controls receive real events
and decide whether to respond. Visibility combines geometry, viewport/content
clipping and the target's computed style; it does not detect pixel occlusion.
Overlays can intercept clicks. `click_at(id, point(px(10.), px(10.)), cx)` can
choose a visible portion of a clipped target without bypassing hit testing.

Observation is feature-gated and therefore not byte-identical to a production
build. The transparent wrapper adds no layout box, but visibility inspection
computes style an additional time; style/drag predicates must not rely on call
counts. GPUI does not expose inherited paint opacity from an unobserved ancestor.
No GPUI fork or Cargo patch is used to bypass these limitations.

On failure, check the reported paths, observation, completed frame, keyboard
focus, clipping/overlays and asynchronous completion, in that order as relevant.

## Run in CI

The Kit repository runs the headless suite in its macOS, Linux and Windows
matrix. A minimal macOS workflow for a Kit checkout is:

```yaml
name: UI tests
on: [push, pull_request]
jobs:
  test:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: ./script/bootstrap
      - run: cargo test -p gpui-kit --features test-support --locked
```

For an application repository, install its platform dependencies and run
`cargo test --test ui --locked` in its test package instead. Make the pinned
Kit source available at the paths declared in its manifest. Add Linux and
Windows jobs using the same system setup as your normal native builds.

The repository suite also covers read-only/disabled inputs, focus changes,
cached views, mount/unmount, window isolation, native hit testing, and cleanup
when a 1,000-element list shrinks. That large-list case checks correctness;
it is not a rendering performance benchmark.
