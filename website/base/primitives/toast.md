---
title: Toast
description: A managed, animated stack of temporary status messages.
order: 31
---

# Toast

A managed, animated stack of temporary status messages.

Like every `gpui-base` primitive, Toast supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- toast
```

## Import

```rust
use gpui_kit::base::{Toast, ToastManager, ToastOptions, ToastStack};
```

## Anatomy and API

The example composes `Toast`, `ToastManager`, `ToastOptions`, `ToastStack`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/toast.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/toast.rs). Native and browser previews compile this same file.

## State and events

Push messages through toast state; transition status retains an item during entry and exit.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/toast.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Choose live-region priority carefully and avoid essential actions only in expiring content.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
