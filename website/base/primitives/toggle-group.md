---
title: Toggle Group
description: Coordinates a set of toggle controls as a single- or multiple-selection group.
order: 33
---

# Toggle Group

Coordinates a set of toggle controls as a single- or multiple-selection group.

Like every `gpui-base` primitive, Toggle Group supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- toggle-group
```

## Import

```rust
use gpui_kit::base::{Toggle, ToggleGroup};
```

## Anatomy and API

The example composes `Toggle`, `ToggleGroup`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/toggle_group.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/toggle_group.rs). Native and browser previews compile this same file.

## State and events

The group coordinates single or multiple selection while children reflect group state.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/toggle_group.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Label the group, expose each selection state, and keep focus order predictable.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
