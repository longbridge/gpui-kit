# UI integration testing in GPUI Kit

A UI integration test renders real components in a headless window, simulates
clicks, keyboard input and scrolling, then checks state, focus, layout and owner
callbacks. `#[gpui_kit::test]` runs the test; `gpui_kit::test` supplies the tools
to operate and inspect its UI.

Enable `gpui-kit/test-support` under development dependencies and import
`gpui_kit::test::{TestWindowExt, TestAppContextExt, TestSupportExt, ElementSnapshot}`.
The implementation uses GPUI public APIs, with no fork, Cargo patch or separate crate.

Read the [testing guide](../../website/docs/test.md) for
setup, a compiled application workflow, the control coverage matrix, scoped IDs,
state assertions, mouse/keyboard/scroll/drag operations, async waits and CI.
The [Chinese guide](../../website/zh-CN/docs/test.md) covers the same API.

For a component regression example, start with the
[Input, Textarea and Editor suite](tests/input/README.md). It groups real editing
workflows by behavior and explains how to add a case without bypassing focus,
keyboard bindings or pointer dispatch. The separate `input_focus` target covers
Tab/Shift-Tab traversal, addon button focus and activation, and Textarea/Editor
body clicks. Run both from the repository root:

```sh
cargo test -p gpui-kit --features test-support --test input --test input_focus --locked
```

For a focused investigation, select a module such as
`--test input --locked -- history::`, or an exact case as shown in the suite README.
Append `-- --list` to the combined command to inspect discovery without running
cases. These are reproduction commands, not recorded test results. A passing run
establishes only the asserted workflows on that revision and platform; OS IME,
accessibility actions, system clipboard adapters and pixels need separate evidence.

