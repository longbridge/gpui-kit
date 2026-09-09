---
title: Tooltip
description: A delayed, positioned description associated with a trigger element.
order: 34
---

# Tooltip

A delayed, positioned description associated with a trigger element.

Like every `gpui-base` primitive, Tooltip supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- tooltip
```

## Import

```rust
use gpui_kit::base::{Tooltip};
```

## Anatomy and API

The example composes `Tooltip`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/tooltip.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/tooltip.rs). Native and browser previews compile this same file.

## State and events

Hover or focus schedules it and exit or blur dismisses it; content is descriptive.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/tooltip.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Show on focus as well as hover; tooltips supplement names and contain no required controls.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
