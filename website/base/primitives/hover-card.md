---
title: Hover Card
description: A delayed floating card associated with a pointer or keyboard trigger.
order: 13
---

# Hover Card

A delayed floating card associated with a pointer or keyboard trigger.

Like every `gpui-base` primitive, Hover Card supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- hover-card
```

## Import

```rust
use gpui_kit::base::{HoverCard};
```

## Anatomy and API

The example composes `HoverCard`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/hover_card.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/hover_card.rs). Native and browser previews compile this same file.

## State and events

Pointer or focus entry schedules opening and exit schedules dismissal.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/hover_card.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Expose it from keyboard focus and duplicate essential information outside hover-only content.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
