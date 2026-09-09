---
title: Testing
description: Test GPUI Kit applications and GPUI behavior with Rust unit tests, TestAppContext, native UI interactions, layout assertions and CI.
order: -2.3
example: false
---

# Testing

This guide covers testing GPUI Kit applications and GPUI behavior. Choose the test level from the behavior you need to verify:

- Use ordinary Rust `#[test]` for pure data transformations, validation and state transitions.
- Use `#[gpui_kit::test]` and `TestAppContext` for entities, actions, subscriptions and async tasks, creating a window when needed.
- For UI integration tests, render the production application view, dispatch events through `gpui_kit::test`, and check control state, layout and the application result.
- Use the separate offscreen renderer for pixel checks, and retain native-window and platform integration tests for those behaviors.

GPUI Kit exposes its types and `#[gpui_kit::test]` through the Kit root; applications do not need an additional GPUI dependency. In test modules, import the types you use explicitly: `use gpui_kit::*;` also imports the GPUI `test` macro and can shadow Rust’s ordinary `#[test]`. The complete example below uses explicit imports.

## What is a UI integration test?

A **UI integration test** renders real components or an application view in a
headless window, simulates clicks, keyboard input and scrolling, then verifies
state, focus, layout and application callbacks. For example, a Checkbox test can
verify that clicking changes the owner's value and that a disabled Checkbox
rejects the same interaction.

`#[gpui_kit::test]` runs the test and provides its GPUI context.
`gpui_kit::test` supplies the tools to operate and inspect the UI:

```rust
use gpui_kit::{TestAppContext, Window};
use gpui_kit::test::TestWindowExt;
```

Use these tests when a behavior depends on components working together, such as
entering a value, saving a dialog and checking the result in the parent view.
Find controls by `ElementId`, dispatch real GPUI events and assert the outcome
with ordinary Rust assertions.

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
| Command | Native option selected state, root focus scope and row bounds |
| Combobox | Native expanded state and focus scope; selection verified through events and retained state |
| Select | Accessibility value (including title prefix), expanded, focus scope |
| ListItem / SidebarMenuItem | Geometry; additional state only when provided by native accessibility properties |
| Accordion | Expanded trigger; header and panel bounds |
| Tree | Native tree/item roles, label, selected and expanded; root focus scope |
| Table / DataTable | Native table parts; DataTable row selection and root focus scope |
| DatePicker / Calendar | DatePicker displayed date value, expanded and focus scope; calendar item labels and bounds |
| Slider | Track and thumb bounds; numeric accessibility values are not exposed by `ElementSnapshot::value()` |
| Stepper | Step and trigger bounds; verify the resulting application content |
| Dialog / Sheet | Host focus scope and surface bounds; child controls retain their own properties |
| Menu | Item label and selection, menu focus scope and submenu bounds |
| Notification | Alert role and bounds; close button uses normal Button observation |
| Dock | Area/group/content bounds and focus scopes; tabs retain native selection |

Use constructor IDs where available. Input and Select accept `.id("name")`;
their defaults include the state entity ID. Tabs inside a TabBar use their
index as ID. Select's existing `"input"` child identifies its trigger:
`window.within("language").click("input", cx)`.

Native divs opt in without supplying a second description of their state:

```rust
use gpui_kit::TestSupportExt as _;

let target = div().id("details").test_support().child(content);
```

`TestSupportExt` is available without `test-support`; in normal builds `.test_support()`
returns the original native element with its exact type. With the feature enabled,
it preserves identity, layout, events and accessibility, without adding a layout
container. Repeated observation keeps one registration. Call `.test_support()` before
`.track_focus(&handle)` so the wrapper sees the actual binding. Kit controls do this
internally. `focused()` checks whether that focus scope contains keyboard focus,
including the nested editor inside an Input frame. If GPUI advertises focus support
but the binding was not observed, `focused()` panics with a diagnostic instead of
silently returning `None`. This catches `.track_focus(&handle).test_support()`;
implicit `.focusable()` handles are also unavailable, so use an explicit handle.
This diagnostic is best effort: it relies on the native accessibility `Action::Focus`.
A custom element that omits this action can still return `None` for a missed binding.
`None` means neither a binding nor an advertised focus action was observed; it does
not prove that the element cannot receive focus. Debug output marks a detected missed
binding as `focused: <binding missed>` without panicking.

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
There is no reliable positive enabled-property assertion through this API. To verify
that a button accepts activation, exercise it and assert its intended result, for example:

```rust
window.click("save", cx);
assert_eq!(window.find("status").label(), Some("Saved: Ada"));
```

Use the actual expected application result; `assert_ne!(button.disabled(), Some(true))`
or `button.disabled().is_none()` does not establish that activation works.

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
| `window.drag_to(from_id, to_id, cx)` | Resolve both targets and drag between their centers using native hit testing. |
| `window.drag(from, to, cx)` | Left-button drag between window-local points, through GPUI drag creation and drop hit testing. |
| `window.press("backspace", cx)` | Named key or shortcut using GPUI's keystroke parser. |
| `window.input(text, cx)` | Per-character text input to the current focus; does not focus or replace the whole value. |

