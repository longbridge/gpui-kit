---
title: Popup
description: A low-level trigger and anchored floating-content host.
order: 22
---

# Popup

`Popup` owns trigger measurement, anchor positioning, deferred rendering, and window-edge snapping. The application owns open state, content, appearance, and motion. Higher-level primitives such as Popover build on the same floating-surface ideas.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- popup
```

## Import

```rust
use gpui_kit::base::Popup;
```

## Anatomy and API

The example composes `Popup`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/popup.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/popup.rs). Native and browser previews compile this same file.

## State and events

The caller owns trigger, anchor, open state, content, and dismissal policy.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/popup.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

The caller must supply suitable menu, listbox, or dialog semantics and focus policy.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
