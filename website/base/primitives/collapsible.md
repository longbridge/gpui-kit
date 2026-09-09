---
title: Collapsible
description: A composable region that shows or hides content without prescribing its trigger styling.
order: 7
---

# Collapsible

A composable region that shows or hides content without prescribing its trigger styling.

Like every `gpui-base` primitive, Collapsible supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- collapsible
```

## Import

```rust
use gpui_kit::base::{Collapsible};
```

## Anatomy and API

The example composes `Collapsible`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/collapsible.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/collapsible.rs). Native and browser previews compile this same file.

## State and events

Pass the controlled expanded value to `open`; update it from the trigger callback.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/collapsible.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Name the trigger, expose expanded state, and remove hidden content from focus order.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
