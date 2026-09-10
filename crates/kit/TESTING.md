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
- Clicks use real hit testing. `click_at` provides a local offset for clipped targets.
- Instrumentation adds no layout container, but evaluates computed style an extra time.
  Snapshots cannot infer an unobserved ancestor's opacity or inspect pixels.
- The component suites cover disclosures, date/calendar selection, virtualized Table/Tree,
  modal forms, notifications, nested menus and Dock drag/zoom workflows. See the guide's
  per-suite contract matrix; this is not exhaustive option coverage or packaged-app automation.

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
