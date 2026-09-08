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

Kit Button uses its constructor ID, such as `Button::new("save")`. Give Input
an explicit ID with `Input::new(&state).id("name")`; its default ID includes the
input state's entity ID.

Native GPUI divs opt in with `.id(...).observe()`. Supply logical text with
`.text(value)` when your view knows it. The wrapper does not discover text
inside arbitrary children. Add it conditionally in production views:

```rust
let status = div().id("status").child(message.clone());
#[cfg(feature = "test-support")]
let status = status.observe().text(message.clone());
```

Here `message` is a `SharedString` and `ObserveElement` must be imported under
the same configuration. If your application uses this pattern, declare an
application `test-support` feature that forwards to `gpui-kit/test-support`
and import `gpui_kit::test::ObserveElement` in the instrumented view.
Run its tests with `--features test-support`. The application library uses its
normal `gpui-kit` dependency; it needs no separate testing dependency.

Use unique IDs for queried targets. For rows, composite IDs such as
`("row", record_id)` remain associated with the record after reordering.
Repeated IDs in separate scopes are ambiguous for a window-wide lookup and
cause a panic.

## Interact and assert

| API | Behavior |
| --- | --- |
| `window.find(id)` | Returns an owned snapshot from the last completed frame, or `None` for an unregistered target. |
| `window.click(id, cx)` | Sends mouse move, down and up at the target's center through GPUI hit testing. |
| `window.input(text, cx)` | Sends text to the current keyboard focus through simulated keystrokes. It does not focus a target or replace the whole value. |
| `cx.simulate_keystrokes(handle.into(), "backspace")` | Uses GPUI's existing API for named keys and shortcuts. |

Snapshots expose `bounds()`, `visible()`, `focused()`, `disabled()` and `text()`.
Check the facts that matter to the behavior: a save result, the focused input,
a disabled control remaining inert, or a popover positioned below its trigger.
Also assert the application state or emitted result so that a rendered label
alone cannot make a broken workflow pass.

Text input simulates per-character input. It does not model the operating
system's complete IME composition workflow. Masked inputs intentionally report
no text; verify their resulting value through application state when needed.

## Complete the frame before querying

Draw before the first query. Click and input helpers refresh and draw around
their dispatch. After a direct state change, focus change, resize or GPUI
keyboard action, explicitly refresh and draw:

```rust
cx.update_window(handle.into(), |_, window, cx| {
    window.refresh();
    window.draw(cx).clear(cx);
    assert!(window.find("name").unwrap().focused());
}).unwrap();
```

Use `TestAppContext::update_window` for these interactions. A typed
`WindowHandle::update` borrows the root entity and cannot safely redraw that
entity inside the same callback. Use typed updates to inspect or change model
state, then return before drawing through `update_window`.

For asynchronous work, drive GPUI's test executor outside the window update
with `cx.run_until_parked()`, then refresh and query. Work waiting for a timer,
network response or external service needs a controlled test clock or a fake
service response; a parked executor does not imply that all such work finished.
Avoid wall-clock sleeps as a substitute for completion.

Snapshots do not update in place. Call `find` again after the next frame.
Cached views retain their last painted facts until invalidated; refresh after
external changes. Unmounted targets disappear after the completed frame that
releases their element state.

## Visibility and failure cases

A missing or invisible click target panics with its ID. A disabled control
still receives native events and decides whether to handle them; test that it
does not change application state.

Visibility uses layout size, viewport/content clipping and the target's
computed style. An overlay may intercept a click even when the target reports
visible. The center of a partially clipped element can fall outside its
visible portion. These helpers do not bypass either case to invoke callbacks.
An unobserved ancestor's transparent paint opacity is not exposed by GPUI and
cannot be inferred by this wrapper.

When a test fails, first check:

1. The target has a stable ID and is registered through Kit instrumentation or
   `.observe()`.
2. A completed, refreshed frame reflects the state being asserted.
3. The intended input has focus, and no overlay or clipping intercepts clicks.
4. Any asynchronous dependency has completed in the test executor.

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