For changes to Input, Textarea or Editor, run `script/test-input` from the
repository root. It combines Kit UI workflows with Base editing/IME/token
regressions and Component InputGroup tests. The [operation coverage and review
gate](tests/input/README.md#operation-coverage-and-review-gate) explains how to
use these tests when reviewing an input change and which native platform checks
remain necessary.

## Core semantics

- `find` requires a target and explains missing/ambiguous paths; `try_find` permits absence.
- `within` follows existing GPUI ID scopes, including unobserved parents. Pointer
  operations and `drag_to` resolve scoped IDs; scoped keyboard operations require
  an observed focus binding inside the scope.
- `ElementSnapshot` is immutable. Re-query after interactions; optional state means
  unreported when `None`, not false.
- `.test_support()` registers identity; native accessibility properties supply state.
  There are no test-only setters. `label` and `value` are accessibility properties,
  not rendered text. Missing state (including disabled) remains `None`. Focus queries
  diagnose native focus support without an observed binding; place `.test_support()`
  before `.track_focus(&handle)`.
- `render_frame` refreshes external changes. Synchronous interactions refresh their
  frames; deferred/async effects use `wait_for` outside a window update.
- `press` sends native key-down/key-up events for commands; Enter must not be
  emulated by injecting IME newline text. `input` supplies text and does not model
  a full OS input method.
- Clicks use real hit testing. `click_at` provides a local offset for clipped targets.
- Instrumentation adds no layout container, but evaluates computed style an extra time.
  Snapshots cannot infer an unobserved ancestor's opacity or inspect pixels.
- The component suites cover disclosures, date/calendar selection, virtualized Table/Tree,
  modal forms, notifications, nested menus and Dock drag/zoom workflows. See the guide's
  per-suite contract matrix; this is not exhaustive option coverage or packaged-app automation.

## Inspect accessibility targets and individual events

Import `gpui_kit::test::{TestQueryExt, TestEventExt}` for additional observation
and event control. `elements()` returns observed elements in the current frame;
`find_by_label(label)`, `try_find_by_label(label)` and `find_all_by_label(label)`
match exact, case-sensitive native accessibility labels. `find_all_by_role(role)` selects an
AccessKit `Role`. Results include invisible registrations, ordered by y, then x
and a path tie-breaker. Required queries panic on absence; all unique queries
panic on ambiguity. Scoped queries include strict descendants, excluding the
scope node itself. These queries also work inside `within` and do not discover
unobserved text or offscreen virtual rows. A label is an accessibility name,
not rendered text; use stable IDs when localization changes the name.

For gestures whose intermediate state matters, dispatch each step explicitly:

```rust
use gpui_kit::test::TestEventExt;

let start = window.find("handle").bounds().center();
window.pointer_down(start, MouseButton::Left, cx);
window.pointer_move(start + point(px(24.), px(0.)), Some(MouseButton::Left), cx);
// Assert the state while the pointer is still held.
window.pointer_up(start + point(px(24.), px(0.)), MouseButton::Left, cx);
```

Positions use window-local pixels. `pointer_move(position, pressed_button, cx)`
receives the held button explicitly; these helpers do not remember button state.
`pointer_down` and `pointer_up` dispatch one click event each. Ordinary click, hover, scroll, drag and individual pointer steps use
the window's current modifiers. Configurable clicks temporarily use their
explicit modifiers and restore the preceding state afterward. `change_modifiers(modifiers, cx)` changes them and
preserves caps lock; restore the previous modifiers when the gesture ends.
`key_down(key, is_held, cx)` and `key_up(key, cx)` allow separate key transitions
and auto-repeat (`is_held = true`). Key events combine modifiers written in the
key string with the window's held modifiers, without changing the retained
modifier state. Each step refreshes before dispatch and
completes a frame afterward. These key events do not insert text; use `input`
for typing. Leave the window update and wait for deferred results when needed.

## Exercise text composition through an input handler

`gpui_kit::test::TestInput` bridges an explicit GPUI `InputHandler` to a window.
For an entity implementing `EntityInputHandler`, construct the adapter with
`gpui_kit::ElementInputHandler::new(bounds, entity.clone())`, then pass it to
`TestInput::new(handler, window)`. Keep the entity owned by the
production view and use its actual input geometry. This bridge calls that
handler directly; it does not select the focused control or run an OS IME.

`commit_text(text, cx)` commits through `replace_text_in_range`.
`compose_text(replacement, text, selection, cx)` updates marked text through
`replace_and_mark_text_in_range`. Replacement and selection ranges use UTF-16
code units, not UTF-8 byte offsets; selection is relative to the composed text.
With no replacement range, the handler replaces its current marked range or
selection. Empty composition text cancels preedit according to the handler.
`marked_text_range(cx)` returns a document-relative range;
`selected_text_range(cx)` returns `Option<UTF16Selection>`, including direction;
`unmark_text(cx)` ends marking while retaining its text. Mutation helpers complete a frame after the
handler call. They can test composition transitions and commit behavior, while
platform candidate windows, focus routing and OS IME integration still need
native tests. Use `window.input` for ordinary text typed at the current focus.

## Resize, display scale and clipboard

Use GPUI's existing `TestAppContext::simulate_window_resize(handle, size)` and
`simulate_window_scale_factor_change(handle, scale)` outside a window update,
then call `render_frame` before inspecting layout. Pointer positions remain
window-local logical pixels at every display scale. Assert both preserved
editing state and resized geometry, then activate a control at its new bounds.

Exercise clipboard behavior through production shortcuts (`secondary-c`,
`secondary-x`, `secondary-v`) and inspect `cx.read_from_clipboard()` when needed.
The test platform shares its clipboard across windows; editor state should
remain window-specific. This verifies the test-platform adapter and application
bindings, not the operating system's clipboard service.

After activating an external Link, inspect `cx.opened_url()` to verify the URL
received by the test platform and assert the application's click callback.
This does not establish that a browser opened the destination. The test
platform exposes no cursor-style reader; URL and callback assertions do not
verify the pointer cursor.

## Adding test support to controls

Register the actual identified element before `.track_focus(&handle)`, using the same
handle as production keyboard behavior. An observed outer container without a tracked
handle does not make an unobserved custom input available to scoped `input` or `press`.
These helpers require observed focus inside the scope and recheck it for every character.

Missed-binding diagnostics are best effort: they depend on native accessibility
`Action::Focus`. A custom element that omits that action can silently return `None`
even when focus tracking was placed before `.test_support()`. Review the builder order
and assert both unfocused and focused snapshots when adding a control; do not treat
absence of a panic as proof of correct registration. Debug prints detected omissions
as `focused: <binding missed>` rather than `None`.

When a component stores an observed native base, forward its public `track_focus`
method to that base as well as forwarding `interactivity()`. The trait's default
setter alone bypasses observation. `native_parts_forward_their_public_focus_binding`
protects this builder path with real Table and Accordion parts.

Queued actions must finish before subsequent test edits mutate the values they read.
Legacy GPUI animations use wall time, so a test-clock wait is not an animation clock.
Use reduced motion for Base motion geometry tests, or wait the actual legacy entrance
before checking final bounds. Preserve real hit testing for hover-only close controls.

## Verification

```sh
cargo test -p gpui-kit --features test-support --locked
cargo test -p gpui-kit --no-default-features --features test-support --locked
```

Regression coverage includes real form controls and selection, immutable snapshots,
scoped duplicate IDs, native hover/right/double click, clipping-aware clicks, scrolling,
virtual row lifetimes, actual drag/drop, deferred Select confirmation, bounded async waits,
real HoverCard delayed opening/closing,
Unicode input, disabled/read-only controls, masked-value privacy, cache invalidation,
mount/remount, reordered composite IDs, multi-window/App isolation, accessibility forwarding,
and stale registration cleanup when a 1,000-element list shrinks to 10 elements.

The full command runs in the existing CI platform matrix. Large-list cases check
correctness, not rendering performance. The `rendering` target additionally checks
real Metal images on macOS, using a main-thread harness. It is opt-in (`test = false`):
run `cargo test -p gpui-kit --features test-support --test rendering --locked` on a
Metal-capable runner. The macOS CI job runs this command as a required step;
Linux and Windows run the portable interaction/layout suite. It detects missing checkbox
marks and invisible input text even when native state stays correct. Other platforms
explicitly skip this target until GPUI supplies a headless renderer; this is not a
complete golden-image suite.
