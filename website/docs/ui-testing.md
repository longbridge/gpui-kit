---
title: UI Automation Testing
description: Write complete headless UI tests for GPUI Kit applications, from dependency setup and real input events to layout assertions and CI.
order: -2.3
example: false
---

# UI Automation Testing

Use `gpui_kit::test` to exercise a GPUI Kit view through real GPUI event dispatch,
then assert its native accessibility properties, layout and application result with ordinary Rust
assertions. A test creates a headless window, finds controls by `ElementId`,
clicks and enters text, and checks focus, values and layout.

This guide covers in-process behavior and layout automation. Element snapshots do not inspect pixels or launch your packaged application.
For pixel checks, use GPUI’s separate offscreen renderer as described below. Keep native-window,
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
the accessible status announcement and layout, and verifies the saved application value. The same
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

| Control | Native properties beyond geometry and visibility |
| --- | --- |
| Button | Accessibility label, focus scope |
| Input | Non-sensitive accessibility value, label, focus scope |
| Checkbox | Checked, indeterminate, label, focus scope |
| Switch / Toggle | Checked, label, focus scope |
| Radio | Checked, selected, label, focus scope |
| Tab | Selected, label |
| Select | Accessibility value (including title prefix), expanded, focus scope |
| ListItem / SidebarMenuItem | Geometry; additional state only when provided by native accessibility properties |

Use constructor IDs where available. Input and Select accept `.id("name")`;
their defaults include the state entity ID. Tabs inside a TabBar use their
index as ID. Select's existing `"input"` child identifies its trigger:
`window.within("language").click("input", cx)`.

Native divs opt in without supplying a second description of their state:

```rust
use gpui_kit::ObserveElement as _;

let target = div().id("details").observe().child(content);
```

`ObserveElement` is available without `test-support`; in normal builds `.observe()`
returns the original native element with its exact type. With the feature enabled,
it preserves identity, layout, events and accessibility, without adding a layout
container. Repeated observation keeps one registration. Call `.observe()` before
`.track_focus(&handle)` so the wrapper sees the actual binding. Kit controls do this
internally. `focused()` checks whether that focus scope contains keyboard focus,
including the nested editor inside an Input frame. It returns `None` when no
focus binding was observed.

Snapshots read native `role`, `aria_toggled`, `aria_selected`, `aria_expanded`,
`aria_label` and `aria_value`. There are no `TestProps` or hand-supplied fallback values.
Input uses its existing accessibility-value path in tests, with the same masking and
sensitive-content restrictions. Select's `value()` is its accessible value, including
any title prefix; it is not a selected item ID.

`label()` means accessibility label, not visible text. `value()` means accessibility
value, not pixels. These properties can still contain component bugs. Do not add
`aria_label` or `aria_value` solely to make a visual assertion pass. The example's
Status role and label serve the production accessibility announcement. Arbitrary
child text is not discovered automatically, and there is no `text()` shortcut that
substitutes model strings for rendered text.

`disabled()` returns `Some(true)` only when the native node exposes its disabled flag;
otherwise it returns `None`. GPUI's div API currently cannot expose a known enabled
state this way. Test disabled behavior by attempting the interaction and checking
that the application result did not change; do not interpret `None` as enabled.

IDs only need to be unique within their GPUI identity scope. Window-wide queries
panic on ambiguity. Use existing scopes without adding test containers:

```rust
window.within("toolbar").click("save", cx);
window.within("dialog").click("save", cx);
let save = window.within("dialog").within("footer").find("save");
assert!(save.visible());
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
are `role()`, `path()`, `bounds()`, `visible()`, `focused()`, `disabled()`, `label()`,
`value()`, `checked()`, `indeterminate()`, `selected()` and `expanded()`.
Focused, disabled, checked, indeterminate, selected and expanded readers return
`Option<bool>`: `None` means unavailable, not false. Label/value are also optional. Re-query after interactions:

```rust
let before = window.find("agree");
window.click("agree", cx);
assert_eq!(before.checked(), Some(false)); // The original frame.
assert_eq!(window.find("agree").checked(), Some(true)); // The new frame.
```

Assert native properties and application results together. Checking saved model
state or an emitted result is a useful part of an integration test; it should
not replace verifying the relevant visible control state.

Text input does not model complete OS IME composition. Masked inputs report
no value; verify sensitive results through application state.

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
an existing native element in a custom view when necessary. Unsupported properties
remain unavailable; there is no manual test-only override.

Missing or invisible click targets panic. Disabled controls receive real events
and decide whether to respond. Visibility combines geometry, viewport/content
clipping and the target's computed style; it does not detect pixel occlusion.
Overlays can intercept clicks. `click_at(id, point(px(10.), px(10.)), cx)` can
choose a visible portion of a clipped target without bypassing hit testing.

Test instrumentation is feature-gated, so the test build is not byte-identical
to a production build. The transparent wrapper adds no layout box, but visibility inspection
computes style an additional time; style/drag predicates must not rely on call
counts. GPUI does not expose inherited paint opacity from an unobserved ancestor.
No GPUI fork or Cargo patch is used to bypass these limitations.

On failure, check the reported paths, observation, completed frame, keyboard
focus, clipping/overlays and asynchronous completion, in that order as relevant.

## Verify rendering independently

A correct value or checked flag does not prove the control was drawn correctly.
GPUI exposes `HeadlessAppContext::with_platform`, `Window::render_to_image` and
`HeadlessAppContext::capture_screenshot` for real offscreen images. The currently
pinned platform crate supplies its headless renderer on macOS (Metal) only. Run
this target on a Mac with Metal available:

```sh
cargo test -p gpui-kit --features test-support --test rendering --locked
```

The target uses `test = false`, so ordinary CI runs the portable interaction suite;
select `--test rendering` explicitly on a runner with Metal. Cargo supports this
[explicit target selection](https://doc.rust-lang.org/cargo/commands/cargo-test.html#target-selection).
It also uses `harness = false` because AppKit initialization requires the main
thread; `--test-threads=1` would still run an ordinary Rust test on a worker thread.
On other platforms it explicitly reports that pixel verification is skipped.
Missing renderer support on macOS fails rather than substituting a fake image.

The tests inject two defects into real Kit controls: a missing check-mark asset
while `checked()` remains true, and transparent input text while `value()` remains
correct. Images must differ from the working control, and repeated working checkbox
renders must match. A separate native-event test disconnects a checkbox's change
handler and checks that clicking cannot fabricate a checked result.

These are sensitivity checks, not a complete golden-image suite. For application
visual regression, compare images against reviewed expectations under controlled
fonts, dimensions, theme, focus and animation state. State assertions and image
assertions detect different defects; neither establishes packaged-app or full IME
correctness. The executable rendering examples are in
[`crates/kit/tests/rendering.rs`](https://github.com/longbridge/gpui-kit/blob/testing/crates/kit/tests/rendering.rs).

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
