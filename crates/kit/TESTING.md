# UI testing in GPUI Kit

Enable `gpui-kit/test-support` under development dependencies and import
`gpui_kit::test::{TestWindowExt, TestAppContextExt, TestSupportExt, ElementSnapshot}`.
The implementation uses GPUI public APIs, with no fork, Cargo patch or separate crate.

Read the [complete UI automation guide](../../website/docs/ui-testing.md) for
setup, a compiled application workflow, the control coverage matrix, scoped IDs,
state assertions, mouse/keyboard/scroll/drag operations, async waits and CI.
The [Chinese guide](../../website/zh-CN/docs/ui-testing.md) covers the same API.

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
- Controls are instrumented incrementally; this is not complete Table/Menu/Dialog/Dock
  coverage or packaged-application automation. See the guide's explicit coverage matrix.

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