Scoped queries support `find`, `try_find`, nested `within`, `click`, `click_at`,
`right_click`, `double_click`, `hover`, `scroll`, `drag_to`, `press` and `input`.
`drag_to` resolves both IDs within the scope. For cross-scope drags or custom offsets,
query the targets and pass window-local points to `window.drag`.

```rust
let mut dialog = window.within("dialog");
dialog.click("name", cx);
dialog.input("Ada", cx);
dialog.press("backspace", cx);
dialog.hover("help", cx);
```

Scoped keyboard operations do not move focus. They require an observed focus binding
inside the scope; otherwise they panic before dispatch. `input` checks before every
character, so a handler moving focus outside the scope cannot redirect the remaining
text. Use `window.press` for deliberate window-wide shortcuts.

For custom input controls, register the actual focus-bearing element with
`.id("editor").test_support().track_focus(&focus_handle)`, using its real focus handle.
An unobserved input, or an observed outer container without a tracked handle, cannot
satisfy this check even if keyboard focus is physically inside the scope. Window-level
`input` and `press` dispatch to the current focus without this scope guarantee.
Scoped input shares the window input loop: one initial refresh, then one refresh per
character, with scope checks against each completed frame.

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

GPUI `dispatch_action` queues work. Complete the dispatch (for example by leaving
`update_window` and running `cx.run_until_parked()`) before editing values that the
action will read. Use `wait_for` for the resulting state or timer completion.
Legacy non-synced GPUI `Animation` uses wall-clock `Instant`; advancing the test clock
does not finish it. The Sheet/Notification geometry tests wait their actual entrance
durations before asserting final bounds. Base motion can instead honor the public
`cx.set_reduce_motion(true)` preference when testing final disclosure geometry.

Snapshots never update in place. Cached views keep their painted facts until
invalidated. Unmounted targets disappear after the frame releasing their
element state; virtualized rows become queryable when painted after scrolling.

## Coverage and failure cases

The repository covers the following component workflows through real input, native
properties and resolved bounds. These are concrete regression contracts, not a claim
that every option or combination of every component has been exhaustively tested.

| Suite | Behavior exercised |
| --- | --- |
| `test_macro.rs` | Published `#[gpui_kit::test]` sync/async compatibility alongside ordinary Rust tests; the independent Kit-only recipes package runs the same contract |
| `search.rs` | Command disabled-item skipping, wraparound, Unicode keywords, empty results, Action dispatch and original-index callbacks, two-stage Escape; Combobox search, single/multi selection, clearing, empty-result recovery, disabled behavior and exactly one Confirm on close |
| `disclosure.rs` | Accordion exclusive expansion/collapse and actual panel geometry; Stepper content navigation; disabled disclosure/steps; Slider track click, thumb drag and disabled behavior |
| `collections.rs` | Tree pointer expansion, keyboard collapse/expansion and selection; DataTable row selection, keyboard virtualization and wheel scrolling |
| `date_picker.rs` | Opening, exact preset/day selection, month navigation, clearing, Escape and disabled behavior |
| `overlays.rs` | Dialog validation → scoped Input → save → Notification; hover-revealed close; auto-dismiss timer; Dialog/Sheet Escape and focus restoration; surface bounds |
| `menu.rs` | Disabled items, keyboard confirmation, Escape, focus restoration, submenu hover and nested item activation |
| `dock.rs` | Tab selection/reordering, cross-group drag/drop, zoom and restored split geometry |

The existing form, Select, HoverCard, virtual-list, pointer, lifecycle and isolation
suites remain in place. Pure presentation components need geometry or pixel assertions,
not invented interaction state. Custom parts register their existing native elements;
unsupported properties remain unavailable, with no manual test-only override.

Views that open dialogs, sheets or notifications through `WindowExt` must render the
corresponding `Root::render_dialog_layer`, `Root::render_sheet_layer` and
`Root::render_notification_layer` children, just as the production application does.
Constructing `Root` alone does not mount those overlay layers.

Use `within` for repeated controls. A Sheet's `"sheet"` host scope contains its
`"sheet-content"` surface; Dialog's `"dialog"` scope contains the layer-indexed surface.
Nested menus also contain a `"popup-menu"`, so retain the resolved parent scope when
opening a submenu, or query under `"submenu"`. Do not assume a previously unique ID
remains unique after another layer opens.

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

The target uses `test = false`, so the default Cargo command does not select it.
The macOS CI job explicitly runs `--test rendering` as a required step, alongside
the portable interaction suite. Linux and Windows run only the portable suite. Cargo supports this
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

The Kit repository runs the interaction/layout suite on macOS, Linux and Windows.
The macOS job additionally runs the two Metal pixel checks; a failure fails the job.
A minimal macOS workflow for a Kit checkout is:

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
      - run: cargo test -p gpui-kit --features test-support --test rendering --locked
```

For an application repository, install its platform dependencies and run
`cargo test --test ui --locked` in its test package instead. Make the pinned
Kit source available at the paths declared in its manifest. Add Linux and
Windows jobs using the same system setup as your normal native builds.

The repository suite also covers read-only/disabled inputs, focus changes,
cached views, mount/unmount, window isolation, native hit testing, and cleanup
when a 1,000-element list shrinks. That large-list case checks correctness;
it is not a rendering performance benchmark.
